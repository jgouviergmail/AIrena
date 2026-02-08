use std::collections::HashMap;
use std::time::Duration;

use tauri::ipc::Channel;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::engine::emotion_engine::{self, EmotionContext};
use crate::engine::json_parser;
use crate::engine::memory_manager;
use crate::engine::prompt_builder;
use crate::engine::turn_manager;
use crate::models::discussion::{DiscussionConfig, DiscussionStatus};
use crate::models::engine_command::EngineCommand;
use crate::models::events::ArenaEvent;
use crate::models::gladiateur::GladIAteurState;
use crate::models::iarbitre::IArbitreState;
use crate::models::message::{Message, Reaction, ReactionType, SpeakerRole};
use crate::ollama::client::OllamaClient;
use crate::ollama::error::OllamaError;

/// Response from the combined memory update LLM call
#[derive(Debug, serde::Deserialize)]
struct MemoryUpdateResponse {
    #[serde(default)]
    summary: String,
    #[serde(default)]
    positions: HashMap<String, String>,
}

pub struct DiscussionEngine {
    discussion_id: String,
    config: DiscussionConfig,
    ollama_client: OllamaClient,
    status: DiscussionStatus,
    current_turn: u32,
    arbitre: IArbitreState,
    gladiateurs: Vec<GladIAteurState>,
    messages_history: Vec<Message>,
    turn_messages: Vec<Message>,
    /// Reaction counts per speaker for the current turn: speaker_id → (likes, dislikes)
    turn_reaction_counts: HashMap<String, (u32, u32)>,
    user_intervention_pending: bool,
    user_intervention_handled: bool,
    cancel_token: CancellationToken,
    /// Whether emotions should influence AI prompts (behavior variation)
    emotion_driven: bool,
}

impl DiscussionEngine {
    pub fn new(
        config: DiscussionConfig,
        discussion_id: String,
        ollama_url: &str,
        ollama_model: &str,
    ) -> Self {
        let ollama_client = OllamaClient::new(ollama_url, ollama_model);
        let arbitre = IArbitreState::new(config.arbitre.clone());
        let gladiateurs = config
            .gladiateurs
            .iter()
            .map(|g| GladIAteurState::new(g.clone()))
            .collect();

        Self {
            discussion_id,
            config,
            ollama_client,
            status: DiscussionStatus::Active,
            current_turn: 0,
            arbitre,
            gladiateurs,
            messages_history: Vec::new(),
            turn_messages: Vec::new(),
            turn_reaction_counts: HashMap::new(),
            user_intervention_pending: false,
            user_intervention_handled: false,
            cancel_token: CancellationToken::new(),
            emotion_driven: false,
        }
    }

    pub fn set_cancel_token(&mut self, token: CancellationToken) {
        self.cancel_token = token;
    }

    pub fn set_emotion_driven(&mut self, enabled: bool) {
        self.emotion_driven = enabled;
    }

    /// Localized message when a speaker's LLM call fails
    fn speaker_difficulty_msg(&self, speaker_name: &str) -> String {
        match self.config.discussion_language.as_str() {
            "en" => format!("[{} seems to be having difficulties]", speaker_name),
            "zh" => format!("[{} 似乎遇到了困难]", speaker_name),
            _ => format!("[{} semble avoir des difficultés]", speaker_name),
        }
    }

    /// Localized message when a speaker is banned
    fn ban_notification_msg(&self, speaker_name: &str, duration: u32, reason: &str) -> String {
        match self.config.discussion_language.as_str() {
            "en" => format!("{} is banned for {} turn(s): {}", speaker_name, duration, reason),
            "zh" => format!("{} 被禁言 {} 回合：{}", speaker_name, duration, reason),
            _ => format!("{} est banni(e) pour {} tour(s) : {}", speaker_name, duration, reason),
        }
    }

    /// Main orchestration loop
    pub async fn run(
        mut self,
        mut cmd_rx: mpsc::Receiver<EngineCommand>,
        channel: Channel<ArenaEvent>,
    ) {
        tracing::info!("Discussion engine started: {}", self.discussion_id);

        // Validate config before starting
        if self.gladiateurs.is_empty() {
            let _ = channel.send(ArenaEvent::Error {
                message: "At least one gladiator is required.".to_string(),
            });
            let _ = channel.send(ArenaEvent::DiscussionEnded);
            return;
        }

        let _ = channel.send(ArenaEvent::DiscussionStarted {
            discussion_id: self.discussion_id.clone(),
        });

        // --- INTRODUCTION ---
        let participant_names: Vec<String> = self
            .gladiateurs
            .iter()
            .map(|g| g.config.name.clone())
            .collect();
        let intro_prompt = prompt_builder::build_introduction_prompt(
            &self.config.topic,
            &participant_names,
            &self.config.discussion_language,
        );
        let intro_request = self.ollama_client.build_request(
            &self.config.arbitre.system_prompt,
            &intro_prompt,
            &self.config.arbitre.llm_params,
            false,
        );
        let ch = channel.clone();
        let arb_id = self.arbitre.config.id.clone();
        let cancel = self.cancel_token.clone();
        match self
            .ollama_client
            .chat_streaming(
                &intro_request,
                |token| {
                    let _ = ch.send(ArenaEvent::MessageChunk {
                        speaker_id: arb_id.clone(),
                        chunk: token.to_string(),
                    });
                },
                cancel,
            )
            .await
        {
            Ok(content) => {
                let arb_id = self.arbitre.config.id.clone();
                let arb_name = self.arbitre.config.name.clone();
                let msg = self.create_message(&arb_id, &arb_name, SpeakerRole::Arbitre, &content);
                let _ = channel.send(ArenaEvent::MessageComplete {
                    message: msg.clone(),
                });
                self.messages_history.push(msg);
            }
            Err(OllamaError::Cancelled) => {
                let _ = channel.send(ArenaEvent::DiscussionEnded);
                return;
            }
            Err(e) => {
                let _ = channel.send(ArenaEvent::Error {
                    message: e.to_string(),
                });
            }
        }

        // --- MAIN LOOP ---
        loop {
            if self.should_stop() {
                break;
            }

            if self.process_commands(&mut cmd_rx, &channel).await {
                break;
            }

            self.current_turn += 1;
            self.turn_messages.clear();
            self.turn_reaction_counts.clear();

            // Determine speaker order
            let order = turn_manager::determine_speaker_order(
                &self.gladiateurs,
                &self.config.arbitre.turn_distribution,
            );

            if order.is_empty() {
                let _ = channel.send(ArenaEvent::TurnSkipped {
                    reason: "All participants are banned".to_string(),
                    next_available_turn: self.current_turn + 1,
                });
                self.current_turn -= 1;
                turn_manager::decrement_bans(&mut self.gladiateurs);
                continue;
            }

            let speaker_names: Vec<String> = order
                .iter()
                .map(|idx| self.gladiateurs[*idx].config.name.clone())
                .collect();
            tracing::info!(
                discussion_id = %self.discussion_id,
                turn = self.current_turn,
                speakers = ?speaker_names,
                "Turn started"
            );
            let speaker_ids: Vec<String> = order
                .iter()
                .map(|idx| self.gladiateurs[*idx].config.id.clone())
                .collect();
            let _ = channel.send(ArenaEvent::TurnStarted {
                turn_number: self.current_turn,
                speaker_order: speaker_ids,
            });

            // Handle pending user intervention at start of turn
            if self.user_intervention_pending && !self.user_intervention_handled {
                self.handle_user_intervention(&mut cmd_rx, &channel).await;
            }

            // FOR EACH ACTIVE GLADIATEUR
            let mut broke_early = false;
            for &glad_idx in &order {
                if self.process_commands(&mut cmd_rx, &channel).await {
                    broke_early = true;
                    break;
                }
                if self.should_stop() {
                    broke_early = true;
                    break;
                }

                let speaker_id = self.gladiateurs[glad_idx].config.id.clone();
                let speaker_name = self.gladiateurs[glad_idx].config.name.clone();

                let _ = channel.send(ArenaEvent::SpeakerActive {
                    speaker_id: speaker_id.clone(),
                });

                // C.2 REACTIONS (turn > 1)
                if self.current_turn > 1 {
                    self.process_reactions(glad_idx, &channel).await;
                }

                // C.3 INNER THOUGHT
                let thought = self.process_thought(glad_idx, &channel).await;

                // C.4 PUBLIC INTERVENTION
                let content = self
                    .process_intervention(glad_idx, thought.as_deref(), &channel)
                    .await;

                if let Some(text) = &content {
                    let mut msg = self.create_message(
                        &speaker_id,
                        &speaker_name,
                        SpeakerRole::Gladiateur,
                        text,
                    );
                    msg.inner_thought = thought.clone();
                    let _ = channel.send(ArenaEvent::MessageComplete {
                        message: msg.clone(),
                    });
                    self.turn_messages.push(msg.clone());
                    self.messages_history.push(msg);
                }

                // C.5 EMOTION UPDATE (rule-based, instant)
                self.update_emotions(glad_idx, &channel);

                // C.6 MODERATION
                if let Some(text) = &content {
                    self.process_moderation(glad_idx, text, &channel).await;
                }
            }

            if broke_early || self.should_stop() {
                break;
            }

            // D. Obligatory user intervention if still pending
            if self.user_intervention_pending && !self.user_intervention_handled {
                self.handle_user_intervention(&mut cmd_rx, &channel).await;
            }

            // E. END OF TURN — memory update
            self.update_memory_all().await;

            // Decrement bans
            let unbanned = turn_manager::decrement_bans(&mut self.gladiateurs);
            for (id, name) in unbanned {
                let _ = channel.send(ArenaEvent::BanLifted {
                    speaker_id: id,
                    speaker_name: name,
                });
            }

            self.user_intervention_handled = false;
        }

        // --- SYNTHESIS (unless force-stopped) ---
        if self.status != DiscussionStatus::ForceStopRequested {
            self.generate_synthesis(&channel).await;
        }

        let _ = channel.send(ArenaEvent::DiscussionEnded);
        tracing::info!("Discussion engine ended: {}", self.discussion_id);
    }

    // ===== Helpers =====

    fn should_stop(&self) -> bool {
        matches!(
            self.status,
            DiscussionStatus::StopRequested
                | DiscussionStatus::ForceStopRequested
                | DiscussionStatus::Completed
        ) || self
            .config
            .max_turns
            .map_or(false, |max| self.current_turn >= max)
    }

    /// Process pending commands. Returns true if engine should stop.
    async fn process_commands(
        &mut self,
        cmd_rx: &mut mpsc::Receiver<EngineCommand>,
        channel: &Channel<ArenaEvent>,
    ) -> bool {
        // Drain non-blocking
        loop {
            match cmd_rx.try_recv() {
                Ok(cmd) => match cmd {
                    EngineCommand::Pause => {
                        self.status = DiscussionStatus::Paused;
                        let _ = channel.send(ArenaEvent::PauseConfirmed);
                    }
                    EngineCommand::Resume => {
                        self.status = DiscussionStatus::Active;
                        let _ = channel.send(ArenaEvent::ResumeConfirmed);
                    }
                    EngineCommand::Stop => {
                        self.status = DiscussionStatus::StopRequested;
                        return true;
                    }
                    EngineCommand::ForceStop => {
                        self.status = DiscussionStatus::ForceStopRequested;
                        return true;
                    }
                    EngineCommand::UserWantsToIntervene => {
                        self.user_intervention_pending = true;
                    }
                    _ => {} // SubmitUserMessage/SkipUserTurn handled elsewhere
                },
                Err(_) => break,
            }
        }

        // Block if paused
        if self.status == DiscussionStatus::Paused {
            loop {
                match cmd_rx.recv().await {
                    Some(EngineCommand::Resume) => {
                        self.status = DiscussionStatus::Active;
                        let _ = channel.send(ArenaEvent::ResumeConfirmed);
                        break;
                    }
                    Some(EngineCommand::Stop) => {
                        self.status = DiscussionStatus::StopRequested;
                        return true;
                    }
                    Some(EngineCommand::ForceStop) => {
                        self.status = DiscussionStatus::ForceStopRequested;
                        return true;
                    }
                    Some(_) => {}
                    None => return true,
                }
            }
        }
        false
    }

    async fn process_reactions(&mut self, glad_idx: usize, channel: &Channel<ArenaEvent>) {
        let speaker_name = self.gladiateurs[glad_idx].config.name.clone();
        let prev_msgs: Vec<&Message> = self
            .messages_history
            .iter()
            .filter(|m| {
                m.turn_number == self.current_turn - 1
                    && m.speaker_name != speaker_name
                    && m.role != SpeakerRole::Arbitre
            })
            .collect();

        if prev_msgs.is_empty() {
            return;
        }

        let known: Vec<String> = self.gladiateurs.iter().map(|g| g.config.name.clone()).collect();
        let prev_interventions: Vec<(String, String)> = prev_msgs
            .iter()
            .map(|m| (m.speaker_name.clone(), m.content.clone()))
            .collect();
        let prompt =
            prompt_builder::build_reaction_prompt(&prev_interventions, &self.config.discussion_language);
        let request = self.ollama_client.build_request(
            &self.gladiateurs[glad_idx].config.system_prompt,
            &prompt,
            &self.gladiateurs[glad_idx].config.llm_params,
            true,
        );

        let cancel = self.cancel_token.clone();
        if let Ok(raw) = self.ollama_client.chat(&request, cancel).await {
            let reactions = json_parser::parse_reactions(&raw, &known);
            for parsed in reactions {
                if let Some(target) = prev_msgs
                    .iter()
                    .find(|m| m.speaker_name.to_lowercase().trim() == parsed.speaker_name.to_lowercase().trim())
                {
                    // Track reaction counts for the emotion system
                    let target_speaker_id = target.speaker_id.clone();
                    let entry = self.turn_reaction_counts
                        .entry(target_speaker_id)
                        .or_insert((0, 0));
                    match &parsed.reaction_type {
                        ReactionType::Like => entry.0 += 1,
                        ReactionType::Dislike => entry.1 += 1,
                    }

                    let _ = channel.send(ArenaEvent::ReactionEmitted {
                        message_id: target.id.clone(),
                        reaction: Reaction {
                            from_speaker_id: self.gladiateurs[glad_idx].config.id.clone(),
                            from_speaker_name: self.gladiateurs[glad_idx].config.name.clone(),
                            reaction_type: parsed.reaction_type,
                            target_message_id: target.id.clone(),
                        },
                    });
                }
            }
        }
    }

    async fn process_thought(
        &self,
        glad_idx: usize,
        channel: &Channel<ArenaEvent>,
    ) -> Option<String> {
        let has_prior_context = !self.turn_messages.is_empty()
            || !self.gladiateurs[glad_idx].memory.immediate.is_empty();
        let prompt = prompt_builder::build_thought_prompt(
            &self.gladiateurs[glad_idx].emotions,
            &self.config.discussion_language,
            has_prior_context,
            self.emotion_driven,
        );
        let request = self.ollama_client.build_request(
            &self.gladiateurs[glad_idx].config.system_prompt,
            &prompt,
            &self.gladiateurs[glad_idx].config.llm_params,
            false,
        );

        let speaker_id = self.gladiateurs[glad_idx].config.id.clone();
        let ch = channel.clone();
        let sid = speaker_id.clone();
        let cancel = self.cancel_token.clone();
        match self
            .ollama_client
            .chat_streaming(
                &request,
                move |token| {
                    let _ = ch.send(ArenaEvent::ThoughtChunk {
                        speaker_id: sid.clone(),
                        chunk: token.to_string(),
                    });
                },
                cancel,
            )
            .await
        {
            Ok(thought) if !thought.is_empty() => {
                let _ = channel.send(ArenaEvent::ThoughtComplete {
                    speaker_id,
                    thought: thought.clone(),
                });
                Some(thought)
            }
            _ => None,
        }
    }

    async fn process_intervention(
        &self,
        glad_idx: usize,
        thought: Option<&str>,
        channel: &Channel<ArenaEvent>,
    ) -> Option<String> {
        let (sys, usr) = prompt_builder::build_intervention_prompt(
            &self.gladiateurs[glad_idx].config.system_prompt,
            &self.config.topic,
            &self.gladiateurs[glad_idx].memory,
            &self.turn_messages,
            thought,
            &self.gladiateurs[glad_idx].emotions,
            &self.config.discussion_language,
            &self.config.user_name,
            self.emotion_driven,
        );

        let request = self.ollama_client.build_request(
            &sys,
            &usr,
            &self.gladiateurs[glad_idx].config.llm_params,
            false,
        );

        let speaker_id = self.gladiateurs[glad_idx].config.id.clone();
        let speaker_name = self.gladiateurs[glad_idx].config.name.clone();
        let ch = channel.clone();
        let sid = speaker_id.clone();
        let cancel = self.cancel_token.clone();

        match self
            .ollama_client
            .chat_streaming(
                &request,
                move |token| {
                    let _ = ch.send(ArenaEvent::MessageChunk {
                        speaker_id: sid.clone(),
                        chunk: token.to_string(),
                    });
                },
                cancel.clone(),
            )
            .await
        {
            Ok(c) if !c.is_empty() => {
                tracing::info!(
                    discussion_id = %self.discussion_id,
                    turn = self.current_turn,
                    speaker = %speaker_name,
                    len = c.len(),
                    "Intervention completed"
                );
                Some(c)
            }
            Ok(_) => {
                tracing::warn!(
                    discussion_id = %self.discussion_id,
                    turn = self.current_turn,
                    speaker = %speaker_name,
                    system_prompt_len = sys.len(),
                    user_prompt_len = usr.len(),
                    "Intervention returned empty — retrying with higher temperature"
                );
                tracing::debug!(
                    speaker = %speaker_name,
                    system_prompt = %sys,
                    user_prompt = %usr,
                    "Prompt that produced empty response"
                );
                // Retry with higher temp
                let mut params = self.gladiateurs[glad_idx].config.llm_params.clone();
                params.temperature = (params.temperature + 0.2).min(2.0);
                let retry = self.ollama_client.build_request(&sys, &usr, &params, false);
                match self.ollama_client.chat(&retry, cancel).await {
                    Ok(c) if !c.is_empty() => {
                        tracing::info!(speaker = %speaker_name, "Retry succeeded");
                        Some(c)
                    }
                    Ok(_) => {
                        tracing::error!(speaker = %speaker_name, "Retry also returned empty");
                        let _ = channel.send(ArenaEvent::Error {
                            message: self.speaker_difficulty_msg(&speaker_name),
                        });
                        None
                    }
                    Err(e) => {
                        tracing::error!(speaker = %speaker_name, error = %e, "Retry failed");
                        let _ = channel.send(ArenaEvent::Error {
                            message: self.speaker_difficulty_msg(&speaker_name),
                        });
                        None
                    }
                }
            }
            Err(OllamaError::Cancelled) => None,
            Err(e) => {
                tracing::error!(
                    discussion_id = %self.discussion_id,
                    turn = self.current_turn,
                    speaker = %speaker_name,
                    error = %e,
                    "Intervention failed"
                );
                let _ = channel.send(ArenaEvent::Error {
                    message: self.speaker_difficulty_msg(&speaker_name),
                });
                None
            }
        }
    }

    fn update_emotions(&mut self, glad_idx: usize, channel: &Channel<ArenaEvent>) {
        let sid = self.gladiateurs[glad_idx].config.id.clone();

        // Get reaction counts from the turn_reaction_counts map
        // Reactions are about PREVIOUS turn's messages, tracked during process_reactions
        let (likes, dislikes) = self.turn_reaction_counts
            .get(&sid)
            .copied()
            .unwrap_or((0, 0));

        let ctx = EmotionContext {
            likes_received: likes,
            dislikes_received: dislikes,
            was_recently_banned: self.gladiateurs[glad_idx].ban_remaining_turns > 0,
            is_discussion_stagnating: self.current_turn > 3,
        };

        let new_emo = emotion_engine::update_emotions(&self.gladiateurs[glad_idx].emotions, &ctx);
        self.gladiateurs[glad_idx].emotions = new_emo.clone();

        let _ = channel.send(ArenaEvent::EmotionUpdated {
            speaker_id: sid,
            emotions: new_emo,
        });
    }

    async fn process_moderation(
        &mut self,
        glad_idx: usize,
        intervention: &str,
        channel: &Channel<ArenaEvent>,
    ) {
        let speaker_name = self.gladiateurs[glad_idx].config.name.clone();
        let prompt = prompt_builder::build_moderation_prompt(
            &speaker_name,
            intervention,
            &self.config.topic,
            &self.config.discussion_language,
        );
        let request = self.ollama_client.build_request(
            &self.arbitre.config.system_prompt,
            &prompt,
            &self.arbitre.config.llm_params,
            true,
        );

        let cancel = self.cancel_token.clone();
        let moderation = match self.ollama_client.chat(&request, cancel).await {
            Ok(raw) => json_parser::parse_moderation(&raw),
            Err(_) => return,
        };

        use crate::models::moderation::ModerationAction;
        match moderation.action {
            ModerationAction::Ban => {
                // Guard: don't ban last active
                if turn_manager::active_count(&self.gladiateurs) <= 1 {
                    if !moderation.comment.is_empty() {
                        self.emit_arbitre_message(&moderation.comment, channel);
                    }
                    return;
                }
                let duration = moderation.ban_duration.max(1).min(3);
                self.gladiateurs[glad_idx].ban_remaining_turns = duration;
                self.gladiateurs[glad_idx].ban_issued_this_turn = true;

                let _ = channel.send(ArenaEvent::BanIssued {
                    banned_id: self.gladiateurs[glad_idx].config.id.clone(),
                    banned_name: speaker_name.clone(),
                    reason: moderation.ban_reason.clone(),
                    duration,
                });

                let ban_text = self.ban_notification_msg(&speaker_name, duration, &moderation.ban_reason);
                self.emit_arbitre_message(&ban_text, channel);
            }
            ModerationAction::Comment if !moderation.comment.is_empty() => {
                self.emit_arbitre_message(&moderation.comment, channel);
            }
            _ => {}
        }
    }

    fn emit_arbitre_message(&mut self, content: &str, channel: &Channel<ArenaEvent>) {
        let arb_id = self.arbitre.config.id.clone();
        let arb_name = self.arbitre.config.name.clone();
        let msg = self.create_message(&arb_id, &arb_name, SpeakerRole::Arbitre, content);
        let _ = channel.send(ArenaEvent::MessageComplete {
            message: msg.clone(),
        });
        self.turn_messages.push(msg.clone());
        self.messages_history.push(msg);
    }

    async fn handle_user_intervention(
        &mut self,
        cmd_rx: &mut mpsc::Receiver<EngineCommand>,
        channel: &Channel<ArenaEvent>,
    ) {
        let _ = channel.send(ArenaEvent::UserTurnReady);

        let timeout = tokio::time::sleep(Duration::from_secs(
            self.config.user_intervention_timeout_secs,
        ));
        tokio::pin!(timeout);

        loop {
            tokio::select! {
                cmd = cmd_rx.recv() => {
                    match cmd {
                        Some(EngineCommand::SubmitUserMessage { content }) => {
                            let msg = self.create_message(
                                "user",
                                &self.config.user_name,
                                SpeakerRole::User,
                                &content,
                            );
                            let _ = channel.send(ArenaEvent::MessageComplete { message: msg.clone() });
                            self.turn_messages.push(msg.clone());
                            self.messages_history.push(msg);
                            self.user_intervention_pending = false;
                            self.user_intervention_handled = true;
                            return;
                        }
                        Some(EngineCommand::SkipUserTurn) => {
                            self.user_intervention_pending = false;
                            self.user_intervention_handled = true;
                            return;
                        }
                        Some(EngineCommand::ForceStop) => {
                            self.status = DiscussionStatus::ForceStopRequested;
                            return;
                        }
                        Some(EngineCommand::Stop) => {
                            self.status = DiscussionStatus::StopRequested;
                            return;
                        }
                        Some(_) => {}
                        None => return,
                    }
                }
                _ = &mut timeout => {
                    let _ = channel.send(ArenaEvent::UserTurnTimeout);
                    self.user_intervention_pending = false;
                    self.user_intervention_handled = true;
                    return;
                }
            }
        }
    }

    async fn update_memory_all(&mut self) {
        if self.turn_messages.is_empty() {
            return;
        }

        let turn = self.current_turn;
        for g in &mut self.gladiateurs {
            memory_manager::add_turn_to_memory(&mut g.memory, turn, &self.turn_messages);
        }
        memory_manager::add_turn_to_memory(&mut self.arbitre.memory, turn, &self.turn_messages);

        // LLM-based contextual + positional update
        let contextual_summary = &self.arbitre.memory.contextual_summary;
        let positional_json = memory_manager::positional_map_to_json(&self.arbitre.memory);
        let turn_text = memory_manager::format_turn_messages(&self.turn_messages);
        let prompt = prompt_builder::build_memory_update_prompt(
            contextual_summary,
            &positional_json,
            turn,
            &turn_text,
            &self.config.discussion_language,
        );
        let request = self.ollama_client.build_request(
            "You are a memory summarizer.",
            &prompt,
            &self.arbitre.config.llm_params,
            true,
        );
        let cancel = self.cancel_token.clone();
        if let Ok(raw) = self.ollama_client.chat(&request, cancel).await {
            // Parse the combined JSON: { "summary": "...", "positions": { ... } }
            if let Ok(parsed) = json_parser::parse_json_response::<MemoryUpdateResponse>(&raw) {
                memory_manager::update_from_llm_response(&mut self.arbitre.memory, parsed.summary.clone(), parsed.positions.clone());
                for g in &mut self.gladiateurs {
                    memory_manager::update_from_llm_response(&mut g.memory, parsed.summary.clone(), parsed.positions.clone());
                }
            } else {
                let end = raw.floor_char_boundary(500);
                tracing::warn!(
                    turn = self.current_turn,
                    raw_len = raw.len(),
                    raw_preview = %&raw[..end],
                    "Failed to parse memory update response"
                );
            }
        }
    }

    async fn generate_synthesis(&self, channel: &Channel<ArenaEvent>) {
        let prompt = prompt_builder::build_synthesis_prompt(
            &self.config.topic,
            &self.arbitre.memory,
            &self.config.discussion_language,
        );
        let request = self.ollama_client.build_request(
            &self.arbitre.config.system_prompt,
            &prompt,
            &self.arbitre.config.llm_params,
            false,
        );

        let ch = channel.clone();
        let cancel = self.cancel_token.clone();
        match self
            .ollama_client
            .chat_streaming(
                &request,
                move |token| {
                    let _ = ch.send(ArenaEvent::SynthesisChunk {
                        chunk: token.to_string(),
                    });
                },
                cancel,
            )
            .await
        {
            Ok(summary) => {
                let _ = channel.send(ArenaEvent::SynthesisComplete { summary });
            }
            Err(e) => {
                tracing::warn!("Synthesis failed: {}", e);
                let _ = channel.send(ArenaEvent::SynthesisComplete {
                    summary: String::new(),
                });
            }
        }
    }

    fn create_message(
        &self,
        speaker_id: &str,
        speaker_name: &str,
        role: SpeakerRole,
        content: &str,
    ) -> Message {
        Message {
            id: uuid::Uuid::new_v4().to_string(),
            discussion_id: self.discussion_id.clone(),
            turn_number: self.current_turn,
            speaker_id: speaker_id.to_string(),
            speaker_name: speaker_name.to_string(),
            role,
            content: content.to_string(),
            inner_thought: None,
            reactions: Vec::new(),
            is_ban_notification: false,
            timestamp: chrono::Utc::now(),
        }
    }
}

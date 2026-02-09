use std::collections::HashMap;
use std::time::Duration;

use tauri::ipc::Channel;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::db::repository;
use crate::engine::emotion_engine::{self, EmotionContext};
use crate::engine::json_parser;
use crate::engine::memory_manager;
use crate::engine::prompt_builder;
use crate::engine::turn_manager;
use crate::models::discussion::{DiscussionConfig, DiscussionStatus, TurnDistribution};
use crate::models::engine_command::EngineCommand;
use crate::models::events::ArenaEvent;
use crate::models::gladiateur::GladIAteurState;
use crate::models::iarbitre::IArbitreState;
use crate::models::message::{Message, Reaction, ReactionType, SpeakerRole};
use crate::models::settings::LlmParams;
use crate::ollama::client::OllamaClient;
use crate::ollama::error::OllamaError;
use crate::tavily::client::TavilyClient;
use crate::tavily::error::TavilyError;

use super::truncate_str;

/// Maximum length for a text to be considered a potential model refusal.
/// Real substantive responses are longer than this threshold.
const MAX_REFUSAL_LENGTH: usize = 300;

/// Tavily free tier monthly quota (credits).
const TAVILY_FREE_MONTHLY_QUOTA: u32 = 1000;

/// Detect model safety refusals (e.g. "I'm sorry, but I can't help with that.")
fn is_model_refusal(text: &str) -> bool {
    let lower = text.to_lowercase();
    let trimmed = lower.trim();
    if trimmed.len() > MAX_REFUSAL_LENGTH {
        return false;
    }
    trimmed.starts_with("i'm sorry")
        || trimmed.starts_with("i cannot")
        || trimmed.starts_with("i can't")
        || trimmed.starts_with("i apologize")
        || trimmed.starts_with("sorry, but")
        || trimmed.starts_with("as an ai")
        || trimmed.contains("i can't help with that")
        || trimmed.contains("i cannot assist")
        || trimmed.contains("i'm not able to")
        || trimmed.contains("i can't assist")
}

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
    /// Tavily web search client (None if no API key configured)
    tavily_client: Option<TavilyClient>,
    /// Database connection for Tavily usage tracking (Arc-internal, cheap clone)
    db: tokio_rusqlite::Connection,
}

impl DiscussionEngine {
    pub fn new(
        config: DiscussionConfig,
        discussion_id: String,
        ollama_url: &str,
        ollama_model: &str,
        tavily_api_key: Option<&str>,
        db: tokio_rusqlite::Connection,
    ) -> Self {
        let ollama_client = OllamaClient::new(ollama_url, ollama_model);
        let arbitre = IArbitreState::new(config.arbitre.clone());
        let gladiateurs = config
            .gladiateurs
            .iter()
            .map(|g| GladIAteurState::new(g.clone()))
            .collect();
        let tavily_client = tavily_api_key
            .filter(|k| !k.is_empty())
            .map(TavilyClient::new);

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
            tavily_client,
            db,
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

    /// Localized message when a speaker's ban is lifted
    fn ban_lifted_msg(&self, speaker_name: &str) -> String {
        match self.config.discussion_language.as_str() {
            "en" => format!("{} is back in the discussion", speaker_name),
            "zh" => format!("{} 已重新加入讨论", speaker_name),
            _ => format!("{} est de retour dans la discussion", speaker_name),
        }
    }

    /// Localized error message: at least one gladiator required
    fn at_least_one_gladiator_msg(&self) -> String {
        match self.config.discussion_language.as_str() {
            "en" => "At least one gladiator is required.".to_string(),
            "zh" => "至少需要一位角斗士。".to_string(),
            _ => "Au moins un GladIAteur est requis.".to_string(),
        }
    }

    /// Localized message when all participants are banned
    fn all_banned_msg(&self) -> String {
        match self.config.discussion_language.as_str() {
            "en" => "All participants are banned".to_string(),
            "zh" => "所有参与者均被禁言".to_string(),
            _ => "Tous les participants sont bannis".to_string(),
        }
    }

    /// Localized system prompt for the memory summarizer
    fn memory_summarizer_prompt(&self) -> String {
        match self.config.discussion_language.as_str() {
            "en" => "You are a memory summarizer.".to_string(),
            "zh" => "你是一个记忆总结器。".to_string(),
            _ => "Tu es un résumeur de mémoire.".to_string(),
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
                message: self.at_least_one_gladiator_msg(),
            });
            let _ = channel.send(ArenaEvent::DiscussionEnded);
            return;
        }

        let _ = channel.send(ArenaEvent::DiscussionStarted {
            discussion_id: self.discussion_id.clone(),
        });

        // --- INTRODUCTION ---
        // Optional web search for IArbitre (1 credit, forced on topic)
        let intro_web_search: Option<String> = if self.config.arbitre.web_search_intro
            && self.tavily_client.is_some()
        {
            let global_usage = repository::get_tavily_usage(&self.db).await.unwrap_or_else(|e| {
                tracing::warn!(error = %e, "Failed to read Tavily usage count — assuming 0");
                0
            });
            if global_usage < TAVILY_FREE_MONTHLY_QUOTA {
                let topic_query = truncate_str(&self.config.topic, 200).to_string();
                let (ctx, _count) = self.process_web_search(
                    &self.arbitre.config.system_prompt,
                    &self.arbitre.config.id,
                    &self.arbitre.config.name,
                    1,
                    "",
                    "",
                    Some(vec![topic_query]),
                    &self.arbitre.config.llm_params,
                    0,
                    &channel,
                ).await;
                ctx
            } else { None }
        } else { None };

        let participant_names: Vec<String> = self
            .gladiateurs
            .iter()
            .map(|g| g.config.name.clone())
            .collect();
        let intro_prompt = prompt_builder::build_introduction_prompt(
            &self.config.topic,
            &participant_names,
            &self.config.discussion_language,
            intro_web_search.as_deref(),
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

            // Determine speaker order — sync for Sequential/Random, async for Democratic/Authoritarian
            let order = match &self.config.arbitre.turn_distribution {
                TurnDistribution::Sequential | TurnDistribution::Random => {
                    turn_manager::determine_speaker_order(
                        &self.gladiateurs,
                        &self.config.arbitre.turn_distribution,
                    )
                }
                TurnDistribution::Democratic | TurnDistribution::Authoritarian => {
                    let _ = channel.send(ArenaEvent::DeterminingOrder {
                        turn_number: self.current_turn,
                    });

                    // Clone fields into owned context to avoid borrow issues across .await
                    let ctx = turn_manager::AsyncTurnContext {
                        ollama_client: self.ollama_client.clone(),
                        cancel_token: self.cancel_token.clone(),
                        arbitre_system_prompt: self.arbitre.config.system_prompt.clone(),
                        arbitre_llm_params: self.arbitre.config.llm_params.clone(),
                        discussion_summary: self.arbitre.memory.contextual_summary.clone(),
                        topic: self.config.topic.clone(),
                        current_turn: self.current_turn,
                        discussion_language: self.config.discussion_language.clone(),
                    };

                    match &self.config.arbitre.turn_distribution {
                        TurnDistribution::Democratic => {
                            turn_manager::determine_order_democratic(
                                &self.gladiateurs,
                                &ctx,
                            )
                            .await
                        }
                        TurnDistribution::Authoritarian => {
                            turn_manager::determine_order_authoritarian(
                                &self.gladiateurs,
                                &ctx,
                            )
                            .await
                        }
                        _ => unreachable!(),
                    }
                }
            };

            // Check for cancellation after async turn determination (prevents phantom TurnStarted)
            if self.cancel_token.is_cancelled() {
                break;
            }
            if self.process_commands(&mut cmd_rx, &channel).await {
                break;
            }

            if order.is_empty() {
                let _ = channel.send(ArenaEvent::TurnSkipped {
                    reason: self.all_banned_msg(),
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
            let total_speakers = order.len();
            for (speaker_pos, &glad_idx) in order.iter().enumerate() {
                if self.process_commands(&mut cmd_rx, &channel).await {
                    broke_early = true;
                    break;
                }
                // Only check user-initiated stops mid-turn, NOT max_turns
                // (max_turns is checked at the top of the outer loop, before incrementing)
                if matches!(
                    self.status,
                    DiscussionStatus::StopRequested | DiscussionStatus::ForceStopRequested
                ) {
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

                // C.2.5 WEB SEARCH (if enabled + quotas OK)
                // First intervention: mandatory search on topic (skip LLM decision)
                // Subsequent turns: LLM decides whether to search
                let web_search_context: Option<String> = {
                    let global_usage = repository::get_tavily_usage(&self.db).await.unwrap_or_else(|e| {
                        tracing::warn!(error = %e, "Failed to read Tavily usage count — assuming 0");
                        0
                    });
                    let (can, max_q) = self.can_search_gladiateur(glad_idx, global_usage);
                    if can {
                        let is_first_search = self.gladiateurs[glad_idx].web_searches_used_discussion == 0;
                        let forced_queries = if is_first_search {
                            Some(vec![truncate_str(&self.config.topic, 200).to_string()])
                        } else {
                            None
                        };
                        let directive = prompt_builder::default_search_directive(
                            &self.config.discussion_language,
                        );
                        let recent = truncate_str(
                            &self.build_recent_exchanges(glad_idx), 500
                        ).to_string();
                        let used_so_far = self.gladiateurs[glad_idx].web_searches_used_discussion;
                        let (ctx, count) = self.process_web_search(
                            &self.gladiateurs[glad_idx].config.system_prompt,
                            &speaker_id,
                            &speaker_name,
                            max_q,
                            directive,
                            &recent,
                            forced_queries,
                            &self.gladiateurs[glad_idx].config.llm_params,
                            used_so_far,
                            &channel,
                        ).await;
                        self.gladiateurs[glad_idx].web_searches_used_discussion += count;
                        ctx
                    } else { None }
                };

                // C.3 INNER THOUGHT + C.4 PUBLIC INTERVENTION
                // When think mode is enabled, the model reasons internally (replaces separate thought phase)
                let use_think = self.should_enable_think(glad_idx);
                let (thought, content) = if use_think {
                    tracing::info!(
                        speaker = %speaker_name,
                        turn = self.current_turn,
                        "Using think mode for intervention"
                    );
                    let (t, c) = self.process_intervention_think(glad_idx, web_search_context.as_deref(), &channel).await;
                    // If think mode failed (HTTP 400 = model doesn't support it),
                    // fall back to the normal thought + intervention path
                    if c.is_none() {
                        tracing::info!(
                            speaker = %speaker_name,
                            "Think mode produced no content, falling back to normal intervention"
                        );
                        let thought = self.process_thought(glad_idx, web_search_context.as_deref(), &channel).await;
                        let content = self
                            .process_intervention(glad_idx, thought.as_deref(), web_search_context.as_deref(), &channel)
                            .await;
                        (thought, content)
                    } else {
                        (t, c)
                    }
                } else {
                    let thought = self.process_thought(glad_idx, web_search_context.as_deref(), &channel).await;
                    let content = self
                        .process_intervention(glad_idx, thought.as_deref(), web_search_context.as_deref(), &channel)
                        .await;
                    (thought, content)
                };

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

                // C.7 MID-TURN OPPORTUNISTIC USER INTERVENTION
                // If user requested to speak and we're past the halfway point,
                // handle it so remaining gladiateurs can react to the user's message.
                if self.user_intervention_pending
                    && !self.user_intervention_handled
                    && (speaker_pos + 1 >= total_speakers / 2 || speaker_pos + 1 == total_speakers)
                {
                    self.handle_user_intervention(&mut cmd_rx, &channel).await;
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
                    speaker_name: name.clone(),
                });
                let lifted_text = self.ban_lifted_msg(&name);
                self.emit_ban_notification(&lifted_text, &channel);
            }

            self.user_intervention_handled = false;
        }

        // --- SYNTHESIS (unless force-stopped) ---
        if self.status != DiscussionStatus::ForceStopRequested {
            tracing::info!(
                discussion_id = %self.discussion_id,
                status = ?self.status,
                turns_completed = self.current_turn,
                "Starting synthesis generation"
            );

            self.generate_synthesis(None, &channel).await;
            tracing::info!(discussion_id = %self.discussion_id, "Synthesis generation complete");
        } else {
            tracing::info!(
                discussion_id = %self.discussion_id,
                "Skipping synthesis (force-stopped)"
            );
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
        ) || self
            .config
            .max_turns
            .is_some_and(|max| self.current_turn >= max)
    }

    /// Process pending commands. Returns true if engine should stop.
    async fn process_commands(
        &mut self,
        cmd_rx: &mut mpsc::Receiver<EngineCommand>,
        channel: &Channel<ArenaEvent>,
    ) -> bool {
        // Drain non-blocking
        while let Ok(cmd) = cmd_rx.try_recv() {
            match cmd {
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
        let speaker_id = self.gladiateurs[glad_idx].config.id.clone();
        let prev_msgs: Vec<&Message> = self
            .messages_history
            .iter()
            .filter(|m| {
                m.turn_number == self.current_turn - 1
                    && m.speaker_id != speaker_id
                    && m.role != SpeakerRole::Arbitre
            })
            .collect();

        if prev_msgs.is_empty() {
            tracing::info!(
                speaker = %speaker_name,
                turn = self.current_turn,
                "No previous turn messages found for reactions"
            );
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
        match self.ollama_client.chat(&request, cancel).await {
            Ok(raw) => {
                let end = raw.floor_char_boundary(300);
                tracing::info!(
                    speaker = %speaker_name,
                    raw_len = raw.len(),
                    raw_preview = %&raw[..end],
                    "Reaction LLM response"
                );
                let reactions = json_parser::parse_reactions(&raw, &known);
                if reactions.is_empty() {
                    tracing::warn!(
                        speaker = %speaker_name,
                        "No valid reactions parsed from response"
                    );
                }
                for parsed in reactions {
                    // Skip self-reactions (by name — handles duplicate gladiateur names)
                    if parsed.speaker_name.to_lowercase().trim() == speaker_name.to_lowercase().trim() {
                        tracing::debug!(
                            speaker = %speaker_name,
                            "Skipped self-reaction"
                        );
                        continue;
                    }
                    if let Some(target) = prev_msgs
                        .iter()
                        .find(|m| m.speaker_name.to_lowercase().trim() == parsed.speaker_name.to_lowercase().trim())
                    {
                        tracing::info!(
                            from = %speaker_name,
                            to = %target.speaker_name,
                            reaction = ?parsed.reaction_type,
                            "Reaction emitted"
                        );
                        // Track reaction counts for the emotion system
                        let target_speaker_id = target.speaker_id.clone();
                        let entry = self.turn_reaction_counts
                            .entry(target_speaker_id)
                            .or_insert((0, 0));
                        match &parsed.reaction_type {
                            ReactionType::Like => entry.0 += 1,
                            ReactionType::Dislike => entry.1 += 1,
                        }

                        let reaction_event = ArenaEvent::ReactionEmitted {
                            message_id: target.id.clone(),
                            reaction: Reaction {
                                from_speaker_id: self.gladiateurs[glad_idx].config.id.clone(),
                                from_speaker_name: self.gladiateurs[glad_idx].config.name.clone(),
                                reaction_type: parsed.reaction_type,
                                target_message_id: target.id.clone(),
                            },
                        };
                        // DEBUG: log exact JSON to verify serialization matches frontend expectations
                        if let Ok(json) = serde_json::to_string(&reaction_event) {
                            tracing::info!(json = %json, "ReactionEmitted JSON payload");
                        }
                        if let Err(e) = channel.send(reaction_event) {
                            tracing::error!(
                                from = %speaker_name,
                                to = %target.speaker_name,
                                target_msg_id = %target.id,
                                error = %e,
                                "Failed to send ReactionEmitted event via channel"
                            );
                        }
                    } else {
                        tracing::warn!(
                            from = %speaker_name,
                            parsed_name = %parsed.speaker_name,
                            "Reaction target not found in previous messages"
                        );
                    }
                }
            }
            Err(e) => {
                tracing::warn!(
                    speaker = %speaker_name,
                    error = %e,
                    "Reaction LLM call failed"
                );
            }
        }
    }

    /// Build a string with recent exchanges for the thought prompt context
    fn build_recent_exchanges(&self, glad_idx: usize) -> String {
        let mut recent = String::new();

        // Previous turn messages (from messages_history)
        if self.current_turn > 1 {
            let prev_turn = self.current_turn - 1;
            for m in &self.messages_history {
                if m.turn_number == prev_turn && m.role != SpeakerRole::Arbitre {
                    recent.push_str(&format!(
                        "{}: {}\n",
                        m.speaker_name,
                        truncate_str(&m.content, 200)
                    ));
                }
            }
        }

        // Current turn messages so far
        for m in &self.turn_messages {
            if m.speaker_id != self.gladiateurs[glad_idx].config.id {
                recent.push_str(&format!(
                    "{}: {}\n",
                    m.speaker_name,
                    truncate_str(&m.content, 200)
                ));
            }
        }

        recent
    }

    async fn process_thought(
        &self,
        glad_idx: usize,
        web_search_results: Option<&str>,
        channel: &Channel<ArenaEvent>,
    ) -> Option<String> {
        let has_prior_context = !self.turn_messages.is_empty()
            || !self.gladiateurs[glad_idx].memory.immediate.is_empty();
        let recent_exchanges = self.build_recent_exchanges(glad_idx);
        let prompt = prompt_builder::build_thought_prompt(
            &recent_exchanges,
            &self.gladiateurs[glad_idx].emotions,
            &self.config.discussion_language,
            has_prior_context,
            self.emotion_driven,
            self.current_turn,
            self.config.max_turns,
            web_search_results,
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
        web_search_results: Option<&str>,
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
            self.current_turn,
            self.config.max_turns,
            web_search_results,
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
            Ok(c) if !c.is_empty() && !is_model_refusal(&c) => {
                tracing::info!(
                    discussion_id = %self.discussion_id,
                    turn = self.current_turn,
                    speaker = %speaker_name,
                    len = c.len(),
                    "Intervention completed"
                );
                Some(c)
            }
            Ok(c) if is_model_refusal(&c) => {
                tracing::warn!(
                    discussion_id = %self.discussion_id,
                    turn = self.current_turn,
                    speaker = %speaker_name,
                    content = %c,
                    "Intervention was a model refusal — retrying with adjusted prompt"
                );
                // Retry with higher temp — model may cooperate on second try
                let mut params = self.gladiateurs[glad_idx].config.llm_params.clone();
                params.temperature = (params.temperature + 0.3).min(2.0);
                let retry = self.ollama_client.build_request(&sys, &usr, &params, false);
                match self.ollama_client.chat(&retry, cancel).await {
                    Ok(c2) if !c2.is_empty() && !is_model_refusal(&c2) => {
                        tracing::info!(speaker = %speaker_name, "Refusal retry succeeded");
                        Some(c2)
                    }
                    _ => {
                        tracing::warn!(speaker = %speaker_name, "Refusal retry also failed");
                        None
                    }
                }
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

    /// Probabilistic heuristic: should this gladiateur use think mode for its intervention?
    /// Think mode is non-systematic to keep discussion dynamic and lively.
    fn should_enable_think(&self, glad_idx: usize) -> bool {
        // Never on turn 1 — keep things quick at the start
        if self.current_turn <= 1 {
            return false;
        }

        let emo = &self.gladiateurs[glad_idx].emotions;

        // Base probability: 20%
        let mut probability: f64 = 0.20;

        // High frustration → more likely to think deeply
        if emo.frustration > 70 {
            probability += 0.15;
        }

        // High engagement → invested, thinks more
        if emo.engagement > 70 {
            probability += 0.10;
        }

        // End of discussion → synthesize thoughts
        if let Some(max) = self.config.max_turns {
            if self.current_turn + 2 >= max {
                probability += 0.15;
            }
        }

        // Was contradicted → needs to think about response
        let (_, dislikes) = self
            .turn_reaction_counts
            .get(&self.gladiateurs[glad_idx].config.id)
            .copied()
            .unwrap_or((0, 0));
        if dislikes >= 2 {
            probability += 0.10;
        }

        // Cap at 60% to keep it non-systematic
        probability = probability.min(0.60);

        use rand::Rng;
        rand::thread_rng().gen_bool(probability)
    }

    /// Process intervention with think mode — model reasons internally, replacing separate thought phase
    async fn process_intervention_think(
        &self,
        glad_idx: usize,
        web_search_results: Option<&str>,
        channel: &Channel<ArenaEvent>,
    ) -> (Option<String>, Option<String>) {
        let (sys, usr) = prompt_builder::build_intervention_prompt(
            &self.gladiateurs[glad_idx].config.system_prompt,
            &self.config.topic,
            &self.gladiateurs[glad_idx].memory,
            &self.turn_messages,
            None, // No separate thought — the model will think internally
            &self.gladiateurs[glad_idx].emotions,
            &self.config.discussion_language,
            &self.config.user_name,
            self.emotion_driven,
            self.current_turn,
            self.config.max_turns,
            web_search_results,
        );

        let mut request = self.ollama_client.build_request(
            &sys,
            &usr,
            &self.gladiateurs[glad_idx].config.llm_params,
            false,
        );
        request.think = Some(true);

        let speaker_id = self.gladiateurs[glad_idx].config.id.clone();
        let speaker_name = self.gladiateurs[glad_idx].config.name.clone();
        let ch_content = channel.clone();
        let sid_content = speaker_id.clone();
        let cancel = self.cancel_token.clone();

        match self
            .ollama_client
            .chat_streaming_with_think(
                &request,
                move |token| {
                    let _ = ch_content.send(ArenaEvent::MessageChunk {
                        speaker_id: sid_content.clone(),
                        chunk: token.to_string(),
                    });
                },
                |_| {
                    // Think-mode reasoning is raw model meta-reasoning (not in-character).
                    // We intentionally discard it — the separate thought phase handles
                    // in-character reflection when think mode is not triggered.
                },
                cancel,
            )
            .await
        {
            Ok(result) => {
                // Think-mode reasoning is NOT displayed to users — it contains
                // raw chain-of-thought like "We need to respond as..." which is
                // not in-character. We only keep the content.
                if let Some(thinking) = result.thinking.as_ref().filter(|t| !t.is_empty()) {
                    tracing::debug!(
                        speaker = %speaker_name,
                        thinking_len = thinking.len(),
                        "Think-mode reasoning produced (discarded from display)"
                    );
                }

                let content = if result.content.is_empty() {
                    tracing::warn!(
                        speaker = %speaker_name,
                        "Think-mode intervention returned empty content"
                    );
                    None
                } else if is_model_refusal(&result.content) {
                    tracing::warn!(
                        speaker = %speaker_name,
                        content = %result.content,
                        "Think-mode intervention was a model refusal — falling back"
                    );
                    None
                } else {
                    tracing::info!(
                        discussion_id = %self.discussion_id,
                        turn = self.current_turn,
                        speaker = %speaker_name,
                        content_len = result.content.len(),
                        "Intervention completed (think mode)"
                    );
                    Some(result.content)
                };

                // Never store think-mode reasoning as inner_thought
                (None, content)
            }
            Err(OllamaError::Cancelled) => (None, None),
            Err(e) => {
                tracing::warn!(
                    speaker = %speaker_name,
                    error = %e,
                    "Intervention with think mode failed — will fall back to normal mode"
                );
                (None, None)
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
                let duration = moderation.ban_duration.clamp(1, 3);
                self.gladiateurs[glad_idx].ban_remaining_turns = duration;
                self.gladiateurs[glad_idx].ban_issued_this_turn = true;

                let _ = channel.send(ArenaEvent::BanIssued {
                    banned_id: self.gladiateurs[glad_idx].config.id.clone(),
                    banned_name: speaker_name.clone(),
                    reason: moderation.ban_reason.clone(),
                    duration,
                });

                let ban_text = self.ban_notification_msg(&speaker_name, duration, &moderation.ban_reason);
                self.emit_ban_notification(&ban_text, channel);
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

    fn emit_ban_notification(&mut self, content: &str, channel: &Channel<ArenaEvent>) {
        let arb_id = self.arbitre.config.id.clone();
        let arb_name = self.arbitre.config.name.clone();
        let mut msg = self.create_message(&arb_id, &arb_name, SpeakerRole::Arbitre, content);
        msg.is_ban_notification = true;
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
        tracing::info!(
            discussion_id = %self.discussion_id,
            turn = self.current_turn,
            "User intervention: waiting for user input"
        );
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
                            tracing::info!(
                                discussion_id = %self.discussion_id,
                                turn = self.current_turn,
                                content_len = content.len(),
                                "User intervention: message received"
                            );
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
                            tracing::info!(
                                discussion_id = %self.discussion_id,
                                turn = self.current_turn,
                                "User intervention: skipped"
                            );
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
        let mem_sys = self.memory_summarizer_prompt();
        let request = self.ollama_client.build_request(
            &mem_sys,
            &prompt,
            &self.arbitre.config.llm_params,
            true,
        );
        let cancel = self.cancel_token.clone();
        let raw = match self.ollama_client.chat(&request, cancel).await {
            Ok(r) if !r.is_empty() => r,
            Ok(_) => {
                // Empty response — retry once with higher temperature
                tracing::warn!(turn = self.current_turn, "Memory update returned empty — retrying");
                let mut retry_params = self.arbitre.config.llm_params.clone();
                retry_params.temperature = (retry_params.temperature + 0.3).min(2.0);
                let retry = self.ollama_client.build_request(
                    &mem_sys,
                    &prompt,
                    &retry_params,
                    true,
                );
                let cancel2 = self.cancel_token.clone();
                match self.ollama_client.chat(&retry, cancel2).await {
                    Ok(r) if !r.is_empty() => r,
                    _ => return,
                }
            }
            Err(_) => return,
        };

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

    async fn generate_synthesis(&self, web_search_results: Option<&str>, channel: &Channel<ArenaEvent>) {
        let prompt = prompt_builder::build_synthesis_prompt(
            &self.config.topic,
            &self.arbitre.memory,
            &self.config.discussion_language,
            web_search_results,
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

    // ===== Web Search =====

    /// Check if a gladiateur can search this turn.
    /// Returns (can_search, max_queries_this_turn).
    /// Uses global `web_search_max_per_gladiateur` from config (max 1 per turn).
    fn can_search_gladiateur(&self, glad_idx: usize, global_usage: u32) -> (bool, u32) {
        let max_per_disc = self.config.web_search_max_per_gladiateur;
        if max_per_disc == 0 || self.tavily_client.is_none() {
            return (false, 0);
        }
        let remaining_disc = max_per_disc
            .saturating_sub(self.gladiateurs[glad_idx].web_searches_used_discussion);
        // Max 1 search per gladiateur per turn
        let max_queries = remaining_disc.min(1);
        let global_remaining = TAVILY_FREE_MONTHLY_QUOTA.saturating_sub(global_usage);
        let max_queries = max_queries.min(global_remaining);
        (max_queries > 0, max_queries)
    }

    /// Execute web search for a speaker. Returns (formatted_context, queries_executed_count).
    /// Uses `&self` — no fields mutated; counter increment happens at call site.
    #[allow(clippy::too_many_arguments)]
    async fn process_web_search(
        &self,
        system_prompt: &str,
        speaker_id: &str,
        speaker_name: &str,
        max_queries: u32,
        search_directive: &str,
        recent_context: &str,
        forced_queries: Option<Vec<String>>,
        llm_params: &LlmParams,
        searches_used_so_far: u32,
        channel: &Channel<ArenaEvent>,
    ) -> (Option<String>, u32) {
        // 1. Determine queries
        let queries: Vec<String> = if let Some(forced) = forced_queries {
            forced.into_iter().take(max_queries as usize).collect()
        } else {
            // LLM decision (non-streaming, JSON)
            let mut decision_params = llm_params.clone();
            decision_params.temperature = 0.3;
            decision_params.num_predict = 100;

            let prompt = prompt_builder::build_web_search_decision_prompt(
                &self.config.topic,
                recent_context,
                search_directive,
                max_queries,
                &self.config.discussion_language,
            );
            let request = self.ollama_client.build_request(
                system_prompt,
                &prompt,
                &decision_params,
                true, // json_mode
            );
            let raw = match self.ollama_client.chat(&request, self.cancel_token.clone()).await {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!(error = %e, "Web search decision LLM call failed — skipping search");
                    return (None, 0);
                }
            };
            let decision = json_parser::parse_json_response::<json_parser::SearchDecisionResponse>(&raw)
                .unwrap_or_default();

            if !decision.needs_search || decision.queries.is_empty() {
                return (None, 0);
            }
            decision.queries.into_iter().take(max_queries as usize).collect()
        };

        if queries.is_empty() {
            return (None, 0);
        }

        // 2. Execute each search
        let tavily = match self.tavily_client.as_ref() {
            Some(c) => c,
            None => return (None, 0),
        };
        let mut all_results: Vec<(String, crate::tavily::TavilySearchResponse)> = Vec::new();
        let mut executed_count = 0u32;

        for query in &queries {
            if self.cancel_token.is_cancelled() {
                break;
            }

            match tavily.search(query, self.cancel_token.clone()).await {
                Ok(response) => {
                    all_results.push((query.clone(), response));
                    executed_count += 1;
                    if let Err(e) = repository::increment_tavily_usage(&self.db).await {
                        tracing::warn!(error = %e, "Failed to increment Tavily usage counter");
                    }
                }
                Err(TavilyError::QuotaExceeded) => {
                    tracing::warn!("Tavily quota exceeded — stopping all searches");
                    break;
                }
                Err(TavilyError::InvalidKey) => {
                    tracing::error!("Tavily API key invalid — stopping all searches");
                    break;
                }
                Err(TavilyError::Cancelled) => break,
                Err(e) => {
                    tracing::warn!(query = %query, error = %e, "Tavily search failed — skipping");
                    continue;
                }
            }
        }

        if executed_count == 0 {
            return (None, 0);
        }

        // 3. Emit batched event
        let executed_queries: Vec<String> = all_results.iter().map(|(q, _)| q.clone()).collect();
        let total_results: u32 = all_results.iter().map(|(_, r)| r.results.len() as u32).sum();
        let _ = channel.send(ArenaEvent::WebSearchPerformed {
            speaker_id: speaker_id.to_string(),
            speaker_name: speaker_name.to_string(),
            queries: executed_queries,
            results_count: total_results,
            searches_used_discussion: searches_used_so_far + executed_count,
        });

        // 4. Format for prompt injection
        let lang = &self.config.discussion_language;
        (
            Some(prompt_builder::build_search_results_context(&all_results, lang)),
            executed_count,
        )
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

#[cfg(test)]
mod tests {
    use super::TAVILY_FREE_MONTHLY_QUOTA;

    /// Test the quota logic directly (same as can_search_gladiateur body)
    fn check_quota(max_per_disc: u32, used: u32, global_usage: u32, has_tavily: bool) -> (bool, u32) {
        if max_per_disc == 0 || !has_tavily {
            return (false, 0);
        }
        let remaining_disc = max_per_disc.saturating_sub(used);
        let max_queries = remaining_disc.min(1); // max 1 per turn
        let global_remaining = TAVILY_FREE_MONTHLY_QUOTA.saturating_sub(global_usage);
        let max_queries = max_queries.min(global_remaining);
        (max_queries > 0, max_queries)
    }

    #[test]
    fn test_quota_disabled() {
        let (can, max) = check_quota(0, 0, 0, true);
        assert!(!can);
        assert_eq!(max, 0);
    }

    #[test]
    fn test_quota_no_tavily() {
        let (can, max) = check_quota(5, 0, 0, false);
        assert!(!can);
        assert_eq!(max, 0);
    }

    #[test]
    fn test_quota_fresh() {
        let (can, max) = check_quota(5, 0, 0, true);
        assert!(can);
        assert_eq!(max, 1); // max 1 per turn
    }

    #[test]
    fn test_quota_discussion_limit() {
        let (can, max) = check_quota(5, 4, 0, true);
        assert!(can);
        assert_eq!(max, 1); // min(5-4, 1) = 1
    }

    #[test]
    fn test_quota_exhausted() {
        let (can, max) = check_quota(5, 5, 0, true);
        assert!(!can);
        assert_eq!(max, 0); // 5-5 = 0
    }

    #[test]
    fn test_quota_global_limit() {
        let (can, max) = check_quota(10, 0, 999, true);
        assert!(can);
        assert_eq!(max, 1); // min(10, 1, 1000-999) = 1
    }

    #[test]
    fn test_quota_global_exhausted() {
        let (can, max) = check_quota(10, 0, 1000, true);
        assert!(!can);
        assert_eq!(max, 0); // 1000-1000 = 0
    }
}

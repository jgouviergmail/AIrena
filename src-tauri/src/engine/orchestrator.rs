use std::collections::{HashMap, HashSet};
use std::time::Duration;

use tauri::ipc::Channel;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::db::repository;
use crate::engine::directive_builder::{self, SpeakerTurnContext, SpeechAct};
use crate::engine::dynamics_parser::{self, ParsedDynamics};
use crate::engine::emotion_engine::{self, EmotionContext};
use crate::engine::json_parser;
use crate::engine::memory_manager;
use crate::engine::prompt_builder;
use crate::engine::turn_manager;
use crate::models::emotion::{EmotionSnapshot, EmotionalProfile};
use crate::engine::mode_prompts;
use crate::models::discussion::{DiscussionConfig, DiscussionMode, DiscussionStatus, DocumentFormat, TurnDistribution};
use crate::models::engine_command::EngineCommand;
use crate::models::events::ArenaEvent;
use crate::models::gladiateur::GladIAteurState;
use crate::models::iarbitre::IArbitreState;
use crate::models::message::{Message, Reaction, ReactionType, SpeakerRole};
use crate::models::settings::LlmParams;
use crate::ollama::client::OllamaClient;
use crate::ollama::error::OllamaError;
use crate::rag::RagStore;
use crate::tavily::client::TavilyClient;
use crate::tavily::error::TavilyError;
use crate::wikipedia::client::WikiClient;

use super::truncate_str;

use crate::constants;

/// Detect model safety refusals (e.g. "I'm sorry, but I can't help with that.")
fn is_model_refusal(text: &str) -> bool {
    let lower = text.to_lowercase();
    let trimmed = lower.trim();
    if trimmed.len() > constants::ORCH_MAX_REFUSAL_LENGTH {
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
    /// Wikipedia search client (always available — free, no API key)
    wiki_client: WikiClient,
    /// Database connection for Tavily usage tracking (Arc-internal, cheap clone)
    db: tokio_rusqlite::Connection,
    /// Cumulative reactions: (speaker_id, target_speaker_id) -> (likes, dislikes)
    cumulative_reactions: HashMap<(String, String), (u32, u32)>,
    /// Speaker's own messages for self-memory: speaker_id -> Vec<String> (last 2)
    speaker_own_messages: HashMap<String, Vec<String>>,
    /// Parsed dynamics cache: speaker_id -> ParsedDynamics
    dynamics_cache: HashMap<String, ParsedDynamics>,
    /// Last speech act per speaker: speaker_id -> SpeechAct
    last_speech_acts: HashMap<String, SpeechAct>,
    /// Queries executed by all speakers THIS turn (for cross-gladiateur dedup)
    turn_search_queries: Vec<(String, String)>,
    /// Global pool counter: web searches consumed across all gladiateurs
    web_searches_used_pool: u32,
    /// Global pool counter: wiki searches consumed across all gladiateurs
    wiki_searches_used_pool: u32,
    /// Gladiateur indices that have completed their forced first web search
    forced_web_done: HashSet<usize>,
    /// Gladiateur indices that have completed their forced first wiki search
    forced_wiki_done: HashSet<usize>,
    /// Co-construction document content (accumulated across turns)
    document_content: String,
    /// In-memory RAG store (taken from AppState, dropped with engine)
    rag_store: Option<RagStore>,
}

impl DiscussionEngine {
    pub fn new(
        config: DiscussionConfig,
        discussion_id: String,
        ollama_url: &str,
        ollama_model: &str,
        tavily_api_key: Option<&str>,
        db: tokio_rusqlite::Connection,
        rag_store: Option<RagStore>,
    ) -> Self {
        let ollama_client = OllamaClient::new(ollama_url, ollama_model);
        let arbitre = IArbitreState::new(config.arbitre.clone());
        let gladiateurs = config
            .gladiateurs
            .iter()
            .map(|g| {
                let initial = EmotionalProfile::from_json_opt(g.initial_emotions.as_deref());
                GladIAteurState::new(g.clone(), Some(initial))
            })
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
            wiki_client: WikiClient::new(),
            db,
            cumulative_reactions: HashMap::new(),
            speaker_own_messages: HashMap::new(),
            dynamics_cache: HashMap::new(),
            last_speech_acts: HashMap::new(),
            turn_search_queries: Vec::new(),
            web_searches_used_pool: 0,
            wiki_searches_used_pool: 0,
            forced_web_done: HashSet::new(),
            forced_wiki_done: HashSet::new(),
            document_content: String::new(),
            rag_store,
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
            "en" => "You are a discussion memory summarizer. Maintain an accurate, concise summary and track each participant's current stance. Respond ONLY with valid JSON.".to_string(),
            "zh" => "你是一个讨论记忆总结器。维护准确简洁的摘要并跟踪每位参与者的当前立场。仅用有效的JSON回复。".to_string(),
            _ => "Tu es un résumeur de mémoire de discussion. Maintiens un résumé précis et concis et suis la position actuelle de chaque participant. Réponds UNIQUEMENT avec du JSON valide.".to_string(),
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

        // Emit initial emotions for all participants so the sidebar is populated immediately
        let lang = self.config.discussion_language.as_str();
        Self::emit_emotion_updated(&channel, &self.arbitre.config.id, &self.arbitre.emotions, lang);
        for g in &self.gladiateurs {
            Self::emit_emotion_updated(&channel, &g.config.id, &g.emotions, lang);
        }

        // Parse dynamics from system prompts and cache them
        for g in &self.gladiateurs {
            if let Some(dynamics) = dynamics_parser::parse_dynamics(&g.config.system_prompt) {
                tracing::debug!(speaker = %g.config.name, "Parsed dynamics from system prompt");
                self.dynamics_cache.insert(g.config.id.clone(), dynamics);
            }
        }
        if let Some(dynamics) = dynamics_parser::parse_dynamics(&self.arbitre.config.system_prompt) {
            self.dynamics_cache.insert(self.arbitre.config.id.clone(), dynamics);
        }

        // --- INTRODUCTION ---
        // Optional web + wiki search for IArbitre (forced on topic)
        let want_web_intro = self.config.arbitre.web_search_intro && self.tavily_client.is_some();
        let want_wiki_intro = self.config.arbitre.wiki_search_intro;
        let topic_query = truncate_str(&self.config.topic, constants::ORCH_TOPIC_FOR_SEARCH).to_string();

        // For wiki intro, extract a broad encyclopedic concept via LLM instead of the raw topic
        let wiki_intro_query = if want_wiki_intro {
            let wiki_sys = match self.config.discussion_language.as_str() {
                "en" => "You are a research assistant. Respond ONLY with valid JSON, no other text.",
                "zh" => "你是研究助手。仅用有效的JSON回复，不要有其他文本。",
                _ => "Tu es un assistant de recherche. Réponds UNIQUEMENT avec du JSON valide, aucun autre texte.",
            };
            let wiki_prompt = prompt_builder::build_wiki_search_decision_prompt(
                &self.config.topic, "", prompt_builder::default_wiki_directive(&self.config.discussion_language),
                1, &self.config.discussion_language, &[], None, &[],
            );
            let q = self.pick_forced_query(
                wiki_sys, &wiki_prompt, &self.arbitre.config.llm_params,
                topic_query.clone(), "IArbitre", "wiki-intro",
            ).await;
            tracing::info!(query = %q, "IArbitre wiki intro query (LLM-picked)");
            q
        } else {
            topic_query.clone()
        };

        let (intro_web_ctx, intro_wiki_ctx) = if want_web_intro && want_wiki_intro {
            // Both active: run sequentially (avoids &self/&mut self borrow conflict)
            let global_usage = repository::get_tavily_usage(&self.db).await.unwrap_or_else(|e| {
                tracing::warn!(error = %e, "Failed to read Tavily usage count — assuming 0");
                0
            });
            let web_ctx = if global_usage < constants::TAVILY_FREE_MONTHLY_QUOTA {
                let (ctx, _, _) = self.process_web_search(
                    &self.arbitre.config.system_prompt,
                    &self.arbitre.config.id, &self.arbitre.config.name,
                    1, "", "", Some(vec![topic_query.clone()]),
                    &self.arbitre.config.llm_params, &channel, &[], &[],
                ).await;
                ctx
            } else { None };
            let wiki_ctx = self.process_wiki_search_intro(
                &wiki_intro_query, &self.arbitre.config.id, &self.arbitre.config.name, &channel,
            ).await;
            (web_ctx, wiki_ctx)
        } else if want_web_intro {
            let global_usage = repository::get_tavily_usage(&self.db).await.unwrap_or_else(|e| {
                tracing::warn!(error = %e, "Failed to read Tavily usage count — assuming 0");
                0
            });
            let web_ctx = if global_usage < constants::TAVILY_FREE_MONTHLY_QUOTA {
                let (ctx, _, _) = self.process_web_search(
                    &self.arbitre.config.system_prompt,
                    &self.arbitre.config.id, &self.arbitre.config.name,
                    1, "", "", Some(vec![topic_query.clone()]),
                    &self.arbitre.config.llm_params, &channel, &[], &[],
                ).await;
                ctx
            } else { None };
            (web_ctx, None)
        } else if want_wiki_intro {
            let wiki_ctx = self.process_wiki_search_intro(
                &wiki_intro_query, &self.arbitre.config.id, &self.arbitre.config.name, &channel,
            ).await;
            (None, wiki_ctx)
        } else {
            (None, None)
        };

        // Combine intro search contexts
        let mut intro_search_context: Option<String> = match (&intro_web_ctx, &intro_wiki_ctx) {
            (Some(w), Some(wiki)) => Some(format!("{w}\n\n{wiki}")),
            (Some(w), None) => Some(w.clone()),
            (None, Some(wiki)) => Some(wiki.clone()),
            (None, None) => None,
        };

        // RAG knowledge base for introduction
        if let Some(ref rag_store) = self.rag_store {
            if !rag_store.is_empty() {
                let lang = &self.config.discussion_language;
                match rag_store
                    .query(
                        &self.config.topic,
                        lang,
                        &self.ollama_client,
                        &self.arbitre.config.llm_params,
                        self.cancel_token.clone(),
                    )
                    .await
                {
                    Ok((ctx_text, chunks)) if !chunks.is_empty() => {
                        tracing::info!(
                            chunk_count = chunks.len(),
                            "RAG context injected for introduction"
                        );
                        let _ = channel.send(ArenaEvent::RagContextInjected {
                            speaker_id: self.arbitre.config.id.clone(),
                            speaker_name: self.arbitre.config.name.clone(),
                            chunks,
                        });
                        intro_search_context = Some(match intro_search_context {
                            Some(existing) => format!("{existing}\n\n{ctx_text}"),
                            None => ctx_text,
                        });
                    }
                    Ok(_) => {} // Empty results
                    Err(e) => {
                        tracing::warn!(error = %e, "RAG query failed for introduction — continuing");
                    }
                }
            }
        }

        let participant_names: Vec<String> = self
            .gladiateurs
            .iter()
            .map(|g| g.config.name.clone())
            .collect();
        let intro_prompt = prompt_builder::build_introduction_prompt(
            &self.config.topic,
            &participant_names,
            &self.config.discussion_language,
            intro_search_context.as_deref(),
            &self.config.discussion_mode,
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

            // UserDriven mode: force user intervention first, then gladiateurs decide to respond
            let order = if self.config.discussion_mode == DiscussionMode::UserDriven {
                // 1. Force user intervention at start of each turn
                self.user_intervention_pending = true;
                self.user_intervention_handled = false;
                self.handle_user_intervention(&mut cmd_rx, &channel).await;

                if self.cancel_token.is_cancelled() { break; }
                if self.process_commands(&mut cmd_rx, &channel).await { break; }

                // 2. Each gladiateur decides to respond or pass
                let mut responding = Vec::new();
                for i in 0..self.gladiateurs.len() {
                    if self.gladiateurs[i].ban_remaining_turns > 0 { continue; }
                    if self.cancel_token.is_cancelled() { break; }
                    if self.ask_respond_or_pass(i).await {
                        responding.push(i);
                    }
                }

                if self.cancel_token.is_cancelled() { break; }

                // If no gladiateur wants to respond, skip the turn
                if responding.is_empty() {
                    let reason = match self.config.discussion_language.as_str() {
                        "en" => "No participant chose to respond this turn.".to_string(),
                        "zh" => "本轮没有参与者选择回应。".to_string(),
                        _ => "Aucun participant n'a choisi de répondre ce tour.".to_string(),
                    };
                    let _ = channel.send(ArenaEvent::TurnSkipped {
                        reason,
                        next_available_turn: self.current_turn + 1,
                    });
                    continue;
                }

                // Emit TurnStarted with only responding speakers
                let speaker_ids: Vec<String> = responding.iter()
                    .map(|&i| self.gladiateurs[i].config.id.clone())
                    .collect();
                let _ = channel.send(ArenaEvent::TurnStarted {
                    turn_number: self.current_turn,
                    speaker_order: speaker_ids,
                });

                responding
            } else {
            // CollaborativeFiction: force user to write the story opening on turn 1
            if self.config.discussion_mode == DiscussionMode::CollaborativeFiction
                && self.current_turn == 1
            {
                self.user_intervention_pending = true;
                self.user_intervention_handled = false;
                self.handle_user_intervention(&mut cmd_rx, &channel).await;
                if self.cancel_token.is_cancelled() { break; }
                if self.process_commands(&mut cmd_rx, &channel).await { break; }
            }

            // CollaborativeFiction: always use Sequential to maintain narrative continuity
            let effective_distribution = if self.config.discussion_mode == DiscussionMode::CollaborativeFiction {
                &TurnDistribution::Sequential
            } else {
                &self.config.arbitre.turn_distribution
            };

            // Determine speaker order — sync for Sequential/Random, async for Democratic/Authoritarian
            let order = match effective_distribution {
                TurnDistribution::Sequential | TurnDistribution::Random => {
                    turn_manager::determine_speaker_order(
                        &self.gladiateurs,
                        effective_distribution,
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

                    match effective_distribution {
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

            order
            }; // end of else (non-UserDriven)

            // Handle pending user intervention at start of turn (skip in UserDriven — user already spoke)
            if self.config.discussion_mode != DiscussionMode::UserDriven
                && self.user_intervention_pending
                && !self.user_intervention_handled
            {
                self.handle_user_intervention(&mut cmd_rx, &channel).await;
            }

            // Socratic mode: IArbitre poses a question each turn (starting turn 2)
            if self.config.discussion_mode == DiscussionMode::Socratic && self.current_turn > 1 {
                let arb_id = self.arbitre.config.id.clone();
                let arb_name = self.arbitre.config.name.clone();
                let _ = channel.send(ArenaEvent::SpeakerActive {
                    speaker_id: arb_id.clone(),
                });
                if let Some(question) = self.generate_socratic_question().await {
                    let msg = self.create_message(&arb_id, &arb_name, SpeakerRole::Arbitre, &question);
                    let _ = channel.send(ArenaEvent::MessageComplete { message: msg.clone() });
                    self.turn_messages.push(msg.clone());
                    self.messages_history.push(msg);
                }
            }

            // Reset cross-gladiateur search dedup for this turn
            self.turn_search_queries.clear();

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
                    // Snapshot reaction counts before this speaker's reactions
                    let reaction_snapshot: HashMap<String, (u32, u32)> =
                        self.turn_reaction_counts.clone();
                    self.process_reactions(glad_idx, &channel).await;
                    // Update cumulative reactions using only the DELTA (this speaker's reactions)
                    self.update_cumulative_reactions(&speaker_id, &reaction_snapshot);
                }

                // C.2.5 SEARCH (web + wiki, if enabled + quotas OK)
                // First turn: FORCED search for all enabled sources (ensure up-to-date context)
                // Subsequent turns: LLM decides whether to search
                let search_context: Option<String> = {
                    let global_usage = repository::get_tavily_usage(&self.db).await.unwrap_or_else(|e| {
                        tracing::warn!(error = %e, "Failed to read Tavily usage count — assuming 0");
                        0
                    });
                    let (web_can, web_max) = self.can_search_web(global_usage);
                    let (wiki_can, _wiki_max) = self.can_search_wiki();
                    let is_first_web = !self.forced_web_done.contains(&glad_idx);
                    let is_first_wiki = !self.forced_wiki_done.contains(&glad_idx);
                    let topic_q = truncate_str(&self.config.topic, constants::ORCH_TOPIC_FOR_SEARCH).to_string();
                    let recent = truncate_str(&self.build_recent_exchanges(glad_idx), constants::ORCH_RECENT_FOR_SEARCH).to_string();
                    let lang = self.config.discussion_language.clone();

                    tracing::info!(
                        speaker = %speaker_name,
                        web_can, wiki_can, is_first_web, is_first_wiki,
                        web_pool = self.config.web_search_pool,
                        wiki_pool = self.config.wiki_search_pool,
                        web_pool_used = self.web_searches_used_pool,
                        wiki_pool_used = self.wiki_searches_used_pool,
                        "Search capabilities"
                    );

                    let mut web_ctx: Option<String> = None;
                    let mut wiki_ctx: Option<String> = None;

                    // Build persona-aware search system prompt (brief extract for JSON utility calls)
                    let persona_extract = truncate_str(
                        &self.gladiateurs[glad_idx].config.system_prompt, constants::ORCH_PERSONA_FOR_SEARCH
                    );
                    let search_sys = match lang.as_str() {
                        "en" => format!(
                            "You are {}. {}\nRespond ONLY with valid JSON, no other text.",
                            speaker_name, persona_extract
                        ),
                        "zh" => format!(
                            "你是{}。{}\n仅用有效的JSON回复，不要有其他文本。",
                            speaker_name, persona_extract
                        ),
                        _ => format!(
                            "Tu es {}. {}\nRéponds UNIQUEMENT avec du JSON valide, aucun autre texte.",
                            speaker_name, persona_extract
                        ),
                    };

                    // Search architecture: ALWAYS web first, then wiki informed by web results.
                    // Phase 1 = forced first-turn, Phase 2 = LLM-decided subsequent turns.
                    let forced_web = web_can && is_first_web;
                    let forced_wiki = wiki_can && is_first_wiki;

                    // ── STEP 1: WEB SEARCH (always first when active) ──
                    if forced_web {
                        // Forced first-turn: LLM picks persona-specific query, fallback to topic
                        let web_remaining = self.config.web_search_pool
                            .saturating_sub(self.web_searches_used_pool);
                        let prompt = prompt_builder::build_web_search_decision_prompt(
                            &self.config.topic, &recent,
                            prompt_builder::default_search_directive(&lang),
                            web_remaining, &lang,
                            &self.gladiateurs[glad_idx].search_queries_history,
                            &self.turn_search_queries,
                        );
                        let query = self.pick_forced_query(
                            &search_sys, &prompt,
                            &self.gladiateurs[glad_idx].config.llm_params,
                            topic_q.clone(), &speaker_name, "web",
                        ).await;
                        tracing::info!(speaker = %speaker_name, query = %query, "Forced first-turn web query");
                        self.gladiateurs[glad_idx].search_queries_history.push(query.clone());
                        let (ctx, count, _) = self.process_web_search(
                            &search_sys, &speaker_id, &speaker_name, web_max, "", "",
                            Some(vec![query.clone()]),
                            &self.gladiateurs[glad_idx].config.llm_params, &channel,
                            &self.gladiateurs[glad_idx].search_queries_history,
                            &self.turn_search_queries,
                        ).await;
                        self.web_searches_used_pool += count;
                        self.forced_web_done.insert(glad_idx);
                        self.turn_search_queries.push((speaker_name.clone(), query));
                        web_ctx = ctx;
                    } else if web_can {
                        // Phase 2: LLM decides whether to search
                        let directive = prompt_builder::default_search_directive(&lang);
                        let (ctx, count, executed) = self.process_web_search(
                            &search_sys, &speaker_id, &speaker_name, web_max, directive, &recent,
                            None, &self.gladiateurs[glad_idx].config.llm_params, &channel,
                            &self.gladiateurs[glad_idx].search_queries_history,
                            &self.turn_search_queries,
                        ).await;
                        self.web_searches_used_pool += count;
                        self.gladiateurs[glad_idx].search_queries_history.extend(executed.iter().cloned());
                        for q in &executed {
                            self.turn_search_queries.push((speaker_name.clone(), q.clone()));
                        }
                        web_ctx = ctx;
                    }

                    // ── STEP 2: WIKI SEARCH (after web, informed by web results) ──
                    if forced_wiki {
                        // Forced first-turn: LLM picks persona-specific concept
                        // Fallback = speaker_name (guarantees unique query per gladiateur)
                        let wiki_remaining = self.config.wiki_search_pool
                            .saturating_sub(self.wiki_searches_used_pool);
                        let prompt = prompt_builder::build_wiki_search_decision_prompt(
                            &self.config.topic, &recent,
                            prompt_builder::default_wiki_directive(&lang),
                            wiki_remaining, &lang,
                            &self.gladiateurs[glad_idx].search_queries_history,
                            web_ctx.as_deref(),
                            &self.turn_search_queries,
                        );
                        let query = self.pick_forced_query(
                            &search_sys, &prompt,
                            &self.gladiateurs[glad_idx].config.llm_params,
                            speaker_name.clone(), &speaker_name, "wiki",
                        ).await;
                        tracing::info!(speaker = %speaker_name, query = %query, "Forced first-turn wiki query");
                        self.gladiateurs[glad_idx].search_queries_history.push(query.clone());
                        let (ctx, count) = self.process_wiki_search(
                            glad_idx, query.clone(), &channel,
                        ).await;
                        self.wiki_searches_used_pool += count;
                        self.forced_wiki_done.insert(glad_idx);
                        self.turn_search_queries.push((speaker_name.clone(), query));
                        wiki_ctx = ctx;
                    } else if wiki_can {
                        // Phase 2: LLM decides whether to search (with web context if available)
                        let directive = prompt_builder::default_wiki_directive(&lang);
                        let wiki_remaining = self.config.wiki_search_pool
                            .saturating_sub(self.wiki_searches_used_pool);
                        let prompt = prompt_builder::build_wiki_search_decision_prompt(
                            &self.config.topic, &recent, directive, wiki_remaining, &lang,
                            &self.gladiateurs[glad_idx].search_queries_history,
                            web_ctx.as_deref(),
                            &self.turn_search_queries,
                        );
                        let query = self.pick_forced_query(
                            &search_sys, &prompt,
                            &self.gladiateurs[glad_idx].config.llm_params,
                            String::new(), &speaker_name, "wiki",
                        ).await;
                        if !query.is_empty() {
                            self.gladiateurs[glad_idx].search_queries_history.push(query.clone());
                            let (ctx, count) = self.process_wiki_search(
                                glad_idx, query.clone(), &channel,
                            ).await;
                            self.wiki_searches_used_pool += count;
                            self.turn_search_queries.push((speaker_name.clone(), query));
                            wiki_ctx = ctx;
                        }
                    }

                    // Combine web + wiki contexts
                    let mut search_ctx: Option<String> = match (web_ctx, wiki_ctx) {
                        (Some(w), Some(wiki)) => Some(format!("{w}\n\n{wiki}")),
                        (Some(w), None) => Some(w),
                        (None, Some(wiki)) => Some(wiki),
                        (None, None) => None,
                    };

                    // RAG knowledge base query (after web + wiki)
                    let rag_ctx = self.process_rag_query(
                        &speaker_id, &speaker_name, glad_idx, &channel,
                    ).await;
                    if let Some(rag) = rag_ctx {
                        search_ctx = Some(match search_ctx {
                            Some(existing) => format!("{existing}\n\n{rag}"),
                            None => rag,
                        });
                    }

                    search_ctx
                };

                if let Some(ref ctx) = search_context {
                    tracing::info!(
                        speaker = %speaker_name,
                        turn = self.current_turn,
                        search_ctx_len = ctx.len(),
                        "Search context ready for injection"
                    );
                }

                // C.2.8 DYNAMIC DIRECTIVE (emotion_driven only)
                let dynamic_directive: Option<String> = if self.emotion_driven {
                    let directive_output = self.build_directive_for_speaker(glad_idx);
                    tracing::info!(
                        speaker = %speaker_name,
                        turn = self.current_turn,
                        speech_act = %directive_output.speech_act,
                        "Dynamic directive generated"
                    );
                    let _ = channel.send(ArenaEvent::DirectiveGenerated {
                        speaker_id: speaker_id.clone(),
                        speaker_name: speaker_name.clone(),
                        speech_act: directive_output.speech_act.clone(),
                        emotion_behavior: directive_output.emotion_behavior.clone(),
                        relationship_summary: directive_output.relationship_summary.clone(),
                    });
                    Some(directive_output.directive_text)
                } else {
                    None
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
                    let (t, c) = self.process_intervention_think(glad_idx, search_context.as_deref(), dynamic_directive.as_deref(), &channel).await;
                    // If think mode failed (HTTP 400 = model doesn't support it),
                    // fall back to the normal thought + intervention path
                    if c.is_none() {
                        tracing::info!(
                            speaker = %speaker_name,
                            "Think mode produced no content, falling back to normal intervention"
                        );
                        let thought = self.process_thought(glad_idx, search_context.as_deref(), &channel).await;
                        let content = self
                            .process_intervention(glad_idx, thought.as_deref(), search_context.as_deref(), dynamic_directive.as_deref(), &channel)
                            .await;
                        (thought, content)
                    } else {
                        (t, c)
                    }
                } else {
                    let thought = self.process_thought(glad_idx, search_context.as_deref(), &channel).await;
                    let content = self
                        .process_intervention(glad_idx, thought.as_deref(), search_context.as_deref(), dynamic_directive.as_deref(), &channel)
                        .await;
                    (thought, content)
                };

                if let Some(text) = &content {
                    // Defense in depth: strip any <document> tags the LLM may have generated
                    // despite not having document context. The extracted doc is ignored —
                    // Pass 2 is authoritative for document updates.
                    let (discussion_text, _) = json_parser::extract_and_strip_document(text);

                    tracing::info!(
                        speaker = %speaker_name,
                        turn = self.current_turn,
                        len = discussion_text.len(),
                        had_search = search_context.is_some(),
                        preview = %truncate_str(&discussion_text, 200),
                        "GladIAteur response preview"
                    );
                    let mut msg = self.create_message(
                        &speaker_id,
                        &speaker_name,
                        SpeakerRole::Gladiateur,
                        &discussion_text,
                    );
                    msg.inner_thought = thought.clone();
                    let _ = channel.send(ArenaEvent::MessageComplete {
                        message: msg.clone(),
                    });
                    self.turn_messages.push(msg.clone());
                    self.messages_history.push(msg);

                    // Update self-memory for anti-repetition (keep last 2 messages)
                    let own_msgs = self.speaker_own_messages
                        .entry(speaker_id.clone())
                        .or_default();
                    own_msgs.push(discussion_text.clone());
                    if own_msgs.len() > 2 {
                        own_msgs.remove(0);
                    }

                    // Pass 2: Generate document update via separate LLM call
                    let llm_params = self.gladiateurs[glad_idx].config.llm_params.clone();
                    if let Some(updated_doc) = self.generate_document_update(
                        &speaker_name,
                        &discussion_text,
                        &llm_params,
                        &channel,
                    ).await {
                        self.document_content = updated_doc.clone();
                        let _ = channel.send(ArenaEvent::DocumentUpdated {
                            speaker_id: speaker_id.clone(),
                            speaker_name: speaker_name.clone(),
                            content: updated_doc,
                            format: self.config.document_format.as_extension().to_string(),
                        });
                    }
                }

                // C.5 EMOTION UPDATE (rule-based, instant)
                self.update_emotions(glad_idx, &channel);

                // C.6 MODERATION
                if let Some(text) = &content {
                    self.process_moderation(glad_idx, text, &channel).await;
                    // Update arbitre emotions based on moderation outcome
                    let ban_issued = self.gladiateurs[glad_idx].ban_issued_this_turn;
                    self.update_arbitre_emotions(ban_issued, &channel);
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

            // E. END OF TURN — emotion analysis + contagion + history + memory update
            // All sequential because they mutate &mut self

            // E.1 LLM emotion analysis (1 call for ALL participants)
            self.analyze_emotions_llm(&channel).await;

            // E.2 Emotional contagion (order-independent)
            self.apply_emotional_contagion(&channel);

            // E.3 Snapshot history + emit EmotionHistoryUpdate
            self.record_emotion_history(&channel);

            // E.4 Memory update (existing)
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

        // --- SYNTHESIS (always, even on force-stop) ---
        // If the cancel token was triggered (force-stop), create a fresh one
        // so the synthesis LLM call can proceed without immediate cancellation.
        if self.cancel_token.is_cancelled() {
            self.cancel_token = CancellationToken::new();
        }
        tracing::info!(
            discussion_id = %self.discussion_id,
            status = ?self.status,
            turns_completed = self.current_turn,
            "Starting synthesis generation"
        );
        self.generate_synthesis(None, &channel).await;
        tracing::info!(discussion_id = %self.discussion_id, "Synthesis generation complete");

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
                EngineCommand::AdjustEmotion { speaker_id, axis, value } => {
                    self.handle_adjust_emotion(&speaker_id, &axis, value, channel);
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
            prompt_builder::build_reaction_prompt(&prev_interventions, &self.config.discussion_language, &self.config.discussion_mode);
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
                                justification: parsed.justification.clone(),
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

    /// Update cumulative reactions from only the DELTA added by the current speaker.
    /// `before` is a snapshot of `turn_reaction_counts` taken before `process_reactions()`.
    fn update_cumulative_reactions(
        &mut self,
        speaker_id: &str,
        before: &HashMap<String, (u32, u32)>,
    ) {
        for (target_id, &(total_likes, total_dislikes)) in &self.turn_reaction_counts {
            let (prev_likes, prev_dislikes) =
                before.get(target_id).copied().unwrap_or((0, 0));
            let new_likes = total_likes.saturating_sub(prev_likes);
            let new_dislikes = total_dislikes.saturating_sub(prev_dislikes);
            if new_likes > 0 || new_dislikes > 0 {
                let entry = self
                    .cumulative_reactions
                    .entry((speaker_id.to_string(), target_id.clone()))
                    .or_insert((0, 0));
                entry.0 += new_likes;
                entry.1 += new_dislikes;
            }
        }
    }

    /// Build the full directive for a specific speaker using the directive builder.
    fn build_directive_for_speaker(&mut self, glad_idx: usize) -> directive_builder::DirectiveOutput {
        let speaker_id = &self.gladiateurs[glad_idx].config.id;
        let speaker_name = &self.gladiateurs[glad_idx].config.name;

        // Build relationships from cumulative reactions
        let reactions_from_me: Vec<(String, String, u32, u32)> = self.gladiateurs
            .iter()
            .filter(|g| g.config.id != *speaker_id)
            .filter_map(|g| {
                let key = (speaker_id.clone(), g.config.id.clone());
                self.cumulative_reactions.get(&key).map(|(l, d)| {
                    (g.config.id.clone(), g.config.name.clone(), *l, *d)
                })
            })
            .collect();

        let reactions_to_me: Vec<(String, String, u32, u32)> = self.gladiateurs
            .iter()
            .filter(|g| g.config.id != *speaker_id)
            .filter_map(|g| {
                let key = (g.config.id.clone(), speaker_id.clone());
                self.cumulative_reactions.get(&key).map(|(l, d)| {
                    (g.config.id.clone(), g.config.name.clone(), *l, *d)
                })
            })
            .collect();

        let relationships = directive_builder::build_relationships(&reactions_from_me, &reactions_to_me);

        // Compute group averages
        let active: Vec<&EmotionalProfile> = self.gladiateurs
            .iter()
            .filter(|g| !g.is_banned())
            .map(|g| &g.emotions)
            .collect();
        let (avg_frustration, avg_engagement) = if active.is_empty() {
            (50, 50)
        } else {
            let sum_f: u32 = active.iter().map(|e| e.frustration as u32).sum();
            let sum_e: u32 = active.iter().map(|e| e.engagement as u32).sum();
            let n = active.len() as u32;
            ((sum_f / n) as u8, (sum_e / n) as u8)
        };

        // Speakers who have already spoken this turn
        let speakers_this_turn: Vec<String> = self.turn_messages
            .iter()
            .filter(|m| m.role == SpeakerRole::Gladiateur)
            .map(|m| m.speaker_name.clone())
            .collect();

        let ctx = SpeakerTurnContext {
            emotions: self.gladiateurs[glad_idx].emotions.clone(),
            relationships,
            own_previous_messages: self.speaker_own_messages
                .get(speaker_id)
                .cloned()
                .unwrap_or_default(),
            dynamics: self.dynamics_cache.get(speaker_id).cloned(),
            ocean: prompt_builder::parse_ocean_values(&self.gladiateurs[glad_idx].config.system_prompt),
            turn_number: self.current_turn,
            speakers_this_turn,
            is_first_speaker_this_turn: self.turn_messages.is_empty(),
            was_recently_banned: self.gladiateurs[glad_idx].ban_remaining_turns == 0
                && self.current_turn > 1
                && self.messages_history.iter().any(|m| {
                    m.is_ban_notification && m.content.contains(speaker_name)
                    && m.turn_number >= self.current_turn.saturating_sub(2)
                }),
            group_avg_frustration: avg_frustration,
            group_avg_engagement: avg_engagement,
            discussion_language: self.config.discussion_language.clone(),
            user_name: self.config.user_name.clone(),
            discussion_mode: self.config.discussion_mode.clone(),
        };

        let last_act = self.last_speech_acts.get(speaker_id);
        let output = directive_builder::build_dynamic_directive(&ctx, last_act);

        // Store last speech act for anti-repetition
        if let Some(act) = SpeechAct::from_name(&output.speech_act) {
            self.last_speech_acts.insert(speaker_id.clone(), act);
        }

        output
    }

    /// Build a string with recent exchanges for the thought prompt context
    fn build_recent_exchanges(&self, glad_idx: usize) -> String {
        let mut recent = String::new();
        let is_fiction = self.config.discussion_mode == DiscussionMode::CollaborativeFiction;

        // Previous turn messages (from messages_history) — including IArbitre directives
        if self.current_turn > 1 {
            let prev_turn = self.current_turn - 1;
            for m in &self.messages_history {
                if m.turn_number == prev_turn {
                    if m.role == SpeakerRole::Arbitre {
                        // Emphasize moderator directives so GladIAteurs notice them
                        recent.push_str(&format!(
                            "[MODERATOR] {}: {}\n",
                            m.speaker_name,
                            truncate_str(&m.content, constants::ORCH_EXCHANGE_MODERATOR)
                        ));
                    } else if is_fiction {
                        // Fiction: show full content for narrative continuity
                        recent.push_str(&format!(
                            "--- {} ---\n{}\n\n",
                            m.speaker_name, m.content
                        ));
                    } else {
                        recent.push_str(&format!(
                            "{}: {}\n",
                            m.speaker_name,
                            truncate_str(&m.content, constants::ORCH_EXCHANGE_GENERIC)
                        ));
                    }
                }
            }
        }

        // Current turn messages so far
        for m in &self.turn_messages {
            if m.speaker_id != self.gladiateurs[glad_idx].config.id {
                if m.role == SpeakerRole::Arbitre {
                    recent.push_str(&format!(
                        "[MODERATOR] {}: {}\n",
                        m.speaker_name,
                        truncate_str(&m.content, constants::ORCH_EXCHANGE_MODERATOR)
                    ));
                } else if is_fiction {
                    // Fiction: show full content for narrative continuity
                    recent.push_str(&format!(
                        "--- {} ---\n{}\n\n",
                        m.speaker_name, m.content
                    ));
                } else {
                    recent.push_str(&format!(
                        "{}: {}\n",
                        m.speaker_name,
                        truncate_str(&m.content, constants::ORCH_EXCHANGE_GENERIC)
                    ));
                }
            }
        }

        recent
    }

    async fn process_thought(
        &self,
        glad_idx: usize,
        search_results: Option<&str>,
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
            search_results,
            &self.config.discussion_mode,
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
        search_results: Option<&str>,
        dynamic_directive: Option<&str>,
        channel: &Channel<ArenaEvent>,
    ) -> Option<String> {
        // Exclude current speaker from participant names to prevent self-addressing
        let other_names: Vec<String> = self.gladiateurs.iter()
            .enumerate()
            .filter(|(i, _)| *i != glad_idx)
            .map(|(_, g)| g.config.name.clone())
            .collect();
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
            search_results,
            &other_names,
            dynamic_directive,
            &self.config.discussion_mode,
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
                params.temperature = (params.temperature + constants::TEMP_DIFFICULTY_BOOST).min(constants::TEMP_MAX);
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
                params.temperature = (params.temperature + constants::TEMP_REFUSAL_BOOST).min(constants::TEMP_MAX);
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

        let mut probability: f64 = constants::THINK_BASE_PROBABILITY;

        // High frustration → more likely to think deeply
        if emo.frustration > constants::THINK_FRUSTRATION_THRESHOLD {
            probability += constants::THINK_FRUSTRATION_BOOST;
        }

        // High engagement → invested, thinks more
        if emo.engagement > constants::THINK_ENGAGEMENT_THRESHOLD {
            probability += constants::THINK_ENGAGEMENT_BOOST;
        }

        // End of discussion → synthesize thoughts
        if let Some(max) = self.config.max_turns {
            if self.current_turn + constants::THINK_NEAR_END_TURNS >= max {
                probability += constants::THINK_NEAR_END_BOOST;
            }
        }

        // Was contradicted → needs to think about response
        let (_, dislikes) = self
            .turn_reaction_counts
            .get(&self.gladiateurs[glad_idx].config.id)
            .copied()
            .unwrap_or((0, 0));
        if dislikes >= constants::EMOTION_CONTRADICTION_THRESHOLD {
            probability += constants::THINK_CONTRADICTED_BOOST;
        }

        // Cap to keep it non-systematic
        probability = probability.min(constants::THINK_MAX_PROBABILITY);

        use rand::Rng;
        rand::thread_rng().gen_bool(probability)
    }

    /// Process intervention with think mode — model reasons internally, replacing separate thought phase
    async fn process_intervention_think(
        &self,
        glad_idx: usize,
        search_results: Option<&str>,
        dynamic_directive: Option<&str>,
        channel: &Channel<ArenaEvent>,
    ) -> (Option<String>, Option<String>) {
        // Exclude current speaker from participant names to prevent self-addressing
        let other_names: Vec<String> = self.gladiateurs.iter()
            .enumerate()
            .filter(|(i, _)| *i != glad_idx)
            .map(|(_, g)| g.config.name.clone())
            .collect();
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
            search_results,
            &other_names,
            dynamic_directive,
            &self.config.discussion_mode,
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

        // Clone before update for threshold detection
        let prev = self.gladiateurs[glad_idx].emotions.clone();
        let new_emo = emotion_engine::update_emotions(&prev, &ctx);
        self.gladiateurs[glad_idx].emotions = new_emo.clone();

        Self::emit_threshold_events(channel, &sid, &prev, &new_emo);
        Self::emit_emotion_updated(channel, &sid, &new_emo, &self.config.discussion_language);
    }

    /// Rule-based emotion update for IArbitre (limited: bans + stagnation only)
    fn update_arbitre_emotions(&mut self, ban_issued: bool, channel: &Channel<ArenaEvent>) {
        let prev = self.arbitre.emotions.clone();

        if ban_issued {
            self.arbitre.emotions.frustration = emotion_engine::add_clamped(self.arbitre.emotions.frustration, 5);
            self.arbitre.emotions.confiance = emotion_engine::add_clamped(self.arbitre.emotions.confiance, 5);
        }
        if self.current_turn > 3 {
            self.arbitre.emotions.engagement = emotion_engine::sub_clamped(self.arbitre.emotions.engagement, 3);
        }

        Self::emit_threshold_events(channel, &self.arbitre.config.id, &prev, &self.arbitre.emotions);
        Self::emit_emotion_updated(channel, &self.arbitre.config.id, &self.arbitre.emotions, &self.config.discussion_language);
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
            &self.config.discussion_mode,
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
                let duration = moderation.ban_duration.clamp(constants::MODERATION_BAN_MIN_TURNS, constants::MODERATION_BAN_MAX_TURNS);
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
                        Some(EngineCommand::AdjustEmotion { speaker_id, axis, value }) => {
                            self.handle_adjust_emotion(&speaker_id, &axis, value, channel);
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
        let is_fiction = self.config.discussion_mode == DiscussionMode::CollaborativeFiction;
        for g in &mut self.gladiateurs {
            memory_manager::add_turn_to_memory(&mut g.memory, turn, &self.turn_messages, is_fiction);
        }
        memory_manager::add_turn_to_memory(&mut self.arbitre.memory, turn, &self.turn_messages, is_fiction);

        // LLM-based contextual + positional update
        let contextual_summary = &self.arbitre.memory.contextual_summary;
        let positional_json = memory_manager::positional_map_to_json(&self.arbitre.memory);
        let turn_text = memory_manager::format_turn_messages(&self.turn_messages, is_fiction);
        let prompt = prompt_builder::build_memory_update_prompt(
            contextual_summary,
            &positional_json,
            turn,
            &turn_text,
            &self.config.discussion_language,
            &self.config.discussion_mode,
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
                retry_params.temperature = (retry_params.temperature + constants::TEMP_DIFFICULTY_BOOST).min(constants::TEMP_MAX);
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

    async fn generate_synthesis(&self, search_results: Option<&str>, channel: &Channel<ArenaEvent>) {
        let document_context = self.build_document_context_for_synthesis();
        let prompt = prompt_builder::build_synthesis_prompt(
            &self.config.topic,
            &self.arbitre.memory,
            &self.config.discussion_language,
            search_results,
            &self.config.discussion_mode,
            document_context.as_deref(),
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

    /// Check if the web search pool has remaining credits.
    /// Returns (can_search, max_queries_this_turn).
    /// Pool is shared between all gladiateurs, max 1 per gladiateur per turn.
    fn can_search_web(&self, global_usage: u32) -> (bool, u32) {
        let pool = self.config.web_search_pool;
        if pool == 0 || self.tavily_client.is_none() {
            return (false, 0);
        }
        let pool_remaining = pool.saturating_sub(self.web_searches_used_pool);
        let max_queries = pool_remaining.min(1);
        let global_remaining = constants::TAVILY_FREE_MONTHLY_QUOTA.saturating_sub(global_usage);
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
        channel: &Channel<ArenaEvent>,
        past_queries: &[String],
        other_queries: &[(String, String)],
    ) -> (Option<String>, u32, Vec<String>) {
        // 1. Determine queries
        let queries: Vec<String> = if let Some(forced) = forced_queries {
            forced.into_iter().take(max_queries as usize).collect()
        } else {
            // LLM decision (non-streaming, JSON)
            let mut decision_params = llm_params.clone();
            decision_params.temperature = constants::TEMP_VOTING;

            let prompt = prompt_builder::build_web_search_decision_prompt(
                &self.config.topic,
                recent_context,
                search_directive,
                max_queries,
                &self.config.discussion_language,
                past_queries,
                other_queries,
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
                    return (None, 0, Vec::new());
                }
            };
            let decision = json_parser::parse_json_response::<json_parser::SearchDecisionResponse>(&raw)
                .unwrap_or_default();

            if !decision.needs_search || decision.queries.is_empty() {
                return (None, 0, Vec::new());
            }
            decision.queries.into_iter().take(max_queries as usize).collect()
        };

        if queries.is_empty() {
            return (None, 0, Vec::new());
        }

        // 2. Execute each search
        let tavily = match self.tavily_client.as_ref() {
            Some(c) => c,
            None => return (None, 0, Vec::new()),
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
            return (None, 0, Vec::new());
        }

        // 3. Emit batched event
        let executed_queries: Vec<String> = all_results.iter().map(|(q, _)| q.clone()).collect();
        let total_results: u32 = all_results.iter().map(|(_, r)| r.results.len() as u32).sum();
        let _ = channel.send(ArenaEvent::WebSearchPerformed {
            speaker_id: speaker_id.to_string(),
            speaker_name: speaker_name.to_string(),
            queries: executed_queries.clone(),
            results_count: total_results,
            pool_used: self.web_searches_used_pool + executed_count,
        });

        // 4. Format for prompt injection
        let lang = &self.config.discussion_language;
        let ctx = prompt_builder::build_search_results_context(&all_results, lang);
        tracing::info!(
            speaker = %speaker_name,
            ctx_len = ctx.len(),
            ctx_preview = %truncate_str(&ctx, 300),
            "Web search context injected into prompt"
        );
        (
            Some(ctx),
            executed_count,
            executed_queries,
        )
    }

    // ===== Search helpers =====

    /// Ask the LLM to pick a search query for forced first-turn search.
    /// Returns the LLM-chosen query, or `fallback` if LLM fails/returns empty.
    async fn pick_forced_query(
        &self,
        system_prompt: &str,
        prompt: &str,
        llm_params: &LlmParams,
        fallback: String,
        speaker_name: &str,
        search_type: &str,
    ) -> String {
        let mut dp = llm_params.clone();
        dp.temperature = constants::TEMP_VOTING;
        let request = self.ollama_client.build_request(system_prompt, prompt, &dp, true);
        let decision = match self.ollama_client.chat(&request, self.cancel_token.clone()).await {
            Ok(raw) => {
                tracing::info!(speaker = %speaker_name, search_type, raw = %raw, "Forced search query LLM response");
                json_parser::parse_json_response::<json_parser::SearchDecisionResponse>(&raw)
                    .unwrap_or_default()
            }
            Err(e) => {
                tracing::warn!(speaker = %speaker_name, search_type, error = %e, "Forced search query LLM call failed");
                json_parser::SearchDecisionResponse::default()
            }
        };
        if !decision.queries.is_empty() {
            decision.queries.into_iter().next().unwrap()
        } else {
            fallback
        }
    }

    // ===== Wikipedia Search =====

    /// Build clickable Wikipedia article URLs from a search response.
    fn build_article_urls(response: &crate::wikipedia::WikiSearchResponse, wiki_lang: &str) -> Vec<String> {
        response.query.as_ref()
            .map(|q| q.pages.iter().map(|p| {
                format!("https://{}.wikipedia.org/wiki/{}", wiki_lang, p.title.replace(' ', "_"))
            }).collect())
            .unwrap_or_default()
    }

    /// Check if the wiki search pool has remaining credits.
    /// Returns (can_search, max_queries_this_turn).
    /// Pool is shared between all gladiateurs, max 1 per gladiateur per turn.
    fn can_search_wiki(&self) -> (bool, u32) {
        let pool = self.config.wiki_search_pool;
        if pool == 0 {
            return (false, 0);
        }
        let remaining = pool.saturating_sub(self.wiki_searches_used_pool);
        let max_queries = remaining.min(1);
        (max_queries > 0, max_queries)
    }

    /// Raw Wikipedia search for intro (returns formatted context, emits WikiSearchPerformed).
    async fn process_wiki_search_intro(
        &self,
        query: &str,
        speaker_id: &str,
        speaker_name: &str,
        channel: &Channel<ArenaEvent>,
    ) -> Option<String> {
        let lang = &self.config.discussion_language;
        tracing::info!(query = %query, lang = %lang, speaker = %speaker_name, "Wikipedia intro search");
        match self.wiki_client.search(query, lang, self.cancel_token.clone()).await {
            Ok((response, actual_lang)) => {
                let has_results = response.query.as_ref().is_some_and(|q| !q.pages.is_empty());
                if has_results {
                    let article_urls = Self::build_article_urls(&response, &actual_lang);
                    let results_count = response.query.as_ref().map(|q| q.pages.len() as u32).unwrap_or(0);

                    tracing::info!(speaker = %speaker_name, urls = ?article_urls, "Wikipedia intro results found");

                    // Emit event so the UI shows the wiki badge on the intro message
                    let _ = channel.send(ArenaEvent::WikiSearchPerformed {
                        speaker_id: speaker_id.to_string(),
                        speaker_name: speaker_name.to_string(),
                        queries: vec![query.to_string()],
                        results_count,
                        pool_used: 0, // IArbitre intro does not consume pool
                        article_urls,
                    });

                    let results = vec![(query.to_string(), response)];
                    let ctx = prompt_builder::build_wiki_results_context(&results, lang);
                    tracing::info!(
                        speaker = %speaker_name,
                        ctx_len = ctx.len(),
                        ctx_preview = %truncate_str(&ctx, 300),
                        "Wikipedia intro context injected into prompt"
                    );
                    Some(ctx)
                } else {
                    tracing::info!("Wikipedia intro search returned no results");
                    None
                }
            }
            Err(crate::wikipedia::error::WikiError::Cancelled) => None,
            Err(e) => {
                tracing::warn!(error = %e, "Wikipedia intro search failed");
                None
            }
        }
    }

    /// Execute Wikipedia search for a gladiateur. Returns (formatted_context, queries_executed_count).
    async fn process_wiki_search(
        &self,
        glad_idx: usize,
        query: String,
        channel: &Channel<ArenaEvent>,
    ) -> (Option<String>, u32) {
        let lang = &self.config.discussion_language;
        let speaker_id = &self.gladiateurs[glad_idx].config.id;
        let speaker_name = &self.gladiateurs[glad_idx].config.name;

        tracing::info!(speaker = %speaker_name, query = %query, lang = %lang, "Wikipedia search for gladiateur");

        let (response, actual_lang) = match self.wiki_client.search(&query, lang, self.cancel_token.clone()).await {
            Ok(r) => r,
            Err(crate::wikipedia::error::WikiError::Cancelled) => return (None, 0),
            Err(e) => {
                tracing::warn!(query = %query, error = %e, "Wikipedia search failed");
                return (None, 0);
            }
        };

        let has_results = response.query.as_ref().is_some_and(|q| !q.pages.is_empty());
        if !has_results {
            tracing::info!(speaker = %speaker_name, query = %query, "Wikipedia returned no results");
            return (None, 0);
        }

        let article_urls = Self::build_article_urls(&response, &actual_lang);

        tracing::info!(speaker = %speaker_name, urls = ?article_urls, "Wikipedia results found");

        let results_count = response.query.as_ref().map(|q| q.pages.len() as u32).unwrap_or(0);

        // Emit event
        let _ = channel.send(ArenaEvent::WikiSearchPerformed {
            speaker_id: speaker_id.to_string(),
            speaker_name: speaker_name.to_string(),
            queries: vec![query.clone()],
            results_count,
            pool_used: self.wiki_searches_used_pool + 1,
            article_urls,
        });

        // Format for prompt injection
        let all_results = vec![(query, response)];
        let ctx = prompt_builder::build_wiki_results_context(&all_results, lang);
        tracing::info!(
            speaker = %speaker_name,
            ctx_len = ctx.len(),
            ctx_preview = %truncate_str(&ctx, 300),
            "Wikipedia context injected into prompt"
        );
        (Some(ctx), 1)
    }

    // ── RAG knowledge base query ──────────────────────────────────────

    /// Query the RAG store for relevant chunks and emit event.
    /// Returns formatted context string for prompt injection, or None.
    async fn process_rag_query(
        &self,
        speaker_id: &str,
        speaker_name: &str,
        glad_idx: usize,
        channel: &Channel<ArenaEvent>,
    ) -> Option<String> {
        let rag_store = self.rag_store.as_ref()?;
        if rag_store.is_empty() {
            return None;
        }

        // Build query context: topic + recent exchanges (reuse existing helpers)
        let topic = truncate_str(&self.config.topic, constants::ORCH_TOPIC_FOR_SEARCH);
        let recent_raw = self.build_recent_exchanges(glad_idx);
        let recent = truncate_str(&recent_raw, constants::ORCH_RECENT_FOR_SEARCH);
        let context = format!("{topic}\n\n{recent}");
        let lang = &self.config.discussion_language;

        match rag_store
            .query(
                &context,
                lang,
                &self.ollama_client,
                &self.gladiateurs[glad_idx].config.llm_params,
                self.cancel_token.clone(),
            )
            .await
        {
            Ok((ctx_text, chunks)) if !chunks.is_empty() => {
                tracing::info!(
                    speaker = %speaker_name,
                    turn = self.current_turn,
                    chunk_count = chunks.len(),
                    ctx_len = ctx_text.len(),
                    "RAG context injected"
                );
                let _ = channel.send(ArenaEvent::RagContextInjected {
                    speaker_id: speaker_id.to_string(),
                    speaker_name: speaker_name.to_string(),
                    chunks,
                });
                Some(ctx_text)
            }
            Ok(_) => None, // Empty results
            Err(crate::ollama::error::OllamaError::Cancelled) => None,
            Err(e) => {
                tracing::warn!(
                    speaker = %speaker_name,
                    error = %e,
                    "RAG query failed — continuing without knowledge base"
                );
                None
            }
        }
    }

    // ── Emotion analysis, contagion, history ──────────────────────────

    /// LLM-based emotion analysis: 1 call for ALL participants.
    /// Graceful fallback: if LLM fails, log warning and keep rule-based values.
    async fn analyze_emotions_llm(&mut self, channel: &Channel<ArenaEvent>) {
        if self.turn_messages.is_empty() || self.cancel_token.is_cancelled() {
            return;
        }

        // Build participants JSON for the prompt
        let mut participants_info = Vec::new();
        participants_info.push(format!(
            "  \"{}\": {{\"role\": \"IArbitre\", \"engagement\": {}, \"accord\": {}, \"confiance\": {}, \"frustration\": {}, \"curiosite\": {}, \"enthousiasme\": {}}}",
            self.arbitre.config.name,
            self.arbitre.emotions.engagement, self.arbitre.emotions.accord,
            self.arbitre.emotions.confiance, self.arbitre.emotions.frustration,
            self.arbitre.emotions.curiosite, self.arbitre.emotions.enthousiasme,
        ));
        for g in &self.gladiateurs {
            participants_info.push(format!(
                "  \"{}\": {{\"role\": \"GladIAteur\", \"engagement\": {}, \"accord\": {}, \"confiance\": {}, \"frustration\": {}, \"curiosite\": {}, \"enthousiasme\": {}}}",
                g.config.name,
                g.emotions.engagement, g.emotions.accord,
                g.emotions.confiance, g.emotions.frustration,
                g.emotions.curiosite, g.emotions.enthousiasme,
            ));
        }
        let participants_json = format!("{{\n{}\n}}", participants_info.join(",\n"));

        // Build recent context from turn messages
        let recent_context = self.turn_messages.iter()
            .map(|m| format!("[{}] {}", m.speaker_name, truncate_str(&m.content, constants::ORCH_EMOTION_CONTEXT)))
            .collect::<Vec<_>>()
            .join("\n");

        // Build events summary (reactions, bans)
        let mut events = Vec::new();
        for (sid, (likes, dislikes)) in &self.turn_reaction_counts {
            let name = self.gladiateurs.iter()
                .find(|g| g.config.id == *sid)
                .map(|g| g.config.name.as_str())
                .unwrap_or(sid);
            if *likes > 0 { events.push(format!("{} received {} like(s)", name, likes)); }
            if *dislikes > 0 { events.push(format!("{} received {} dislike(s)", name, dislikes)); }
        }
        for g in &self.gladiateurs {
            if g.ban_issued_this_turn {
                events.push(format!("{} was banned this turn", g.config.name));
            }
        }
        let events_summary = if events.is_empty() {
            "No notable events".to_string()
        } else {
            events.join(", ")
        };

        let prompt = prompt_builder::build_emotion_analysis_prompt(
            &participants_json,
            &recent_context,
            &events_summary,
            &self.config.discussion_language,
        );

        let sys_prompt = match self.config.discussion_language.as_str() {
            "en" => "You are an emotion analyst. Base your analysis strictly on the exchanges and reactions provided. Do not invent events. Respond only with JSON.",
            "zh" => "你是情绪分析师。严格根据提供的交流和反应进行分析。不要捏造事件。仅用JSON回复。",
            _ => "Tu es un analyste émotionnel. Base ton analyse strictement sur les échanges et réactions fournis. N'invente pas d'événements. Réponds uniquement en JSON.",
        };

        let request = self.ollama_client.build_request(
            sys_prompt,
            &prompt,
            &self.arbitre.config.llm_params,
            true, // JSON mode
        );

        let cancel = self.cancel_token.clone();
        let raw = match self.ollama_client.chat(&request, cancel).await {
            Ok(raw) => raw,
            Err(OllamaError::Cancelled) => return,
            Err(e) => {
                tracing::warn!(error = %e, "LLM emotion analysis failed — keeping rule-based values");
                return;
            }
        };

        // Parse deltas and apply
        let known_names: Vec<String> = std::iter::once(self.arbitre.config.name.clone())
            .chain(self.gladiateurs.iter().map(|g| g.config.name.clone()))
            .collect();

        let deltas = json_parser::parse_emotion_deltas(&raw, &known_names);

        // Apply to arbitre
        if let Some(delta) = deltas.get(&self.arbitre.config.name) {
            let prev = self.arbitre.emotions.clone();
            self.arbitre.emotions.apply_delta(delta);
            Self::emit_threshold_events(channel, &self.arbitre.config.id, &prev, &self.arbitre.emotions);
            Self::emit_emotion_updated(channel, &self.arbitre.config.id, &self.arbitre.emotions, &self.config.discussion_language);
        }

        // Apply to gladiateurs
        for g in &mut self.gladiateurs {
            if let Some(delta) = deltas.get(&g.config.name) {
                let prev = g.emotions.clone();
                g.emotions.apply_delta(delta);
                Self::emit_threshold_events(channel, &g.config.id, &prev, &g.emotions);
                Self::emit_emotion_updated(channel, &g.config.id, &g.emotions, &self.config.discussion_language);
            }
        }
    }

    /// Apply emotional contagion: compute average, move everyone toward it (order-independent).
    fn apply_emotional_contagion(&mut self, channel: &Channel<ArenaEvent>) {
        // Collect all profiles for averaging
        let mut profiles: Vec<&EmotionalProfile> = Vec::new();
        profiles.push(&self.arbitre.emotions);
        for g in &self.gladiateurs {
            if !g.is_banned() {
                profiles.push(&g.emotions);
            }
        }
        if profiles.len() < 2 {
            return; // No contagion with < 2 participants
        }

        let avg = emotion_engine::compute_average(&profiles);

        // Apply to arbitre
        emotion_engine::apply_contagion(&avg, &mut self.arbitre.emotions);
        Self::emit_emotion_updated(channel, &self.arbitre.config.id, &self.arbitre.emotions, &self.config.discussion_language);

        // Apply to non-banned gladiateurs
        for g in &mut self.gladiateurs {
            if !g.is_banned() {
                emotion_engine::apply_contagion(&avg, &mut g.emotions);
                Self::emit_emotion_updated(channel, &g.config.id, &g.emotions, &self.config.discussion_language);
            }
        }
    }

    /// Record emotion history snapshots and emit EmotionHistoryUpdate events.
    fn record_emotion_history(&mut self, channel: &Channel<ArenaEvent>) {
        let turn = self.current_turn;

        // Arbitre
        self.arbitre.emotion_history.push(EmotionSnapshot {
            turn,
            emotions: self.arbitre.emotions.clone(),
        });
        if self.arbitre.emotion_history.len() > constants::ORCH_MAX_EMOTION_HISTORY {
            self.arbitre.emotion_history.remove(0);
        }
        let _ = channel.send(ArenaEvent::EmotionHistoryUpdate {
            speaker_id: self.arbitre.config.id.clone(),
            history: self.arbitre.emotion_history.clone(),
        });

        // Gladiateurs
        for g in &mut self.gladiateurs {
            g.emotion_history.push(EmotionSnapshot {
                turn,
                emotions: g.emotions.clone(),
            });
            if g.emotion_history.len() > constants::ORCH_MAX_EMOTION_HISTORY {
                g.emotion_history.remove(0);
            }
            let _ = channel.send(ArenaEvent::EmotionHistoryUpdate {
                speaker_id: g.config.id.clone(),
                history: g.emotion_history.clone(),
            });
        }
    }

    /// Handle manual emotion adjustment from the frontend
    fn handle_adjust_emotion(
        &mut self,
        speaker_id: &str,
        axis: &str,
        value: u8,
        channel: &Channel<ArenaEvent>,
    ) {
        let value = value.min(100);

        // Try matching arbitre
        if speaker_id == self.arbitre.config.id {
            Self::set_emotion_axis(&mut self.arbitre.emotions, axis, value);
            Self::emit_emotion_updated(channel, &self.arbitre.config.id, &self.arbitre.emotions, &self.config.discussion_language);
            return;
        }

        // Try matching gladiateurs
        for g in &mut self.gladiateurs {
            if g.config.id == speaker_id {
                Self::set_emotion_axis(&mut g.emotions, axis, value);
                Self::emit_emotion_updated(channel, &g.config.id, &g.emotions, &self.config.discussion_language);
                return;
            }
        }
    }

    /// Emit threshold-crossing events for axes that newly crossed HIGH or LOW boundaries.
    fn emit_threshold_events(
        channel: &Channel<ArenaEvent>,
        speaker_id: &str,
        prev: &EmotionalProfile,
        current: &EmotionalProfile,
    ) {
        for (axis, direction, value) in emotion_engine::detect_thresholds(prev, current) {
            let _ = channel.send(ArenaEvent::EmotionalThresholdCrossed {
                speaker_id: speaker_id.to_string(),
                axis,
                direction,
                value,
            });
        }
    }

    fn emit_emotion_updated(
        channel: &Channel<ArenaEvent>,
        speaker_id: &str,
        emotions: &EmotionalProfile,
        lang: &str,
    ) {
        let mood = prompt_builder::summarize_emotional_state(emotions, lang);
        let _ = channel.send(ArenaEvent::EmotionUpdated {
            speaker_id: speaker_id.to_string(),
            emotions: emotions.clone(),
            mood_summary: Some(mood),
        });
    }

    fn set_emotion_axis(emotions: &mut EmotionalProfile, axis: &str, value: u8) {
        match axis {
            "engagement" => emotions.engagement = value,
            "accord" => emotions.accord = value,
            "confiance" => emotions.confiance = value,
            "frustration" => emotions.frustration = value,
            "curiosite" => emotions.curiosite = value,
            "enthousiasme" => emotions.enthousiasme = value,
            _ => {}
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

    /// Build read-only document context for synthesis, or None if document format is disabled.
    fn build_document_context_for_synthesis(&self) -> Option<String> {
        if self.config.document_format == DocumentFormat::None {
            return None;
        }
        Some(prompt_builder::build_document_context_readonly(
            &self.document_content,
            self.config.document_format.as_extension(),
            &self.config.discussion_language,
            &self.config.discussion_mode,
        ))
    }

    /// Pass 2: Generate document update via separate non-streaming LLM call.
    /// The LLM receives only the discussion text + current document, and outputs the updated document.
    /// Returns the updated document content, or None if document mode is disabled or an error occurs.
    async fn generate_document_update(
        &self,
        speaker_name: &str,
        discussion_text: &str,
        llm_params: &LlmParams,
        _channel: &Channel<ArenaEvent>,
    ) -> Option<String> {
        if self.config.document_format == DocumentFormat::None {
            return None;
        }
        if self.cancel_token.is_cancelled() {
            return None;
        }

        tracing::info!(speaker = %speaker_name, "Pass 2: generating document update");

        let (sys, usr) = prompt_builder::build_document_update_prompt(
            &self.document_content,
            self.config.document_format.as_extension(),
            discussion_text,
            &self.config.discussion_mode,
            &self.config.discussion_language,
            &self.config.topic,
        );

        // Use speaker's LLM params with enough tokens for the full document.
        // The document grows over turns — estimate current tokens (≈ chars/3 for multilingual)
        // and set num_predict to 2× current size + padding, so the LLM can reproduce + extend.
        let mut params = llm_params.clone();
        let estimated_doc_tokens = (self.document_content.len() / 3) as i32;
        params.num_predict = params.num_predict.max(estimated_doc_tokens * 2 + 1024).max(4096);

        let request = self.ollama_client.build_request(&sys, &usr, &params, false);

        match self.ollama_client.chat(&request, self.cancel_token.clone()).await {
            Ok(raw) if !raw.trim().is_empty() => {
                // If the LLM wrapped in <document> tags, extract; otherwise use as-is
                let doc = if let (_, Some(extracted)) = json_parser::extract_and_strip_document(&raw) {
                    extracted
                } else {
                    raw.trim().to_string()
                };
                tracing::info!(
                    speaker = %speaker_name,
                    len = doc.len(),
                    preview = %truncate_str(&doc, 200),
                    "Document update generated"
                );
                Some(doc)
            }
            Ok(_) => {
                tracing::warn!(speaker = %speaker_name, "Pass 2 returned empty — document unchanged");
                None
            }
            Err(OllamaError::Cancelled) => None,
            Err(e) => {
                tracing::warn!(speaker = %speaker_name, error = %e, "Pass 2 document update failed — document unchanged");
                None
            }
        }
    }

    /// Ask a gladiateur whether they want to respond in UserDriven mode.
    /// Returns true if the speaker wants to respond, false if they pass.
    async fn ask_respond_or_pass(&self, glad_idx: usize) -> bool {
        let recent = self.build_recent_exchanges(glad_idx);
        let prompt = mode_prompts::build_respond_or_pass_prompt(
            &self.config.topic,
            &recent,
            &self.gladiateurs[glad_idx].config.name,
            &self.config.discussion_language,
        );
        let mut params = self.gladiateurs[glad_idx].config.llm_params.clone();
        params.num_predict = 50; // Short response only
        let request = self.ollama_client.build_request(
            &self.gladiateurs[glad_idx].config.system_prompt,
            &prompt,
            &params,
            false,
        );
        let cancel = self.cancel_token.clone();
        // No-op callback: respond-or-pass is internal logic, tokens should NOT stream to frontend
        match self.ollama_client.chat_streaming(
            &request,
            |_| {},
            cancel,
        ).await {
            Ok(raw) => {
                // Parse {"respond": true/false}
                if let Ok(val) = json_parser::parse_json_response::<serde_json::Value>(&raw) {
                    val.get("respond").and_then(|v| v.as_bool()).unwrap_or(true)
                } else {
                    true // Default to responding if parsing fails
                }
            }
            Err(e) => {
                tracing::warn!("respond-or-pass LLM failed for {}: {e}", self.gladiateurs[glad_idx].config.name);
                true // Default to responding on error
            }
        }
    }

    /// Generate a Socratic question from IArbitre.
    async fn generate_socratic_question(&self) -> Option<String> {
        let recent = self.turn_messages.iter()
            .chain(self.messages_history.iter().rev().take(constants::ORCH_RECENT_MESSAGES_TAKE))
            .map(|m| format!("{}: {}", m.speaker_name, truncate_str(&m.content, constants::ORCH_SOCRATIC_CONTEXT)))
            .collect::<Vec<_>>()
            .join("\n");
        let prompt = mode_prompts::build_socratic_question_prompt(
            &self.config.topic,
            &recent,
            &self.config.discussion_language,
        );
        let mut params = self.arbitre.config.llm_params.clone();
        params.num_predict = 200; // Short question
        let request = self.ollama_client.build_request(
            &self.arbitre.config.system_prompt,
            &prompt,
            &params,
            false,
        );
        let cancel = self.cancel_token.clone();
        // No-op callback: the full question is emitted as MessageComplete at the call site
        match self.ollama_client.chat_streaming(
            &request,
            |_| {},
            cancel,
        ).await {
            Ok(text) => {
                if text.is_empty() { None } else { Some(text) }
            }
            Err(e) => {
                tracing::warn!("Socratic question generation failed: {e}");
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::constants;

    /// Test the web pool quota logic directly (same as can_search_web body)
    fn check_web_pool(pool: u32, pool_used: u32, global_usage: u32, has_tavily: bool) -> (bool, u32) {
        if pool == 0 || !has_tavily {
            return (false, 0);
        }
        let pool_remaining = pool.saturating_sub(pool_used);
        let max_queries = pool_remaining.min(1);
        let global_remaining = constants::TAVILY_FREE_MONTHLY_QUOTA.saturating_sub(global_usage);
        let max_queries = max_queries.min(global_remaining);
        (max_queries > 0, max_queries)
    }

    #[test]
    fn test_quota_disabled() {
        let (can, max) = check_web_pool(0, 0, 0, true);
        assert!(!can);
        assert_eq!(max, 0);
    }

    #[test]
    fn test_quota_no_tavily() {
        let (can, max) = check_web_pool(5, 0, 0, false);
        assert!(!can);
        assert_eq!(max, 0);
    }

    #[test]
    fn test_quota_fresh() {
        let (can, max) = check_web_pool(5, 0, 0, true);
        assert!(can);
        assert_eq!(max, 1);
    }

    #[test]
    fn test_quota_pool_limit() {
        let (can, max) = check_web_pool(5, 4, 0, true);
        assert!(can);
        assert_eq!(max, 1); // min(5-4, 1) = 1
    }

    #[test]
    fn test_quota_exhausted() {
        let (can, max) = check_web_pool(5, 5, 0, true);
        assert!(!can);
        assert_eq!(max, 0);
    }

    #[test]
    fn test_quota_global_limit() {
        let (can, max) = check_web_pool(10, 0, 999, true);
        assert!(can);
        assert_eq!(max, 1);
    }

    #[test]
    fn test_quota_global_exhausted() {
        let (can, max) = check_web_pool(10, 0, 1000, true);
        assert!(!can);
        assert_eq!(max, 0);
    }

    // ── Wiki pool tests (simpler: no global usage, no API key check) ──

    fn check_wiki_pool(pool: u32, pool_used: u32) -> (bool, u32) {
        if pool == 0 {
            return (false, 0);
        }
        let remaining = pool.saturating_sub(pool_used);
        let max_queries = remaining.min(1);
        (max_queries > 0, max_queries)
    }

    #[test]
    fn test_wiki_quota_disabled() {
        let (can, max) = check_wiki_pool(0, 0);
        assert!(!can);
        assert_eq!(max, 0);
    }

    #[test]
    fn test_wiki_quota_fresh() {
        let (can, max) = check_wiki_pool(5, 0);
        assert!(can);
        assert_eq!(max, 1);
    }

    #[test]
    fn test_wiki_quota_exhausted() {
        let (can, max) = check_wiki_pool(3, 3);
        assert!(!can);
        assert_eq!(max, 0);
    }

    #[test]
    fn test_wiki_quota_one_remaining() {
        let (can, max) = check_wiki_pool(3, 2);
        assert!(can);
        assert_eq!(max, 1);
    }
}

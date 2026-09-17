use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use tauri::ipc::Channel;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::db::repository;
use std::future::Future;
use std::pin::Pin;
use std::time::Instant;

use rand::Rng;

use crate::engine::argument_merge;
use crate::engine::diagnostics::{DiagnosticsCounters, TurnTimer};
use crate::engine::directive_builder::{self, CoalitionRole, SpeakerTurnContext, SpeechAct};
use crate::engine::dramaturgy::{self, ActKey, TurnPosition};
use crate::engine::scene_events::{self, SceneContext, SceneEvent, SceneEventKind};
use crate::engine::dynamics_parser::{self, ParsedDynamics};
use crate::engine::emotion_engine::{self, EmotionContext, GivenReactions, PersonaGains, ReactionTally, ShiftZones};
use crate::engine::focus::{self, Focus, FocusInputs};
use crate::engine::json_parser::{self, MemoryUpdateResponse};
use crate::engine::memory_manager;
use crate::engine::mode_roles;
use crate::engine::open_loops::{OpenLoop, OpenLoopKind, OpenLoopRegistry};
use crate::engine::prompt_builder::{self, StageBlock};
use crate::engine::reactions::{self, ReactionPropensity, ReactionScope};
use crate::engine::relationships::RelationshipScores;
use crate::engine::stage_directions::{self, StageCue};
use crate::engine::tuning::Tuning;
use crate::engine::token_budget::{BudgetFeatures, BudgetParams, SectionPriority, TokenBudget};
use crate::engine::turn_manager;
use crate::models::emotion::{EmotionDelta, EmotionSnapshot, EmotionalProfile, RoomMood};
use crate::engine::mode_prompts;
use crate::models::discussion::{DiscussionConfig, DiscussionMode, DiscussionStatus, DocumentFormat, DocumentInjectionMode, DocumentUpdateGranularity, ReactionTiming, TurnDistribution};
use crate::models::engine_command::EngineCommand;
use crate::models::events::{ArenaEvent, RoleAssignment};
use crate::models::outcome::{ModeOutcome, PartyDecision, VerdictVote, VotePhase, VOTE_AGAINST, VOTE_FOR};
use crate::models::persona_memory::PersonaRecapRecord;
use crate::engine::prompt_builder::RecapInput;
use crate::models::gladiateur::GladIAteurState;
use crate::models::iarbitre::IArbitreState;
use crate::llm::metered::MeteredProvider;
use crate::llm::parallel::run_bounded;
use crate::llm::pricing;
use crate::llm::{LlmError, LlmProvider, LlmRequest};
use crate::models::agenda::{Agenda, AgendaReveal};
use crate::models::argument_map::{ArgumentMap, ArgumentNode};
use crate::models::intention::Intention;
use crate::models::llm::{CallKind, ReasoningLevel, ReasoningPace};
#[cfg(test)]
use crate::models::llm::UsageLedger;
use crate::models::message::{Message, MessageKind, Reaction, ReactionType, SpeakerRole, ThoughtKind};
use crate::models::relationship::RelationshipKind;
use crate::models::settings::LlmParams;
use crate::models::source::{self, SourceKind, SourceRecord, WebSourceInfo, WikiSourceInfo};
use crate::rag::RagStore;
use crate::tavily::client::TavilyClient;
use crate::tavily::error::TavilyError;
use crate::wikipedia::client::WikiClient;

use super::cast;
use super::{is_model_refusal, truncate_at_sentence_boundary, truncate_at_word_boundary, truncate_str};

use crate::constants;

mod analysis;
mod knowledge;
mod staging;
mod structured;

/// Check if a query is a near-duplicate of any of the speaker's own past queries.
/// Normalization: lowercase + trim.
/// Match: exact (always) OR substring (only if the shorter string has ≥ MIN_SUBSTRING_LEN chars).
fn is_duplicate_query(query: &str, past_queries: &[String]) -> bool {
    let normalized = query.trim().to_lowercase();
    if normalized.is_empty() {
        return true;
    }
    past_queries.iter().any(|past| {
        let past_norm = past.trim().to_lowercase();
        if normalized == past_norm {
            return true;
        }
        // Substring match only if the shorter string is long enough to be meaningful
        let shorter_len = normalized.len().min(past_norm.len());
        if shorter_len >= constants::SEARCH_DEDUP_MIN_SUBSTRING_LEN {
            normalized.contains(&past_norm) || past_norm.contains(&normalized)
        } else {
            false
        }
    })
}

/// (title, pseudo-url) of a document chunk for the sources registry — "file#index" mirrors the frontend.
fn rag_source_ref(c: &crate::rag::RagChunkInfo) -> (String, String) {
    (format!("{} #{}", c.file_name, c.chunk_index + 1), format!("{}#{}", c.file_name, c.chunk_index + 1))
}

/// One end-of-turn LLM phase (v1.17): prepared from `&self`, run concurrently
/// with the others, applied on `&mut self` in the historical order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EndOfTurnPhase {
    Document,
    /// Fused memory + emotion analysis (sequential providers)
    TurnAnalyst,
    Emotion,
    Memory,
    ArgumentMap,
}

impl EndOfTurnPhase {
    /// Name used in `TurnTimings`.
    fn name(self) -> &'static str {
        match self {
            Self::Document => "document",
            Self::TurnAnalyst => "turnAnalyst",
            Self::Emotion => "emotion",
            Self::Memory => "memory",
            Self::ArgumentMap => "argumentMap",
        }
    }
}

/// Result of one end-of-turn phase.
struct PhaseOutcome {
    phase: EndOfTurnPhase,
    request: LlmRequest,
    raw: Result<String, LlmError>,
    /// The empty first answer was retried once with a warmer temperature
    retried: bool,
    ms: u64,
}

/// Run one phase: the call, an optional retry on an empty answer, and its duration.
async fn run_phase(
    llm: Arc<MeteredProvider>,
    cancel: CancellationToken,
    phase: EndOfTurnPhase,
    request: LlmRequest,
    retry_on_empty: bool,
) -> PhaseOutcome {
    let started = Instant::now();
    let mut retried = false;
    let mut raw = llm.chat(&request, cancel.clone()).await.map(|r| r.content);
    if retry_on_empty && matches!(&raw, Ok(r) if r.trim().is_empty()) && !cancel.is_cancelled() {
        tracing::info!(phase = phase.name(), "Empty answer — retrying with a higher temperature");
        retried = true;
        let mut retry = request.clone();
        retry.params.temperature = (retry.params.temperature + constants::TEMP_DIFFICULTY_BOOST).min(constants::TEMP_MAX);
        raw = llm.chat(&retry, cancel.clone()).await.map(|r| r.content);
    }
    PhaseOutcome { phase, request, raw, retried, ms: started.elapsed().as_millis() as u64 }
}

/// Cached RAG query result for a single speaker.
struct RagCacheEntry {
    cached_at_turn: u32,
    context_text: String,
    chunks: Vec<crate::rag::RagChunkInfo>,
}

pub struct DiscussionEngine {
    discussion_id: String,
    config: DiscussionConfig,
    /// LLM provider (metered: every call's token usage lands in the ledger)
    llm: Arc<MeteredProvider>,
    status: DiscussionStatus,
    current_turn: u32,
    arbitre: IArbitreState,
    gladiateurs: Vec<GladIAteurState>,
    messages_history: Vec<Message>,
    turn_messages: Vec<Message>,
    /// Reactions received per speaker during the current turn (drought signal, analyst
    /// context, and the deferred emotion update)
    turn_reaction_counts: HashMap<String, ReactionTally>,
    /// Reactions received on each speaker's LAST message (reset when they speak again) —
    /// feeds the reasoning heuristic in both timings
    last_reactions_received: HashMap<String, ReactionTally>,
    /// Audience reactions given on each message (cap per message)
    audience_reactions_by_message: HashMap<String, u32>,
    /// Participants (names) the audience reacted to this turn (focus bonus)
    turn_audience_targets: HashSet<String>,
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
    /// Weighted reaction graph between participants (counts + decaying scores, v1.17)
    relationship_scores: RelationshipScores,
    /// Former rival who just approved a speaker: consumed by their next directive (v1.17)
    reconciliation_pending: HashMap<String, String>,
    /// Tunable dynamics (defaults = constants)
    tuning: Tuning,
    /// How strongly each gladiateur feels the rule deltas (from OCEAN, v1.17)
    persona_gains: HashMap<String, PersonaGains>,
    /// Stage directions already shown per speaker this turn (anti-noise cap)
    stage_directions_this_turn: HashMap<String, usize>,
    /// Notable-zone occupancy per speaker and axis (relative threshold crossings, v1.20.5)
    shift_zones: ShiftZones,
    /// Latest temperature of the room (v1.17)
    room_mood: Option<RoomMood>,
    /// What went wrong or not, reported before the end (v1.17)
    diagnostics: DiagnosticsCounters,
    /// Wall-clock phases of the current turn (v1.17)
    turn_timer: TurnTimer,
    /// Act of the mode's script in progress (v1.18)
    current_act: Option<ActKey>,
    /// Turn of the last scene event (never two turns in a row)
    last_scene_event_turn: Option<u32>,
    /// Scene event of the current turn, if any
    turn_scene_event: Option<SceneEvent>,
    /// Coalition of the current turn: (leader id, follower id)
    turn_coalition: Option<(String, String)>,
    /// Test hook: the next eligible turn gets this scene event
    #[cfg(test)]
    forced_scene_event: Option<SceneEventKind>,
    /// Test hook: coalitions form whenever an ally pair exists
    #[cfg(test)]
    coalitions_forced: bool,
    /// Test hook: no random scene event or coalition (forced ones still apply)
    #[cfg(test)]
    random_staging_disabled: bool,
    /// Speaker's own messages for self-memory: speaker_id -> Vec<String> (last 2)
    speaker_own_messages: HashMap<String, Vec<String>>,
    /// Parsed dynamics cache: speaker_id -> ParsedDynamics
    dynamics_cache: HashMap<String, ParsedDynamics>,
    /// Recent speech acts per speaker (newest last, bounded window): speaker_id -> acts
    recent_speech_acts: HashMap<String, Vec<SpeechAct>>,
    /// Colours of the last reactions each gladiateur gave (v1.20.5): one 💡 per
    /// `INSIGHTFUL_CREDIT_WINDOW` reactions at most
    recent_given_kinds: HashMap<String, VecDeque<ReactionType>>,
    /// Participants already chosen as conversational focus this turn (names)
    turn_focus_targets: HashSet<String>,
    /// Contextual summary after the previous turn (stagnation detection)
    previous_summary: String,
    /// Stagnation signals — see `is_stagnating()`
    summary_stagnating: bool,
    llm_stagnation_flag: bool,
    turns_without_reactions: u32,
    /// Socratic questions already asked by IArbitre (anti-repetition)
    socratic_questions: Vec<String>,
    /// Co-construction (per-turn granularity): contributions to integrate at end of turn
    turn_document_contributions: Vec<(String, String)>,
    /// Every reference injected into a prompt (web, wiki, document), for the synthesis
    sources_registry: Vec<SourceRecord>,
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
    /// Per-speaker RAG query cache: speaker_id → cached result.
    rag_cache: HashMap<String, RagCacheEntry>,
    /// Whether to extract and track argument map
    argument_map_enabled: bool,
    /// Accumulated argument map across all turns
    argument_map: ArgumentMap,
    /// Per-speaker token budgets (speaker_id → TokenBudget).
    /// Computed once at engine construction; used by prompt_builder for dynamic truncation.
    budgets: HashMap<String, TokenBudget>,
    /// Global default reasoning level (speakers may override it in their LlmParams)
    default_reasoning_level: ReasoningLevel,
    /// Forward displayable model reasoning to the frontend as thought chunks
    show_model_reasoning: bool,
    /// Wall-clock pace of the reasoning (`Fast` caps `Auto` at `Low`, providers shrink allowances)
    reasoning_pace: ReasoningPace,
    /// Questions and commitments each speaker still owes an answer to (v1.17)
    open_loops: OpenLoopRegistry,
    /// Secret objective per gladiateur id, generated after the introduction (v1.19)
    agendas: HashMap<String, Agenda>,
    /// Role (trial, Oxford) or hat (six hats) per gladiateur id (v1.19)
    roles: HashMap<String, String>,
    /// Crisis dispatches still to deliver, one per turn (crisis cell, v1.19)
    crisis_dispatches: std::collections::VecDeque<String>,
    /// Audience votes on the motion before and after an Oxford debate (v1.19)
    vote_before: Option<String>,
    vote_after: Option<String>,
    /// Mode-specific result, resolved before the synthesis (v1.19)
    outcome: Option<ModeOutcome>,
    /// Long memory (v1.20): recalled memories block per gladiateur id, and whether recaps are written
    memory_blocks: HashMap<String, String>,
    /// "[Les autres participants — qui ils sont]" per speaker id (v1.20.3)
    cast_blocks: HashMap<String, String>,
    /// The moderator's system prompt with its cast (v1.20.3)
    arbitre_system: String,
    persona_memory_enabled: bool,
    /// Consecutive reasoning-mode failures; reasoning is disabled past REASONING_MAX_FAILURES
    reasoning_failures: u32,
    /// Cloud spend already accumulated this period before the discussion (USD)
    period_spent_usd: f64,
    /// Monthly spending cap (0 = unlimited)
    monthly_budget_usd: f64,
    /// Budget alert already emitted ("warning" / "exceeded") — each fires once
    budget_warning_sent: bool,
    budget_exceeded_sent: bool,
    /// The audience's last message, awaiting an answer from the next speaker (excerpt, v1.20.2)
    user_reply_pending: Option<String>,
    /// The audience has spoken at least once: a participant, no longer an observer (v1.20.2)
    user_has_spoken: bool,
    /// Openings of the moderator's own lines (introduction, comments), oldest first (v1.20.2)
    arbitre_recent_openings: VecDeque<String>,
    /// Scene-event kinds already played (each kind once until all were played, v1.20.3)
    scene_kinds_used: HashSet<SceneEventKind>,
    /// Interventions moderated and comments issued so far (comment rate, v1.20.2)
    moderation_checks: u32,
    moderation_comments: u32,
    /// Step mode (v1.20.1): wait for the audience's cue before each speaker (voice "follow")
    step_mode: bool,
    /// Cues received ahead of a wait — each one frees one speaker
    cues_pending: u32,
}

impl DiscussionEngine {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        config: DiscussionConfig,
        discussion_id: String,
        llm: Arc<dyn LlmProvider>,
        tavily_api_key: Option<&str>,
        db: tokio_rusqlite::Connection,
        rag_store: Option<RagStore>,
        priorities: Vec<SectionPriority>,
    ) -> Self {
        let llm = Arc::new(MeteredProvider::new(llm));
        let arbitre = IArbitreState::new(config.arbitre.clone());
        let gladiateurs: Vec<GladIAteurState> = config
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

        // Compute per-speaker token budgets.
        let has_rag = rag_store.as_ref().is_some_and(|r| !r.is_empty());
        let rag_total_chars = rag_store.as_ref().map_or(0, |r| r.total_char_count());
        // The secret agenda block lives in the system prompt of every intervention (v1.19)
        let agenda_chars = if Self::agendas_enabled_for(&config) { constants::AGENDA_MAX_CHARS + constants::AGENDA_BLOCK_OVERHEAD_CHARS } else { 0 };
        // The role / hat block sits in the system prompt of the structured modes (v1.19)
        let role_chars = if Self::mode_has_roles(&config.discussion_mode) { constants::ROLE_BLOCK_MAX_CHARS } else { 0 };
        let features = match config.document_injection_mode {
            DocumentInjectionMode::FullInjection => BudgetFeatures {
                web_search_enabled: config.web_search_pool > 0
                    || config.arbitre.web_search_intro,
                wiki_search_enabled: config.wiki_search_pool > 0
                    || config.arbitre.wiki_search_intro,
                // Safety net: keep RAG enabled when docs exist so the budget allocator
                // provides RAG fallback if the full document doesn't fit.
                // token_budget.rs zeroes rag_context_chars when full_document_mode is true.
                rag_enabled: has_rag,
                document_chars: rag_total_chars,
                agenda_chars,
                argument_map_enabled: config.argument_map_enabled,
            },
            DocumentInjectionMode::Rag => BudgetFeatures {
                web_search_enabled: config.web_search_pool > 0
                    || config.arbitre.web_search_intro,
                wiki_search_enabled: config.wiki_search_pool > 0
                    || config.arbitre.wiki_search_intro,
                rag_enabled: has_rag,
                document_chars: 0,
                agenda_chars,
                argument_map_enabled: config.argument_map_enabled,
            },
        };

        let mut budgets = HashMap::new();

        // Budget for IArbitre
        let provider_kind = llm.kind();
        let arbitre_params = BudgetParams {
            num_ctx: config.arbitre.llm_params.num_ctx,
            num_predict: config.arbitre.llm_params.num_predict,
            system_prompt_chars: config.arbitre.system_prompt.len() + constants::CAST_BLOCK_MAX_CHARS,
            n_gladiateurs: config.gladiateurs.len(),
            language: config.discussion_language.clone(),
            features: features.clone(),
            provider: provider_kind,
        };
        let (arbitre_budget, arbitre_warnings) =
            TokenBudget::compute(&arbitre_params, &priorities);
        for w in &arbitre_warnings {
            tracing::warn!("[BUDGET] IArbitre '{}': {}", config.arbitre.name, w);
        }
        tracing::info!(
            "[BUDGET] IArbitre '{}': ctx={} predict={} sys_prompt={}chars → \
             current_turn={}c/msg imm_mem={}c/msg summary={}c cognitive={}c arbitre={}c \
             full_doc={}c(mode={}) web_wiki={}c rag={}c pos_map={}c",
            config.arbitre.name,
            arbitre_params.num_ctx,
            arbitre_params.num_predict,
            arbitre_params.system_prompt_chars,
            arbitre_budget.current_turn_msg_chars,
            arbitre_budget.immediate_memory_msg_chars,
            arbitre_budget.contextual_summary_chars,
            arbitre_budget.cognitive_directives_chars,
            arbitre_budget.arbitre_directives_chars,
            arbitre_budget.full_document_chars,
            arbitre_budget.full_document_mode,
            arbitre_budget.web_wiki_chars,
            arbitre_budget.rag_context_chars,
            arbitre_budget.positional_map_chars,
        );
        budgets.insert(config.arbitre.id.clone(), arbitre_budget);

        // Budget for each GladIAteur
        for g in &config.gladiateurs {
            let glad_params = BudgetParams {
                num_ctx: g.llm_params.num_ctx,
                num_predict: g.llm_params.num_predict,
                system_prompt_chars: g.system_prompt.len() + role_chars + constants::CAST_BLOCK_MAX_CHARS + if g.source_profile_id.is_some() { constants::PERSONA_MEMORY_MAX_CHARS } else { 0 },
                n_gladiateurs: config.gladiateurs.len(),
                language: config.discussion_language.clone(),
                features: features.clone(),
                provider: provider_kind,
            };
            let (glad_budget, glad_warnings) =
                TokenBudget::compute(&glad_params, &priorities);
            for w in &glad_warnings {
                tracing::warn!("[BUDGET] GladIAteur '{}': {}", g.name, w);
            }
            tracing::info!(
                "[BUDGET] GladIAteur '{}': ctx={} predict={} sys_prompt={}chars → \
                 current_turn={}c/msg imm_mem={}c/msg summary={}c cognitive={}c arbitre={}c \
                 full_doc={}c(mode={}) web_wiki={}c rag={}c pos_map={}c",
                g.name,
                glad_params.num_ctx,
                glad_params.num_predict,
                glad_params.system_prompt_chars,
                glad_budget.current_turn_msg_chars,
                glad_budget.immediate_memory_msg_chars,
                glad_budget.contextual_summary_chars,
                glad_budget.cognitive_directives_chars,
                glad_budget.arbitre_directives_chars,
                glad_budget.full_document_chars,
                glad_budget.full_document_mode,
                glad_budget.web_wiki_chars,
                glad_budget.rag_context_chars,
                glad_budget.positional_map_chars,
            );
            budgets.insert(g.id.clone(), glad_budget);
        }

        // Warn if full injection was requested but budget couldn't fit the document.
        if config.document_injection_mode == DocumentInjectionMode::FullInjection {
            for (id, budget) in &budgets {
                if !budget.full_document_mode {
                    tracing::warn!(
                        "[BUDGET] Speaker '{}': full injection requested but budget insufficient \
                         (allocated {}chars < {}chars needed). Falling back to RAG.",
                        id, budget.full_document_chars, rag_total_chars,
                    );
                }
            }
        }

        let roles = Self::deal_fixed_roles(&config);
        // Who is who (v1.20.3): a portrait of every gladiateur, read once from the kernels
        let portraits: Vec<(String, cast::CastPortrait)> = config.gladiateurs.iter().map(|g| (g.id.clone(), cast::portrait_from_kernel(&g.name, &g.system_prompt))).collect();
        let cast_blocks: HashMap<String, String> = portraits
            .iter()
            .map(|(id, _)| {
                let others: Vec<cast::CastPortrait> = portraits.iter().filter(|(o, _)| o != id).map(|(_, p)| p.clone()).collect();
                (id.clone(), cast::build_cast_block(&others, &config.discussion_language, false))
            })
            .collect();
        let arbitre_system = {
            let all: Vec<cast::CastPortrait> = portraits.iter().map(|(_, p)| p.clone()).collect();
            let block = cast::build_cast_block(&all, &config.discussion_language, true);
            if block.is_empty() { config.arbitre.system_prompt.clone() } else { format!("{}

{block}", config.arbitre.system_prompt) }
        };
        Self {
            discussion_id,
            config,
            llm,
            status: DiscussionStatus::Active,
            current_turn: 0,
            arbitre,
            gladiateurs,
            messages_history: Vec::new(),
            turn_messages: Vec::new(),
            turn_reaction_counts: HashMap::new(),
            last_reactions_received: HashMap::new(),
            audience_reactions_by_message: HashMap::new(),
            turn_audience_targets: HashSet::new(),
            user_intervention_pending: false,
            user_intervention_handled: false,
            cancel_token: CancellationToken::new(),
            emotion_driven: false,
            tavily_client,
            wiki_client: WikiClient::new(),
            db,
            relationship_scores: RelationshipScores::default(),
            reconciliation_pending: HashMap::new(),
            tuning: Tuning::default(),
            persona_gains: HashMap::new(),
            stage_directions_this_turn: HashMap::new(),
            shift_zones: ShiftZones::default(),
            room_mood: None,
            diagnostics: DiagnosticsCounters::default(),
            turn_timer: TurnTimer::default(),
            current_act: None,
            last_scene_event_turn: None,
            turn_scene_event: None,
            turn_coalition: None,
            #[cfg(test)]
            forced_scene_event: None,
            #[cfg(test)]
            coalitions_forced: false,
            #[cfg(test)]
            random_staging_disabled: false,
            speaker_own_messages: HashMap::new(),
            dynamics_cache: HashMap::new(),
            recent_speech_acts: HashMap::new(),
            recent_given_kinds: HashMap::new(),
            turn_focus_targets: HashSet::new(),
            previous_summary: String::new(),
            summary_stagnating: false,
            llm_stagnation_flag: false,
            turns_without_reactions: 0,
            socratic_questions: Vec::new(),
            turn_document_contributions: Vec::new(),
            sources_registry: Vec::new(),
            turn_search_queries: Vec::new(),
            web_searches_used_pool: 0,
            wiki_searches_used_pool: 0,
            forced_web_done: HashSet::new(),
            forced_wiki_done: HashSet::new(),
            document_content: String::new(),
            rag_store,
            rag_cache: HashMap::new(),
            argument_map_enabled: false,
            argument_map: ArgumentMap::default(),
            budgets,
            default_reasoning_level: ReasoningLevel::Auto,
            show_model_reasoning: true,
            reasoning_pace: ReasoningPace::Normal,
            open_loops: OpenLoopRegistry::default(),
            agendas: HashMap::new(),
            roles,
            crisis_dispatches: std::collections::VecDeque::new(),
            vote_before: None,
            vote_after: None,
            outcome: None,
            memory_blocks: HashMap::new(),
            cast_blocks,
            arbitre_system,
            persona_memory_enabled: false,
            reasoning_failures: 0,
            period_spent_usd: 0.0,
            monthly_budget_usd: 0.0,
            budget_warning_sent: false,
            budget_exceeded_sent: false,
            user_reply_pending: None,
            user_has_spoken: false,
            arbitre_recent_openings: VecDeque::new(),
            scene_kinds_used: HashSet::new(),
            moderation_checks: 0,
            moderation_comments: 0,
            step_mode: false,
            cues_pending: 0,
        }
    }

    /// Safe accessor for per-speaker token budgets.
    /// Returns the budget for the given speaker, or the default fallback budget
    /// if the speaker ID is unexpectedly missing (should never happen).
    fn budget_for(&self, speaker_id: &str) -> &TokenBudget {
        static DEFAULT_BUDGET: std::sync::LazyLock<TokenBudget> =
            std::sync::LazyLock::new(TokenBudget::default);
        self.budgets.get(speaker_id).unwrap_or_else(|| {
            tracing::error!(
                speaker_id = speaker_id,
                "Budget not found for speaker — using default fallback"
            );
            &DEFAULT_BUDGET
        })
    }

    /// Returns the full document text for injection when the speaker's budget allows it.
    /// When `full_document_mode` is true, the entire RAG document is injected directly
    /// instead of using chunk-based RAG search.
    fn full_document_for(&self, speaker_id: &str) -> Option<String> {
        let budget = self.budget_for(speaker_id);
        if !budget.full_document_mode {
            return None;
        }
        let rag_store = self.rag_store.as_ref()?;
        rag_store.get_full_text()
    }

    /// Non-streaming LLM call returning the content only (JSON utilities).
    async fn chat_text(&self, request: &LlmRequest) -> Result<String, LlmError> {
        self.llm
            .chat(request, self.cancel_token.clone())
            .await
            .map(|r| r.content)
    }

    /// Ledger accessor that outlives the engine (`run()` consumes `self`). Test harness only.
    #[cfg(test)]
    pub fn usage_snapshot_handle(&self) -> impl Fn() -> UsageLedger + Send + Sync + 'static {
        let llm = Arc::clone(&self.llm);
        move || llm.snapshot()
    }

    /// Resolve a configured level against provider capabilities and runtime state.
    /// `Auto` becomes `auto_fallback`; providers without distinct levels get `High`
    /// for any active level (Ollama: "let the model think").
    fn resolve_reasoning(&self, speaker_id: &str, configured: ReasoningLevel, auto_fallback: ReasoningLevel) -> ReasoningLevel {
        let caps = self.llm.capabilities_for(Some(speaker_id));
        if !caps.supports_reasoning || self.reasoning_failures >= constants::REASONING_MAX_FAILURES {
            return ReasoningLevel::Off;
        }
        let level = match configured {
            ReasoningLevel::Auto => self.reasoning_pace.cap_auto(auto_fallback),
            other => other,
        };
        match level {
            ReasoningLevel::Off | ReasoningLevel::Auto => ReasoningLevel::Off,
            active if caps.reasoning_levels => active,
            _ => ReasoningLevel::High,
        }
    }

    /// Reasoning level for the IArbitre's own content (introduction).
    fn arbitre_reasoning_level(&self) -> ReasoningLevel {
        let configured = self
            .arbitre
            .config
            .llm_params
            .reasoning_level
            .unwrap_or(self.default_reasoning_level);
        self.resolve_reasoning(&self.arbitre.config.id, configured, ReasoningLevel::Low)
    }

    /// Synthesis is long-form and benefits from deep reasoning where levels exist.
    fn synthesis_reasoning_level(&self) -> ReasoningLevel {
        if self.llm.capabilities_for(Some(&self.arbitre.config.id)).reasoning_levels {
            self.resolve_reasoning(&self.arbitre.config.id, ReasoningLevel::High, ReasoningLevel::High)
        } else {
            ReasoningLevel::Off
        }
    }

    /// Reasoning level for a gladiateur's intervention.
    ///
    /// `Auto` keeps the historical non-systematic heuristic (never on turn 1,
    /// probability boosted by frustration, engagement, end proximity and
    /// contradiction, capped at `THINK_MAX_PROBABILITY`); when the gate passes,
    /// strong triggers select `High`, otherwise `Low`.
    fn resolve_gladiateur_reasoning(&self, glad_idx: usize) -> ReasoningLevel {
        let configured = self.gladiateurs[glad_idx]
            .config
            .llm_params
            .reasoning_level
            .unwrap_or(self.default_reasoning_level);
        let auto = if configured == ReasoningLevel::Auto {
            self.auto_reasoning_heuristic(glad_idx)
        } else {
            ReasoningLevel::Off
        };
        self.resolve_reasoning(&self.gladiateurs[glad_idx].config.id, configured, auto)
    }

    fn auto_reasoning_heuristic(&self, glad_idx: usize) -> ReasoningLevel {
        // Never on turn 1 — keep things quick at the start
        if self.current_turn <= 1 {
            return ReasoningLevel::Off;
        }
        let emo = &self.gladiateurs[glad_idx].emotions;
        let mut probability: f64 = constants::THINK_BASE_PROBABILITY;
        let mut strong_trigger = false;

        if emo.frustration > constants::THINK_FRUSTRATION_THRESHOLD {
            probability += constants::THINK_FRUSTRATION_BOOST;
            strong_trigger = true;
        }
        if emo.engagement > constants::THINK_ENGAGEMENT_THRESHOLD {
            probability += constants::THINK_ENGAGEMENT_BOOST;
        }
        if let Some(max) = self.config.max_turns {
            if self.current_turn + constants::THINK_NEAR_END_TURNS >= max {
                probability += constants::THINK_NEAR_END_BOOST;
                strong_trigger = true;
            }
        }
        let dislikes = self
            .last_reactions_received
            .get(&self.gladiateurs[glad_idx].config.id)
            .map(|t| t.dislikes)
            .unwrap_or(0);
        if dislikes >= constants::EMOTION_CONTRADICTION_THRESHOLD {
            probability += constants::THINK_CONTRADICTED_BOOST;
            strong_trigger = true;
        }
        // v1.20.4 — shaken since the start: think harder
        if matches!(emotion_engine::dominant_shift(emo, &self.gladiateurs[glad_idx].initial_emotions), Some(("confiance", shift)) if shift < 0) {
            probability += constants::THINK_SHAKEN_BOOST;
            strong_trigger = true;
        }
        probability = probability.min(constants::THINK_MAX_PROBABILITY);

        if !rand::thread_rng().gen_bool(probability) {
            return ReasoningLevel::Off;
        }
        if strong_trigger { ReasoningLevel::High } else { ReasoningLevel::Low }
    }

    /// Count a reasoning-mode failure (empty content / refusal); past the
    /// threshold, reasoning is disabled for the rest of the discussion.
    fn note_reasoning_failure(&mut self, speaker_name: &str) {
        self.reasoning_failures += 1;
        if self.reasoning_failures >= constants::REASONING_MAX_FAILURES {
            tracing::warn!(speaker = %speaker_name, failures = self.reasoning_failures, "Reasoning mode disabled for the rest of the discussion");
        } else {
            tracing::info!(speaker = %speaker_name, failures = self.reasoning_failures, "Reasoning mode failed — falling back to thought + intervention");
        }
    }

    pub fn set_cancel_token(&mut self, token: CancellationToken) {
        self.cancel_token = token;
    }

    pub fn set_reasoning_options(&mut self, default_level: ReasoningLevel, show_model_reasoning: bool, pace: ReasoningPace) {
        self.default_reasoning_level = default_level;
        self.show_model_reasoning = show_model_reasoning;
        self.reasoning_pace = pace;
    }

    /// Configure the monthly spend guard (cloud providers only).
    pub fn set_budget_guard(&mut self, period_spent_usd: f64, monthly_budget_usd: f64) {
        self.period_spent_usd = period_spent_usd.max(0.0);
        self.monthly_budget_usd = monthly_budget_usd.max(0.0);
    }

    /// Emit the usage snapshot and enforce the monthly budget.
    /// Returns true when the budget is exhausted (caller should soft-stop).
    fn emit_usage_and_check_budget(&mut self, channel: &Channel<ArenaEvent>) -> bool {
        let ledger = self.llm.snapshot();
        let _ = channel.send(ArenaEvent::LlmUsageUpdated {
            provider: self.llm.kind(),
            model: self.llm.model_name().to_string(),
            total: ledger.total.clone(),
            calls: ledger.calls,
            estimated_cost_usd: ledger.estimated_cost_usd,
            period_spent_usd: self.period_spent_usd,
            budget_usd: self.monthly_budget_usd,
            peak: pricing::is_peak_hour(chrono::Utc::now()),
        });

        if self.monthly_budget_usd <= 0.0 {
            return false;
        }
        let spent = self.period_spent_usd + ledger.estimated_cost_usd.unwrap_or(0.0);
        if spent >= self.monthly_budget_usd {
            if !self.budget_exceeded_sent {
                self.budget_exceeded_sent = true;
                tracing::warn!(spent_usd = spent, budget_usd = self.monthly_budget_usd, "Monthly LLM budget exhausted — soft stop");
                let _ = channel.send(ArenaEvent::BudgetAlert {
                    level: "exceeded".to_string(),
                    spent_usd: spent,
                    budget_usd: self.monthly_budget_usd,
                });
            }
            return true;
        }
        if spent >= self.monthly_budget_usd * constants::LLM_BUDGET_WARN_RATIO && !self.budget_warning_sent {
            self.budget_warning_sent = true;
            tracing::info!(spent_usd = spent, budget_usd = self.monthly_budget_usd, "Monthly LLM budget warning threshold reached");
            let _ = channel.send(ArenaEvent::BudgetAlert {
                level: "warning".to_string(),
                spent_usd: spent,
                budget_usd: self.monthly_budget_usd,
            });
        }
        false
    }

    /// Localized message when the provider returned a fatal error.
    fn provider_fatal_msg(&self) -> String {
        match self.config.discussion_language.as_str() {
            "en" => "The LLM provider rejected the request (invalid key, insufficient balance or unknown model). The discussion is stopped.".to_string(),
            "zh" => "LLM 提供商拒绝了请求（密钥无效、余额不足或模型未知）。讨论已停止。".to_string(),
            _ => "Le fournisseur LLM a rejeté la requête (clé invalide, solde insuffisant ou modèle inconnu). La discussion est arrêtée.".to_string(),
        }
    }

    /// Persist this discussion's cloud usage into the rolling monthly period.
    async fn record_period_usage(&self) {
        if !self.llm.capabilities().billable {
            return;
        }
        let ledger = self.llm.snapshot();
        if ledger.total.is_empty() {
            return;
        }
        let cost = ledger.estimated_cost_usd.unwrap_or(0.0);
        match repository::record_deepseek_usage(&self.db, &ledger.total, cost).await {
            Ok(period) => tracing::info!(
                tokens = ledger.total.total_tokens(),
                cost_usd = cost,
                period_cost_usd = period.cost_usd,
                "Cloud usage recorded into the monthly period"
            ),
            Err(e) => tracing::warn!(error = %e, "Failed to record cloud usage for the period"),
        }
    }

    pub fn set_emotion_driven(&mut self, enabled: bool) {
        self.emotion_driven = enabled;
    }

    /// Advanced tuning (v1.20): validated values from the settings (defaults = constants).
    pub fn set_tuning(&mut self, tuning: Tuning) {
        if tuning != Tuning::default() {
            tracing::info!(?tuning, "Advanced tuning applied");
        }
        self.tuning = tuning;
    }

    /// Long memory (v1.20): recall past recaps at the start and write new ones at the end.
    /// Step mode (v1.20.1): leaving it drops the cues received ahead.
    pub fn set_step_mode(&mut self, enabled: bool) {
        self.step_mode = enabled;
        if !enabled {
            self.cues_pending = 0;
        }
    }

    pub fn set_persona_memory(&mut self, enabled: bool) {
        self.persona_memory_enabled = enabled;
    }

    pub fn set_argument_map_enabled(&mut self, enabled: bool) {
        self.argument_map_enabled = enabled;
        tracing::info!(argument_map_enabled = enabled, "Argument map feature configured");
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
        // How strongly each persona feels the rule deltas (OCEAN gains, v1.17)
        for g in &self.gladiateurs {
            let gains = emotion_engine::gains_from_ocean(prompt_builder::parse_ocean_values(&g.config.system_prompt), &self.tuning);
            if gains != PersonaGains::default() {
                tracing::debug!(speaker = %g.config.name, ?gains, "Persona gains from OCEAN");
            }
            self.persona_gains.insert(g.config.id.clone(), gains);
        }

        tracing::info!(
            provider = self.llm.kind().as_str(),
            model = self.llm.model_name(),
            supports_reasoning = self.llm.capabilities().supports_reasoning,
            "LLM provider ready"
        );

        // Long memory (v1.20): what each persona remembers of its past discussions
        self.recall_memories().await;

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
                topic_query.clone(), &self.arbitre.config.id, "IArbitre", "wiki-intro",
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
                let (ctx, _, _, sources) = self.process_web_search(
                    &self.arbitre.config.system_prompt,
                    &self.arbitre.config.id, &self.arbitre.config.name,
                    1, "", "", Some(vec![topic_query.clone()]),
                    &self.arbitre.config.llm_params, &channel, &[], &[],
                ).await;
                let arb_name = self.arbitre.config.name.clone();
                self.register_sources(&arb_name, SourceKind::Web, sources.into_iter().map(|s| (s.title, s.url)));
                ctx
            } else { None };
            let (arb_id, arb_name) = (self.arbitre.config.id.clone(), self.arbitre.config.name.clone());
            let (wiki_ctx, _) = self.process_wiki_search_intro(&wiki_intro_query, &arb_id, &arb_name, &channel).await;
            (web_ctx, wiki_ctx)
        } else if want_web_intro {
            let global_usage = repository::get_tavily_usage(&self.db).await.unwrap_or_else(|e| {
                tracing::warn!(error = %e, "Failed to read Tavily usage count — assuming 0");
                0
            });
            let web_ctx = if global_usage < constants::TAVILY_FREE_MONTHLY_QUOTA {
                let (ctx, _, _, sources) = self.process_web_search(
                    &self.arbitre.config.system_prompt,
                    &self.arbitre.config.id, &self.arbitre.config.name,
                    1, "", "", Some(vec![topic_query.clone()]),
                    &self.arbitre.config.llm_params, &channel, &[], &[],
                ).await;
                let arb_name = self.arbitre.config.name.clone();
                self.register_sources(&arb_name, SourceKind::Web, sources.into_iter().map(|s| (s.title, s.url)));
                ctx
            } else { None };
            (web_ctx, None)
        } else if want_wiki_intro {
            let (arb_id, arb_name) = (self.arbitre.config.id.clone(), self.arbitre.config.name.clone());
            let (wiki_ctx, _) = self.process_wiki_search_intro(&wiki_intro_query, &arb_id, &arb_name, &channel).await;
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
        // Skip RAG search when full document injection is active — the full text
        // is injected directly by the prompt builder.
        if !self.budget_for(&self.arbitre.config.id).full_document_mode {
            // Deferred embeddings (no-op if ready); without them the store answers lexically (BM25)
            if let Some(ref mut store) = self.rag_store {
                store.ensure_embeddings().await;
            }
            {
                if let Some(ref rag_store) = self.rag_store {
                    if !rag_store.is_empty() {
                        let lang = &self.config.discussion_language;
                        match rag_store
                            .query(
                                &self.config.topic,
                                lang,
                                self.llm.as_ref(),
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
                                let arb_name = self.arbitre.config.name.clone();
                                let refs: Vec<(String, String)> = chunks.iter().map(rag_source_ref).collect();
                                let _ = channel.send(ArenaEvent::RagContextInjected {
                                    speaker_id: self.arbitre.config.id.clone(),
                                    speaker_name: self.arbitre.config.name.clone(),
                                    chunks,
                                    cached: false,
                                });
                                self.register_sources(&arb_name, SourceKind::Rag, refs);
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
            }
        }

        let participant_names: Vec<String> = self
            .gladiateurs
            .iter()
            .map(|g| g.config.name.clone())
            .collect();
        let intro_full_doc = self.full_document_for(&self.arbitre.config.id);
        let intro_prompt = prompt_builder::build_introduction_prompt(
            &self.config.topic,
            &participant_names,
            &self.config.discussion_language,
            intro_search_context.as_deref(),
            &self.config.discussion_mode,
            intro_full_doc.as_deref(),
            self.budget_for(&self.arbitre.config.id),
        );
        let intro_request = LlmRequest::new(
            &self.arbitre_system,
            &intro_prompt,
            &self.config.arbitre.llm_params,
            CallKind::Introduction,
        )
        .reasoning(self.arbitre_reasoning_level())
        .pace(self.reasoning_pace)
        .speaker(&self.arbitre.config.id);
        let ch = channel.clone();
        let arb_id = self.arbitre.config.id.clone();
        let on_token = move |token: &str| {
            let _ = ch.send(ArenaEvent::MessageChunk {
                speaker_id: arb_id.clone(),
                chunk: token.to_string(),
            });
        };
        match self
            .llm
            .chat_stream(&intro_request, &on_token, &|_| {}, self.cancel_token.clone())
            .await
        {
            Ok(resp) => {
                let content = resp.content;
                let arb_id = self.arbitre.config.id.clone();
                let arb_name = self.arbitre.config.name.clone();
                let msg = self.create_message(&arb_id, &arb_name, SpeakerRole::Arbitre, &content);
                let _ = channel.send(ArenaEvent::MessageComplete {
                    message: msg.clone(),
                });
                self.remember_arbitre_opening(&content);
                self.messages_history.push(msg);
            }
            Err(LlmError::Cancelled) => {
                // Tokens already consumed by the introduction are billed: never lose them
                self.record_period_usage().await;
                let _ = channel.send(ArenaEvent::DiscussionEnded);
                return;
            }
            Err(e) => {
                let _ = channel.send(ArenaEvent::Error {
                    message: e.to_string(),
                });
            }
        }
        if self.llm.has_fatal_error() {
            self.record_period_usage().await;
            let _ = channel.send(ArenaEvent::Error { message: self.provider_fatal_msg() });
            let _ = channel.send(ArenaEvent::DiagnosticsReady { diagnostics: self.diagnostics.report() });
            let _ = channel.send(ArenaEvent::DiscussionEnded);
            return;
        }
        if self.emit_usage_and_check_budget(&channel) {
            self.status = DiscussionStatus::StopRequested;
        }

        // --- HIDDEN AGENDAS (v1.19): one secret objective per gladiateur ---
        if !self.should_stop() {
            self.generate_agendas().await;
        }
        // --- STRUCTURED MODES (v1.19): the crisis dispatches, the audience's first vote ---
        if !self.should_stop() {
            self.generate_crisis_dispatches().await;
            self.collect_audience_vote(VotePhase::Before, &mut cmd_rx, &channel).await;
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
            self.turn_audience_targets.clear();
            self.stage_directions_this_turn.clear();
            self.turn_timer.reset();
            self.turn_timer.start("speakers");

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
                    } else {
                        let _ = channel.send(ArenaEvent::SpeakerPassed {
                            speaker_id: self.gladiateurs[i].config.id.clone(),
                            speaker_name: self.gladiateurs[i].config.name.clone(),
                        });
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
                        llm: self.llm.clone(),
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

            // Dramaturgy (v1.18): act of the turn, scene event, coalition — may reorder or shorten the turn
            let order = self.stage_turn(order, &channel).await;

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
                    self.socratic_questions.push(question);
                }
            }

            // Reset per-turn state: search dedup, focus rotation, document contributions
            self.turn_search_queries.clear();
            self.turn_focus_targets.clear();
            self.turn_document_contributions.clear();

            // FOR EACH ACTIVE GLADIATEUR
            let mut broke_early = false;
            let total_speakers = order.len();
            for (speaker_pos, &glad_idx) in order.iter().enumerate() {
                self.process_commands(&mut cmd_rx, &channel).await;
                // A forced stop ends the turn at once. A soft stop (v1.18) lets the
                // remaining speakers of the turn deliver their closing statements,
                // then the end of turn runs and the discussion moves to the synthesis.
                // (max_turns is checked at the top of the outer loop, before incrementing)
                if self.status == DiscussionStatus::ForceStopRequested {
                    broke_early = true;
                    break;
                }

                // Step mode (v1.20.1): the voice may still be reading — wait for the audience's cue
                self.await_cue(glad_idx, &mut cmd_rx, &channel).await;
                if self.status == DiscussionStatus::ForceStopRequested || self.cancel_token.is_cancelled() {
                    broke_early = true;
                    break;
                }

                let speaker_id = self.gladiateurs[glad_idx].config.id.clone();
                let speaker_name = self.gladiateurs[glad_idx].config.name.clone();

                let _ = channel.send(ArenaEvent::SpeakerActive {
                    speaker_id: speaker_id.clone(),
                });

                // C.2 DEFERRED REACTIONS (turn > 1): react to the previous turn before speaking
                if self.current_turn > 1 && self.config.features.reaction_timing == ReactionTiming::Deferred {
                    self.process_reactions(glad_idx, &channel).await;
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
                            topic_q.clone(), &speaker_id, &speaker_name, "web",
                        ).await;
                        tracing::info!(speaker = %speaker_name, query = %query, "Forced first-turn web query");
                        self.gladiateurs[glad_idx].search_queries_history.push(query.clone());
                        let (ctx, count, _, sources) = self.process_web_search(
                            &search_sys, &speaker_id, &speaker_name, web_max, "", "",
                            Some(vec![query.clone()]),
                            &self.gladiateurs[glad_idx].config.llm_params, &channel,
                            &self.gladiateurs[glad_idx].search_queries_history,
                            &self.turn_search_queries,
                        ).await;
                        self.register_sources(&speaker_name, SourceKind::Web, sources.into_iter().map(|s| (s.title, s.url)));
                        self.web_searches_used_pool += count;
                        self.forced_web_done.insert(glad_idx);
                        self.turn_search_queries.push((speaker_name.clone(), query));
                        web_ctx = ctx;
                    } else if web_can {
                        // Phase 2: LLM decides whether to search
                        let directive = prompt_builder::default_search_directive(&lang);
                        let (ctx, count, executed, sources) = self.process_web_search(
                            &search_sys, &speaker_id, &speaker_name, web_max, directive, &recent,
                            None, &self.gladiateurs[glad_idx].config.llm_params, &channel,
                            &self.gladiateurs[glad_idx].search_queries_history,
                            &self.turn_search_queries,
                        ).await;
                        self.register_sources(&speaker_name, SourceKind::Web, sources.into_iter().map(|s| (s.title, s.url)));
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
                            speaker_name.clone(), &speaker_id, &speaker_name, "wiki",
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
                            String::new(), &speaker_id, &speaker_name, "wiki",
                        ).await;
                        if !query.is_empty() {
                            // Hard dedup: skip if query is near-duplicate of speaker's own past searches
                            if is_duplicate_query(&query, &self.gladiateurs[glad_idx].search_queries_history) {
                                tracing::info!(speaker = %speaker_name, query = %query, "Wiki query is near-duplicate of own past — skipping");
                            } else {
                                self.gladiateurs[glad_idx].search_queries_history.push(query.clone());
                                let (ctx, count) = self.process_wiki_search(
                                    glad_idx, query.clone(), &channel,
                                ).await;
                                self.wiki_searches_used_pool += count;
                                self.turn_search_queries.push((speaker_name.clone(), query));
                                wiki_ctx = ctx;
                            }
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
                    // Skip RAG when full document mode is active — the full text
                    // is injected directly by the prompt builder.
                    if !self.budget_for(&speaker_id).full_document_mode {
                        let rag_ctx = self.process_rag_query(
                            &speaker_id, &speaker_name, glad_idx, &channel,
                        ).await;
                        if let Some(rag) = rag_ctx {
                            search_ctx = Some(match search_ctx {
                                Some(existing) => format!("{existing}\n\n{rag}"),
                                None => rag,
                            });
                        }
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

                // C.2.7 CONVERSATIONAL FOCUS (rotating, turns ≥ 2) + reasoning level
                let focus = self.pick_focus(glad_idx);
                let reasoning = self.resolve_gladiateur_reasoning(glad_idx);

                // C.2.8 DYNAMIC DIRECTIVE (emotion_driven only)
                let dynamic_directive: Option<String> = if self.emotion_driven {
                    let directive_output = self.build_directive_for_speaker(glad_idx, focus.clone());
                    tracing::info!(
                        speaker = %speaker_name,
                        turn = self.current_turn,
                        speech_act = %directive_output.speech_act,
                        focus = ?focus,
                        "Dynamic directive generated"
                    );
                    let _ = channel.send(ArenaEvent::DirectiveGenerated {
                        speaker_id: speaker_id.clone(),
                        speaker_name: speaker_name.clone(),
                        speech_act: directive_output.speech_act.clone(),
                        emotion_behavior: directive_output.emotion_behavior.clone(),
                        relationship_summary: directive_output.relationship_summary.clone(),
                        focus_speaker: focus.as_ref().and_then(Focus::speaker_name).map(str::to_string),
                        reasoning_level: reasoning.as_str().to_string(),
                    });
                    Some(directive_output.directive_text)
                } else {
                    None
                };

                // C.3 INNER THOUGHT + C.4 PUBLIC INTERVENTION
                // Full document injection: when the budget allows, inject the entire RAG document
                // directly into the prompt instead of using chunk-based search.
                let full_doc_text = self.full_document_for(&speaker_id);

                // Staging of the turn for this speaker (act + scene event, v1.18)
                self.refresh_act_on_stop(&channel).await;
                let stage = self.stage_block_for(&speaker_name);
                let stage_ref = stage.as_ref();

                // C.3 INTENTION (v1.17): the pre-speech contract, decided in character
                // before any reasoning; its thought is the persona's private reflection.
                let (intention, thought, intention_broken) = self.process_intention(glad_idx, search_context.as_deref(), focus.as_ref(), &channel).await;
                if intention_broken {
                    self.diagnostics.note_parse_failure(CallKind::Intention);
                }
                let loops: Vec<OpenLoop> = self.open_loops.for_speaker(&speaker_id).into_iter().cloned().collect();
                let loop_refs: Vec<&OpenLoop> = loops.iter().collect();

                // With an active reasoning level the model reasons internally on top
                // of the intention (the native reasoning keeps the display priority).
                let (thought, content, thought_kind) = if reasoning.is_active() {
                    tracing::info!(
                        speaker = %speaker_name,
                        turn = self.current_turn,
                        level = reasoning.as_str(),
                        "Using model reasoning for intervention"
                    );
                    let (shown_reasoning, c) = self
                        .process_intervention(glad_idx, thought.as_deref(), search_context.as_deref(), dynamic_directive.as_deref(), full_doc_text.as_deref(), reasoning, focus.as_ref(), intention.as_ref(), &loop_refs, stage_ref, &channel)
                        .await;
                    if c.is_none() {
                        // Reasoning produced nothing usable — plain intervention with the same contract
                        self.note_reasoning_failure(&speaker_name);
                        let (_, content) = self
                            .process_intervention(glad_idx, thought.as_deref(), search_context.as_deref(), dynamic_directive.as_deref(), full_doc_text.as_deref(), ReasoningLevel::Off, focus.as_ref(), intention.as_ref(), &loop_refs, stage_ref, &channel)
                            .await;
                        (thought, content, ThoughtKind::Persona)
                    } else {
                        (shown_reasoning.or(thought), c, ThoughtKind::Reasoning)
                    }
                } else {
                    let (_, content) = self
                        .process_intervention(glad_idx, thought.as_deref(), search_context.as_deref(), dynamic_directive.as_deref(), full_doc_text.as_deref(), ReasoningLevel::Off, focus.as_ref(), intention.as_ref(), &loop_refs, stage_ref, &channel)
                        .await;
                    (thought, content, ThoughtKind::Persona)
                };
                if let Some(text) = &content {
                    self.settle_open_loops(glad_idx, intention.as_ref());
                    // Intention compliance: the declared target must be named in the spoken text (case-insensitive)
                    if let Some(target) = intention.as_ref().and_then(|i| i.target.as_deref()) {
                        self.diagnostics.note_intention(crate::engine::json_parser::mentions_name(text, target));
                    }
                }

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
                    msg.thought_kind = thought_kind;
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
                    if own_msgs.len() > constants::SELF_MEMORY_MESSAGES {
                        own_msgs.remove(0);
                    }

                    // Pass 2: document update — per intervention (legacy) or batched at end of turn
                    if self.config.document_format != DocumentFormat::None {
                        match self.config.document_update_granularity {
                            DocumentUpdateGranularity::Intervention => {
                                let llm_params = self.gladiateurs[glad_idx].config.llm_params.clone();
                                self.apply_document_update(&speaker_id, &speaker_name, &discussion_text, &llm_params, &channel).await;
                            }
                            DocumentUpdateGranularity::Turn => {
                                self.turn_document_contributions.push((speaker_name.clone(), discussion_text.clone()));
                            }
                        }
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
                    // The audience's message has had its answer (the moderator saw whether it was ignored)
                    if self.user_reply_pending.take().is_some() {
                        tracing::info!(speaker = %speaker_name, turn = self.current_turn, "Audience message answered (or at least addressed)");
                    }
                }

                // C.6.5 IMMEDIATE REACTIONS: everyone else reacts to what was just said
                if content.is_some() {
                    if let Some(last) = self.turn_messages.iter().rev().find(|m| m.speaker_id == speaker_id && m.kind == MessageKind::Normal).cloned() {
                        self.process_reaction_round(&last, &channel).await;
                    }
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

                // C.8 USAGE + BUDGET GUARD + FATAL PROVIDER ERRORS
                if self.llm.has_fatal_error() {
                    let _ = channel.send(ArenaEvent::Error { message: self.provider_fatal_msg() });
                    self.status = DiscussionStatus::ForceStopRequested;
                    broke_early = true;
                    break;
                }
                if self.emit_usage_and_check_budget(&channel) {
                    // Budget exhausted: finish this turn, skip the rest, synthesize.
                    self.status = DiscussionStatus::StopRequested;
                    broke_early = true;
                    break;
                }
            }

            // Only a hard stop (or cancellation) skips the end-of-turn processing.
            // A soft stop and the max_turns limit still get memory, emotions and the
            // argument map updated for the last turn — the synthesis relies on them.
            if broke_early || self.status == DiscussionStatus::ForceStopRequested || self.cancel_token.is_cancelled() {
                break;
            }

            // D. Obligatory user intervention if still pending
            if self.user_intervention_pending && !self.user_intervention_handled {
                self.handle_user_intervention(&mut cmd_rx, &channel).await;
            }

            // E. END OF TURN — document + emotion analysis + contagion + history + memory update
            // All sequential because they mutate &mut self

            // E.0/E.1/E.4/E.5 — document, emotions, memory and argument map: the LLM
            // calls run together (bounded by the provider), the results are applied
            // in the historical order (v1.17). Contagion, room mood, decay and the
            // history snapshot slot in between exactly as before.
            self.turn_timer.start("endOfTurn");
            self.run_end_of_turn_calls(&channel).await;

            // Reaction drought (turn ≥ 2 without a single like/dislike). Only a
            // signal when reactions are possible at all: a lone active gladiateur
            // has nobody to react to.
            let any_reaction = self.turn_reaction_counts.values().any(|t| !t.is_empty());
            let reactions_possible = turn_manager::active_count(&self.gladiateurs) >= 2;
            self.turns_without_reactions = if self.current_turn > 1 && reactions_possible && !any_reaction {
                self.turns_without_reactions + 1
            } else {
                0
            };

            // Decrement bans
            let unbanned = turn_manager::decrement_bans(&mut self.gladiateurs);
            for (id, name) in unbanned {
                let _ = channel.send(ArenaEvent::BanLifted {
                    speaker_id: id.clone(),
                    speaker_name: name.clone(),
                });
                let lifted_text = self.ban_lifted_msg(&name);
                self.emit_ban_notification(&lifted_text, &channel);
                self.emit_stage_direction(&id, StageCue::Returned, &channel);
            }

            self.user_intervention_handled = false;
            let timings = self.turn_timer.finish(self.current_turn);
            tracing::info!(turn = timings.turn, phases = ?timings.phases, "Turn timings");
            let _ = channel.send(ArenaEvent::TurnTimings { timings });
            if self.emit_usage_and_check_budget(&channel) {
                self.status = DiscussionStatus::StopRequested;
            }
        }

        // --- SYNTHESIS (always, even on force-stop) ---
        // If the cancel token was triggered (force-stop), create a fresh one
        // so the synthesis LLM call can proceed without immediate cancellation.
        if self.cancel_token.is_cancelled() {
            self.cancel_token = CancellationToken::new();
        }
        // --- MODE OUTCOME (v1.19): the audience's last vote, the verdict or the agreement ---
        if self.status != DiscussionStatus::ForceStopRequested {
            self.collect_audience_vote(VotePhase::After, &mut cmd_rx, &channel).await;
        }
        if !self.llm.has_fatal_error() {
            self.outcome = self.resolve_outcome().await;
            if let Some(outcome) = &self.outcome {
                tracing::info!(?outcome, "Mode outcome");
                let _ = channel.send(ArenaEvent::OutcomeReady { outcome: outcome.clone() });
            }
        }
        tracing::info!(
            discussion_id = %self.discussion_id,
            status = ?self.status,
            turns_completed = self.current_turn,
            "Starting synthesis generation"
        );
        let summary = if self.llm.has_fatal_error() {
            tracing::warn!(discussion_id = %self.discussion_id, "Skipping synthesis after a fatal provider error");
            let _ = channel.send(ArenaEvent::SynthesisComplete { summary: String::new() });
            String::new()
        } else {
            let summary = self.generate_synthesis(&channel).await;
            tracing::info!(discussion_id = %self.discussion_id, "Synthesis generation complete");
            summary
        };
        // The secret agendas come to light once the moderator has judged them (v1.19)
        let agendas = self.agenda_reveals(&summary);
        if !agendas.is_empty() {
            let _ = channel.send(ArenaEvent::AgendaRevealed { agendas });
        }
        // Long memory (v1.20): what each persona keeps of this discussion
        self.write_recaps(&channel).await;
        self.emit_usage_and_check_budget(&channel);
        self.record_period_usage().await;

        let diagnostics = self.diagnostics.report();
        tracing::info!(?diagnostics, "Discussion diagnostics");
        let _ = channel.send(ArenaEvent::DiagnosticsReady { diagnostics });
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
                EngineCommand::AudienceReaction { message_id, reaction_type } => {
                    self.handle_audience_reaction(&message_id, reaction_type, channel);
                }
                EngineCommand::AudienceVote { choice } => {
                    tracing::info!(choice, turn = self.current_turn, "Audience vote outside a voting window — ignored");
                }
                EngineCommand::SetStepMode { enabled } => self.set_step_mode(enabled),
                EngineCommand::NextSpeaker => self.cues_pending += 1,
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
                    Some(EngineCommand::AudienceReaction { message_id, reaction_type }) => {
                        self.handle_audience_reaction(&message_id, reaction_type, channel);
                    }
                    Some(EngineCommand::SetStepMode { enabled }) => self.set_step_mode(enabled),
                    Some(EngineCommand::NextSpeaker) => self.cues_pending += 1,
                    Some(_) => {}
                    None => return true,
                }
            }
        }
        false
    }

    /// Deferred timing (v1.16): the speaker reacts to the previous turn just before speaking.
    async fn process_reactions(&mut self, glad_idx: usize, channel: &Channel<ArenaEvent>) {
        let speaker_name = self.gladiateurs[glad_idx].config.name.clone();
        let speaker_id = self.gladiateurs[glad_idx].config.id.clone();
        let prev_msgs: Vec<Message> = self
            .messages_history
            .iter()
            .filter(|m| {
                m.turn_number == self.current_turn - 1
                    && m.speaker_id != speaker_id
                    && m.role != SpeakerRole::Arbitre
                    && m.kind == MessageKind::Normal
            })
            .cloned()
            .collect();

        if prev_msgs.is_empty() {
            tracing::info!(speaker = %speaker_name, turn = self.current_turn, "No previous turn messages found for reactions");
            return;
        }

        let request = self.reaction_request(glad_idx, &prev_msgs, ReactionScope::PreviousTurn);
        let raw = match self.chat_text(&request).await {
            Ok(raw) => raw,
            Err(e) => {
                tracing::warn!(speaker = %speaker_name, error = %e, "Reaction LLM call failed");
                return;
            }
        };
        let applied = self.apply_parsed_reactions(glad_idx, &raw, &prev_msgs, channel);
        if applied > 0 {
            self.emit_relationships(channel);
        }
    }

    /// Immediate timing: every other active gladiateur reacts to the message that
    /// just ended (parallel when the provider allows it), then the target feels it.
    async fn process_reaction_round(&mut self, message: &Message, channel: &Channel<ArenaEvent>) {
        if self.config.features.reaction_timing != ReactionTiming::Immediate || self.cancel_token.is_cancelled() {
            return;
        }
        let reactors: Vec<usize> = (0..self.gladiateurs.len())
            .filter(|&i| !self.gladiateurs[i].is_banned() && self.gladiateurs[i].config.id != message.speaker_id)
            .collect();
        if reactors.is_empty() {
            return;
        }
        let targets = vec![message.clone()];
        let requests: Vec<(usize, LlmRequest)> = reactors
            .iter()
            .map(|&i| (i, self.reaction_request(i, &targets, ReactionScope::LastIntervention)))
            .collect();
        let llm: Arc<dyn LlmProvider> = self.llm.clone();
        let cancel = self.cancel_token.clone();
        let futures = requests.into_iter().map(|(i, req)| {
            let llm = Arc::clone(&llm);
            let cancel = cancel.clone();
            async move { (i, llm.chat(&req, cancel).await.map(|r| r.content)) }
        });
        let limit = self.llm.capabilities().max_parallel_calls;
        let results = run_bounded(futures.collect(), limit).await;

        let before = self.turn_reaction_counts.get(&message.speaker_id).copied().unwrap_or_default();
        let mut applied = 0usize;
        for (i, result) in results {
            match result {
                Ok(raw) => applied += self.apply_parsed_reactions(i, &raw, &targets, channel),
                Err(LlmError::Cancelled) => return,
                Err(e) => tracing::warn!(speaker = %self.gladiateurs[i].config.name, error = %e, "Reaction LLM call failed"),
            }
        }
        if applied == 0 {
            return;
        }
        // The target feels the round right away (rule effects on what was just received)
        let after = self.turn_reaction_counts.get(&message.speaker_id).copied().unwrap_or_default();
        let round = after.since(&before);
        self.apply_received_to_speaker(&message.speaker_id, &round, channel);
        self.emit_relationships(channel);
    }

    /// Current reaction graph (kinds, scores, trends) for the UI.
    fn emit_relationships(&mut self, channel: &Channel<ArenaEvent>) {
        let edges = self.relationship_scores.edges(&self.tuning);
        let _ = channel.send(ArenaEvent::RelationshipsUpdated { edges });
    }

    /// One theatre line about `subject_id` (gladiateur or moderator), emotion-driven
    /// only, at most `STAGE_DIRECTIONS_PER_SPEAKER_PER_TURN` per speaker and turn.
    /// Shown in the feed and persisted by the frontend; never part of the prompts.
    fn emit_stage_direction(&mut self, subject_id: &str, cue: StageCue<'_>, channel: &Channel<ArenaEvent>) {
        if !self.emotion_driven {
            return;
        }
        let shown = self.stage_directions_this_turn.entry(subject_id.to_string()).or_insert(0);
        if *shown >= constants::STAGE_DIRECTIONS_PER_SPEAKER_PER_TURN {
            return;
        }
        let (name, role) = if subject_id == self.arbitre.config.id {
            (self.arbitre.config.name.clone(), SpeakerRole::Arbitre)
        } else if let Some(g) = self.gladiateurs.iter().find(|g| g.config.id == subject_id) {
            (g.config.name.clone(), SpeakerRole::Gladiateur)
        } else {
            return;
        };
        *shown += 1;
        let line = stage_directions::describe(&name, cue, self.dynamics_cache.get(subject_id), &self.config.discussion_language);
        let mut msg = self.create_message(subject_id, &name, role, &line);
        msg.kind = MessageKind::StageDirection;
        tracing::info!(subject = %name, line = %line, "Stage direction");
        let _ = channel.send(ArenaEvent::MessageComplete { message: msg });
    }

    /// Temperature of the room from the active gladiateurs (none below two):
    /// remembered for the moderator and emitted for the UI.
    fn emit_room_mood(&mut self, channel: &Channel<ArenaEvent>) {
        let profiles: Vec<&EmotionalProfile> = self.gladiateurs.iter().filter(|g| !g.is_banned()).map(|g| &g.emotions).collect();
        let Some((avg, label)) = emotion_engine::room_mood(&profiles) else {
            self.room_mood = None;
            return;
        };
        self.room_mood = Some(label);
        let _ = channel.send(ArenaEvent::RoomMoodUpdated { avg, label });
    }

    /// JSON reaction request of a gladiateur about `targets` (persona-aware, with their propensity).
    fn reaction_request(&self, glad_idx: usize, targets: &[Message], scope: ReactionScope) -> LlmRequest {
        let interventions: Vec<(String, String)> = targets.iter().map(|m| (m.speaker_name.clone(), m.content.clone())).collect();
        let propensity = ReactionPropensity::from_ocean(prompt_builder::parse_ocean_values(&self.gladiateurs[glad_idx].config.system_prompt));
        let prompt = prompt_builder::build_reaction_prompt(
            &interventions,
            &self.config.discussion_language,
            &self.config.discussion_mode,
            scope,
            propensity.as_ref(),
            self.insightful_credit(&self.gladiateurs[glad_idx].config.id),
        );
        LlmRequest::new(
            &self.gladiateurs[glad_idx].config.system_prompt,
            &prompt,
            &self.gladiateurs[glad_idx].config.llm_params,
            CallKind::Reaction,
        )
        .json()
        .speaker(&self.gladiateurs[glad_idx].config.id)
    }

    /// Whether `reactor_id` may still single out a strong point: none of its last
    /// `INSIGHTFUL_CREDIT_WINDOW` reactions was a 💡 (v1.20.5).
    fn insightful_credit(&self, reactor_id: &str) -> bool {
        self.recent_given_kinds.get(reactor_id).is_none_or(|kinds| !kinds.contains(&ReactionType::Insightful))
    }

    /// Remember a colour a gladiateur gave, within the credit window.
    fn note_given_kind(&mut self, reactor_id: &str, kind: ReactionType) {
        let kinds = self.recent_given_kinds.entry(reactor_id.to_string()).or_default();
        kinds.push_back(kind);
        while kinds.len() > constants::INSIGHTFUL_CREDIT_WINDOW {
            kinds.pop_front();
        }
    }

    /// Parse a reactor's JSON and apply every valid reaction to the matching target
    /// message. Returns how many were applied.
    fn apply_parsed_reactions(&mut self, glad_idx: usize, raw: &str, targets: &[Message], channel: &Channel<ArenaEvent>) -> usize {
        let reactor_name = self.gladiateurs[glad_idx].config.name.clone();
        let reactor_id = self.gladiateurs[glad_idx].config.id.clone();
        let known: Vec<String> = targets.iter().map(|m| m.speaker_name.clone()).collect();
        tracing::info!(speaker = %reactor_name, raw_len = raw.len(), raw_preview = %truncate_str(raw, 300), "Reaction LLM response");
        let parsed = json_parser::parse_reactions(raw, &known);
        if parsed.is_empty() {
            tracing::info!(speaker = %reactor_name, "No reaction parsed (silence or invalid JSON)");
            return 0;
        }
        let allowed = reactions::allowed_reaction_types(&self.config.discussion_mode);
        let mut applied = 0;
        for p in parsed {
            // Skip self-reactions (by name — handles duplicate gladiateur names)
            if p.speaker_name.eq_ignore_ascii_case(reactor_name.trim()) {
                continue;
            }
            if !allowed.contains(&p.reaction_type) {
                tracing::debug!(speaker = %reactor_name, kind = p.reaction_type.as_str(), "Reaction colour not allowed in this mode — skipped");
                continue;
            }
            let Some(target) = targets.iter().find(|m| m.speaker_name.eq_ignore_ascii_case(p.speaker_name.trim())) else {
                tracing::warn!(from = %reactor_name, parsed_name = %p.speaker_name, "Reaction target not found");
                continue;
            };
            // v1.20.5 — a 💡 beyond the credit is recorded as the approval it also is
            let kind = if p.reaction_type == ReactionType::Insightful && !self.insightful_credit(&reactor_id) {
                tracing::info!(from = %reactor_name, to = %target.speaker_name, "Insightful credit spent — recorded as a like");
                ReactionType::Like
            } else {
                p.reaction_type
            };
            self.note_given_kind(&reactor_id, kind);
            let reaction = Reaction {
                from_speaker_id: reactor_id.clone(),
                from_speaker_name: reactor_name.clone(),
                reaction_type: kind,
                target_message_id: target.id.clone(),
                justification: p.justification.clone(),
                quote: super::validated_quote(&target.content, p.quote.as_deref()),
            };
            self.apply_reaction(Some(glad_idx), target, reaction, channel);
            applied += 1;
        }
        applied
    }

    /// Record one reaction on `target`: engine-side message copies, per-turn and
    /// last-message tallies, the reaction graph (gladiateurs only), the reactor's
    /// own emotions, and the frontend event. The target's emotions are applied by
    /// the caller (deferred: at their next intervention; immediate: after the round).
    fn apply_reaction(&mut self, reactor_idx: Option<usize>, target: &Message, reaction: Reaction, channel: &Channel<ArenaEvent>) {
        tracing::info!(from = %reaction.from_speaker_name, to = %target.speaker_name, reaction = reaction.reaction_type.as_str(), "Reaction emitted");
        for list in [&mut self.turn_messages, &mut self.messages_history] {
            if let Some(m) = list.iter_mut().find(|m| m.id == target.id) {
                m.reactions.push(reaction.clone());
            }
        }
        self.turn_reaction_counts.entry(target.speaker_id.clone()).or_default().add(reaction.reaction_type);
        self.last_reactions_received.entry(target.speaker_id.clone()).or_default().add(reaction.reaction_type);
        if reaction.reaction_type == ReactionType::Question && target.role == SpeakerRole::Gladiateur {
            if let Some(text) = reaction.justification.as_deref().or(reaction.quote.as_deref()) {
                self.open_loops.push(&target.speaker_id, OpenLoop::new(OpenLoopKind::Question, text, &reaction.from_speaker_name, self.current_turn));
            }
        }
        if let Some(i) = reactor_idx {
            let kind = reaction.reaction_type;
            if !kind.is_neutral() {
                let (from, to) = (reaction.from_speaker_id.as_str(), target.speaker_id.as_str());
                let before = self.relationship_scores.classify_pair(from, to, &self.tuning);
                self.relationship_scores.record(from, to, kind.is_positive());
                let after = self.relationship_scores.classify_pair(from, to, &self.tuning);
                if before != after {
                    let _ = channel.send(ArenaEvent::RelationshipShift {
                        a: from.to_string(),
                        b: to.to_string(),
                        from: before.map_or("none", |k| k.as_str()).to_string(),
                        to: after.map_or("none", |k| k.as_str()).to_string(),
                    });
                    // A rival who approves: reconciliation — the receiver may acknowledge it once
                    if before == Some(RelationshipKind::Rival) && kind.is_positive() {
                        let (reactor_name, target_name, target_id) = (reaction.from_speaker_name.clone(), target.speaker_name.clone(), target.speaker_id.clone());
                        let reactor_id = reaction.from_speaker_id.clone();
                        self.reconciliation_pending.insert(target_id, reactor_name.clone());
                        self.emit_stage_direction(&reactor_id, StageCue::Reconciled { with: &target_name }, channel);
                    }
                }
            }
            let mut given = GivenReactions::default();
            given.add(kind);
            let prev = self.gladiateurs[i].emotions.clone();
            emotion_engine::apply_given_reactions(&mut self.gladiateurs[i].emotions, &given);
            if self.gladiateurs[i].emotions != prev {
                let sid = self.gladiateurs[i].config.id.clone();
                let current = self.gladiateurs[i].emotions.clone();
                self.emit_threshold_events(channel, &sid, &prev, &current);
                Self::emit_emotion_updated(channel, &sid, &current, &self.config.discussion_language);
            }
        }
        let _ = channel.send(ArenaEvent::ReactionEmitted { message_id: target.id.clone(), reaction });
    }

    /// Rule effects of reactions RECEIVED on a participant (gladiateur or moderator; no-op for the user).
    fn apply_received_to_speaker(&mut self, speaker_id: &str, tally: &ReactionTally, channel: &Channel<ArenaEvent>) {
        if tally.is_empty() {
            return;
        }
        let gains = self.persona_gains.get(speaker_id).copied().unwrap_or_default();
        let emotions = if speaker_id == self.arbitre.config.id {
            &mut self.arbitre.emotions
        } else if let Some(g) = self.gladiateurs.iter_mut().find(|g| g.config.id == speaker_id) {
            &mut g.emotions
        } else {
            return;
        };
        let prev = emotions.clone();
        emotion_engine::apply_received_reactions(emotions, tally, &gains);
        let current = emotions.clone();
        self.emit_threshold_events(channel, speaker_id, &prev, &current);
        Self::emit_emotion_updated(channel, speaker_id, &current, &self.config.discussion_language);
    }

    /// The audience (the user) reacts to a message: capped per message, felt by the
    /// target right away, and the room's interest nudges the next speakers' focus.
    fn handle_audience_reaction(&mut self, message_id: &str, kind: ReactionType, channel: &Channel<ArenaEvent>) {
        if !self.config.features.audience_reactions {
            tracing::info!("Audience reaction ignored: feature disabled for this discussion");
            return;
        }
        let Some(target) = self.messages_history.iter().find(|m| m.id == message_id && m.kind == MessageKind::Normal && m.role != SpeakerRole::User).cloned() else {
            tracing::warn!(message_id, "Audience reaction on an unknown or non-reactable message — ignored");
            return;
        };
        let given = self.audience_reactions_by_message.entry(target.id.clone()).or_insert(0);
        if *given >= constants::AUDIENCE_REACTIONS_PER_MESSAGE_MAX {
            tracing::info!(message_id, "Audience reaction cap reached for this message — ignored");
            return;
        }
        *given += 1;
        let reaction = Reaction {
            from_speaker_id: constants::USER_SPEAKER_ID.to_string(),
            from_speaker_name: self.config.user_name.clone(),
            reaction_type: kind,
            target_message_id: target.id.clone(),
            justification: None,
            quote: None,
        };
        self.apply_reaction(None, &target, reaction, channel);
        let mut tally = ReactionTally::default();
        tally.add(kind);
        self.apply_received_to_speaker(&target.speaker_id, &tally, channel);
        self.turn_audience_targets.insert(target.speaker_name.clone());
    }

    /// Relationship hints (ally / rival / tense) of a speaker, from the weighted reaction graph.
    fn relationships_for(&self, glad_idx: usize) -> Vec<directive_builder::RelationshipHint> {
        let speaker_id = &self.gladiateurs[glad_idx].config.id;
        self.gladiateurs
            .iter()
            .filter(|g| g.config.id != *speaker_id)
            .filter_map(|g| {
                self.relationship_scores
                    .classify_pair(speaker_id, &g.config.id, &self.tuning)
                    .map(|kind| directive_builder::RelationshipHint { other_name: g.config.name.clone(), kind, lean: self.relationship_scores.lean_of(speaker_id, &g.config.id) })
            })
            .collect()
    }

    /// Draw who this speaker should address (turns ≥ 2, non-fiction) and record
    /// the target so the next speakers of the turn favour someone else.
    fn pick_focus(&mut self, glad_idx: usize) -> Option<Focus> {
        // Fiction is a relay (no addressee); UserDriven answers the user's direction.
        if matches!(self.config.discussion_mode, DiscussionMode::CollaborativeFiction | DiscussionMode::UserDriven) {
            return None;
        }
        // The audience member who just spoke is answered first, whatever the turn (v1.20.2)
        if self.user_reply_pending.is_some() {
            let user = self.config.user_name.clone();
            self.turn_focus_targets.insert(user.clone());
            return Some(Focus::Speaker(user));
        }
        if self.current_turn < 2 {
            return None;
        }
        let speaker_id = &self.gladiateurs[glad_idx].config.id;
        let user_spoke_this_turn = self.turn_messages.iter().any(|m| m.role == SpeakerRole::User);
        let candidates: Vec<String> = self.gladiateurs
            .iter()
            .filter(|g| g.config.id != *speaker_id && !g.is_banned())
            .map(|g| g.config.name.clone())
            // Once they spoke this turn, the audience member is a candidate like the others
            .chain(user_spoke_this_turn.then(|| self.config.user_name.clone()))
            .collect();
        let prev_turn = self.current_turn - 1;
        let spoke_previous_turn: HashSet<String> = self.messages_history
            .iter()
            .filter(|m| m.turn_number == prev_turn && m.role == SpeakerRole::Gladiateur)
            .map(|m| m.speaker_name.clone())
            .collect();
        let spoke_this_turn: HashSet<String> = self.turn_messages
            .iter()
            .filter(|m| m.role == SpeakerRole::Gladiateur)
            .map(|m| m.speaker_name.clone())
            .collect();
        let related: HashSet<String> = self.relationships_for(glad_idx)
            .into_iter()
            .map(|r| r.other_name)
            .collect();
        let weights = focus::focus_weights(&FocusInputs {
            candidates: &candidates,
            spoke_previous_turn: &spoke_previous_turn,
            spoke_this_turn: &spoke_this_turn,
            targeted_this_turn: &self.turn_focus_targets,
            related: &related,
            audience_favoured: &self.turn_audience_targets,
        });
        let drawn = focus::draw_focus(&weights, &mut rand::thread_rng());
        if let Some(name) = drawn.speaker_name() {
            self.turn_focus_targets.insert(name.to_string());
        }
        Some(drawn)
    }

    /// Real stagnation signal (never a turn-number heuristic): near-identical
    /// summaries, a reaction drought, or the emotion analyst's flag.
    fn is_stagnating(&self) -> bool {
        self.summary_stagnating
            || self.llm_stagnation_flag
            || self.turns_without_reactions >= constants::EMOTION_STAGNATION_REACTION_DROUGHT_TURNS
    }

    /// Build the full directive for a specific speaker using the directive builder.
    fn build_directive_for_speaker(&mut self, glad_idx: usize, focus: Option<Focus>) -> directive_builder::DirectiveOutput {
        let relationships = self.relationships_for(glad_idx);
        let speaker_id = &self.gladiateurs[glad_idx].config.id;
        let speaker_name = &self.gladiateurs[glad_idx].config.name;

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

        // Fiction: who wrote the story opening this turn (user or a co-author)
        let opening_author = self.turn_messages
            .iter()
            .find(|m| m.role != SpeakerRole::Arbitre)
            .map(|m| m.speaker_name.clone());

        let is_first_speaker_this_turn = speakers_this_turn.is_empty();
        let ctx = SpeakerTurnContext {
            emotions: self.gladiateurs[glad_idx].emotions.clone(),
            baseline: self.gladiateurs[glad_idx].initial_emotions.clone(),
            relationships,
            own_previous_messages: self.speaker_own_messages
                .get(speaker_id)
                .cloned()
                .unwrap_or_default(),
            dynamics: self.dynamics_cache.get(speaker_id).cloned(),
            ocean: prompt_builder::parse_ocean_values(&self.gladiateurs[glad_idx].config.system_prompt),
            turn_number: self.current_turn,
            speakers_this_turn,
            is_first_speaker_this_turn,
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
            focus,
            recent_speech_acts: self.recent_speech_acts.get(speaker_id).cloned().unwrap_or_default(),
            opening_author,
            reconciliation_with: self.reconciliation_pending.remove(speaker_id),
            coalition: self.coalition_role_for(speaker_id),
            unanswered_objection: self.unanswered_objection_for(speaker_id),
            audience_message: self.user_reply_pending.clone(),
            user_has_spoken: self.user_has_spoken,
        };

        let output = directive_builder::build_dynamic_directive(&ctx);

        // Remember the act for the anti-repetition window
        if let Some(act) = SpeechAct::from_name(&output.speech_act) {
            let recent = self.recent_speech_acts.entry(speaker_id.clone()).or_default();
            recent.push(act);
            if recent.len() > constants::SPEECH_ACT_RECENT_WINDOW {
                recent.remove(0);
            }
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

    /// Other participants a speaker may address: active gladiateurs, plus the
    /// user when they spoke this turn. Nobody in fiction (a relay has no addressee).
    fn addressable_names(&self, glad_idx: usize) -> Vec<String> {
        if self.config.discussion_mode == DiscussionMode::CollaborativeFiction {
            return Vec::new();
        }
        let mut names: Vec<String> = self.gladiateurs
            .iter()
            .enumerate()
            .filter(|(i, g)| *i != glad_idx && !g.is_banned())
            .map(|(_, g)| g.config.name.clone())
            .collect();
        if self.user_reply_pending.is_some() || self.turn_messages.iter().any(|m| m.role == SpeakerRole::User) {
            names.push(self.config.user_name.clone());
        }
        names
    }

    /// Names a position may be recorded for (gladiateurs and the user — never the
    /// moderator, never an invented participant). Fuzzy keys are normalised so a
    /// participant's trajectory is one entry across turns.
    fn normalise_position_names(&self, positions: HashMap<String, json_parser::PositionInput>) -> HashMap<String, json_parser::PositionInput> {
        let mut known: Vec<String> = self.gladiateurs.iter().map(|g| g.config.name.clone()).collect();
        known.push(self.config.user_name.clone());
        positions
            .into_iter()
            .filter_map(|(name, input)| json_parser::match_speaker_name(&name, &known).map(|n| (n.clone(), input)))
            .collect()
    }

    /// Pre-speech contract (v1.17): one JSON call replacing the free-text thought.
    /// Returns `(intention, thought)`: a prose answer (no JSON) becomes the thought
    /// alone (v1.16 behaviour); a JSON answer yields both, its `thought` field
    /// being the persona's private reflection. An unknown or banned target is
    /// replaced by the current focus (or the topic). Emits `IntentionGenerated`
    /// and `ThoughtComplete` (when there is a thought).
    async fn process_intention(
        &self,
        glad_idx: usize,
        search_results: Option<&str>,
        focus: Option<&Focus>,
        channel: &Channel<ArenaEvent>,
    ) -> (Option<Intention>, Option<String>, bool) {
        let has_prior_context = self.turn_messages.iter().any(|m| m.kind == MessageKind::Normal)
            || !self.gladiateurs[glad_idx].memory.immediate.is_empty();
        let recent_exchanges = self.build_recent_exchanges(glad_idx);
        let speaker_id = self.gladiateurs[glad_idx].config.id.clone();
        let speaker_name = self.gladiateurs[glad_idx].config.name.clone();
        // Nobody has spoken yet (first speaker of the discussion): no one to address —
        // a target declared here could only be invented and would never be honoured
        let names = if has_prior_context { self.addressable_names(glad_idx) } else { Vec::new() };
        let loops = self.open_loops.for_speaker(&speaker_id);
        let prompt = prompt_builder::build_intention_prompt(
            &recent_exchanges,
            &self.gladiateurs[glad_idx].emotions,
            &self.config.discussion_language,
            has_prior_context,
            self.emotion_driven,
            self.current_turn,
            self.config.max_turns,
            search_results,
            &self.config.discussion_mode,
            self.budget_for(&speaker_id),
            &names,
            focus,
            &loops,
            self.agendas.get(&speaker_id),
        );
        let request = LlmRequest::new(
            &self.system_prompt_for(glad_idx),
            &prompt,
            &self.gladiateurs[glad_idx].config.llm_params,
            CallKind::Intention,
        )
        .json()
        .speaker(&speaker_id);

        let raw = match self.chat_text(&request).await {
            Ok(r) => r,
            Err(_) => return (None, None, false),
        };
        let raw = raw.trim();
        if raw.is_empty() {
            return (None, None, false);
        }
        let mut broken = false;
        let (intention, thought) = match json_parser::parse_intention(raw, &names) {
            Some(mut intention) => {
                // Unknown or banned target → the focus of the turn, else the topic
                if intention.target.as_deref().is_some_and(|t| !names.iter().any(|n| n == t)) {
                    let replacement = focus.and_then(Focus::speaker_name).map(str::to_string);
                    tracing::info!(speaker = %speaker_name, target = ?intention.target, replacement = ?replacement, "Intention target unknown or absent — replaced");
                    intention.target = replacement;
                }
                let thought = if intention.thought.is_empty() { None } else { Some(intention.thought.clone()) };
                (Some(intention), thought)
            }
            None if raw.starts_with('{') || raw.starts_with('[') => {
                tracing::warn!(speaker = %speaker_name, preview = %truncate_str(raw, 120), "Intention answer looks like broken JSON — ignored");
                broken = true;
                (None, None)
            }
            None => {
                tracing::info!(speaker = %speaker_name, "Intention answered in prose — used as the persona thought");
                (None, Some(raw.to_string()))
            }
        };
        if let Some(i) = &intention {
            tracing::info!(speaker = %speaker_name, target = ?i.target, goal = i.goal.as_str(), answers = ?i.answers, "Intention generated");
            let _ = channel.send(ArenaEvent::IntentionGenerated {
                speaker_id: speaker_id.clone(),
                speaker_name: speaker_name.clone(),
                target: i.target.clone(),
                goal: i.goal.as_str().to_string(),
                angle: i.angle.clone(),
                concession: i.concession.clone(),
                question: i.question.clone(),
            });
        }
        if let Some(t) = &thought {
            let _ = channel.send(ArenaEvent::ThoughtComplete { speaker_id, thought: t.clone() });
        }
        (intention, thought, broken)
    }

    /// After a spoken intervention: the loop the speaker declared answered is
    /// closed, the others lose a life; the intention's question opens a loop on
    /// its target and its concession becomes a commitment of the speaker.
    fn settle_open_loops(&mut self, glad_idx: usize, intention: Option<&Intention>) {
        let speaker_id = self.gladiateurs[glad_idx].config.id.clone();
        let speaker_name = self.gladiateurs[glad_idx].config.name.clone();
        if let Some(i) = intention {
            if let Some(index) = i.answers {
                self.open_loops.resolve(&speaker_id, index);
            }
        }
        self.open_loops.tick(&speaker_id);
        let Some(i) = intention else { return };
        if let (Some(question), Some(target)) = (&i.question, &i.target) {
            if let Some(target_id) = self.gladiateurs.iter().find(|g| g.config.name == *target).map(|g| g.config.id.clone()) {
                self.open_loops.push(&target_id, OpenLoop::new(OpenLoopKind::Question, question, &speaker_name, self.current_turn));
            }
        }
        if let Some(concession) = &i.concession {
            self.open_loops.push(&speaker_id, OpenLoop::new(OpenLoopKind::Commitment, concession, &speaker_name, self.current_turn));
        }
    }

    /// Generate a gladiateur's public intervention.
    ///
    /// With an active `reasoning` level the model reasons before answering (no
    /// separate thought phase); displayable reasoning is streamed as thought chunks
    /// and returned so it can be stored on the message. Returns
    /// `(displayed_reasoning, content)`.
    ///
    /// Empty or refused answers: under reasoning, returns `(None, None)` so the
    /// caller falls back to the thought + intervention path; otherwise the call is
    /// retried once without reasoning and with a higher temperature.
    #[allow(clippy::too_many_arguments)]
    async fn process_intervention(
        &mut self,
        glad_idx: usize,
        thought: Option<&str>,
        search_results: Option<&str>,
        dynamic_directive: Option<&str>,
        full_document: Option<&str>,
        reasoning: ReasoningLevel,
        focus: Option<&Focus>,
        intention: Option<&Intention>,
        open_loops: &[&OpenLoop],
        stage: Option<&StageBlock>,
        channel: &Channel<ArenaEvent>,
    ) -> (Option<String>, Option<String>) {
        // Exclude current speaker from participant names to prevent self-addressing
        let other_names: Vec<String> = self.gladiateurs.iter()
            .enumerate()
            .filter(|(i, _)| *i != glad_idx)
            .map(|(_, g)| g.config.name.clone())
            .collect();
        let (sys, usr) = prompt_builder::build_intervention_prompt(
            &self.system_prompt_for(glad_idx),
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
            full_document,
            self.budget_for(&self.gladiateurs[glad_idx].config.id),
            focus,
            intention,
            open_loops,
            stage,
            self.agendas.get(&self.gladiateurs[glad_idx].config.id),
            self.debate_state_block().as_deref(),
        );

        let speaker_id = self.gladiateurs[glad_idx].config.id.clone();
        let speaker_name = self.gladiateurs[glad_idx].config.name.clone();
        // Emotion-driven sampling (v1.17): enthusiasm → temperature, engagement → length
        let params = if self.emotion_driven {
            emotion_engine::modulate_sampling(&self.gladiateurs[glad_idx].config.llm_params, &self.gladiateurs[glad_idx].emotions, &self.tuning)
        } else {
            self.gladiateurs[glad_idx].config.llm_params.clone()
        };
        let request = LlmRequest::new(
            &sys,
            &usr,
            &params,
            CallKind::Intervention,
        )
        .reasoning(reasoning)
        .pace(self.reasoning_pace)
        .speaker(&speaker_id);

        let display_reasoning = reasoning.is_active()
            && self.show_model_reasoning
            && self.llm.capabilities_for(Some(&speaker_id)).reasoning_displayable;

        let ch = channel.clone();
        let sid = speaker_id.clone();
        let on_content = move |token: &str| {
            let _ = ch.send(ArenaEvent::MessageChunk {
                speaker_id: sid.clone(),
                chunk: token.to_string(),
            });
        };
        let ch_r = channel.clone();
        let sid_r = speaker_id.clone();
        let on_reasoning = move |token: &str| {
            if display_reasoning {
                let _ = ch_r.send(ArenaEvent::ThoughtChunk {
                    speaker_id: sid_r.clone(),
                    chunk: token.to_string(),
                });
            }
        };

        match self
            .llm
            .chat_stream(&request, &on_content, &on_reasoning, self.cancel_token.clone())
            .await
        {
            Ok(resp) if !resp.content.is_empty() && !is_model_refusal(&resp.content) => {
                tracing::info!(
                    discussion_id = %self.discussion_id,
                    turn = self.current_turn,
                    speaker = %speaker_name,
                    len = resp.content.len(),
                    reasoning = reasoning.as_str(),
                    truncated = resp.truncated,
                    "Intervention completed"
                );
                // The reasoning is kept with the message whenever the provider's
                // reasoning is displayable; the setting only controls live streaming.
                let stored = if self.llm.capabilities_for(Some(&speaker_id)).reasoning_displayable { resp.reasoning } else { None };
                if display_reasoning {
                    if let Some(r) = &stored {
                        let _ = channel.send(ArenaEvent::ThoughtComplete {
                            speaker_id: speaker_id.clone(),
                            thought: r.clone(),
                        });
                    }
                }
                (stored, Some(resp.content))
            }
            Ok(resp) if reasoning.is_active() => {
                // The caller owns the fallback (thought + plain intervention).
                if is_model_refusal(&resp.content) {
                    self.diagnostics.note_refusal();
                }
                self.diagnostics.note_retry();
                tracing::warn!(
                    speaker = %speaker_name,
                    empty = resp.content.is_empty(),
                    preview = %truncate_str(&resp.content, 120),
                    "Reasoning intervention unusable (empty or refusal) — falling back"
                );
                (None, None)
            }
            Ok(resp) => {
                let refusal = is_model_refusal(&resp.content);
                if refusal {
                    self.diagnostics.note_refusal();
                }
                self.diagnostics.note_retry();
                tracing::warn!(
                    discussion_id = %self.discussion_id,
                    turn = self.current_turn,
                    speaker = %speaker_name,
                    refusal,
                    system_prompt_len = sys.len(),
                    user_prompt_len = usr.len(),
                    "Intervention unusable — retrying with higher temperature"
                );
                tracing::debug!(
                    speaker = %speaker_name,
                    content = %resp.content,
                    "Response that triggered the retry"
                );
                let boost = if refusal { constants::TEMP_REFUSAL_BOOST } else { constants::TEMP_DIFFICULTY_BOOST };
                let content = self.retry_intervention(&request, boost).await;
                if content.is_none() {
                    let _ = channel.send(ArenaEvent::Error {
                        message: self.speaker_difficulty_msg(&speaker_name),
                    });
                }
                (None, content)
            }
            Err(LlmError::Cancelled) => (None, None),
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
                (None, None)
            }
        }
    }

    /// One non-streaming retry of an intervention without reasoning and with a
    /// temperature boost (temperature only acts outside reasoning mode).
    async fn retry_intervention(&self, request: &LlmRequest, temp_boost: f32) -> Option<String> {
        let mut retry = request.clone().reasoning(ReasoningLevel::Off);
        retry.params.temperature = (retry.params.temperature + temp_boost).min(constants::TEMP_MAX);
        let speaker = request.speaker_id.clone().unwrap_or_default();
        match self.chat_text(&retry).await {
            Ok(c) if !c.is_empty() && !is_model_refusal(&c) => {
                tracing::info!(speaker = %speaker, "Intervention retry succeeded");
                Some(c)
            }
            Ok(_) => {
                tracing::error!(speaker = %speaker, "Intervention retry also unusable");
                None
            }
            Err(e) => {
                tracing::error!(speaker = %speaker, error = %e, "Intervention retry failed");
                None
            }
        }
    }

    /// Per-intervention rule update of the speaker who just spoke. Deferred timing:
    /// the reactions received this turn on their previous message apply now (the
    /// ones they gave were applied as they reacted); immediate timing: received
    /// reactions were applied after each round, only the turn drift remains.
    fn update_emotions(&mut self, glad_idx: usize, channel: &Channel<ArenaEvent>) {
        let sid = self.gladiateurs[glad_idx].config.id.clone();
        let received = match self.config.features.reaction_timing {
            ReactionTiming::Deferred => self.turn_reaction_counts.get(&sid).copied().unwrap_or_default(),
            ReactionTiming::Immediate => ReactionTally::default(),
        };
        let ctx = EmotionContext { received, given: GivenReactions::default(), is_discussion_stagnating: self.is_stagnating() };

        let prev = self.gladiateurs[glad_idx].emotions.clone();
        let gains = self.persona_gains.get(&sid).copied().unwrap_or_default();
        let new_emo = emotion_engine::update_emotions(&prev, &self.gladiateurs[glad_idx].initial_emotions, &ctx, &gains);
        self.gladiateurs[glad_idx].emotions = new_emo.clone();
        // The speaker just spoke: reactions to this new message start from zero
        self.last_reactions_received.remove(&sid);

        self.emit_threshold_events(channel, &sid, &prev, &new_emo);
        Self::emit_emotion_updated(channel, &sid, &new_emo, &self.config.discussion_language);
        self.emit_room_mood(channel);
    }

    /// Once per turn (v1.20.5): the moderator feels a stagnating discussion and
    /// returns toward its neutral baseline like everyone else — once, not at every
    /// moderation, so a turn with six speakers does not damp it six times.
    fn settle_arbitre_emotions(&mut self, channel: &Channel<ArenaEvent>) {
        let prev = self.arbitre.emotions.clone();
        let stagnating = self.is_stagnating();
        emotion_engine::apply_turn_effects(&mut self.arbitre.emotions, &EmotionalProfile::default(), stagnating);
        if self.arbitre.emotions != prev {
            let (arb_id, current) = (self.arbitre.config.id.clone(), self.arbitre.emotions.clone());
            self.emit_threshold_events(channel, &arb_id, &prev, &current);
            Self::emit_emotion_updated(channel, &arb_id, &current, &self.config.discussion_language);
        }
    }

    /// Rule-based emotion update for IArbitre right after a moderation (bans only —
    /// stagnation and drift are settled once per turn by `settle_arbitre_emotions`)
    fn update_arbitre_emotions(&mut self, ban_issued: bool, channel: &Channel<ArenaEvent>) {
        let prev = self.arbitre.emotions.clone();

        if ban_issued {
            self.arbitre.emotions.frustration = emotion_engine::add_clamped(self.arbitre.emotions.frustration, constants::EMOTION_ARBITRE_BAN_DELTA);
            self.arbitre.emotions.confiance = emotion_engine::add_clamped(self.arbitre.emotions.confiance, constants::EMOTION_ARBITRE_BAN_DELTA);
        }

        let (arb_id, current) = (self.arbitre.config.id.clone(), self.arbitre.emotions.clone());
        self.emit_threshold_events(channel, &arb_id, &prev, &current);
        Self::emit_emotion_updated(channel, &arb_id, &current, &self.config.discussion_language);
    }

    async fn process_moderation(
        &mut self,
        glad_idx: usize,
        intervention: &str,
        channel: &Channel<ArenaEvent>,
    ) {
        let speaker_name = self.gladiateurs[glad_idx].config.name.clone();
        // The act's hint, the objection the speaker still owes (v1.20.1) and the audience's
        // message they were expected to answer (v1.20.2)
        let hint: Option<String> = {
            let parts: Vec<String> = self
                .current_act
                .map(|a| a.moderation_hint(&self.config.discussion_language).to_string())
                .into_iter()
                .chain(self.depth_hint_for(&self.gladiateurs[glad_idx].config.id))
                .chain(self.audience_hint_for(intervention))
                .collect();
            (!parts.is_empty()).then(|| parts.join("\n"))
        };
        let openings: Vec<String> = self.arbitre_recent_openings.iter().cloned().collect();
        let style = prompt_builder::ModerationStyle { recent_openings: &openings, comments: self.moderation_comments, moderated: self.moderation_checks };
        let prompt = prompt_builder::build_moderation_prompt(
            &speaker_name,
            intervention,
            &self.config.topic,
            &self.config.discussion_language,
            &self.config.discussion_mode,
            &prompt_builder::ModerationSituation { room_mood: self.room_mood, hint: hint.as_deref(), style: Some(&style) },
        );
        self.moderation_checks += 1;
        let request = LlmRequest::new(
            &self.arbitre_system,
            &prompt,
            &self.arbitre.config.llm_params,
            CallKind::Moderation,
        )
        .json()
        .speaker(&self.arbitre.config.id);

        let moderation = match self.chat_text(&request).await {
            Ok(raw) => json_parser::parse_moderation(&raw).unwrap_or_else(|e| {
                self.diagnostics.note_parse_failure(CallKind::Moderation);
                tracing::warn!(speaker = %speaker_name, error = %e, "Moderation answer unusable — no action");
                Default::default()
            }),
            Err(_) => return,
        };

        use crate::models::moderation::ModerationAction;
        tracing::info!(speaker = %speaker_name, turn = self.current_turn, action = ?moderation.action, comment = %truncate_str(&moderation.comment, 120), "Moderation decided");
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

                // Immediate emotional impact of the sanction
                let banned_id = self.gladiateurs[glad_idx].config.id.clone();
                let prev = self.gladiateurs[glad_idx].emotions.clone();
                emotion_engine::apply_ban_penalty(&mut self.gladiateurs[glad_idx].emotions);
                let current = self.gladiateurs[glad_idx].emotions.clone();
                // The sanction itself is the stage cue of the turn (thresholds it crosses stay silent)
                self.stage_directions_this_turn.insert(banned_id.clone(), constants::STAGE_DIRECTIONS_PER_SPEAKER_PER_TURN);
                self.emit_threshold_events(channel, &banned_id, &prev, &current);
                Self::emit_emotion_updated(channel, &banned_id, &current, &self.config.discussion_language);

                let _ = channel.send(ArenaEvent::BanIssued {
                    banned_id: self.gladiateurs[glad_idx].config.id.clone(),
                    banned_name: speaker_name.clone(),
                    reason: moderation.ban_reason.clone(),
                    duration,
                });

                let ban_text = self.ban_notification_msg(&speaker_name, duration, &moderation.ban_reason);
                self.emit_ban_notification(&ban_text, channel);
                self.stage_directions_this_turn.remove(&banned_id);
                self.emit_stage_direction(&banned_id, StageCue::Banned, channel);
            }
            ModerationAction::Comment if !moderation.comment.is_empty() => {
                self.moderation_comments += 1;
                self.remember_arbitre_opening(&moderation.comment);
                self.emit_arbitre_message(&moderation.comment, channel);
            }
            _ => {}
        }
    }

    /// The moderator says an announcement in its own voice (v1.20.3): one short call
    /// briefed by the template; the brief itself is the fallback (failure, refusal,
    /// empty answer, cancellation). Remembered as one of its openings.
    pub(super) async fn voice_announcement(&mut self, brief: &str) -> String {
        let voiced = if self.cancel_token.is_cancelled() || self.llm.has_fatal_error() {
            None
        } else {
            let openings: Vec<String> = self.arbitre_recent_openings.iter().cloned().collect();
            let prompt = prompt_builder::build_announcement_prompt(brief, &openings, &self.config.discussion_language);
            let mut params = self.arbitre.config.llm_params.clone();
            params.num_predict = constants::ANNOUNCEMENT_NUM_PREDICT;
            let request = LlmRequest::new(&self.arbitre_system, &prompt, &params, CallKind::Announcement).speaker(&self.arbitre.config.id);
            match self.chat_text(&request).await {
                Ok(raw) => {
                    let text = raw.trim().trim_matches('"').trim();
                    (!text.is_empty() && !is_model_refusal(text)).then(|| truncate_at_sentence_boundary(text, constants::ANNOUNCEMENT_MAX_CHARS))
                }
                Err(LlmError::Cancelled) => None,
                Err(e) => {
                    tracing::warn!(error = %e, "Announcement call failed — templated line used");
                    None
                }
            }
        };
        let text = voiced.unwrap_or_else(|| brief.to_string());
        self.remember_arbitre_opening(&text);
        text
    }

    /// Remember how the moderator opened a line it wrote itself (v1.20.2), so the
    /// next moderation prompt can forbid the same attack.
    fn remember_arbitre_opening(&mut self, content: &str) {
        self.arbitre_recent_openings.push_back(directive_builder::opening_of(content));
        while self.arbitre_recent_openings.len() > constants::ARBITRE_RECENT_OPENINGS {
            self.arbitre_recent_openings.pop_front();
        }
    }

    /// Moderator hint (v1.20.2): the audience member spoke and this intervention was
    /// expected to answer them — when it does not even name them, the moderator says so.
    fn audience_hint_for(&self, intervention: &str) -> Option<String> {
        let message = self.user_reply_pending.as_deref()?;
        let user = self.config.user_name.as_str();
        if intervention.to_lowercase().contains(&user.to_lowercase()) {
            return None;
        }
        Some(match self.config.discussion_language.as_str() {
            "en" => format!("{user}, from the audience, spoke just before: \"{message}\". This intervention ignores them: remind everyone in one sentence that an answer is owed to {user}."),
            "zh" => format!("现场观众{user}刚才发言：\"{message}\"。这次发言忽视了他：用一句话提醒大家应该回应{user}。"),
            _ => format!("{user}, dans le public, a pris la parole juste avant : « {message} ». Cette intervention l'ignore : rappelle en une phrase qu'on lui doit une réponse."),
        })
    }

    /// A templated line of the moderator (act announcement, scene event…), shown
    /// in the feed and remembered by the engine so the speakers see it.
    fn emit_arbitre_line(&mut self, kind: MessageKind, content: &str, channel: &Channel<ArenaEvent>) {
        let arb_id = self.arbitre.config.id.clone();
        let arb_name = self.arbitre.config.name.clone();
        let mut msg = self.create_message(&arb_id, &arb_name, SpeakerRole::Arbitre, content);
        msg.kind = kind;
        let _ = channel.send(ArenaEvent::MessageComplete { message: msg.clone() });
        self.turn_messages.push(msg.clone());
        self.messages_history.push(msg);
    }

}

#[cfg(test)]
mod tests {
    use super::is_duplicate_query;
    use crate::constants;
    use crate::engine::is_model_refusal;

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

    // ── Search dedup tests ──


    #[test]
    fn test_is_model_refusal_trilingual() {
        assert!(is_model_refusal("I'm sorry, but I can't help with that."));
        assert!(is_model_refusal("Je suis désolé, mais je ne peux pas participer à ce débat."));
        assert!(is_model_refusal("En tant qu'IA, je ne peux pas prendre position."));
        assert!(is_model_refusal("抱歉，我无法回答这个问题。"));
        assert!(is_model_refusal("Malheureusement je ne suis pas en mesure de répondre."));
        // Content that merely mentions a refusal is not a refusal
        assert!(!is_model_refusal("Contrairement à ce que dit Le Sceptique, je suis désolé de constater que les chiffres lui donnent tort : la productivité a bondi."));
        let long = "Je suis désolé, mais ".to_string() + &"x".repeat(constants::ORCH_MAX_REFUSAL_LENGTH);
        assert!(!is_model_refusal(&long), "long answers are content");
        assert!(!is_model_refusal("Les données montrent une transformation, pas un remplacement."));
    }

    #[test]
    fn test_is_duplicate_query_exact() {
        let past = vec!["Révolution industrielle".to_string(), "Intelligence artificielle".to_string()];
        assert!(is_duplicate_query("Révolution industrielle", &past));
        assert!(is_duplicate_query("révolution industrielle", &past));         // case-insensitive
        assert!(is_duplicate_query("  Révolution industrielle  ", &past));     // trim
    }

    #[test]
    fn test_is_duplicate_query_substring() {
        let past = vec!["Révolution industrielle".to_string()];
        // Long enough for substring check (min(34,25)=25 ≥ 8)
        assert!(is_duplicate_query("Révolution industrielle en France", &past));
        // "industrielle" (12 bytes) is substring of "Révolution industrielle" (25 bytes), min=12 ≥ 8
        assert!(is_duplicate_query("industrielle", &past));
    }

    #[test]
    fn test_is_duplicate_query_short_no_substring() {
        // Short query → only exact match, no substring check
        assert!(!is_duplicate_query("IA", &["IA et éducation".to_string()]));
        assert!(is_duplicate_query("IA", &["IA".to_string()])); // exact match always works
    }

    #[test]
    fn test_is_duplicate_query_different() {
        let past = vec!["Révolution industrielle".to_string()];
        assert!(!is_duplicate_query("Changement climatique", &past));
        assert!(!is_duplicate_query("Robotique", &[])); // empty history
    }

    #[test]
    fn test_is_duplicate_query_empty() {
        assert!(is_duplicate_query("", &["anything".to_string()])); // empty query → true
    }
}

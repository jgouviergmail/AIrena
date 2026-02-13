//! Centralized configuration constants for the AIrena engine.
//!
//! All tunable parameters are gathered here for easy adjustment.
//! Local LLMs (Ollama) make token economy irrelevant — prefer richer context.

// ── Memory ──────────────────────────────────────────────────────────────

/// Maximum number of recent turns kept in immediate memory before eviction.
pub const MEMORY_MAX_IMMEDIATE_TURNS: usize = 3;

/// Maximum characters per message stored in immediate memory (generic modes).
pub const MEMORY_MAX_MESSAGE_CHARS: usize = 1500;

/// Maximum characters per message stored in immediate memory (CollaborativeFiction).
/// Fiction segments are 1500-2500 chars — store them in full for narrative continuity.
pub const MEMORY_MAX_FICTION_MESSAGE_CHARS: usize = 3000;

/// Maximum characters for the contextual summary (LLM-generated arc).
pub const MEMORY_MAX_SUMMARY_CHARS: usize = 3000;

/// Maximum characters per message when formatting turns for the memory update prompt (generic modes).
pub const MEMORY_FORMAT_TURN_CHARS: usize = 1000;

// ── Prompt — truncation limits ──────────────────────────────────────────

/// Characters kept per intervention in the reaction prompt.
pub const TRUNC_REACTION_CONTENT: usize = 800;

/// Characters kept per message when rendering immediate memory in speaker prompts (generic modes).
pub const TRUNC_IMMEDIATE_MEMORY: usize = 800;

/// Characters kept per message when rendering the current turn in speaker prompts (generic modes).
pub const TRUNC_CURRENT_TURN: usize = 800;

/// Characters kept per IArbitre moderation directive in speaker prompts.
pub const TRUNC_MODERATOR_DIRECTIVE: usize = 1500;

/// Characters kept for the fiction continuation anchor (tail of last segment).
pub const TRUNC_FICTION_ANCHOR: usize = 500;

// ── Search context ──────────────────────────────────────────────────────

/// Hard limit on total web/wiki search context injected into prompts (characters).
pub const SEARCH_MAX_CONTEXT_LEN: usize = 5000;

/// Characters kept for Tavily answer summary.
pub const SEARCH_TAVILY_ANSWER: usize = 1500;

/// Characters kept for web search result titles.
pub const SEARCH_WEB_TITLE: usize = 200;

/// Characters kept for web search result content.
pub const SEARCH_WEB_CONTENT: usize = 800;

/// Characters kept for Wikipedia article titles in prompt context.
pub const SEARCH_WIKI_TITLE: usize = 200;

/// Characters kept for Wikipedia article extracts in prompt context.
pub const SEARCH_WIKI_EXTRACT: usize = 1000;

// ── Orchestrator — context limits ───────────────────────────────────────

/// Characters kept for the topic when building search decision prompts.
pub const ORCH_TOPIC_FOR_SEARCH: usize = 500;

/// Characters kept for recent exchanges when building search decision prompts.
pub const ORCH_RECENT_FOR_SEARCH: usize = 1500;

/// Characters kept for the speaker persona extract in search system prompts.
pub const ORCH_PERSONA_FOR_SEARCH: usize = 500;

/// Characters kept per moderator message in `build_recent_exchanges`.
pub const ORCH_EXCHANGE_MODERATOR: usize = 800;

/// Characters kept per generic message in `build_recent_exchanges`.
pub const ORCH_EXCHANGE_GENERIC: usize = 600;

/// Characters kept per message in emotion assessment context.
pub const ORCH_EMOTION_CONTEXT: usize = 600;

/// Characters kept per message in Socratic question context.
pub const ORCH_SOCRATIC_CONTEXT: usize = 600;

/// Number of recent messages to include in Socratic question context.
pub const ORCH_RECENT_MESSAGES_TAKE: usize = 10;

/// Maximum response length to be considered a model safety refusal (longer = real response).
pub const ORCH_MAX_REFUSAL_LENGTH: usize = 300;

/// Maximum emotion history snapshots to keep per participant.
pub const ORCH_MAX_EMOTION_HISTORY: usize = 30;

// ── Tavily ──────────────────────────────────────────────────────────────

/// Tavily API free tier monthly credit limit.
pub const TAVILY_FREE_MONTHLY_QUOTA: u32 = 1000;

// ── Wikipedia API ───────────────────────────────────────────────────────

/// Max results per Wikipedia search query (3 to allow disambiguation filtering).
pub const WIKI_RESULTS_LIMIT: u8 = 3;

/// Max characters for the plain-text extract returned by Wikipedia API (intro only).
pub const WIKI_EXTRACT_CHARS: u16 = 1500;

/// Wikipedia maxlag parameter (seconds) — request is retried server-side if lag exceeds this.
pub const WIKI_MAX_LAG_SECS: u8 = 5;

/// HTTP timeout for Wikipedia API calls (seconds).
pub const WIKI_TIMEOUT_SECS: u64 = 15;

// ── Ollama client ───────────────────────────────────────────────────────

/// Default HTTP request timeout for Ollama API calls (seconds).
pub const OLLAMA_HTTP_TIMEOUT_SECS: u64 = 120;

/// Maximum retry attempts for streaming calls (0..=N → N+1 total attempts).
pub const OLLAMA_MAX_RETRIES: u32 = 2;

/// Extended timeout for model preloading (seconds).
pub const OLLAMA_PRELOAD_TIMEOUT_SECS: u64 = 300;

/// Quick connection check timeout (seconds).
pub const OLLAMA_CHECK_TIMEOUT_SECS: u64 = 5;

// ── Emotion engine ──────────────────────────────────────────────────────

/// Emotional axis value at or above which a "high" threshold crossing is detected.
pub const EMOTION_HIGH_THRESHOLD: u8 = 85;

/// Emotional axis value at or below which a "low" threshold crossing is detected.
pub const EMOTION_LOW_THRESHOLD: u8 = 15;

/// Minimum dislikes to count as "contradicted".
pub const EMOTION_CONTRADICTION_THRESHOLD: u32 = 2;

/// Minimum likes to count as "supported".
pub const EMOTION_SUPPORT_THRESHOLD: u32 = 2;

// Likes deltas: per-like factor and cap
pub const EMOTION_LIKE_CONF_FACTOR: u16 = 5;
pub const EMOTION_LIKE_CONF_CAP: u16 = 15;
pub const EMOTION_LIKE_ENG_FACTOR: u16 = 3;
pub const EMOTION_LIKE_ENG_CAP: u16 = 10;

// Dislikes deltas: per-dislike factor and cap
pub const EMOTION_DISLIKE_FRUST_FACTOR: u16 = 5;
pub const EMOTION_DISLIKE_FRUST_CAP: u16 = 15;
pub const EMOTION_DISLIKE_CONF_FACTOR: u16 = 3;
pub const EMOTION_DISLIKE_CONF_CAP: u16 = 10;

// Contradiction bonus deltas
pub const EMOTION_CONTRADICTION_FRUST: u8 = 8;
pub const EMOTION_CONTRADICTION_ENG: u8 = 5;

// Support bonus deltas
pub const EMOTION_SUPPORT_ENTHOUSIASME: u8 = 8;
pub const EMOTION_SUPPORT_CONF: u8 = 5;

// Ban penalty deltas
pub const EMOTION_BAN_FRUST: u8 = 15;
pub const EMOTION_BAN_ENG: u8 = 10;

// Stagnation penalty deltas
pub const EMOTION_STAGNATION_ENG: u8 = 5;
pub const EMOTION_STAGNATION_CURIOSITE: u8 = 5;

// Natural decay — extremes return toward target at the given rate
pub const EMOTION_DECAY_FRUSTRATION_TARGET: u8 = 50;
pub const EMOTION_DECAY_FRUSTRATION_RATE: u8 = 2;
pub const EMOTION_DECAY_ENTHUSIASM_TARGET: u8 = 50;
pub const EMOTION_DECAY_ENTHUSIASM_RATE: u8 = 1;

// Emotional contagion — weak pull toward group average
pub const EMOTION_CONTAGION_RATE: f32 = 0.05;
pub const EMOTION_CONTAGION_MAX_DELTA: f32 = 3.0;

// ── Personality description thresholds ───────────────────────────────────

/// Emotion axis value at or above which a personality phrase is generated (e.g. "confident").
pub const PERSONALITY_HIGH: u8 = 70;

/// Emotion axis value at or below which a personality phrase is generated (e.g. "hesitant").
/// Frustration uses a tighter threshold — see `PERSONALITY_LOW_FRUSTRATION`.
pub const PERSONALITY_LOW: u8 = 30;

/// Low threshold specifically for frustration (20 vs 30 for other axes).
/// Frustration defaults at 10 — a low bar avoids triggering on near-default values.
pub const PERSONALITY_LOW_FRUSTRATION: u8 = 20;

// ── Moderation ──────────────────────────────────────────────────────────

/// Minimum ban duration (turns) that IArbitre can issue.
pub const MODERATION_BAN_MIN_TURNS: u32 = 1;

/// Maximum ban duration (turns) that IArbitre can issue.
pub const MODERATION_BAN_MAX_TURNS: u32 = 3;

// ── Search rendering ────────────────────────────────────────────────────

/// Maximum number of web search results rendered into the prompt context.
pub const SEARCH_WEB_RENDER_LIMIT: usize = 5;

// ── Think mode heuristic ────────────────────────────────────────────────

/// Base probability of enabling think mode (20%).
pub const THINK_BASE_PROBABILITY: f64 = 0.20;

/// Additional probability when frustration exceeds threshold.
pub const THINK_FRUSTRATION_BOOST: f64 = 0.15;
/// Frustration axis value above which the boost applies.
pub const THINK_FRUSTRATION_THRESHOLD: u8 = 70;

/// Additional probability when engagement exceeds threshold.
pub const THINK_ENGAGEMENT_BOOST: f64 = 0.10;
/// Engagement axis value above which the boost applies.
pub const THINK_ENGAGEMENT_THRESHOLD: u8 = 70;

/// Additional probability near the end of the discussion.
pub const THINK_NEAR_END_BOOST: f64 = 0.15;
/// How many turns before the end to start applying the near-end boost.
pub const THINK_NEAR_END_TURNS: u32 = 2;

/// Additional probability when the speaker was contradicted (>= 2 dislikes).
pub const THINK_CONTRADICTED_BOOST: f64 = 0.10;

/// Maximum think mode probability (cap to keep it non-systematic).
pub const THINK_MAX_PROBABILITY: f64 = 0.60;

// ── Temperature adjustments ─────────────────────────────────────────────

/// Temperature boost when a speaker has difficulty generating content.
pub const TEMP_DIFFICULTY_BOOST: f32 = 0.3;

/// Temperature boost after a model safety refusal (to encourage more creative output).
pub const TEMP_REFUSAL_BOOST: f32 = 0.2;

/// Maximum temperature after boosts.
pub const TEMP_MAX: f32 = 2.0;

/// Low temperature used for voting, ordering, and other structured JSON responses.
pub const TEMP_VOTING: f32 = 0.3;

// ── RAG ──────────────────────────────────────────────────────────────

/// Maximum file size for RAG import (bytes). 10 MB.
pub const RAG_MAX_FILE_SIZE_BYTES: usize = 10 * 1024 * 1024;

/// Target chunk size (characters). ~512 tokens ≈ 2000 chars.
/// Benchmark-validated optimal for general-purpose RAG (2025-2026).
pub const RAG_CHUNK_TARGET_CHARS: usize = 2000;

/// Overlap between consecutive chunks (characters). ~10% of target.
pub const RAG_CHUNK_OVERLAP_CHARS: usize = 200;

/// Candidates from vector similarity search (Stage 1a).
pub const RAG_RETRIEVAL_TOP_K: usize = 30;

/// Candidates after RRF fusion (Stage 1c).
pub const RAG_RRF_TOP_K: usize = 10;

/// RRF constant k (standard value from original paper).
pub const RAG_RRF_K: f32 = 60.0;

/// Maximum chunks the LLM selects (Stage 2).
pub const RAG_LLM_SELECT_MAX: usize = 5;

/// BM25 parameter k1 (term frequency saturation).
pub const RAG_BM25_K1: f32 = 1.2;

/// BM25 parameter b (document length normalization).
pub const RAG_BM25_B: f32 = 0.75;

/// Batch size for embedding API calls (texts per request).
pub const RAG_EMBED_BATCH_SIZE: usize = 32;

/// HTTP timeout for embedding API calls (seconds).
/// First call may load the model — needs extra time.
pub const RAG_EMBED_TIMEOUT_SECS: u64 = 60;

/// Maximum characters for RAG context injected into prompts.
pub const RAG_MAX_CONTEXT_LEN: usize = 5000;

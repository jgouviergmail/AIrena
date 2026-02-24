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

/// num_predict for respond-or-pass check (short JSON boolean).
pub const ORCH_NUM_PREDICT_RESPOND_PASS: i32 = 50;

/// num_predict for Socratic question generation (short question).
pub const ORCH_NUM_PREDICT_SOCRATIC: i32 = 200;

/// Minimum num_predict for document update (must fit full document + extension).
pub const ORCH_DOC_MIN_NUM_PREDICT: i32 = 4096;

/// Token padding added to estimated document size for document update.
pub const ORCH_DOC_TOKEN_PADDING: i32 = 1024;

/// Approximate chars-per-token ratio for multilingual text estimation.
pub const CHARS_PER_TOKEN_ESTIMATE: usize = 3;

/// Maximum emotion history snapshots to keep per participant.
pub const ORCH_MAX_EMOTION_HISTORY: usize = 30;

// ── Tavily ──────────────────────────────────────────────────────────────

/// HTTP timeout for Tavily API calls (seconds).
pub const TAVILY_HTTP_TIMEOUT_SECS: u64 = 15;

/// Maximum search results returned by Tavily per query.
pub const TAVILY_MAX_RESULTS: u8 = 5;

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

/// Base for exponential backoff on retry (seconds): sleep = base^attempt.
pub const OLLAMA_RETRY_BACKOFF_BASE_SECS: u64 = 2;

/// Delay (ms) after unloading models to let GPU VRAM fully release.
pub const OLLAMA_VRAM_SETTLE_MS: u64 = 500;

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

// ── Search deduplication ──────────────────────────────────────────────

/// Minimum character length (byte length) for substring-based dedup matching.
/// Below this threshold, only exact (case-insensitive) match is checked to avoid
/// false positives (e.g. "IA" blocking "IA et éducation").
pub const SEARCH_DEDUP_MIN_SUBSTRING_LEN: usize = 8;

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

/// Multiplier applied to num_predict for all requests from thinking models.
/// `num_predict` caps the total generated tokens (thinking + content). Without the
/// multiplier, reasoning consumes the entire budget and content is empty.
/// Applied transparently in `DiscussionEngine::build_discussion_request()`.
pub const THINK_NUM_PREDICT_MULTIPLIER: i32 = 3;

// ── Temperature adjustments ─────────────────────────────────────────────

/// Temperature boost when a speaker has difficulty generating content.
pub const TEMP_DIFFICULTY_BOOST: f32 = 0.3;

/// Temperature boost after a model safety refusal (to encourage more creative output).
pub const TEMP_REFUSAL_BOOST: f32 = 0.2;

/// Maximum temperature after boosts.
pub const TEMP_MAX: f32 = 2.0;

/// Low temperature used for voting, ordering, and other structured JSON responses.
pub const TEMP_VOTING: f32 = 0.3;

// ── LLM default parameters ──────────────────────────────────────────

/// Default temperature for LLM generation.
pub const LLM_DEFAULT_TEMPERATURE: f32 = 0.8;

/// Default top_p (nucleus sampling) for LLM generation.
pub const LLM_DEFAULT_TOP_P: f32 = 0.9;

/// Default top_k for LLM generation.
pub const LLM_DEFAULT_TOP_K: u32 = 40;

/// Default max tokens to generate per response.
pub const LLM_DEFAULT_NUM_PREDICT: i32 = 2048;

/// num_predict for synthesis generation (comprehensive summary of entire debate).
/// Synthesis is a single, long-form output that must cover all participants and arguments.
/// 2× default to avoid truncation on rich discussions.
pub const SYNTHESIS_NUM_PREDICT: i32 = 4096;

/// Default context window size (tokens).
pub const LLM_DEFAULT_NUM_CTX: u32 = 8192;

/// Default repeat penalty to reduce repetitive output.
pub const LLM_DEFAULT_REPEAT_PENALTY: f32 = 1.3;

/// Default Ollama server URL.
pub const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434";

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

/// Fallback top-K when LLM chunk selection fails (uses top RRF results).
pub const RAG_FALLBACK_TOP_K: usize = 3;

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

// ── Token Budget ──────────────────────────────────────────────────────

/// Conservative chars-per-token ratio for Latin languages (FR, EN, etc.).
/// Under-estimates chars to leave headroom (actual ≈ 4.0–4.5).
pub const CHARS_PER_TOKEN_LATIN: f64 = 3.8;

/// Conservative chars-per-token ratio for CJK languages (ZH, JA, KO).
/// Each CJK character ≈ 1 token.
pub const CHARS_PER_TOKEN_CJK: f64 = 1.5;

/// Fixed overhead for deterministic prompt sections (preamble, mode, language,
/// datetime, emotions description, emotion thresholds). Measured at ~2 378 chars.
pub const BUDGET_DETERMINISTIC_OVERHEAD_CHARS: usize = 2_400;

/// Minimum num_ctx below which the discussion is refused outright.
pub const BUDGET_MIN_VIABLE_NUM_CTX: usize = 2_048;

/// Upper bound for recommended num_ctx (beyond this, diminishing returns).
pub const BUDGET_MAX_RECOMMENDED_NUM_CTX: usize = 131_072;

/// Approximate characters per printed page (~500 words × ~4 chars/word).
/// Used to display document capacity as page equivalents in the UI.
pub const APPROX_CHARS_PER_PAGE: usize = 2_000;

/// VRAM safety margin (MiB) subtracted from available VRAM before recommending num_ctx.
pub const VRAM_SAFETY_MARGIN_MB: usize = 512;

/// Default KV cache dtype size in bytes (f16 = 2, q8_0 = 1, q4_0 ≈ 0.5).
pub const KV_CACHE_DTYPE_BYTES: usize = 2;

// ── Token Budget — section floors (minimum chars) ─────────────────────

/// Floor: per-message chars for current turn messages.
pub const BUDGET_FLOOR_CURRENT_TURN: usize = 400;
/// Floor: per-message chars for immediate memory (3 turns).
pub const BUDGET_FLOOR_IMMEDIATE_MEMORY: usize = 300;
/// Floor: total chars for contextual summary.
pub const BUDGET_FLOOR_CONTEXTUAL_SUMMARY: usize = 500;
/// Floor: total chars for cognitive directives.
pub const BUDGET_FLOOR_COGNITIVE_DIRECTIVES: usize = 500;
/// Floor: total chars for IArbitre directives.
pub const BUDGET_FLOOR_ARBITRE_DIRECTIVES: usize = 300;
/// Floor: total chars for full document injection (0 = all-or-nothing).
pub const BUDGET_FLOOR_FULL_DOCUMENT: usize = 0;
/// Floor: total chars for RAG context.
pub const BUDGET_FLOOR_RAG_CONTEXT: usize = 500;
/// Floor: total chars for web+wiki search results.
pub const BUDGET_FLOOR_WEB_WIKI: usize = 500;
/// Floor: total chars for positional map.
pub const BUDGET_FLOOR_POSITIONAL_MAP: usize = 50;

// ── Token Budget — section ceilings (maximum chars) ───────────────────

/// Ceiling: per-message chars for current turn messages.
pub const BUDGET_CEIL_CURRENT_TURN: usize = 2_000;
/// Ceiling: per-message chars for immediate memory (3 turns).
pub const BUDGET_CEIL_IMMEDIATE_MEMORY: usize = 1_500;
/// Ceiling: total chars for contextual summary.
pub const BUDGET_CEIL_CONTEXTUAL_SUMMARY: usize = 8_000;
/// Ceiling: total chars for cognitive directives.
pub const BUDGET_CEIL_COGNITIVE_DIRECTIVES: usize = 3_000;
/// Ceiling: total chars for IArbitre directives.
pub const BUDGET_CEIL_ARBITRE_DIRECTIVES: usize = 2_000;
/// Ceiling: total chars for RAG context.
pub const BUDGET_CEIL_RAG_CONTEXT: usize = 10_000;
/// Ceiling: total chars for web+wiki search results.
pub const BUDGET_CEIL_WEB_WIKI: usize = 8_000;
/// Ceiling: per-participant chars for positional map.
pub const BUDGET_CEIL_POSITIONAL_MAP_PER_PARTICIPANT: usize = 200;

// ── Argument Map ──────────────────────────────────────────────────────

/// Characters kept per message in argument extraction context.
pub const ARGMAP_CONTEXT_CHARS: usize = 600;

/// Minimum characters for a thesis label to be considered valid.
/// Filters out numeric indices, single words, and garbage labels.
pub const ARGMAP_MIN_THESIS_LABEL_CHARS: usize = 8;

/// Maximum bytes for a thesis label (one short complete sentence).
/// Truncation uses word boundaries to avoid mid-word cuts.
pub const ARGMAP_MAX_THESIS_LABEL: usize = 200;

/// Maximum bytes for an argument label (1-2 short complete sentences).
/// Truncation uses word boundaries to avoid mid-word cuts.
pub const ARGMAP_MAX_ARGUMENT_LABEL: usize = 400;

/// Maximum number of theses in the argument map.
pub const ARGMAP_MAX_THESES: usize = 20;

/// Maximum total number of arguments across all theses.
pub const ARGMAP_MAX_ARGUMENTS: usize = 100;

/// Minimum num_predict for argument map extraction (generous for quality JSON output).
pub const ARGMAP_NUM_PREDICT: i32 = 4096;

/// Minimum num_ctx for argument map extraction (prompt + response must both fit).
pub const ARGMAP_NUM_CTX: u32 = 16384;

/// Temperature for argument map extraction (low for structured JSON output).
pub const ARGMAP_TEMPERATURE: f32 = 0.3;

// ── License ──────────────────────────────────────────────────────────

/// License key payload version.
pub const LICENSE_VERSION: u8 = 1;

/// Clock drift tolerance for anti-manipulation check (seconds). 2 hours covers DST.
pub const LICENSE_CLOCK_TOLERANCE_SECS: i64 = 7200;

/// Discussion quota per 24 hours of license duration.
pub const LICENSE_DISCUSSIONS_PER_DAY: u32 = 50;

/// Ed25519 public key for license signature verification (hex, 32 bytes).
pub const LICENSE_ED25519_PUBLIC_KEY_HEX: &str = "758f08355ba45e51fc77559c3a16a419163a06438698b1054d18834188051fd4";

/// AES-256-GCM shared key for license encryption (hex, 32 bytes).
pub const LICENSE_AES_KEY_HEX: &str = "ab0bede65de2e957c25846a21420657eae852cd88288c4fbfa87077909cb1bbe";

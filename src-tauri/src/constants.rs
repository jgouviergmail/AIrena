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

// Stagnation penalty deltas (applied only when stagnation is actually detected)
pub const EMOTION_STAGNATION_ENG: u8 = 5;
pub const EMOTION_STAGNATION_CURIOSITE: u8 = 5;

/// Jaccard similarity between two consecutive contextual summaries at or above
/// which the discussion is considered to be going in circles.
pub const EMOTION_STAGNATION_SIMILARITY: f32 = 0.8;

/// Consecutive turns without any like/dislike after which the discussion is
/// considered stagnating.
pub const EMOTION_STAGNATION_REACTION_DROUGHT_TURNS: u32 = 2;

// Accord follows the reactions a speaker GIVES: per net reaction, and cap
pub const EMOTION_ACCORD_GIVEN_FACTOR: u8 = 4;
pub const EMOTION_ACCORD_GIVEN_CAP: u8 = 12;

/// Bound (±) on each axis of an LLM-provided emotion delta. Reactions and bans
/// are already applied by rules; the model only adds tone/content nuance.
pub const EMOTION_LLM_DELTA_CAP: i8 = 10;

// Natural decay — extremes return toward the persona's INITIAL profile at the given rate
pub const EMOTION_DECAY_FRUSTRATION_RATE: u8 = 2;
pub const EMOTION_DECAY_ENTHUSIASM_RATE: u8 = 1;

// Emotional contagion — weak pull toward group average
pub const EMOTION_CONTAGION_RATE: f32 = 0.05;
pub const EMOTION_CONTAGION_MAX_DELTA: f32 = 3.0;

/// Whether the IArbitre's profile is part of the contagion average.
/// The moderator observes more than it participates: it feels the room but
/// does not set its mood.
pub const EMOTION_CONTAGION_INCLUDE_ARBITRE: bool = false;

// ── Personality description thresholds ───────────────────────────────────

/// Emotion axis value at or above which a personality phrase is generated (e.g. "confident").
pub const PERSONALITY_HIGH: u8 = 70;

/// Emotion axis value at or below which a personality phrase is generated (e.g. "hesitant").
/// Frustration uses a tighter threshold — see `PERSONALITY_LOW_FRUSTRATION`.
pub const PERSONALITY_LOW: u8 = 30;

/// Low threshold specifically for frustration (20 vs 30 for other axes).
/// Frustration defaults at 10 — a low bar avoids triggering on near-default values.
pub const PERSONALITY_LOW_FRUSTRATION: u8 = 20;

// ── Conversational focus (anti "everyone against X") ───────────────────

/// Base weight of any recent speaker as a focus candidate.
pub const FOCUS_WEIGHT_BASE: u32 = 1;
/// Extra weight when nobody has addressed that participant yet this turn.
pub const FOCUS_WEIGHT_UNTARGETED: u32 = 3;
/// Extra weight when an ally/rival/tense relationship exists with that participant.
pub const FOCUS_WEIGHT_RELATIONSHIP: u32 = 2;
/// Weight of "advance the topic without addressing anyone".
pub const FOCUS_WEIGHT_TOPIC: u32 = 2;
/// Divisor applied to the truncation budget of non-focus messages in the
/// current-turn block (the focus message keeps the full per-message budget).
pub const FOCUS_OTHER_MESSAGE_DIVISOR: usize = 2;

// ── Speech acts ─────────────────────────────────────────────────────────

/// How many of a speaker's most recent speech acts are penalised for variety.
pub const SPEECH_ACT_RECENT_WINDOW: usize = 3;
/// Weight multiplier (percent) applied to recently used speech acts.
pub const SPEECH_ACT_RECENT_WEIGHT_PERCENT: u32 = 30;

// ── Socratic mode ───────────────────────────────────────────────────────

/// Previously asked questions injected into the next question prompt (anti-repetition).
pub const SOCRATIC_PREVIOUS_QUESTIONS: usize = 3;

// ── Emotion analysis JSON ───────────────────────────────────────────────

/// Top-level key the emotion analyst uses to flag a stalling discussion.
pub const EMOTION_STAGNATION_JSON_KEY: &str = "stagnating";

// ── IArbitre rule-based emotions ────────────────────────────────────────

/// Frustration and confidence gained by IArbitre when it issues a ban.
pub const EMOTION_ARBITRE_BAN_DELTA: u8 = 5;
/// Engagement lost by IArbitre per intervention while the discussion stagnates.
pub const EMOTION_ARBITRE_STAGNATION_ENG: u8 = 3;

// ── Model refusal detection (trilingual) ────────────────────────────────

/// A short answer starting with one of these is a safety refusal, not content.
pub const REFUSAL_PREFIXES: &[&str] = &[
    "i'm sorry", "i cannot", "i can't", "i apologize", "sorry, but", "as an ai",
    "je suis désolé", "je suis désolée", "je ne peux pas", "je ne suis pas en mesure", "en tant qu'ia", "désolé, mais", "désolée, mais",
    "抱歉", "对不起", "我不能", "我无法", "作为一个人工智能", "作为ai",
];
/// A short answer containing one of these is a safety refusal.
pub const REFUSAL_SUBSTRINGS: &[&str] = &[
    "i can't help with that", "i cannot assist", "i'm not able to", "i can't assist",
    "je ne peux pas vous aider", "je ne peux pas t'aider", "je ne suis pas en mesure de",
    "我无法帮助", "我不能协助",
];

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

/// Low temperature used for every structured JSON call (reactions, moderation,
/// memory, emotions, votes, search decisions, RAG selection, respond/pass).
pub const TEMP_JSON_OUTPUT: f32 = 0.3;

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

/// Number of speaker turns a per-speaker RAG cache entry remains valid.
/// After this many turns for the same speaker, the next RAG query runs the full pipeline.
pub const RAG_CACHE_TTL_TURNS: u32 = 3;

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

/// Maximum total number of arguments across all theses (recursive count).
pub const ARGMAP_MAX_ARGUMENTS: usize = 100;

/// Maximum nesting depth for arguments below a thesis.
/// Allows chains like: Support → Counter → Refutation → Evidence (depth 4).
/// If a target argument is at max depth, the new argument attaches flat to the thesis.
pub const ARGMAP_MAX_ARGUMENT_DEPTH: usize = 4;

/// Maximum number of existing argument lines shown in the extraction prompt.
/// Only used to cap context size — arguments at all depths are included recursively.
pub const ARGMAP_PROMPT_MAX_EXISTING_ARGUMENTS: usize = 60;

/// Maximum characters for argument labels in the extraction prompt context.
/// Shorter than ARGMAP_MAX_ARGUMENT_LABEL to keep the prompt compact.
pub const ARGMAP_PROMPT_LABEL_CHARS: usize = 100;

/// Minimum num_predict for argument map extraction (generous for quality JSON output).
pub const ARGMAP_NUM_PREDICT: i32 = 4096;

/// Minimum num_ctx for argument map extraction (prompt + response must both fit).
/// Only the Ollama adapter consumes `num_ctx`; cloud providers ignore it.
pub const ARGMAP_NUM_CTX: u32 = 16384;

/// Minimum non-moderator messages in a turn before extraction is worth a call.
pub const ARGMAP_MIN_TURN_MESSAGES: usize = 2;

/// Jaccard similarity (normalised tokens) at or above which two thesis labels
/// are the same thesis (reformulation by the model).
pub const ARGMAP_THESIS_SIMILARITY_THRESHOLD: f32 = 0.7;
/// Looser Jaccard threshold used to resolve a *reference* to an existing thesis
/// (`for_thesis` / `against_thesis`): the model quotes from the list it was
/// shown, so the best candidate above this score is the intended one.
pub const ARGMAP_REFERENCE_SIMILARITY_THRESHOLD: f32 = 0.5;
/// Share of the shorter label's tokens found in the longer one for a
/// containment match (a reformulation with extra words).
pub const ARGMAP_CONTAINMENT_THRESHOLD: f32 = 0.85;
/// Containment only applies to labels with at least this many tokens —
/// a short label contained in a longer one may state the opposite.
pub const ARGMAP_CONTAINMENT_MIN_TOKENS: usize = 4;

/// Marker prefixed to nodes added by the latest extraction (with its trailing space).
pub const ARGMAP_NEW_MARKER_PREFIX: &str = "✨ ";

/// Function words ignored when comparing labels (FR + EN; CJK is per character).
pub const ARGMAP_STOP_WORDS: &[&str] = &[
    // FR
    "le", "la", "les", "de", "des", "du", "un", "une", "et", "ou", "au", "aux", "en", "est", "sont",
    "que", "qui", "ne", "pas", "plus", "pour", "par", "sur", "dans", "ce", "cette", "ces", "son", "sa",
    "ses", "il", "elle", "ils", "elles", "on", "nous", "vous", "leur", "leurs", "qu", "se", "sa", "mais",
    "donc", "car", "avec", "sans", "tout", "tous", "toute", "toutes", "peut", "doit", "etre", "avoir",
    // EN
    "the", "an", "of", "to", "in", "on", "and", "or", "is", "are", "be", "that", "this", "these",
    "those", "it", "its", "for", "with", "as", "by", "not", "than", "but", "at", "from", "can", "will",
    "should", "must", "have", "has", "do", "does",
];


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

// ── LLM providers (generic) ──────────────────────────────────────────

/// Consecutive reasoning-mode failures (empty content / refusal) before the
/// engine disables reasoning for the rest of the discussion.
pub const REASONING_MAX_FAILURES: u32 = 2;

/// Share of the monthly cloud budget at which a warning is emitted (0.8 = 80%).
pub const LLM_BUDGET_WARN_RATIO: f64 = 0.8;

// ── DeepSeek — API ───────────────────────────────────────────────────

/// OpenAI-compatible base URL.
pub const DEEPSEEK_BASE_URL: &str = "https://api.deepseek.com";
pub const DEEPSEEK_CHAT_PATH: &str = "/chat/completions";
pub const DEEPSEEK_MODELS_PATH: &str = "/models";
pub const DEEPSEEK_BALANCE_PATH: &str = "/user/balance";

/// Model used when none is configured.
pub const DEEPSEEK_DEFAULT_MODEL: &str = "deepseek-flash";

/// Fallback model list when `GET /models` is unreachable (doc 2026-09-10).
pub const DEEPSEEK_KNOWN_MODELS: &[&str] = &["deepseek-flash", "deepseek-v4-pro"];

/// TCP/TLS connection timeout (seconds).
pub const DEEPSEEK_CONNECT_TIMEOUT_SECS: u64 = 15;

/// Maximum silence between two SSE chunks (seconds). Keep-alive comments reset it.
/// Reasoning at `max` effort can legitimately take minutes before the first token.
pub const DEEPSEEK_IDLE_TIMEOUT_SECS: u64 = 180;

/// Retry attempts on transient errors (429/5xx/network) when no token was emitted yet.
pub const DEEPSEEK_MAX_RETRIES: u32 = 3;

/// Exponential backoff base (ms): sleep = base × 2^attempt + jitter.
pub const DEEPSEEK_RETRY_BASE_MS: u64 = 1_000;

/// Random jitter added to each backoff (ms).
pub const DEEPSEEK_RETRY_JITTER_MS: u64 = 250;

/// Hard cap on `max_tokens` accepted by the API.
pub const DEEPSEEK_MAX_OUTPUT_TOKENS: i32 = 393_216;

/// Extra output tokens granted on top of `num_predict` when reasoning is active.
/// Assumption (H-DS-1): `max_tokens` bounds reasoning + content together — the API
/// defaults (8K without thinking vs 64K with) strongly suggest it.
pub const DEEPSEEK_REASONING_ALLOWANCE_LOW: i32 = 4_096;
pub const DEEPSEEK_REASONING_ALLOWANCE_HIGH: i32 = 12_288;
pub const DEEPSEEK_REASONING_ALLOWANCE_MAX: i32 = 32_768;

/// `top_p` lower bound enforced by the API while thinking is enabled.
pub const DEEPSEEK_TOP_P_MIN_THINKING: f32 = 0.95;

/// Chars-per-token ratios from DeepSeek's tokenizer guide (≈0.3 token/char EN, ≈0.6 ZH).
pub const DEEPSEEK_CHARS_PER_TOKEN_LATIN: f64 = 3.3;
pub const DEEPSEEK_CHARS_PER_TOKEN_CJK: f64 = 1.7;

/// Context budget (tokens) fed to the waterfall allocator in DeepSeek mode.
/// The model accepts 1M, but prompt size is a direct cost driver.
pub const DEEPSEEK_DEFAULT_CONTEXT_BUDGET: u32 = 32_768;
pub const DEEPSEEK_MIN_CONTEXT_BUDGET: u32 = 4_096;
pub const DEEPSEEK_MAX_CONTEXT_BUDGET: u32 = 262_144;

/// Default UI bound for `num_predict` in DeepSeek mode (Ollama keeps 4096).
pub const DEEPSEEK_MAX_NUM_PREDICT_UI: i32 = 16_384;

// ── DeepSeek — pricing (USD per 1M tokens, peak hours) ───────────────

/// Date of the official price list these constants were copied from.
pub const DEEPSEEK_PRICING_DATE: &str = "2026-09-10";

/// Off-peak prices are this fraction of peak prices.
pub const DEEPSEEK_OFFPEAK_FACTOR: f64 = 0.5;

/// Peak windows in UTC hours, `[start, end)`, Monday to Friday.
pub const DEEPSEEK_PEAK_WINDOWS_UTC: &[(u32, u32)] = &[(1, 4), (6, 10)];

pub const DEEPSEEK_PRICE_FLASH_INPUT_HIT: f64 = 0.006;
pub const DEEPSEEK_PRICE_FLASH_INPUT_MISS: f64 = 0.30;
pub const DEEPSEEK_PRICE_FLASH_OUTPUT: f64 = 1.20;

pub const DEEPSEEK_PRICE_V4PRO_INPUT_HIT: f64 = 0.044;
pub const DEEPSEEK_PRICE_V4PRO_INPUT_MISS: f64 = 1.32;
pub const DEEPSEEK_PRICE_V4PRO_OUTPUT: f64 = 3.96;

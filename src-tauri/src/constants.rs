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

// Typed reactions (v1.17) — effects added on top of the like/dislike rules
/// Extra confidence per "insightful" received.
pub const EMOTION_INSIGHTFUL_CONF_BONUS: u8 = 3;
/// Curiosity per "question" received, and cap per intervention.
pub const EMOTION_QUESTION_CURIOSITY: u16 = 4;
pub const EMOTION_QUESTION_CURIOSITY_CAP: u16 = 12;
/// Frustration per "off-topic" received (softer than a dislike, whose factor is 5).
pub const EMOTION_OFFTOPIC_FRUST_FACTOR: u16 = 3;
/// Enthusiasm per "laugh" received or given, and cap per intervention.
pub const EMOTION_LAUGH_ENTHUSIASM: u16 = 4;
pub const EMOTION_LAUGH_ENTHUSIASM_CAP: u16 = 12;

// Accord follows the reactions a speaker GIVES: per net reaction, and cap
pub const EMOTION_ACCORD_GIVEN_FACTOR: u8 = 2;
pub const EMOTION_ACCORD_GIVEN_CAP: u8 = 6;

/// Bound (±) on each axis of an LLM-provided emotion delta. Reactions and bans
/// are already applied by rules; the model only adds tone/content nuance.
pub const EMOTION_LLM_DELTA_CAP: i8 = 10;

// Natural decay — extremes return toward the persona's INITIAL profile at the given rate
pub const EMOTION_DECAY_FRUSTRATION_RATE: u8 = 2;
pub const EMOTION_DECAY_ENTHUSIASM_RATE: u8 = 2;

// ── Émotions : réalisme et variabilité (v1.20.4) ────────────────────────

/// Above this value an increase meets resistance (below `100 - it`, a decrease does):
/// the comfort band where a rule delta applies in full.
pub const EMOTION_COMFORT_HIGH: u8 = 65;
/// Share of a delta that still applies at the very extreme (0 or 100): never zero,
/// so a saturated axis keeps moving — slowly. The braking is quadratic in the
/// distance past the comfort band (v1.20.5: 58 % left at 75, 31 % at 85, 17 % at
/// 95), which caps a unanimous chorus around 85 instead of 90.
pub const EMOTION_EXTREME_RESISTANCE: f32 = 0.15;
/// Net change one reception round (every reaction to one intervention) may make
/// on a single axis, before the resistance.
pub const EMOTION_ROUND_AXIS_CAP: u8 = 12;
/// Homeostasis: share of the distance to the persona's baseline recovered at every
/// intervention on every axis (percent), one point at least; for frustration and
/// enthousiasme the explicit `EMOTION_DECAY_*_RATE` is the floor instead (v1.20.5),
/// so a storm of dislikes plateaus around 80 instead of creeping to 100.
pub const EMOTION_HOMEOSTASIS_PERCENT: u8 = 12;
/// A move of at least this many points away from the persona's baseline on one
/// axis is a notable shift: the directive names it ("ébranlé", "convaincu"…)
/// and a shaken speaker thinks harder.
pub const EMOTION_NOTABLE_SHIFT: u8 = 15;
/// Hysteresis of the notable zone (v1.20.5): entered at `EMOTION_NOTABLE_SHIFT`,
/// left only below `EMOTION_NOTABLE_SHIFT - EMOTION_SHIFT_REARM`, so a speaker
/// hovering around the boundary (homeostasis pulls back, a reaction pushes
/// again) is announced once, not at every intervention.
pub const EMOTION_SHIFT_REARM: u8 = 5;

// Emotional contagion — weak pull toward group average
pub const EMOTION_CONTAGION_RATE: f32 = 0.05;
pub const EMOTION_CONTAGION_MAX_DELTA: f32 = 3.0;

/// Whether the IArbitre's profile is part of the contagion average.
/// The moderator observes more than it participates: it feels the room but
/// does not set its mood.
pub const EMOTION_CONTAGION_INCLUDE_ARBITRE: bool = false;

// ── Émotions incarnées (v1.17) ───────────────────────────────────────────

/// OCEAN value at or above which a trait is "extreme high" (directives, gains).
pub const OCEAN_EXTREME_HIGH: u8 = 8;
/// OCEAN value at or below which a trait is "extreme low".
pub const OCEAN_EXTREME_LOW: u8 = 3;
/// Bounds of the persona gains derived from OCEAN (1.0 = the v1.16 rule deltas).
pub const OCEAN_GAIN_MIN: f32 = 0.6;
pub const OCEAN_GAIN_MAX: f32 = 1.4;

/// Sampling modulation (emotion-driven only, never on JSON calls):
/// temperature moves by ±`EMOTION_TEMP_SPAN` with enthusiasm, within [MIN, MAX].
pub const EMOTION_TEMP_SPAN: f32 = 0.15;
pub const EMOTION_TEMP_MIN: f32 = 0.3;
pub const EMOTION_TEMP_MAX: f32 = 1.2;
/// `num_predict` scales between these factors with engagement (0 → MIN, 100 → MAX).
pub const EMOTION_LEN_MIN: f32 = 0.8;
pub const EMOTION_LEN_MAX: f32 = 1.2;

/// Stage directions (theatre lines about a participant's state, no LLM) — whole
/// sentences (v1.20.4), a persona's own line rarely exceeds this.
pub const STAGE_DIRECTION_MAX_CHARS: usize = 240;
pub const STAGE_DIRECTIONS_PER_SPEAKER_PER_TURN: usize = 1;

/// Weighted relationship scores: decay applied at the end of every turn, and
/// the thresholds from which a pair is classified (v1.20.5: on the **sum** of
/// both directions' net warmth — sincere reactions leave half of the reactions
/// neutral, so a single direction rarely reaches 2 on its own).
pub const RELATIONSHIP_DECAY_PER_TURN: f32 = 0.85;
/// Allies: combined net warmth at or above this, each direction warm (`RELATIONSHIP_MUTUAL_MIN`).
pub const RELATIONSHIP_ALLY_SCORE: f32 = 2.5;
/// Rivals: combined net warmth at or below minus this, each direction cold.
pub const RELATIONSHIP_RIVAL_SCORE: f32 = 2.5;
/// Tense: opposite signs with a spread at or above this, or one direction cold
/// by this much on its own (a persistent critic makes a pair tense whatever the
/// other does).
pub const RELATIONSHIP_TENSE_SCORE: f32 = 2.0;
/// Net warmth a direction must show (in absolute value) to count as warm / cold:
/// about one approval still in memory.
pub const RELATIONSHIP_MUTUAL_MIN: f32 = 0.8;
/// Net-score move below which a relationship trend is "stable".
pub const RELATIONSHIP_TREND_EPSILON: f32 = 0.5;

/// Room mood thresholds on the average profile of the active gladiateurs.
pub const ROOM_MOOD_TENSE_FRUSTRATION: u8 = 65;
pub const ROOM_MOOD_FLAT_ENGAGEMENT: u8 = 35;
pub const ROOM_MOOD_LIVELY_ENTHUSIASM: u8 = 65;

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

// ── Reactions (v1.17) ───────────────────────────────────────────────────

/// Shortest excerpt a reaction may quote (shorter strings match almost anything).
pub const REACTION_QUOTE_MIN_CHARS: usize = 8;
/// Longest excerpt kept from a reaction's quote (word-boundary truncation).
pub const REACTION_QUOTE_MAX_CHARS: usize = 120;
/// "insightful" is a scarce token (v1.20.5): a reactor may distinguish one strong
/// point per this many reactions given — the prompt says when the credit is spent,
/// and a 💡 given anyway is recorded as a like. Real models ignored "rare" and
/// "at most one out of five" (deepseek-flash: 36-43 % of 💡 in a debate).
pub const INSIGHTFUL_CREDIT_WINDOW: usize = 5;
/// Audience reactions accepted on one message (the rest is ignored).
pub const AUDIENCE_REACTIONS_PER_MESSAGE_MAX: u32 = 3;
/// Focus weight bonus for a participant the audience just reacted to.
pub const FOCUS_WEIGHT_AUDIENCE: u32 = 2;
/// Reactions shown under a message of the current turn in the speaker prompt.
pub const PROMPT_REACTIONS_PER_MESSAGE_MAX: usize = 3;
/// Characters of a reaction justification kept in the speaker prompt.
pub const PROMPT_REACTION_JUSTIFICATION_CHARS: usize = 120;
/// Reaction hints (quotes flagged insightful / questioned) added to the argument extraction context.
pub const ARGMAP_REACTION_HINTS_MAX: usize = 6;
/// Extraversion (OCEAN E) at or below which a persona reacts rarely.
pub const REACTION_PROPENSITY_LOW_E: u8 = 4;
/// Extraversion at or above which a persona reacts often.
pub const REACTION_PROPENSITY_HIGH_E: u8 = 7;
/// Agreeableness (OCEAN A) at or above which a persona is lenient.
pub const REACTION_PROPENSITY_HIGH_A: u8 = 8;
/// Agreeableness at or below which a persona is demanding.
pub const REACTION_PROPENSITY_LOW_A: u8 = 3;
/// Reaction colours allowed in collaborative fiction (a relay has no disagreement).
pub const REACTION_TYPES_FICTION: [crate::models::message::ReactionType; 3] = [
    crate::models::message::ReactionType::Like,
    crate::models::message::ReactionType::Insightful,
    crate::models::message::ReactionType::Laugh,
];

// ── Intention and open loops (v1.17) ────────────────────────────────────

/// Longest "[Ton intention]" block injected into the intervention prompt
/// (counted in the deterministic overhead).
pub const INTENTION_BLOCK_MAX_CHARS: usize = 300;
/// Longest angle kept from an intention (chars).
pub const INTENTION_ANGLE_MAX_CHARS: usize = 160;
/// Longest concession / question kept from an intention (chars).
pub const INTENTION_FIELD_MAX_CHARS: usize = 120;
/// Chars kept per intention field for the display (backstage, report); the prompt bounds above still apply.
pub const INTENTION_DISPLAY_MAX_CHARS: usize = 600;
/// Words that mean "no participant in particular" for an intention target.
pub const INTENTION_TOPIC_WORDS: [&str; 6] = ["sujet", "topic", "主题", "null", "none", "aucun"];
/// Own interventions during which an open loop stays in a speaker's prompt.
pub const OPEN_LOOPS_TTL_TURNS: u8 = 2;
/// Open loops kept per speaker (FIFO: the oldest is dropped first).
pub const OPEN_LOOPS_MAX_PER_SPEAKER: usize = 3;
/// Longest text kept for one open loop (chars).
pub const OPEN_LOOP_TEXT_MAX_CHARS: usize = 200;

// ── Agendas cachés et casting (v1.19) ───────────────────────────────────

/// Longest agenda content (the three fields together) kept per participant.
pub const AGENDA_MAX_CHARS: usize = 300;
/// Longest text kept per agenda field (objective, red line, victory).
pub const AGENDA_FIELD_MAX_CHARS: usize = AGENDA_MAX_CHARS / 3;
/// Labels and instruction around the fields in the "[Ton agenda secret]" system
/// block: the block never exceeds `AGENDA_MAX_CHARS + AGENDA_BLOCK_OVERHEAD_CHARS`,
/// which the token budget reserves when the feature is on.
pub const AGENDA_BLOCK_OVERHEAD_CHARS: usize = 260;
/// Output budget of one agenda call (tokens) — three short sentences of JSON.
pub const AGENDA_NUM_PREDICT: i32 = 256;
/// Longest catalogue line per profile in the casting prompt (chars).
pub const CASTING_PERSONALITY_MAX_CHARS: usize = 120;
/// Output budget of the casting call (tokens).
pub const CASTING_NUM_PREDICT: i32 = 512;
/// Gladiateurs a casting may suggest, at most.
pub const CASTING_MAX_GLADIATEURS: u32 = 8;
/// Upper bound of the catalogue text handed to the casting call (chars), before
/// the context-derived bound (`num_ctx` minus the output and the instructions).
pub const CASTING_CATALOGUE_MAX_CHARS: usize = 16_000;
/// Prompt chars of the casting instructions (kept out of the catalogue bound).
pub const CASTING_INSTRUCTIONS_CHARS: usize = 1_200;

// ── Modes structurés : rôles, verdicts, dépêches (v1.19) ─────────────────

/// Longest "[Ton rôle]" block appended to a persona (roles and hats).
pub const ROLE_BLOCK_MAX_CHARS: usize = 600;
/// Output budget of a verdict / agreement call (tokens): a choice and a reason.
pub const VERDICT_NUM_PREDICT: i32 = 256;
/// Longest reason kept from a verdict or an agreement answer.
pub const VERDICT_REASON_MAX_CHARS: usize = 300;
/// Crisis dispatches generated at the start when the discussion has no turn limit.
pub const CRISIS_DISPATCH_DEFAULT_COUNT: u32 = 5;
/// Crisis dispatches generated at most (one per turn).
pub const CRISIS_DISPATCH_MAX_COUNT: u32 = 12;
/// Longest dispatch kept (chars).
pub const CRISIS_DISPATCH_MAX_CHARS: usize = 280;
/// Output budget of the dispatches call (tokens).
pub const CRISIS_DISPATCHES_NUM_PREDICT: i32 = 1_536;

// ── Mémoire longue des personas (v1.20) ─────────────────────────────────

/// Past recaps injected at most per speaker (the closest to the topic, BM25).
pub const PERSONA_MEMORY_MAX_RECAPS: usize = 3;
/// Query tokens shorter than this (articles, prepositions) are ignored by the recall ranking.
pub const PERSONA_MEMORY_MIN_TOKEN_CHARS: usize = 3;
/// Longest "[Souvenirs de discussions passées]" block (chars), reserved in the budget.
pub const PERSONA_MEMORY_MAX_CHARS: usize = 900;
/// Output budget of one recap call (tokens).
pub const RECAP_NUM_PREDICT: i32 = 512;
/// Items kept per recap list (positions, best lines, allies, rivals).
pub const RECAP_LIST_MAX_ITEMS: usize = 3;
/// Longest item or lesson kept from a recap (chars).
pub const RECAP_ITEM_MAX_CHARS: usize = 160;
/// Own interventions (most recent first) handed to the recap prompt.
pub const RECAP_OWN_MESSAGES_MAX: usize = 4;
/// Chars of each own intervention kept in the recap prompt.
pub const RECAP_OWN_MESSAGE_MAX_CHARS: usize = 600;

// ── Exploitation : journal, mises à jour (v1.20) ────────────────────────

/// Directory of the rotating log files, next to the executable.
pub const LOG_DIR_NAME: &str = "logs";
/// Prefix of the daily log files (`airena.log.YYYY-MM-DD`).
pub const LOG_FILE_PREFIX: &str = "airena.log";
/// Bytes of the backend log exported by "export the journal" (its tail).
pub const LOG_EXPORT_MAX_BYTES: usize = 512 * 1024;
/// Where new releases are published (manual update check until the updater is activated).
pub const RELEASES_URL: &str = "https://github.com/jgouviergmail/AIrena/releases";

// ── Historique enrichi et modèles de discussion (v1.20) ─────────────────

/// Tags a discussion may carry, at most (the UI mirrors it as `TAG_MAX_LENGTH` / chips).
pub const HISTORY_TAGS_MAX: usize = 12;
/// Characters of one tag (lower-cased, trimmed) — the UI input mirrors it.
pub const HISTORY_TAG_MAX_CHARS: usize = 24;
/// Characters of a template name.
pub const TEMPLATE_NAME_MAX_CHARS: usize = 80;
/// Bytes of a template configuration (a full cast with prompts stays far below).
pub const TEMPLATE_CONFIG_MAX_BYTES: usize = 64 * 1024;

// ── Audio (v1.18) ───────────────────────────────────────────────────────

/// Default volumes (0.0–1.0) of the voice and of the procedural sounds.
pub const AUDIO_DEFAULT_TTS_VOLUME: f32 = 1.0;
pub const AUDIO_DEFAULT_SOUND_VOLUME: f32 = 0.5;

// ── Dramaturgie (v1.18) ─────────────────────────────────────────────────

/// Chance of a scene event on an eligible turn, and its boost while the discussion stagnates.
pub const SCENE_EVENT_BASE_PROBABILITY: f64 = 0.15;
pub const SCENE_EVENT_STAGNATION_BOOST: f64 = 0.35;
/// Turns between two scene events (never two turns in a row).
pub const SCENE_EVENT_MIN_GAP_TURNS: u32 = 2;
/// Longest scene instruction injected into a speaker's prompt (deterministic overhead).
pub const SCENE_EVENT_INSTRUCTION_MAX_CHARS: usize = 300;
/// Active speakers needed for a duel or a hot seat.
pub const SCENE_EVENT_MIN_ACTIVE_FOR_DUEL: usize = 3;
/// Longest surprise fact quoted by the moderator (bytes, whole sentences).
pub const SURPRISE_FACT_MAX_CHARS: usize = 500;
/// Chance, per turn, that two allies relay each other (debate-like modes, ≥ 3 active).
pub const COALITION_PROBABILITY: f64 = 0.2;
pub const COALITION_MIN_ACTIVE: usize = 3;

// ── Sources (v1.17) ─────────────────────────────────────────────────────

/// Characters of a source excerpt kept for display and persistence.
pub const SOURCE_SNIPPET_CHARS: usize = 200;
/// Longest "[Sources utilisées]" block handed to the synthesis prompt.
pub const SYNTHESIS_SOURCES_MAX_CHARS: usize = 1_500;
/// Most recent sources listed in the synthesis prompt (one line each).
pub const SYNTHESIS_SOURCES_MAX_ENTRIES: usize = 20;

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
/// Boost when the speaker's confiance dropped by `EMOTION_NOTABLE_SHIFT` or more
/// since the start (v1.20.4): a shaken speaker thinks harder before answering.
pub const THINK_SHAKEN_BOOST: f64 = 0.15;

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
/// datetime, emotions description, emotion thresholds — measured at ~2 378 chars)
/// plus the intention block (`INTENTION_BLOCK_MAX_CHARS`, v1.17).
pub const BUDGET_DETERMINISTIC_OVERHEAD_CHARS: usize = 2_400 + INTENTION_BLOCK_MAX_CHARS;

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
/// Floor: per-participant chars for positional map (one short stance each).
pub const BUDGET_FLOOR_POSITIONAL_MAP: usize = 50;
/// Floor: total chars for open loops (questions and commitments awaiting the
/// speaker). Non-zero on purpose: the waterfall fills surplus in rank order, so a
/// zero floor would hide the loops below ~16k contexts (one or two loops fit here).
pub const BUDGET_FLOOR_OPEN_LOOPS: usize = 300;

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
/// Ceiling: per-participant chars for positional map (stance + shift + condition, v1.17).
pub const BUDGET_CEIL_POSITIONAL_MAP_PER_PARTICIPANT: usize = 320;
/// Ceiling: total chars for open loops.
pub const BUDGET_CEIL_OPEN_LOOPS: usize = 900;

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
/// Chars of an unusable extraction answer kept in the log (diagnosis of a model's JSON habits).
pub const ARGMAP_RAW_LOG_MAX_CHARS: usize = 4_000;

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

/// Concurrent requests a local Ollama server is asked to serve. Measured on
/// 2026-09-16: two concurrent short chats finish only ~15 % faster than two
/// sequential ones on a consumer GPU (the server interleaves them), so the
/// engine keeps local calls strictly sequential.
pub const OLLAMA_MAX_PARALLEL_CALLS: usize = 1;

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

/// Concurrent requests sent to the DeepSeek API (end-of-turn analyses, reaction rounds).
/// Bounded so that a burst never trips the rate limiter; 429s still back off.
pub const DEEPSEEK_MAX_PARALLEL_CALLS: usize = 4;

/// Hard cap on `max_tokens` accepted by the API.
pub const DEEPSEEK_MAX_OUTPUT_TOKENS: i32 = 393_216;

/// Extra output tokens granted on top of `num_predict` when reasoning is active.
/// Assumption (H-DS-1): `max_tokens` bounds reasoning + content together — the API
/// defaults (8K without thinking vs 64K with) strongly suggest it.
pub const DEEPSEEK_REASONING_ALLOWANCE_LOW: i32 = 4_096;
pub const DEEPSEEK_REASONING_ALLOWANCE_HIGH: i32 = 12_288;
pub const DEEPSEEK_REASONING_ALLOWANCE_MAX: i32 = 32_768;
/// Fast reasoning pace: the allowances above are scaled down by this factor (v1.17).
pub const DEEPSEEK_FAST_PACE_ALLOWANCE_FACTOR: f64 = 0.5;

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

// ── OpenAI-compatible servers (v1.20) ───────────────────────────────────
// The transport timings (connect / idle timeouts, retries, backoff) are the
// `DEEPSEEK_*` ones above: one SSE transport serves both dialects.

/// Suggested base URL shown in the settings (LM Studio's default).
pub const OPENAI_COMPAT_DEFAULT_BASE_URL: &str = "http://localhost:1234/v1";
/// Hard cap on `max_tokens` for a generic server (its own limit applies below).
pub const OPENAI_COMPAT_MAX_OUTPUT_TOKENS: i32 = 65_536;
/// Concurrent calls worth issuing to a generic server (local ones interleave).
pub const OPENAI_COMPAT_MAX_PARALLEL_CALLS: usize = 2;
/// Context window assumed when the settings hold none.
pub const OPENAI_COMPAT_DEFAULT_CONTEXT_TOKENS: u32 = 32_768;

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

// ── Profondeur argumentative (v1.20.1) ─────────────────────────────────

/// Weight added to the `Deepen` speech act in the argumentative modes.
pub const SPEECH_ACT_DEEPEN_BONUS: u32 = 6;
/// Weight added instead when the speaker still owes an answer to an objection.
pub const SPEECH_ACT_DEEPEN_OWED_BONUS: u32 = 16;
/// Unanswered objections listed in the extraction prompt (newest first), so an
/// answer gets nested under the objection it addresses.
pub const ARGMAP_PROMPT_MAX_OBJECTIONS: usize = 6;

// ── Le public dans le débat, variété de forme (v1.20.2) ─────────────────

/// Chars of the audience's message quoted to the speaker who owes them an answer and to the moderator.
pub const AUDIENCE_MESSAGE_EXCERPT_CHARS: usize = 240;
/// Own previous interventions a speaker is reminded of (anti-repetition of content and openings).
pub const SELF_MEMORY_MESSAGES: usize = 3;
/// Chars of an intervention's opening quoted back to its author ("do not open like this again").
pub const OPENING_EXCERPT_CHARS: usize = 80;
/// Recent openings of the moderator's own lines quoted in the moderation prompt.
pub const ARBITRE_RECENT_OPENINGS: usize = 4;
/// Share of interventions the moderator may comment on before being told to hold back (percent).
pub const MODERATION_COMMENT_RATE_MAX_PERCENT: u32 = 25;
/// Speaker id of the audience member (the user) in messages, reactions and the argument map.
pub const USER_SPEAKER_ID: &str = "user";

// ── Réalisme de la mise en scène (v1.20.3) ──────────────────────────────

/// Output allowance of the moderator's voiced announcement (an act, a scene event):
/// two full sentences, even verbose ones.
pub const ANNOUNCEMENT_NUM_PREDICT: i32 = 220;
/// Bytes kept of a voiced announcement, whole sentences only (v1.20.4); the
/// brief stays the fallback.
pub const ANNOUNCEMENT_MAX_CHARS: usize = 900;
/// Output allowance of the audience-question call (JSON).
pub const AUDIENCE_QUESTION_NUM_PREDICT: i32 = 200;
/// Chars kept of the audience's question.
pub const AUDIENCE_QUESTION_MAX_CHARS: usize = 240;
/// A room question shorter than this is not one (a bare "Oui ?").
pub const AUDIENCE_QUESTION_MIN_CHARS: usize = 12;
/// Chars of one participant's portrait in the cast block (role, creed, register).
pub const CAST_PORTRAIT_MAX_CHARS: usize = 220;
/// Chars of the whole cast block appended to a system prompt.
pub const CAST_BLOCK_MAX_CHARS: usize = 1_400;
/// Theses listed in the "[État du débat]" block (the most argued first).
pub const DEBATE_STATE_MAX_THESES: usize = 6;
/// Unanswered objections listed in the "[État du débat]" block (the newest first).
pub const DEBATE_STATE_MAX_OBJECTIONS: usize = 4;
/// Floor / ceiling (chars) of the debate-state section of the waterfall budget.
pub const BUDGET_FLOOR_DEBATE_STATE: usize = 300;
pub const BUDGET_CEIL_DEBATE_STATE: usize = 1_200;

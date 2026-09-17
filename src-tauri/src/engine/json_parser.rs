use std::collections::{HashMap, HashSet};

use serde::de::DeserializeOwned;

use crate::constants;
use crate::engine::truncate_str;
use crate::models::emotion::EmotionDelta;
use crate::engine::truncate_at_word_boundary;
use crate::models::agenda::{Agenda, CastingPick, CastingSuggestion};
use crate::models::outcome::{VERDICT_DEFENSE, VERDICT_PROSECUTION};
use crate::models::persona_memory::PersonaRecap;
use crate::models::intention::{Intention, IntentionGoal};
use crate::models::message::ReactionType;
use crate::models::moderation::{ModerationResult, RawReaction};

/// Trimmed text, or `None` when blank (the models' way of saying "nothing").
pub(crate) fn non_empty(s: &str) -> Option<String> {
    let trimmed = s.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("null") { None } else { Some(trimmed.to_string()) }
}

// ── Intention (v1.17) ───────────────────────────────────────────────────

/// Wire shape of the intention call; every field optional (partial JSON is common).
#[derive(Debug, Default, serde::Deserialize)]
struct RawIntention {
    #[serde(default, alias = "cible", alias = "目标")]
    target: Option<serde_json::Value>,
    #[serde(default, alias = "objectif", alias = "目的")]
    goal: String,
    #[serde(default, alias = "角度")]
    angle: String,
    #[serde(default)]
    concession: Option<serde_json::Value>,
    #[serde(default, alias = "问题")]
    question: Option<serde_json::Value>,
    #[serde(default, alias = "repond_a", alias = "répond_à", alias = "answers_to")]
    answers: Option<serde_json::Value>,
    #[serde(default, alias = "pensee", alias = "pensée", alias = "reflexion", alias = "思考")]
    thought: String,
}

/// Text of a JSON value that may be a string, a number or null.
fn value_text(v: &Option<serde_json::Value>) -> Option<String> {
    match v.as_ref()? {
        serde_json::Value::String(s) => non_empty(s),
        serde_json::Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

/// Parse the intention JSON. `None` when the answer is not JSON at all (the
/// caller then treats the raw text as a plain persona thought — v1.16 behaviour).
///
/// The target is resolved against `known_names` with the usual fuzzy matching;
/// an unknown name is kept as-is (the engine decides what to do with it) and a
/// "topic" word yields `None`.
pub fn parse_intention(raw: &str, known_names: &[String]) -> Option<Intention> {
    let parsed = parse_json_response::<RawIntention>(raw).ok()?;
    let target = value_text(&parsed.target).and_then(|t| {
        if constants::INTENTION_TOPIC_WORDS.iter().any(|w| t.eq_ignore_ascii_case(w)) {
            return None;
        }
        Some(match_speaker_name(&t, known_names).cloned().unwrap_or(t))
    });
    let bounded = |s: Option<String>, max: usize| s.map(|t| truncate_str(&t, max).to_string());
    let answers = match parsed.answers.as_ref() {
        Some(serde_json::Value::Number(n)) => n.as_u64().map(|n| n as usize),
        Some(serde_json::Value::String(s)) => s.trim().parse::<usize>().ok(),
        _ => None,
    }
    .filter(|n| *n >= 1);
    Some(Intention {
        target,
        goal: IntentionGoal::parse(&parsed.goal),
        // Display bound only: the intervention prompt cuts each field to its own bound
        angle: bounded(non_empty(&parsed.angle), constants::INTENTION_DISPLAY_MAX_CHARS).unwrap_or_default(),
        concession: bounded(value_text(&parsed.concession), constants::INTENTION_DISPLAY_MAX_CHARS),
        question: bounded(value_text(&parsed.question), constants::INTENTION_DISPLAY_MAX_CHARS),
        answers,
        thought: non_empty(&parsed.thought).unwrap_or_default(),
    })
}

// ── Hidden agenda and casting (v1.19) ───────────────────────────────────

#[derive(Debug, Default, serde::Deserialize)]
struct RawAgenda {
    #[serde(default, alias = "objectif", alias = "goal", alias = "目标")]
    objective: String,
    #[serde(default, alias = "ligne_rouge", alias = "redLine", alias = "red_line", alias = "底线")]
    red_line: String,
    #[serde(default, alias = "victoire", alias = "win", alias = "胜利")]
    victory: String,
}

/// Parse a hidden agenda; `None` when the answer is not JSON or holds nothing.
pub fn parse_agenda(raw: &str) -> Option<Agenda> {
    let parsed = parse_json_response::<RawAgenda>(raw).ok()?;
    let field = |s: &str| non_empty(s).map(|t| truncate_str(&t, constants::AGENDA_FIELD_MAX_CHARS).to_string()).unwrap_or_default();
    let agenda = Agenda { objective: field(&parsed.objective), red_line: field(&parsed.red_line), victory: field(&parsed.victory) };
    (!agenda.is_empty()).then_some(agenda)
}

/// Did the synthesis say the objective of `name` was reached? Only the lines of
/// the "## Agendas" section (any language) are read, so a "reached consensus"
/// elsewhere never counts; `None` when the section or the verdict is missing
/// (a partial outcome is reported as unknown).
pub fn agenda_outcome(synthesis: &str, name: &str) -> Option<bool> {
    const SECTION_KEYWORDS: [&str; 2] = ["agenda", "议程"];
    const PARTIAL: [&str; 4] = ["partiellement", "partially", "partly", "部分"];
    const NEGATIVE: [&str; 8] = ["non atteint", "pas atteint", "not achieved", "not reached", "unmet", "missed", "未达成", "未实现"];
    const POSITIVE: [&str; 5] = ["atteint", "achieved", "reached", "达成", "实现"];
    let lower_name = name.to_lowercase();
    let mut in_section = false;
    for line in synthesis.lines() {
        let l = line.trim().to_lowercase();
        if l.starts_with('#') {
            in_section = SECTION_KEYWORDS.iter().any(|k| l.contains(k));
            continue;
        }
        if !in_section || !l.contains(&lower_name) {
            continue;
        }
        if PARTIAL.iter().any(|p| l.contains(p)) {
            return None;
        }
        if NEGATIVE.iter().any(|n| l.contains(n)) {
            return Some(false);
        }
        if POSITIVE.iter().any(|p| l.contains(p)) {
            return Some(true);
        }
    }
    None
}

#[derive(Debug, Default, serde::Deserialize)]
struct RawCastingPick {
    #[serde(default)]
    id: String,
    #[serde(default, alias = "raison", alias = "理由")]
    reason: String,
}

#[derive(Debug, Default, serde::Deserialize)]
struct RawCasting {
    #[serde(default, alias = "gladiators", alias = "participants")]
    gladiateurs: Vec<RawCastingPick>,
    #[serde(default, alias = "moderator", alias = "moderateur", alias = "modérateur")]
    arbitre: String,
}

/// Parse a casting suggestion, keeping only known ids (unknown ones are dropped, duplicates too).
pub fn parse_casting(raw: &str, known_gladiateurs: &[String], known_arbitres: &[String]) -> CastingSuggestion {
    let Ok(parsed) = parse_json_response::<RawCasting>(raw) else { return CastingSuggestion::default() };
    let mut seen = HashSet::new();
    let gladiateurs = parsed
        .gladiateurs
        .into_iter()
        .filter(|p| known_gladiateurs.iter().any(|k| k == &p.id) && seen.insert(p.id.clone()))
        .map(|p| CastingPick { id: p.id, reason: non_empty(&p.reason).unwrap_or_default() })
        .collect();
    let arbitre = non_empty(&parsed.arbitre).filter(|id| known_arbitres.iter().any(|k| k == id));
    CastingSuggestion { gladiateurs, arbitre }
}

// ── Persona recap (long memory, v1.20) ──────────────────────────────────

#[derive(Debug, Default, serde::Deserialize)]
struct RawRecap {
    #[serde(default, alias = "positions_defendues", alias = "立场")]
    positions: Vec<String>,
    #[serde(default, alias = "bestLines", alias = "meilleures_phrases", alias = "best_sentences", alias = "金句")]
    best_lines: Vec<String>,
    #[serde(default, alias = "allies", alias = "alliés", alias = "盟友")]
    allies: Vec<String>,
    #[serde(default, alias = "rivaux", alias = "对手")]
    rivals: Vec<String>,
    #[serde(default, alias = "leçon", alias = "lecon", alias = "takeaway", alias = "教训")]
    lesson: String,
}

/// Parse a persona recap: lists bounded in number and length, names limited to
/// the known participants; `None` when unusable or empty.
pub fn parse_recap(raw: &str, known_names: &[String]) -> Option<PersonaRecap> {
    let parsed = parse_json_response::<RawRecap>(raw).ok()?;
    let list = |items: Vec<String>| -> Vec<String> {
        items
            .into_iter()
            .filter_map(|s| non_empty(&s))
            .map(|s| truncate_at_word_boundary(&s, constants::RECAP_ITEM_MAX_CHARS))
            .take(constants::RECAP_LIST_MAX_ITEMS)
            .collect()
    };
    let names = |items: Vec<String>| -> Vec<String> {
        items
            .into_iter()
            .filter_map(|s| match_speaker_name(&s, known_names).map(|n| n.to_string()))
            .take(constants::RECAP_LIST_MAX_ITEMS)
            .collect()
    };
    let recap = PersonaRecap {
        positions: list(parsed.positions),
        best_lines: list(parsed.best_lines),
        allies: names(parsed.allies),
        rivals: names(parsed.rivals),
        lesson: non_empty(&parsed.lesson).map(|s| truncate_at_word_boundary(&s, constants::RECAP_ITEM_MAX_CHARS)).unwrap_or_default(),
    };
    (!recap.is_empty()).then_some(recap)
}

// ── Verdicts, agreements, dispatches (structured modes, v1.19) ──────────

#[derive(Debug, Default, serde::Deserialize)]
struct RawVerdict {
    #[serde(default, alias = "choice", alias = "side", alias = "camp", alias = "裁决")]
    verdict: String,
    #[serde(default, alias = "raison", alias = "motivation", alias = "理由")]
    reason: String,
}

/// A juror's verdict: the side (any language, "accusation" / "prosecution" /
/// "控方" → prosecution, "défense" / "defence" / "辩方" → defence) and the reason.
pub fn parse_verdict(raw: &str) -> Option<(String, String)> {
    let parsed = parse_json_response::<RawVerdict>(raw).ok()?;
    let v = parsed.verdict.to_lowercase();
    let side = if v.contains("accus") || v.contains("prosecut") || v.contains("控") || v.contains("guilty") && !v.contains("not guilty") || v.contains("coupable") && !v.contains("non coupable") {
        VERDICT_PROSECUTION
    } else if v.contains("défen") || v.contains("defen") || v.contains("辩") || v.contains("not guilty") || v.contains("non coupable") || v.contains("acquit") {
        VERDICT_DEFENSE
    } else {
        return None;
    };
    Some((side.to_string(), truncate_str(parsed.reason.trim(), constants::VERDICT_REASON_MAX_CHARS).to_string()))
}

#[derive(Debug, Default, serde::Deserialize)]
struct RawAgreement {
    #[serde(default, alias = "accept", alias = "accepte", alias = "agrees", alias = "接受")]
    accepts: serde_json::Value,
    #[serde(default, alias = "raison", alias = "motivation", alias = "理由")]
    reason: String,
}

/// A party's decision on the deal: a boolean, or a yes/no word in any language.
pub fn parse_agreement(raw: &str) -> Option<(bool, String)> {
    let parsed = parse_json_response::<RawAgreement>(raw).ok()?;
    let accepts = match &parsed.accepts {
        serde_json::Value::Bool(b) => *b,
        serde_json::Value::String(s) => {
            let s = s.trim().to_lowercase();
            if ["oui", "yes", "true", "accepte", "accept", "是", "同意", "接受"].iter().any(|y| s.starts_with(y)) {
                true
            } else if ["non", "no", "false", "refuse", "reject", "否", "不", "拒绝"].iter().any(|n| s.starts_with(n)) {
                false
            } else {
                return None;
            }
        }
        _ => return None,
    };
    Some((accepts, truncate_str(parsed.reason.trim(), constants::VERDICT_REASON_MAX_CHARS).to_string()))
}

#[derive(Debug, Default, serde::Deserialize)]
struct RawDispatches {
    #[serde(default, alias = "depeches", alias = "dépêches", alias = "急电", alias = "items")]
    dispatches: Vec<String>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct RawAudienceQuestion {
    #[serde(default, alias = "q", alias = "問題", alias = "问题")]
    question: serde_json::Value,
}

/// The room's question (v1.20.3), bounded; `None` when unusable or too short to be one.
pub fn parse_audience_question(raw: &str) -> Option<String> {
    let parsed = parse_json_response::<RawAudienceQuestion>(raw).ok()?;
    let q = loose_text(&parsed.question)?;
    (q.chars().count() >= constants::AUDIENCE_QUESTION_MIN_CHARS).then(|| truncate_at_word_boundary(&q, constants::AUDIENCE_QUESTION_MAX_CHARS))
}

/// The crisis dispatches, bounded in number and length; empty on an unusable answer.
pub fn parse_dispatches(raw: &str, max: usize) -> Vec<String> {
    let Ok(parsed) = parse_json_response::<RawDispatches>(raw) else { return Vec::new() };
    parsed
        .dispatches
        .into_iter()
        .filter_map(|d| non_empty(&d))
        .map(|d| truncate_at_word_boundary(&d, constants::CRISIS_DISPATCH_MAX_CHARS))
        .take(max)
        .collect()
}

// ── Memory update (positions trajectory, v1.17) ─────────────────────────

/// A position as the memory model may write it: the v1.16 one-line stance, or
/// the v1.17 object with its trajectory. Mixed maps are accepted.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
pub enum PositionInput {
    Text(String),
    Detailed {
        #[serde(default, alias = "position")]
        stance: String,
        #[serde(default, alias = "evolution", alias = "évolution")]
        shift: Option<serde_json::Value>,
        #[serde(default, alias = "wouldChangeIf", alias = "changerait_si")]
        would_change_if: Option<serde_json::Value>,
    },
}

impl PositionInput {
    /// `(stance, shift, would_change_if)` with blanks and "null" strings normalised away.
    pub fn into_parts(self) -> (String, Option<String>, Option<String>) {
        match self {
            Self::Text(s) => (s.trim().to_string(), None, None),
            Self::Detailed { stance, shift, would_change_if } => {
                (stance.trim().to_string(), value_text(&shift), value_text(&would_change_if))
            }
        }
    }
}

/// A question the memory model spotted as still unanswered.
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct OpenQuestion {
    #[serde(default, alias = "à", alias = "a", alias = "target")]
    pub to: String,
    #[serde(default, alias = "de", alias = "by")]
    pub from: String,
    #[serde(default, alias = "text")]
    pub question: String,
}

/// Response of the combined memory update call.
#[derive(Debug, Default, serde::Deserialize)]
pub struct MemoryUpdateResponse {
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub positions: HashMap<String, PositionInput>,
    #[serde(default, alias = "openQuestions", alias = "questions_ouvertes")]
    pub open_questions: Vec<OpenQuestion>,
}

#[derive(Debug, thiserror::Error)]
pub enum JsonParseError {
    #[error("Failed to parse JSON: {0}")]
    ParseFailed(String),
}

/// Multi-layer JSON extraction
pub fn parse_json_response<T: DeserializeOwned>(raw: &str) -> Result<T, JsonParseError> {
    // 1. Direct parse
    if let Ok(val) = serde_json::from_str::<T>(raw) {
        return Ok(val);
    }

    // 2. Extract from markdown ```json ... ``` block
    if let Some(block) = extract_markdown_json(raw) {
        if let Ok(val) = serde_json::from_str::<T>(&block) {
            return Ok(val);
        }
        // 2b. Try fixing common issues on the extracted block
        let cleaned = fix_common_json_issues(&block);
        if let Ok(val) = serde_json::from_str::<T>(&cleaned) {
            return Ok(val);
        }
    }

    // 3. Find the first { ... } or [ ... ] via brace counting
    if let Some(obj) = extract_first_json_object(raw) {
        if let Ok(val) = serde_json::from_str::<T>(&obj) {
            return Ok(val);
        }
        // 3b. Try fixing common issues on the extracted object
        let cleaned = fix_common_json_issues(&obj);
        if let Ok(val) = serde_json::from_str::<T>(&cleaned) {
            return Ok(val);
        }
    }

    // 4. Clean common issues on raw text then try extraction
    let cleaned = fix_common_json_issues(raw);
    if let Ok(val) = serde_json::from_str::<T>(&cleaned) {
        return Ok(val);
    }
    if let Some(obj) = extract_first_json_object(&cleaned) {
        if let Ok(val) = serde_json::from_str::<T>(&obj) {
            return Ok(val);
        }
    }

    // Safe truncation for error message
    let truncated = safe_truncate(raw, 200);
    Err(JsonParseError::ParseFailed(truncated))
}

/// Parse a moderation answer (the caller falls back to "none" and counts the failure).
pub fn parse_moderation(raw: &str) -> Result<ModerationResult, JsonParseError> {
    parse_json_response(raw)
}

/// Fused end-of-turn analysis (v1.17, sequential providers): the memory update
/// fields plus an `emotions` object shaped like the emotion analysis answer.
/// `None` when the answer is not JSON at all; a missing part leaves its state
/// untouched (empty memory summary / no deltas).
pub fn parse_turn_analyst(raw: &str, known_speakers: &[String]) -> Option<(MemoryUpdateResponse, EmotionAnalysis)> {
    let value = parse_json_response::<serde_json::Value>(raw).ok()?;
    let memory = serde_json::from_value::<MemoryUpdateResponse>(value.clone()).unwrap_or_default();
    let mut emotions = value
        .get("emotions")
        .filter(|e| e.is_object())
        .map(|e| parse_emotion_analysis(&e.to_string(), known_speakers))
        .unwrap_or_default();
    if emotions.stagnating.is_none() {
        emotions.stagnating = value.get(constants::EMOTION_STAGNATION_JSON_KEY).and_then(serde_json::Value::as_bool);
    }
    Some((memory, emotions))
}

/// Response from a democratic vote: gladiator ranks who should speak
#[derive(Debug, serde::Deserialize)]
struct VoteResponse {
    #[serde(default)]
    ranking: Vec<String>,
}

/// Response from IArbitre ordering speakers
#[derive(Debug, serde::Deserialize)]
struct AuthoritarianOrderResponse {
    #[serde(default)]
    order: Vec<String>,
}

/// Response from LLM deciding whether to search the web
#[derive(Debug, Default, serde::Deserialize)]
pub(crate) struct SearchDecisionResponse {
    #[serde(default)]
    pub needs_search: bool,
    #[serde(default)]
    pub queries: Vec<String>,
}

/// Parse a democratic vote response, returning a ranked list of names.
/// Falls back to empty Vec on parse failure.
pub fn parse_vote(raw: &str) -> Vec<String> {
    parse_json_response::<VoteResponse>(raw)
        .map(|r| r.ranking)
        .ok()
        .filter(|r| !r.is_empty())
        .or_else(|| parse_json_response::<Vec<String>>(raw).ok())
        .unwrap_or_default()
}

/// Parse an authoritarian order response, returning ordered speaker names.
/// Falls back to empty Vec on parse failure.
pub fn parse_authoritarian_order(raw: &str) -> Vec<String> {
    parse_json_response::<AuthoritarianOrderResponse>(raw)
        .map(|r| r.order)
        .ok()
        .filter(|r| !r.is_empty())
        .or_else(|| parse_json_response::<Vec<String>>(raw).ok())
        .unwrap_or_default()
}

/// Wrapper struct for when the LLM wraps the array in an object
/// e.g. {"reactions": [...]} or {"responses": [...]} or {"interventions": [...]}
#[derive(Debug, serde::Deserialize)]
struct WrappedRawReactions {
    #[serde(default, alias = "responses", alias = "interventions")]
    reactions: Vec<RawReaction>,
}

/// Parse reactions with validation against known speakers
pub fn parse_reactions(raw: &str, known_speakers: &[String]) -> Vec<ParsedReaction> {
    // 1. Try parsing as bare array: [{"speaker":"A","reaction":"like"}, ...]
    // 2. Try wrapped object: {"reactions": [...]} or {"responses": [...]} or {"interventions": [...]}
    // 3. Try single object: {"speaker":"A","reaction":"like"}
    // 4. Try duplicate-key flat object via regex: {"speaker":"A","reaction":"like","speaker":"B","reaction":"dislike"}
    let raw_reactions = parse_json_response::<Vec<RawReaction>>(raw)
        .or_else(|_| {
            parse_json_response::<WrappedRawReactions>(raw).and_then(|w| {
                if w.reactions.is_empty() {
                    Err(JsonParseError::ParseFailed("empty wrapped reactions".to_string()))
                } else {
                    Ok(w.reactions)
                }
            })
        })
        .or_else(|_| {
            parse_json_response::<RawReaction>(raw).map(|r| vec![r])
        })
        .or_else(|_| {
            // Fallback: extract speaker/reaction pairs from duplicate-key flat objects
            // e.g. {"speaker":"A","reaction":"like","speaker":"B","reaction":"dislike"}
            extract_duplicate_key_reactions(raw)
        });

    raw_reactions
        .map(|r| validate_reactions(r, known_speakers))
        .unwrap_or_default()
}

/// Extract reactions from invalid JSON with duplicate keys
/// e.g. {"speaker":"A","reaction":"like","speaker":"B","reaction":"dislike"}
fn extract_duplicate_key_reactions(raw: &str) -> Result<Vec<RawReaction>, JsonParseError> {
    let mut reactions = Vec::new();
    let mut search_from = 0;

    // Find all "speaker":"value" patterns and pair them with following "reaction":"value"
    while let Some(sp_start) = raw[search_from..].find("\"speaker\"") {
        let sp_abs = search_from + sp_start;
        // Find the value after the colon
        if let Some(speaker) = extract_json_string_value(&raw[sp_abs..]) {
            // Search for "reaction" starting right after `"speaker":` (always ASCII-safe offset)
            let search_reaction_from = sp_abs + 10; // len('"speaker":') = 10
            if let Some(rx_start) = raw[search_reaction_from..].find("\"reaction\"") {
                let rx_abs = search_reaction_from + rx_start;
                if let Some(reaction) = extract_json_string_value(&raw[rx_abs..]) {
                    // Best-effort: try to extract justification after reaction
                    let search_just_from = rx_abs + 11;
                    let justification = raw[search_just_from..]
                        .find("\"justification\"")
                        .and_then(|j_start| {
                            extract_json_string_value(&raw[search_just_from + j_start..])
                        })
                        .unwrap_or_default();
                    reactions.push(RawReaction { speaker, reaction, justification, quote: String::new(), reacts: None });
                    search_from = rx_abs + 11; // len('"reaction":') = 11
                    continue;
                }
            }
        }
        search_from = sp_abs + 10;
    }

    if reactions.is_empty() {
        Err(JsonParseError::ParseFailed("no duplicate-key reactions found".to_string()))
    } else {
        Ok(reactions)
    }
}

/// Extract a JSON string value from a pattern like `"key":"value"` or `"key": "value"`
fn extract_json_string_value(s: &str) -> Option<String> {
    // Skip past the key and colon
    let colon_pos = s.find(':')?;
    let after_colon = s[colon_pos + 1..].trim_start();
    if !after_colon.starts_with('"') {
        return None;
    }
    let value_content = &after_colon[1..]; // skip opening quote
    // Find closing quote (not escaped), tracking byte offsets for UTF-8 safety
    let mut byte_offset = 0;
    let mut chars = value_content.chars();
    loop {
        let ch = chars.next()?;
        if ch == '\\' {
            byte_offset += ch.len_utf8();
            if let Some(escaped) = chars.next() {
                byte_offset += escaped.len_utf8();
            }
        } else if ch == '"' {
            return Some(value_content[..byte_offset].to_string());
        } else {
            byte_offset += ch.len_utf8();
        }
    }
}

/// Validated reaction ready to be converted to a Reaction
#[derive(Debug)]
pub struct ParsedReaction {
    pub speaker_name: String,
    pub reaction_type: ReactionType,
    pub justification: Option<String>,
    /// Raw excerpt claimed by the model (not yet validated against the message)
    pub quote: Option<String>,
}

/// Normalize Unicode punctuation variants to ASCII equivalents.
///
/// - Dashes: en-dash, em-dash, non-breaking hyphen, etc. → ASCII hyphen (U+002D)
/// - Apostrophes: right single quote, left single quote, modifier letter → ASCII apostrophe (U+0027)
///
/// LLMs often output typographic variants that break exact string matching.
fn normalize_punctuation(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\u{2010}' | '\u{2011}' | '\u{2012}' | '\u{2013}' | '\u{2014}' | '\u{2015}'
            | '\u{2212}' => result.push('-'),
            '\u{2019}' | '\u{2018}' | '\u{201A}' | '\u{02BC}' => result.push('\''),
            _ => result.push(ch),
        }
    }
    result
}

/// Does `text` mention `name` (v1.20.5)? Case-insensitive, and the name
/// without its article counts ("Créatif, tu nous emmènes loin" addresses
/// "Le Créatif") when what remains is at least three characters long. Shared by
/// the engine's intention-compliance diagnostic and the bench metrics.
pub fn mentions_name(text: &str, name: &str) -> bool {
    let name = name.trim().to_lowercase();
    if name.is_empty() {
        return false;
    }
    let text = text.to_lowercase();
    if text.contains(&name) {
        return true;
    }
    let stripped = strip_french_article(&name);
    stripped != name && stripped.chars().count() >= 3 && text.contains(stripped)
}

/// Strip leading articles: "Le ", "La ", "L'", "Les " (and "The " for English casts)
fn strip_french_article(name: &str) -> &str {
    let trimmed = name.trim();
    for prefix in &["le ", "la ", "l'", "les ", "the "] {
        if trimmed.len() > prefix.len() {
            let lower_start: String = trimmed.chars().take(prefix.len()).collect::<String>().to_lowercase();
            if lower_start == *prefix {
                return trimmed[prefix.len()..].trim();
            }
        }
    }
    trimmed
}

/// Match a LLM-returned name against a list of known names.
/// 4 layers: exact case-insensitive → article-stripped → prefix (min 3 chars) → contains (min 4 chars)
/// Normalizes Unicode dashes so "Le Psycho‑rigide" (U+2011) matches "Le Psycho-rigide" (U+002D).
pub fn match_speaker_name<'a>(llm_name: &str, known_names: &'a [String]) -> Option<&'a String> {
    let llm_lower = normalize_punctuation(llm_name.to_lowercase().trim());
    let llm_stripped = strip_french_article(&llm_lower);

    // 1. Exact match (case-insensitive, trimmed, dash-normalized)
    known_names
        .iter()
        .find(|s| normalize_punctuation(s.to_lowercase().trim()) == llm_lower)
        // 2. Article-stripped match: "Scientifique" matches "Le Scientifique"
        .or_else(|| {
            if llm_stripped.len() >= 3 {
                known_names
                    .iter()
                    .find(|s| strip_french_article(&normalize_punctuation(&s.to_lowercase())) == llm_stripped)
            } else {
                None
            }
        })
        // 3. Prefix match: known name starts with the LLM-provided string (min 3 chars)
        .or_else(|| {
            if llm_lower.len() >= 3 {
                known_names
                    .iter()
                    .find(|s| normalize_punctuation(&s.to_lowercase()).starts_with(&llm_lower))
            } else {
                None
            }
        })
        // 4. Contains match: LLM output contains the stripped known name or vice versa
        .or_else(|| {
            if llm_stripped.len() >= 4 {
                known_names.iter().find(|s| {
                    let s_lower = normalize_punctuation(&s.to_lowercase());
                    let s_stripped = strip_french_article(&s_lower);
                    s_stripped.contains(llm_stripped) || llm_stripped.contains(s_stripped)
                })
            } else {
                None
            }
        })
}

/// Parse LLM emotion deltas: `{"Speaker Name": {"engagement": 5, "accord": -3, ...}, ...}`
/// Uses fuzzy name matching to resolve speaker names.
/// Result of the end-of-turn LLM emotion analysis.
#[derive(Debug, Default)]
pub struct EmotionAnalysis {
    /// Deltas per known participant name (unknown names dropped).
    pub deltas: HashMap<String, EmotionDelta>,
    /// Top-level `"stagnating"` flag when the model provided one.
    pub stagnating: Option<bool>,
}

/// Parse `{"Name": {deltas…}, …, "stagnating": true}`. Participant entries that
/// are not objects (or whose name is unknown) are ignored.
pub fn parse_emotion_analysis(raw: &str, known_speakers: &[String]) -> EmotionAnalysis {
    let mut out = EmotionAnalysis::default();
    let Ok(serde_json::Value::Object(map)) = parse_json_response::<serde_json::Value>(raw) else {
        return out;
    };
    for (key, value) in map {
        if key.eq_ignore_ascii_case(constants::EMOTION_STAGNATION_JSON_KEY) {
            out.stagnating = value.as_bool();
            continue;
        }
        let Some(matched) = match_speaker_name(&key, known_speakers) else { continue };
        if let Ok(delta) = serde_json::from_value::<EmotionDelta>(value) {
            out.deltas.insert(matched.clone(), delta);
        }
    }
    out
}

fn validate_reactions(raw: Vec<RawReaction>, known_speakers: &[String]) -> Vec<ParsedReaction> {
    let mut seen = HashSet::new();
    raw.into_iter()
        .filter_map(|r| {
            let speaker = match_speaker_name(&r.speaker, known_speakers)?;

            // A declined reaction is a silence (v1.20.5), like "none"
            if r.declined() {
                return None;
            }
            // Normalize the reaction value FIRST (before dedup)
            // so that "none" reactions don't consume a dedup slot
            let reaction_type = ReactionType::parse(&r.reaction)?;

            // Deduplicate: keep only the first valid reaction per target speaker
            if !seen.insert(speaker.clone()) {
                return None;
            }

            Some(ParsedReaction {
                speaker_name: speaker.clone(),
                reaction_type,
                justification: non_empty(&r.justification),
                quote: non_empty(&r.quote),
            })
        })
        .collect()
}

fn extract_markdown_json(raw: &str) -> Option<String> {
    let start_markers = [
        "```json\n",
        "```json\r\n",
        "```JSON\n",
        "```Json\n",
        "```json \n",
        "```\n", // bare code block without language tag
    ];
    let end_marker = "```";

    for marker in &start_markers {
        if let Some(start) = raw.find(marker) {
            let content_start = start + marker.len();
            if let Some(end) = raw[content_start..].find(end_marker) {
                return Some(raw[content_start..content_start + end].trim().to_string());
            }
        }
    }
    None
}

fn extract_first_json_object(raw: &str) -> Option<String> {
    // Find the EARLIEST delimiter ({ or [) and match accordingly
    let brace_pos = raw.find('{');
    let bracket_pos = raw.find('[');

    let (open, open_char, close_char) = match (brace_pos, bracket_pos) {
        (Some(b), Some(a)) => {
            if a < b {
                (a, '[', ']')
            } else {
                (b, '{', '}')
            }
        }
        (Some(b), None) => (b, '{', '}'),
        (None, Some(a)) => (a, '[', ']'),
        (None, None) => return None,
    };

    let mut depth = 0;
    let mut in_string = false;
    let mut escape_next = false;

    for (i, ch) in raw[open..].char_indices() {
        if escape_next {
            escape_next = false;
            continue;
        }
        if ch == '\\' && in_string {
            escape_next = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        if ch == open_char {
            depth += 1;
        } else if ch == close_char {
            depth -= 1;
            if depth == 0 {
                return Some(raw[open..open + i + 1].to_string());
            }
        }
    }
    None
}

/// Extract the extractions array directly by finding the key ("extractions" or "results")
/// and then extracting the `[...]` array that follows.
/// Handles malformed JSON where the wrapper object has duplicate keys or trailing garbage.
fn extract_extractions_array_direct(raw: &str) -> Option<Vec<RawArgumentExtraction>> {
    for key in &["\"extractions\"", "\"results\""] {
        if let Some(key_pos) = raw.find(key) {
            let after_key = &raw[key_pos + key.len()..];
            // Skip whitespace and colon
            let trimmed = after_key.trim_start();
            if let Some(rest) = trimmed.strip_prefix(':') {
                let rest = rest.trim_start();
                if rest.starts_with('[') {
                    if let Some(arr_str) = extract_first_json_object(rest) {
                        if let Ok(v) = serde_json::from_str::<Vec<RawArgumentExtraction>>(&arr_str) {
                            if !v.is_empty() {
                                return Some(v);
                            }
                        }
                        // Try with fix_common_json_issues on the extracted array
                        let cleaned = fix_common_json_issues(&arr_str);
                        if let Ok(v) = serde_json::from_str::<Vec<RawArgumentExtraction>>(&cleaned) {
                            if !v.is_empty() {
                                return Some(v);
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

fn fix_common_json_issues(raw: &str) -> String {
    let mut result = String::with_capacity(raw.len());

    // Smart single-quote replacement: only replace single quotes that act as
    // JSON delimiters (outside of double-quoted strings). This avoids corrupting
    // apostrophes in natural language like "That's" or "l'argument".
    let mut in_double_string = false;
    let mut in_single_string = false;
    let mut escape_next = false;
    let chars: Vec<char> = raw.chars().collect();

    for i in 0..chars.len() {
        let ch = chars[i];
        if escape_next {
            result.push(ch);
            escape_next = false;
            continue;
        }
        if ch == '\\' && (in_double_string || in_single_string) {
            result.push(ch);
            escape_next = true;
            continue;
        }
        if ch == '"' && !in_single_string {
            in_double_string = !in_double_string;
            result.push(ch);
            continue;
        }
        if ch == '\'' && !in_double_string {
            // Check if this looks like a JSON delimiter (before : or , or ] or })
            // or after ({ or [ or : or ,)
            if !in_single_string {
                // Opening single quote — check if it's a JSON key/value delimiter
                // Heuristic: preceded by { [ , : or whitespace
                let prev_non_ws = chars[..i]
                    .iter()
                    .rev()
                    .find(|c| !c.is_whitespace())
                    .copied();
                if matches!(prev_non_ws, Some('{') | Some('[') | Some(',') | Some(':') | None) {
                    result.push('"');
                    in_single_string = true;
                    continue;
                }
            } else {
                // Closing single quote — check if followed by : , } ] or whitespace
                let next_non_ws = chars[i + 1..]
                    .iter()
                    .find(|c| !c.is_whitespace())
                    .copied();
                if matches!(
                    next_non_ws,
                    Some(':') | Some(',') | Some('}') | Some(']') | None
                ) {
                    result.push('"');
                    in_single_string = false;
                    continue;
                }
            }
            // Not a JSON delimiter — keep as-is (it's an apostrophe)
            result.push(ch);
            continue;
        }
        result.push(ch);
    }

    // Remove trailing commas before } or ]
    remove_trailing_commas(&result)
}

/// Remove trailing commas before } or ] (regex-free)
fn remove_trailing_commas(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == ',' {
            // Look ahead for optional whitespace followed by } or ]
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if j < chars.len() && (chars[j] == '}' || chars[j] == ']') {
                // Skip the trailing comma
                i += 1;
                continue;
            }
        }
        result.push(chars[i]);
        i += 1;
    }

    result
}

fn safe_truncate(s: &str, max_bytes: usize) -> String {
    super::truncate_str(s, max_bytes).to_string()
}

// ── Argument map extraction ──────────────────────────────────────────

use crate::models::argument_map::ArgumentType;

/// Parsed argument ready for merging into the argument map.
#[derive(Debug)]
pub struct ParsedArgument {
    pub text: String,
    pub arg_type: ArgumentType,
    pub for_thesis: Option<String>,
    pub against_thesis: Option<String>,
    pub targets_argument: Option<String>,
}

/// Parsed extraction result for one speaker.
#[derive(Debug)]
pub struct ParsedArgumentExtraction {
    pub speaker_name: String,
    pub new_theses: Vec<String>,
    pub arguments: Vec<ParsedArgument>,
}

#[derive(Debug, serde::Deserialize)]
struct RawArgumentExtraction {
    #[serde(default, alias = "name", alias = "participant")]
    speaker: serde_json::Value,
    #[serde(default, alias = "theses", alias = "new_thesis")]
    new_theses: serde_json::Value,
    #[serde(default, alias = "argument")]
    arguments: serde_json::Value,
}

#[derive(Debug, serde::Deserialize)]
struct RawArgument {
    #[serde(default, alias = "label", alias = "argument")]
    text: serde_json::Value,
    #[serde(default, alias = "type", alias = "kind")]
    arg_type: serde_json::Value,
    #[serde(default, alias = "thesis", alias = "supports_thesis")]
    for_thesis: serde_json::Value,
    #[serde(default, alias = "counters_thesis")]
    against_thesis: serde_json::Value,
    #[serde(default, alias = "target_argument", alias = "parent_argument", alias = "responds_to")]
    targets_argument: serde_json::Value,
}

/// Text of a loosely typed value: a string, a number, or an object's text-like field.
fn loose_text(v: &serde_json::Value) -> Option<String> {
    match v {
        serde_json::Value::String(s) => non_empty(s),
        serde_json::Value::Number(n) => Some(n.to_string()),
        serde_json::Value::Object(o) => ["text", "label", "thesis", "title", "name"].iter().find_map(|k| o.get(*k).and_then(loose_text)),
        _ => None,
    }
}

/// Items of a loosely typed list: an array, or the single item the model forgot to wrap.
fn loose_list(v: &serde_json::Value) -> Vec<serde_json::Value> {
    match v {
        serde_json::Value::Array(items) => items.clone(),
        serde_json::Value::Null => Vec::new(),
        other => vec![other.clone()],
    }
}

/// One argument of an extraction, from an object or a bare string.
fn loose_argument(v: serde_json::Value) -> Option<RawArgument> {
    match v {
        serde_json::Value::String(s) => Some(RawArgument {
            text: serde_json::Value::String(s),
            arg_type: serde_json::Value::Null,
            for_thesis: serde_json::Value::Null,
            against_thesis: serde_json::Value::Null,
            targets_argument: serde_json::Value::Null,
        }),
        other => serde_json::from_value(other).ok(),
    }
}

#[derive(Debug, serde::Deserialize)]
struct WrappedExtractions {
    #[serde(default, alias = "results")]
    extractions: Vec<RawArgumentExtraction>,
}

/// Check whether a thesis label is meaningful (not a bare number, index, or trivially short).
/// LLMs sometimes return thesis indices ("4", "9") instead of label text when existing
/// theses are presented as a numbered list.
pub(crate) fn is_valid_thesis_label(label: &str) -> bool {
    let trimmed = label.trim();
    // Reject purely numeric strings (indices the LLM copied from the list)
    if trimmed.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    // Reject labels shorter than the minimum (likely garbage or partial IDs)
    if trimmed.len() < constants::ARGMAP_MIN_THESIS_LABEL_CHARS {
        return false;
    }
    true
}

/// Parse argument extraction JSON from an LLM response.
/// Uses multi-layer JSON extraction and fuzzy speaker name matching.
pub fn parse_argument_extraction(
    raw: &str,
    known_speakers: &[String],
) -> Vec<ParsedArgumentExtraction> {
    // Try wrapped format first, then bare array, then with escaped-quote cleanup
    let raw_extractions = match parse_json_response::<WrappedExtractions>(raw) {
        Ok(w) => {
            tracing::debug!(count = w.extractions.len(), "Parsed as WrappedExtractions");
            w.extractions
        }
        Err(e1) => match parse_json_response::<Vec<RawArgumentExtraction>>(raw) {
            Ok(v) => {
                tracing::debug!(count = v.len(), "Parsed as bare array");
                v
            }
            Err(e2) => {
                // Fallback: LLMs sometimes mix normal and escaped quotes in a single
                // JSON response (e.g. `"speaker":"A"` then `"speaker\":\"B\"`).
                // Strip stray backslash-escapes and retry.
                let cleaned = raw.replace("\\\"", "\"");
                match parse_json_response::<WrappedExtractions>(&cleaned)
                    .map(|w| w.extractions)
                    .or_else(|_| parse_json_response::<Vec<RawArgumentExtraction>>(&cleaned))
                {
                    Ok(v) if !v.is_empty() => {
                        tracing::info!(
                            count = v.len(),
                            "Parsed after escaped-quote cleanup"
                        );
                        v
                    }
                    _ => {
                        // Last resort: extract the extractions array directly from the key position.
                        // Handles malformed JSON where the model duplicates keys or adds trailing garbage
                        // (e.g. {"extractions":[{...}],"extractions"  ]}).
                        if let Some(v) = extract_extractions_array_direct(raw) {
                            tracing::info!(
                                count = v.len(),
                                "Parsed via direct array extraction from key position"
                            );
                            v
                        } else {
                            tracing::warn!(
                                wrapped_err = %e1,
                                array_err = %e2,
                                raw_len = raw.len(),
                                raw_tail = %super::truncate_tail(raw, 200),
                                "Argument extraction: JSON parse failed all formats"
                            );
                            return vec![];
                        }
                    }
                }
            }
        },
    };

    if raw_extractions.is_empty() {
        tracing::warn!("Argument extraction: parsed OK but extractions array is empty");
        return vec![];
    }

    raw_extractions
        .into_iter()
        .filter_map(|ext| {
            // Fuzzy match speaker name
            let llm_speaker = loose_text(&ext.speaker).unwrap_or_default();
            let speaker = match match_speaker_name(&llm_speaker, known_speakers) {
                Some(s) => s,
                None => {
                    tracing::warn!(
                        llm_speaker = %llm_speaker,
                        known = ?known_speakers,
                        "Argument extraction: speaker name not matched, skipping"
                    );
                    return None;
                }
            };

            let new_theses: Vec<String> = loose_list(&ext.new_theses)
                .iter()
                .filter_map(loose_text)
                .map(|t| super::truncate_at_word_boundary(&t, constants::ARGMAP_MAX_THESIS_LABEL))
                .filter(|t| !t.is_empty() && is_valid_thesis_label(t))
                .collect();

            let arguments: Vec<ParsedArgument> = loose_list(&ext.arguments)
                .into_iter()
                .filter_map(loose_argument)
                .filter_map(|a| {
                    let text = loose_text(&a.text)?;
                    let for_thesis = loose_text(&a.for_thesis);
                    let against_thesis = loose_text(&a.against_thesis);
                    let kind = loose_text(&a.arg_type).unwrap_or_default().to_lowercase();
                    let arg_type = match kind.as_str() {
                        "counter" | "contre" | "counterargument" | "counter-argument"
                        | "opposition" | "réfutation" | "refutation" | "objection" => ArgumentType::Counter,
                        "evidence" | "preuve" | "proof" | "données" | "data" | "source" => {
                            ArgumentType::Evidence
                        }
                        "support" | "soutien" | "pour" | "supporting" => ArgumentType::Support,
                        // Missing or unknown kind: only a thesis to argue against makes it a counter
                        _ if against_thesis.is_some() && for_thesis.is_none() => ArgumentType::Counter,
                        _ => ArgumentType::Support,
                    };
                    Some(ParsedArgument {
                        text: super::truncate_at_word_boundary(&text, constants::ARGMAP_MAX_ARGUMENT_LABEL),
                        arg_type,
                        for_thesis,
                        against_thesis,
                        targets_argument: loose_text(&a.targets_argument),
                    })
                })
                .collect();

            if new_theses.is_empty() && arguments.is_empty() {
                return None;
            }

            Some(ParsedArgumentExtraction {
                speaker_name: speaker.clone(),
                new_theses,
                arguments,
            })
        })
        .collect()
}

/// Extract document content from `<document>...</document>` tags in a response.
/// Returns (text_without_tags, Option<document_content>).
/// If no valid tags are found, returns the original text unchanged.
pub fn extract_and_strip_document(raw: &str) -> (String, Option<String>) {
    let open = "<document>";
    let close = "</document>";
    if let Some(start) = raw.find(open) {
        if let Some(end) = raw.find(close) {
            if end > start {
                let doc = raw[start + open.len()..end].trim().to_string();
                let before = raw[..start].trim_end();
                let after = raw[end + close.len()..].trim_start();
                let stripped = if after.is_empty() {
                    before.to_string()
                } else {
                    format!("{}\n{}", before, after)
                };
                if !doc.is_empty() {
                    return (stripped.trim().to_string(), Some(doc));
                }
            }
        }
    }
    (raw.to_string(), None)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// v1.20.3 — the room's question: string or object, bounded, refused when too short.
    #[test]
    fn audience_question_is_bounded_and_needs_substance() {
        assert_eq!(parse_audience_question(r#"{"question": " Sur quelles données fondez-vous ce chiffre ? "}"#).as_deref(), Some("Sur quelles données fondez-vous ce chiffre ?"));
        assert_eq!(parse_audience_question(r#"{"question": {"text": "Et le coût social ?"}}"#).as_deref(), Some("Et le coût social ?"));
        assert!(parse_audience_question(r#"{"question": "Oui ?"}"#).is_none(), "too short to be a question");
        assert!(parse_audience_question("pas du json").is_none());
        let long = format!(r#"{{"question": "{}"}}"#, "mot ".repeat(200));
        assert!(parse_audience_question(&long).unwrap().chars().count() <= constants::AUDIENCE_QUESTION_MAX_CHARS + 1);
    }

    /// v1.20.1 — the shapes mistral-small3.1 produced on the bench: null and object
    /// theses, a single argument object instead of an array, a null type, trailing
    /// spaces in names. Nothing usable may be lost to a strict schema.
    #[test]
    fn argument_extraction_tolerates_loose_shapes() {
        let known = vec!["Le Scientifique".to_string(), "Le Philosophe".to_string(), "L'Avocat du Diable".to_string()];
        let raw = r#"{
  "extractions":
    [
      {
        "speaker":"Le Scientifique ",
        "new_theses":[null],
          "arguments":{
            "text":"Les prédictions sur le remplacement des développeurs par IA sont souvent déconnectées du réel.",
              "type": "support",
                "for_thesis":"L'IA ne remplacera pas tous les développeurs d'ici dix ans"
        }
      },
      {
        "speaker":"Le Philosophe  ",   "new_theses":[
          {"text":"Les humains superviseront l'activité des IA."}],
           "arguments":
            [{"text":"La réduction du rôle humain à la supervision est une conséquence logique de cette prévision.",
              "type": null,
               "for_thesis": null,
                  "against_thesis":"L'IA remplace les développeurs d'ici dix ans"
                 },
             "Un argument nu, sans type"]
      },
      {
        "speaker":"Quelqu'un d'inconnu", "new_theses":["Ignorée"], "arguments":[]
      }
    ]
}"#;
        let parsed = parse_argument_extraction(raw, &known);
        assert_eq!(parsed.len(), 2, "{parsed:?}");
        assert_eq!(parsed[0].speaker_name, "Le Scientifique");
        assert!(parsed[0].new_theses.is_empty(), "a null thesis is nothing");
        assert_eq!(parsed[0].arguments.len(), 1, "a single object is one argument");
        assert_eq!(parsed[0].arguments[0].arg_type, ArgumentType::Support);
        assert_eq!(parsed[0].arguments[0].for_thesis.as_deref(), Some("L'IA ne remplacera pas tous les développeurs d'ici dix ans"));
        assert_eq!(parsed[1].speaker_name, "Le Philosophe");
        assert_eq!(parsed[1].new_theses, vec!["Les humains superviseront l'activité des IA."]);
        assert_eq!(parsed[1].arguments.len(), 2);
        assert_eq!(parsed[1].arguments[0].arg_type, ArgumentType::Counter, "no type but a thesis to argue against");
        assert_eq!(parsed[1].arguments[1].text, "Un argument nu, sans type");
        assert_eq!(parsed[1].arguments[1].arg_type, ArgumentType::Support);
    }
    use crate::models::moderation::ModerationAction;

    #[test]
    fn test_parse_direct_json() {
        let raw = r#"{"action":"none","comment":"","ban_reason":"","ban_duration":0}"#;
        let result: ModerationResult = parse_json_response(raw).unwrap();
        assert_eq!(result.action, ModerationAction::None);
    }

    #[test]
    fn test_parse_markdown_json() {
        let raw = "Here is my response:\n```json\n{\"action\":\"comment\",\"comment\":\"Good point\",\"ban_reason\":\"\",\"ban_duration\":0}\n```\n";
        let result: ModerationResult = parse_json_response(raw).unwrap();
        assert_eq!(result.action, ModerationAction::Comment);
        assert_eq!(result.comment, "Good point");
    }

    #[test]
    fn test_parse_bare_markdown_block() {
        let raw = "Here is my response:\n```\n{\"action\":\"comment\",\"comment\":\"Good\",\"ban_reason\":\"\",\"ban_duration\":0}\n```\n";
        let result: ModerationResult = parse_json_response(raw).unwrap();
        assert_eq!(result.action, ModerationAction::Comment);
    }

    #[test]
    fn test_parse_embedded_json() {
        let raw = "I think the moderation should be: {\"action\":\"ban\",\"comment\":\"\",\"ban_reason\":\"Off topic\",\"ban_duration\":2} because reasons.";
        let result: ModerationResult = parse_json_response(raw).unwrap();
        assert_eq!(result.action, ModerationAction::Ban);
        assert_eq!(result.ban_duration, 2);
    }

    #[test]
    fn test_parse_trailing_comma() {
        let raw = r#"{"action":"none","comment":"","ban_reason":"","ban_duration":0,}"#;
        let result: ModerationResult = parse_json_response(raw).unwrap();
        assert_eq!(result.action, ModerationAction::None);
    }

    #[test]
    fn test_parse_moderation_fallback() {
        assert!(parse_moderation("This is not JSON at all").is_err(), "the caller falls back to none and counts it");
        assert_eq!(parse_moderation(r#"{"action":"none"}"#).unwrap().action, ModerationAction::None);
    }

    #[test]
    fn test_parse_reactions_valid() {
        let raw = r#"[{"speaker":"Alice","reaction":"like"},{"speaker":"Bob","reaction":"dislike"}]"#;
        let known = vec!["Alice".to_string(), "Bob".to_string()];
        let reactions = parse_reactions(raw, &known);
        assert_eq!(reactions.len(), 2);
        assert_eq!(reactions[0].reaction_type, ReactionType::Like);
        assert_eq!(reactions[1].reaction_type, ReactionType::Dislike);
    }

    #[test]
    fn test_parse_reactions_filters_none() {
        let raw = r#"[{"speaker":"Alice","reaction":"none"},{"speaker":"Bob","reaction":"like"}]"#;
        let known = vec!["Alice".to_string(), "Bob".to_string()];
        let reactions = parse_reactions(raw, &known);
        assert_eq!(reactions.len(), 1);
        assert_eq!(reactions[0].speaker_name, "Bob");
    }

    #[test]
    fn test_parse_reactions_case_insensitive() {
        let raw = r#"[{"speaker":"alice","reaction":"LIKE"}]"#;
        let known = vec!["Alice".to_string()];
        let reactions = parse_reactions(raw, &known);
        assert_eq!(reactions.len(), 1);
    }

    #[test]
    fn test_parse_reactions_invalid_json() {
        let reactions = parse_reactions("not json", &["Alice".to_string()]);
        assert!(reactions.is_empty());
    }

    #[test]
    fn test_parse_reactions_rejects_partial_match() {
        let raw = r#"[{"speaker":"Ali","reaction":"like"}]"#;
        let known = vec!["Alice".to_string(), "Malik".to_string()];
        let reactions = parse_reactions(raw, &known);
        // "Ali" (3 chars) starts_with match on "Alice" only
        assert_eq!(reactions.len(), 1);
        assert_eq!(reactions[0].speaker_name, "Alice");
    }

    #[test]
    fn test_parse_reactions_rejects_too_short() {
        let raw = r#"[{"speaker":"a","reaction":"like"}]"#;
        let known = vec!["Alice".to_string()];
        let reactions = parse_reactions(raw, &known);
        // "a" is < 3 chars, no fallback
        assert!(reactions.is_empty());
    }

    #[test]
    fn test_parse_reactions_single_object() {
        let raw = r#"{"speaker":"Alice","reaction":"like"}"#;
        let known = vec!["Alice".to_string(), "Bob".to_string()];
        let reactions = parse_reactions(raw, &known);
        assert_eq!(reactions.len(), 1);
        assert_eq!(reactions[0].speaker_name, "Alice");
        assert_eq!(reactions[0].reaction_type, ReactionType::Like);
    }

    #[test]
    fn test_parse_reactions_wrapped_reactions_key() {
        let raw = r#"{"reactions":[{"speaker":"Alice","reaction":"like"},{"speaker":"Bob","reaction":"dislike"}]}"#;
        let known = vec!["Alice".to_string(), "Bob".to_string()];
        let reactions = parse_reactions(raw, &known);
        assert_eq!(reactions.len(), 2);
        assert_eq!(reactions[0].speaker_name, "Alice");
        assert_eq!(reactions[0].reaction_type, ReactionType::Like);
        assert_eq!(reactions[1].speaker_name, "Bob");
        assert_eq!(reactions[1].reaction_type, ReactionType::Dislike);
    }

    #[test]
    fn test_parse_reactions_wrapped_responses_key() {
        let raw = r#"{"responses":[{"speaker":"Alice","reaction":"like"}]}"#;
        let known = vec!["Alice".to_string()];
        let reactions = parse_reactions(raw, &known);
        assert_eq!(reactions.len(), 1);
        assert_eq!(reactions[0].speaker_name, "Alice");
    }

    #[test]
    fn test_parse_reactions_wrapped_interventions_key() {
        let raw = r#"{"interventions":[{"speaker":"Alice","reaction":"dislike"},{"speaker":"Bob","reaction":"like"}]}"#;
        let known = vec!["Alice".to_string(), "Bob".to_string()];
        let reactions = parse_reactions(raw, &known);
        assert_eq!(reactions.len(), 2);
        assert_eq!(reactions[0].speaker_name, "Alice");
        assert_eq!(reactions[0].reaction_type, ReactionType::Dislike);
        assert_eq!(reactions[1].speaker_name, "Bob");
        assert_eq!(reactions[1].reaction_type, ReactionType::Like);
    }

    #[test]
    fn test_parse_reactions_duplicate_key_flat_object() {
        let raw = r#"{"speaker":"Alice","reaction":"like","speaker":"Bob","reaction":"dislike"}"#;
        let known = vec!["Alice".to_string(), "Bob".to_string()];
        let reactions = parse_reactions(raw, &known);
        assert_eq!(reactions.len(), 2);
        assert_eq!(reactions[0].speaker_name, "Alice");
        assert_eq!(reactions[0].reaction_type, ReactionType::Like);
        assert_eq!(reactions[1].speaker_name, "Bob");
        assert_eq!(reactions[1].reaction_type, ReactionType::Dislike);
    }

    #[test]
    fn test_parse_reactions_duplicate_key_with_none() {
        let raw = r#"{"speaker":"Alice","reaction":"like","speaker":"Bob","reaction":"none","speaker":"Carol","reaction":"dislike"}"#;
        let known = vec!["Alice".to_string(), "Bob".to_string(), "Carol".to_string()];
        let reactions = parse_reactions(raw, &known);
        // "none" is filtered by validate_reactions
        assert_eq!(reactions.len(), 2);
        assert_eq!(reactions[0].speaker_name, "Alice");
        assert_eq!(reactions[1].speaker_name, "Carol");
    }

    #[test]
    fn test_parse_reactions_duplicate_key_multibyte_utf8() {
        // Regression test: multi-byte chars like 'é' caused panics in extract_json_string_value
        // because char indices were used as byte indices.
        let raw = r#"{"speaker":"La Singularité","reaction":"like","speaker":"Satan","reaction":"dislike","speaker":"Le Pragmatique","reaction":"like"}"#;
        let known = vec![
            "La Singularité".to_string(),
            "Satan".to_string(),
            "Le Pragmatique".to_string(),
        ];
        let reactions = parse_reactions(raw, &known);
        assert_eq!(reactions.len(), 3);
        assert_eq!(reactions[0].speaker_name, "La Singularité");
        assert_eq!(reactions[0].reaction_type, ReactionType::Like);
        assert_eq!(reactions[1].speaker_name, "Satan");
        assert_eq!(reactions[1].reaction_type, ReactionType::Dislike);
        assert_eq!(reactions[2].speaker_name, "Le Pragmatique");
        assert_eq!(reactions[2].reaction_type, ReactionType::Like);
    }

    #[test]
    fn test_parse_reactions_deduplicates_same_speaker() {
        // LLM outputs duplicate reactions for the same speaker — only the first should be kept
        let raw = r#"[{"speaker":"Alice","reaction":"like"},{"speaker":"Alice","reaction":"dislike"},{"speaker":"Bob","reaction":"like"}]"#;
        let known = vec!["Alice".to_string(), "Bob".to_string()];
        let reactions = parse_reactions(raw, &known);
        assert_eq!(reactions.len(), 2, "Expected 2 reactions (dedup Alice), got {}", reactions.len());
        assert_eq!(reactions[0].speaker_name, "Alice");
        assert_eq!(reactions[0].reaction_type, ReactionType::Like); // first wins
        assert_eq!(reactions[1].speaker_name, "Bob");
    }

    #[test]
    fn test_parse_reactions_none_then_valid_same_speaker() {
        // "none" reaction should NOT consume the dedup slot — a subsequent valid reaction should pass
        let raw = r#"[{"speaker":"Alice","reaction":"none"},{"speaker":"Alice","reaction":"like"},{"speaker":"Bob","reaction":"dislike"}]"#;
        let known = vec!["Alice".to_string(), "Bob".to_string()];
        let reactions = parse_reactions(raw, &known);
        assert_eq!(reactions.len(), 2, "Expected 2 reactions (none filtered, like kept), got {}", reactions.len());
        assert_eq!(reactions[0].speaker_name, "Alice");
        assert_eq!(reactions[0].reaction_type, ReactionType::Like);
        assert_eq!(reactions[1].speaker_name, "Bob");
    }

    #[test]
    fn test_fix_common_json_preserves_apostrophes() {
        let input = r#"{'action':'none','comment':'That's a good point','ban_reason':'','ban_duration':0}"#;
        let fixed = fix_common_json_issues(input);
        // The apostrophe in "That's" should NOT become a double quote
        assert!(fixed.contains("That's"));
        // But JSON delimiters should be double-quoted
        let result: Result<ModerationResult, _> = serde_json::from_str(&fixed);
        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_array_before_object() {
        let raw = r#"Here is my answer: [{"speaker":"Alice","reaction":"like"}]"#;
        let extracted = extract_first_json_object(raw);
        assert!(extracted.is_some());
        let s = extracted.unwrap();
        assert!(s.starts_with('['));
    }

    #[test]
    fn test_parse_moderation_missing_fields() {
        // Missing ban_reason and ban_duration — serde(default) should fill them
        let raw = r#"{"action":"comment","comment":"Good point"}"#;
        let result: ModerationResult = parse_json_response(raw).unwrap();
        assert_eq!(result.action, ModerationAction::Comment);
        assert_eq!(result.ban_reason, "");
        assert_eq!(result.ban_duration, 0);
    }

    /// v1.20.5 — "reacts": false is a silence whatever the colour written next to it.
    #[test]
    fn declined_reactions_are_silences() {
        let known = vec!["Alice".to_string(), "Bob".to_string(), "Carol".to_string()];
        let raw = r#"[{"speaker":"Alice","reacts":false,"reaction":"like","justification":"x"},{"speaker":"Bob","reacts":"non","reaction":"insightful"},{"speaker":"Carol","reacts":true,"reaction":"question","justification":"?"}]"#;
        let parsed = parse_reactions(raw, &known);
        assert_eq!(parsed.len(), 1, "{parsed:?}");
        assert_eq!((parsed[0].speaker_name.as_str(), parsed[0].reaction_type), ("Carol", ReactionType::Question));
        // Absent: the reaction counts as before
        let legacy = parse_reactions(r#"[{"speaker":"Alice","reaction":"like"}]"#, &known);
        assert_eq!(legacy.len(), 1);
    }

    /// v1.20.5 — "Créatif, tu…" addresses Le Créatif: the article is optional.
    #[test]
    fn mentions_name_tolerates_the_missing_article() {
        assert!(mentions_name("Créatif, tu nous emmènes loin !", "Le Créatif"));
        assert!(mentions_name("LE PRAGMATIQUE se trompe.", "Le Pragmatique"));
        assert!(mentions_name("Expert IA, chiffres ?", "L'Expert IA"));
        assert!(mentions_name("Scientist, your data?", "The Scientist"));
        assert!(!mentions_name("Personne n'est nommé.", "Le Créatif"));
        assert!(!mentions_name("un x ici", "Le X"), "too short once the article is gone");
        assert!(!mentions_name("quelque chose", ""));
    }

    #[test]
    fn test_parse_reactions_french_article_stripped() {
        // LLM outputs "Scientifique" but known name is "Le Scientifique"
        let raw = r#"[{"speaker":"Scientifique","reaction":"like"}]"#;
        let known = vec!["Le Scientifique".to_string(), "L'Avocat du Diable".to_string()];
        let reactions = parse_reactions(raw, &known);
        assert_eq!(reactions.len(), 1);
        assert_eq!(reactions[0].speaker_name, "Le Scientifique");
        assert_eq!(reactions[0].reaction_type, ReactionType::Like);
    }

    #[test]
    fn test_parse_reactions_french_apostrophe_article() {
        // LLM outputs "Avocat du Diable" without "L'"
        let raw = r#"[{"speaker":"Avocat du Diable","reaction":"dislike"}]"#;
        let known = vec!["Le Scientifique".to_string(), "L'Avocat du Diable".to_string()];
        let reactions = parse_reactions(raw, &known);
        assert_eq!(reactions.len(), 1);
        assert_eq!(reactions[0].speaker_name, "L'Avocat du Diable");
        assert_eq!(reactions[0].reaction_type, ReactionType::Dislike);
    }

    #[test]
    fn test_parse_reactions_full_french_names() {
        // LLM outputs exact full names — should still work
        let raw = r#"[{"speaker":"Le Scientifique","reaction":"like"},{"speaker":"L'Avocat du Diable","reaction":"dislike"}]"#;
        let known = vec!["Le Scientifique".to_string(), "L'Avocat du Diable".to_string()];
        let reactions = parse_reactions(raw, &known);
        assert_eq!(reactions.len(), 2);
        assert_eq!(reactions[0].speaker_name, "Le Scientifique");
        assert_eq!(reactions[1].speaker_name, "L'Avocat du Diable");
    }

    #[test]
    fn test_parse_reactions_alias_name_field() {
        // LLM uses "name" instead of "speaker"
        let raw = r#"[{"name":"Alice","reaction":"like"}]"#;
        let known = vec!["Alice".to_string()];
        let reactions = parse_reactions(raw, &known);
        assert_eq!(reactions.len(), 1);
        assert_eq!(reactions[0].speaker_name, "Alice");
    }

    #[test]
    fn test_parse_reactions_alias_opinion_field() {
        // LLM uses "opinion" instead of "reaction"
        let raw = r#"[{"speaker":"Alice","opinion":"like"}]"#;
        let known = vec!["Alice".to_string()];
        let reactions = parse_reactions(raw, &known);
        assert_eq!(reactions.len(), 1);
        assert_eq!(reactions[0].reaction_type, ReactionType::Like);
    }

    #[test]
    fn test_parse_reactions_french_reaction_values() {
        // LLM uses French reaction values
        let raw = r#"[{"speaker":"Alice","reaction":"positif"},{"speaker":"Bob","reaction":"négatif"}]"#;
        let known = vec!["Alice".to_string(), "Bob".to_string()];
        let reactions = parse_reactions(raw, &known);
        assert_eq!(reactions.len(), 2);
        assert_eq!(reactions[0].reaction_type, ReactionType::Like);
        assert_eq!(reactions[1].reaction_type, ReactionType::Dislike);
    }

    #[test]
    fn test_strip_french_article() {
        assert_eq!(strip_french_article("Le Scientifique"), "Scientifique");
        assert_eq!(strip_french_article("La Féministe"), "Féministe");
        assert_eq!(strip_french_article("L'Avocat du Diable"), "Avocat du Diable");
        assert_eq!(strip_french_article("Les Experts"), "Experts");
        assert_eq!(strip_french_article("Dieu"), "Dieu"); // no article
        assert_eq!(strip_french_article("Satan"), "Satan"); // no article
    }

    #[test]
    fn test_match_speaker_name_exact() {
        let known = vec!["Alice".to_string(), "Bob".to_string()];
        assert_eq!(match_speaker_name("Alice", &known), Some(&known[0]));
        assert_eq!(match_speaker_name("alice", &known), Some(&known[0]));
        assert_eq!(match_speaker_name("  Bob  ", &known), Some(&known[1]));
    }

    #[test]
    fn test_match_speaker_name_article_stripped() {
        let known = vec!["Le Scientifique".to_string(), "L'Avocat du Diable".to_string()];
        assert_eq!(match_speaker_name("Scientifique", &known), Some(&known[0]));
        assert_eq!(match_speaker_name("Avocat du Diable", &known), Some(&known[1]));
    }

    #[test]
    fn test_match_speaker_name_prefix() {
        let known = vec!["Le Scientifique".to_string()];
        assert_eq!(match_speaker_name("Le Sci", &known), Some(&known[0]));
    }

    #[test]
    fn test_match_speaker_name_contains() {
        let known = vec!["L'Avocat du Diable".to_string()];
        assert_eq!(match_speaker_name("Avocat du Diable", &known), Some(&known[0]));
    }

    #[test]
    fn test_match_speaker_name_no_match() {
        let known = vec!["Alice".to_string()];
        assert_eq!(match_speaker_name("Charlie", &known), None);
        assert_eq!(match_speaker_name("a", &known), None); // too short
    }

    #[test]
    fn test_match_speaker_name_unicode_hyphen() {
        // LLM outputs U+2011 (non-breaking hyphen), seed has ASCII U+002D
        let known = vec!["Le Psycho-rigide".to_string()];
        assert_eq!(
            match_speaker_name("Le Psycho\u{2011}rigide", &known),
            Some(&known[0])
        );
        // Also test reverse: seed has unicode, LLM has ASCII
        let known2 = vec!["Le Psycho\u{2011}rigide".to_string()];
        assert_eq!(
            match_speaker_name("Le Psycho-rigide", &known2),
            Some(&known2[0])
        );
    }

    #[test]
    fn test_normalize_punctuation() {
        // Dashes
        assert_eq!(normalize_punctuation("Psycho\u{2011}rigide"), "Psycho-rigide");
        assert_eq!(normalize_punctuation("en\u{2013}dash"), "en-dash");
        assert_eq!(normalize_punctuation("no dashes"), "no dashes");
        // Apostrophes
        assert_eq!(normalize_punctuation("L\u{2019}Adolescent"), "L'Adolescent");
        assert_eq!(normalize_punctuation("L\u{2018}Humoriste"), "L'Humoriste");
        assert_eq!(normalize_punctuation("L\u{02BC}Artiste"), "L'Artiste");
    }

    #[test]
    fn test_match_speaker_typographic_apostrophe() {
        // LLM outputs typographic apostrophe, seed has straight apostrophe
        let known = vec!["L'Adolescent".to_string()];
        assert_eq!(
            match_speaker_name("L\u{2019}Adolescent", &known),
            Some(&known[0])
        );
        // Reverse: seed has typographic, LLM has straight
        let known2 = vec!["L\u{2019}Humoriste".to_string()];
        assert_eq!(
            match_speaker_name("L'Humoriste", &known2),
            Some(&known2[0])
        );
    }

    #[test]
    fn test_parse_vote_response() {
        let raw = r#"{"ranking":["Alice","Bob","Charlie"]}"#;
        let result = parse_vote(raw);
        assert_eq!(result, vec!["Alice", "Bob", "Charlie"]);
    }

    #[test]
    fn test_parse_vote_bare_array() {
        let raw = r#"["Alice","Bob"]"#;
        let result = parse_vote(raw);
        assert_eq!(result, vec!["Alice", "Bob"]);
    }

    #[test]
    fn test_parse_vote_invalid() {
        let result = parse_vote("not json at all");
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_vote_markdown_wrapped() {
        let raw = "Here is my ranking:\n```json\n{\"ranking\":[\"Bob\",\"Alice\"]}\n```\n";
        let result = parse_vote(raw);
        assert_eq!(result, vec!["Bob", "Alice"]);
    }

    #[test]
    fn test_parse_authoritarian_order() {
        let raw = r#"{"order":["Charlie","Alice","Bob"]}"#;
        let result = parse_authoritarian_order(raw);
        assert_eq!(result, vec!["Charlie", "Alice", "Bob"]);
    }

    #[test]
    fn test_parse_authoritarian_order_bare_array() {
        let raw = r#"["Charlie","Alice"]"#;
        let result = parse_authoritarian_order(raw);
        assert_eq!(result, vec!["Charlie", "Alice"]);
    }

    #[test]
    fn test_parse_authoritarian_order_invalid() {
        let result = parse_authoritarian_order("garbage");
        assert!(result.is_empty());
    }

    // ── SearchDecisionResponse tests ──

    #[test]
    fn test_parse_search_decision_valid() {
        let raw = r#"{"needs_search": true, "queries": ["climate change 2026", "CO2 levels"]}"#;
        let decision: SearchDecisionResponse = parse_json_response(raw).unwrap();
        assert!(decision.needs_search);
        assert_eq!(decision.queries.len(), 2);
        assert_eq!(decision.queries[0], "climate change 2026");
    }

    #[test]
    fn test_parse_search_decision_no_search() {
        let raw = r#"{"needs_search": false, "queries": []}"#;
        let decision: SearchDecisionResponse = parse_json_response(raw).unwrap();
        assert!(!decision.needs_search);
        assert!(decision.queries.is_empty());
    }

    #[test]
    fn test_parse_search_decision_missing_fields() {
        // Missing queries field — serde(default) should provide empty Vec
        let raw = r#"{"needs_search": true}"#;
        let decision: SearchDecisionResponse = parse_json_response(raw).unwrap();
        assert!(decision.needs_search);
        assert!(decision.queries.is_empty());
    }

    #[test]
    fn test_parse_search_decision_garbage_defaults() {
        let raw = "This is not JSON at all";
        let decision: SearchDecisionResponse =
            parse_json_response(raw).unwrap_or_default();
        assert!(!decision.needs_search);
        assert!(decision.queries.is_empty());
    }

    #[test]
    fn test_parse_search_decision_markdown_wrapped() {
        let raw = "Here:\n```json\n{\"needs_search\": true, \"queries\": [\"test\"]}\n```\n";
        let decision: SearchDecisionResponse = parse_json_response(raw).unwrap();
        assert!(decision.needs_search);
        assert_eq!(decision.queries, vec!["test"]);
    }

    // ── extract_and_strip_document tests ──────────────────────────

    #[test]
    fn test_extract_document_present() {
        let raw = "Here is my analysis.\n\n<document>\n# Title\nContent here\n</document>";
        let (text, doc) = extract_and_strip_document(raw);
        assert_eq!(text, "Here is my analysis.");
        assert_eq!(doc.unwrap(), "# Title\nContent here");
    }

    #[test]
    fn test_extract_document_absent() {
        let raw = "Just a regular response with no document tags.";
        let (text, doc) = extract_and_strip_document(raw);
        assert_eq!(text, raw);
        assert!(doc.is_none());
    }

    #[test]
    fn test_extract_document_empty() {
        let raw = "Text before\n<document>\n</document>\nText after";
        let (text, doc) = extract_and_strip_document(raw);
        assert_eq!(text, raw);
        assert!(doc.is_none());
    }

    #[test]
    fn test_extract_document_open_only() {
        let raw = "Text <document> some content without closing";
        let (text, doc) = extract_and_strip_document(raw);
        assert_eq!(text, raw);
        assert!(doc.is_none());
    }

    #[test]
    fn test_extract_document_multiline_content() {
        let raw = "Discussion text.\n\n<document>\nLine 1\nLine 2\nLine 3\n</document>\n";
        let (text, doc) = extract_and_strip_document(raw);
        assert_eq!(text, "Discussion text.");
        assert_eq!(doc.unwrap(), "Line 1\nLine 2\nLine 3");
    }

    #[test]
    fn test_extract_document_with_text_after() {
        let raw = "Before\n<document>Content</document>\nAfter";
        let (text, doc) = extract_and_strip_document(raw);
        assert_eq!(text, "Before\nAfter");
        assert_eq!(doc.unwrap(), "Content");
    }

    // ── Thesis label validation ─────────────────────────────────────

    #[test]
    fn test_valid_thesis_label_accepts_normal() {
        assert!(is_valid_thesis_label("L'IA crée plus d'emplois qu'elle n'en détruit"));
    }

    #[test]
    fn test_valid_thesis_label_rejects_numeric() {
        assert!(!is_valid_thesis_label("4"));
        assert!(!is_valid_thesis_label("12"));
        assert!(!is_valid_thesis_label(" 9 "));
    }

    #[test]
    fn test_valid_thesis_label_rejects_short() {
        assert!(!is_valid_thesis_label("AI"));
        assert!(!is_valid_thesis_label("ok"));
        assert!(!is_valid_thesis_label("thesis"));
    }

    #[test]
    fn test_valid_thesis_label_accepts_min_length() {
        // Exactly at the minimum (8 chars)
        assert!(is_valid_thesis_label("AI helps"));
    }

    // ── Argument extraction with numeric filtering ──────────────────

    #[test]
    fn test_arg_extraction_filters_numeric_theses() {
        let raw = r#"{"extractions": [{"speaker": "Alice", "new_theses": ["4", "L'IA est bénéfique pour l'emploi", "9"], "arguments": []}]}"#;
        let result = parse_argument_extraction(raw, &["Alice".to_string()]);
        assert_eq!(result.len(), 1);
        // Only the real thesis should remain — "4" and "9" are filtered out
        assert_eq!(result[0].new_theses.len(), 1);
        assert_eq!(result[0].new_theses[0], "L'IA est bénéfique pour l'emploi");
    }

    #[test]
    fn test_arg_extraction_all_numeric_theses_skips_speaker() {
        let raw = r#"{"extractions": [{"speaker": "Bob", "new_theses": ["4", "9"], "arguments": []}]}"#;
        let result = parse_argument_extraction(raw, &["Bob".to_string()]);
        // All theses were numeric → extraction is empty → speaker is skipped
        assert!(result.is_empty());
    }

    // ── Intention (v1.17) ───────────────────────────────────────────────

    fn names() -> Vec<String> {
        vec!["Le Scientifique".to_string(), "La Juriste".to_string()]
    }

    #[test]
    fn intention_is_parsed_with_fuzzy_target_and_bounded_fields() {
        let raw = r#"```json
{"target": "Scientifique", "goal": "Contester", "angle": "les chiffres sont partiels", "concession": "null", "question": "Quelle source ?", "answers": "2", "thought": "Je dois le pousser sur ses sources."}
```"#;
        let i = parse_intention(raw, &names()).unwrap();
        assert_eq!(i.target.as_deref(), Some("Le Scientifique"));
        assert_eq!(i.goal, IntentionGoal::Contest);
        assert_eq!(i.angle, "les chiffres sont partiels");
        assert_eq!(i.concession, None, "\"null\" string is nothing");
        assert_eq!(i.question.as_deref(), Some("Quelle source ?"));
        assert_eq!(i.answers, Some(2));
        assert_eq!(i.thought, "Je dois le pousser sur ses sources.");

        // French keys, topic target, numeric answers, long angle
        let long = "a".repeat(constants::INTENTION_DISPLAY_MAX_CHARS + 20);
        let raw = format!(r#"{{"cible": "sujet", "objectif": "nuancer", "angle": "{long}", "repond_a": 0, "pensee": " ok "}}"#);
        let i = parse_intention(&raw, &names()).unwrap();
        assert_eq!(i.target, None);
        assert_eq!(i.goal, IntentionGoal::Nuance);
        assert_eq!(i.angle.len(), constants::INTENTION_DISPLAY_MAX_CHARS, "kept in full for the backstage, bounded for the prompt by the block builder");
        assert_eq!(i.answers, None, "0 is not a loop index");
        assert_eq!(i.thought, "ok");

        // Unknown name is kept for the engine to resolve; unknown goal → relaunch
        let i = parse_intention(r#"{"target": "Quelqu'un", "goal": "danser"}"#, &names()).unwrap();
        assert_eq!(i.target.as_deref(), Some("Quelqu'un"));
        assert_eq!(i.goal, IntentionGoal::Relaunch);
        assert!(i.angle.is_empty() && i.thought.is_empty());
    }

    #[test]
    fn intention_free_text_is_not_json() {
        assert!(parse_intention("Je pense que les données comptent.", &names()).is_none());
        assert!(parse_intention("", &names()).is_none());
    }

    // ── Agenda and casting (v1.19) ──────────────────────────────────────

    #[test]
    fn agenda_is_parsed_bounded_and_empty_ones_are_dropped() {
        let long = "x".repeat(constants::AGENDA_FIELD_MAX_CHARS + 10);
        let a = parse_agenda(&format!(r#"{{"objectif": "faire admettre le coût", "ligne_rouge": " ", "victory": "{long}"}}"#)).unwrap();
        assert_eq!(a.objective, "faire admettre le coût");
        assert_eq!(a.red_line, "");
        assert_eq!(a.victory.len(), constants::AGENDA_FIELD_MAX_CHARS);
        assert!(parse_agenda(r#"{"objective": "", "red_line": ""}"#).is_none());
        assert!(parse_agenda("pas du json").is_none());
        // Outcome read from the "## Agendas" section only
        let synthesis = "## Positions\n- **Le Philosophe** : a atteint un consensus.\n\n## Agendas secrets\n- **Le Scientifique** : objectif atteint, il a imposé les chiffres.\n- **Le Philosophe** : objectif non atteint.\n- **La Juriste** : partiellement atteint.\n- **L'Ingénieure** : on ne sait pas.\n\n## Conclusion\nL'Ingénieure a atteint son but.";
        assert_eq!(agenda_outcome(synthesis, "Le Scientifique"), Some(true));
        assert_eq!(agenda_outcome(synthesis, "Le Philosophe"), Some(false));
        assert_eq!(agenda_outcome(synthesis, "La Juriste"), None);
        assert_eq!(agenda_outcome(synthesis, "L'Ingénieure"), None);
        assert_eq!(agenda_outcome(synthesis, "Absent"), None);
        assert_eq!(agenda_outcome("## Agendas\n- **Dr. Chen**: objective achieved.\n- **Amara**: unmet.", "Amara"), Some(false));
        assert_eq!(agenda_outcome("## 议程\n- **李明**：目标达成。", "李明"), Some(true));
        assert_eq!(agenda_outcome("pas de section", "Le Scientifique"), None);
    }

    #[test]
    fn recap_is_bounded_and_names_are_matched() {
        let names = vec!["Le Scientifique".to_string(), "Le Philosophe".to_string(), "La Juriste".to_string()];
        let long = "p".repeat(constants::RECAP_ITEM_MAX_CHARS + 40);
        let raw = format!(r#"{{"positions": ["a", "", "b", "c", "d"], "best_lines": ["{long}"], "allies": ["Juriste", "Inconnu"], "rivals": ["le philosophe"], "lesson": " mesurer "}}"#);
        let r = parse_recap(&raw, &names).unwrap();
        assert_eq!(r.positions, vec!["a", "b", "c"]);
        assert!(r.best_lines[0].chars().count() <= constants::RECAP_ITEM_MAX_CHARS + 1);
        assert_eq!(r.allies, vec!["La Juriste"]);
        assert_eq!(r.rivals, vec!["Le Philosophe"]);
        assert_eq!(r.lesson, "mesurer");
        assert!(parse_recap(r#"{"positions": [], "lesson": ""}"#, &names).is_none());
        assert!(parse_recap("nope", &names).is_none());
        assert_eq!(parse_recap(r#"{"leçon": "x"}"#, &names).unwrap().lesson, "x");
    }

    #[test]
    fn verdicts_agreements_and_dispatches_are_parsed_tolerantly() {
        assert_eq!(parse_verdict(r#"{"verdict": "Accusation", "reason": "les preuves"}"#), Some((VERDICT_PROSECUTION.to_string(), "les preuves".to_string())));
        assert_eq!(parse_verdict(r#"{"choice": "the defence", "raison": ""}"#), Some((VERDICT_DEFENSE.to_string(), String::new())));
        assert_eq!(parse_verdict(r#"{"side": "控方"}"#).map(|v| v.0), Some(VERDICT_PROSECUTION.to_string()));
        assert_eq!(parse_verdict(r#"{"verdict": "not guilty"}"#).map(|v| v.0), Some(VERDICT_DEFENSE.to_string()));
        assert_eq!(parse_verdict(r#"{"verdict": "peut-être"}"#), None);
        assert_eq!(parse_verdict("nope"), None);
        let long = "r".repeat(constants::VERDICT_REASON_MAX_CHARS + 20);
        assert_eq!(parse_verdict(&format!(r#"{{"verdict": "défense", "reason": "{long}"}}"#)).unwrap().1.len(), constants::VERDICT_REASON_MAX_CHARS);

        assert_eq!(parse_agreement(r#"{"accepts": true, "reason": "bon compromis"}"#), Some((true, "bon compromis".to_string())));
        assert_eq!(parse_agreement(r#"{"accepte": "Non, pas sans garantie"}"#), Some((false, String::new())));
        assert_eq!(parse_agreement(r#"{"accept": "是"}"#).map(|a| a.0), Some(true));
        assert_eq!(parse_agreement(r#"{"accepts": "peut-être"}"#), None);
        assert_eq!(parse_agreement(r#"{"accepts": 1}"#), None);

        let long = "d".repeat(constants::CRISIS_DISPATCH_MAX_CHARS + 50);
        let d = parse_dispatches(&format!(r#"{{"dépêches": ["Première alerte.", " ", "{long} fin", "Quatre", "Cinq"]}}"#), 3);
        assert_eq!(d.len(), 3);
        assert_eq!(d[0], "Première alerte.");
        assert!(d[1].chars().count() <= constants::CRISIS_DISPATCH_MAX_CHARS + 1);
        assert!(parse_dispatches("{}", 3).is_empty() && parse_dispatches("x", 3).is_empty());
    }

    #[test]
    fn casting_keeps_known_ids_only() {
        let glads = vec!["scientist".to_string(), "philosopher".to_string(), "lawyer".to_string()];
        let arbs = vec!["arb-impartial".to_string()];
        let raw = r#"{"gladiateurs": [{"id": "scientist", "reason": "rigueur"}, {"id": "ghost", "reason": "?"}, {"id": "scientist", "reason": "doublon"}, {"id": "lawyer"}], "arbitre": "arb-impartial"}"#;
        let c = parse_casting(raw, &glads, &arbs);
        assert_eq!(c.gladiateurs.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(), vec!["scientist", "lawyer"]);
        assert_eq!(c.gladiateurs[0].reason, "rigueur");
        assert_eq!(c.arbitre.as_deref(), Some("arb-impartial"));
        let c = parse_casting(r#"{"gladiators": [{"id": "philosopher"}], "moderator": "unknown"}"#, &glads, &arbs);
        assert_eq!(c.gladiateurs.len(), 1);
        assert_eq!(c.arbitre, None);
        assert_eq!(parse_casting("nope", &glads, &arbs), CastingSuggestion::default());
    }

    // ── Turn analyst (v1.17) ────────────────────────────────────────────

    #[test]
    fn turn_analyst_splits_memory_and_emotions_and_tolerates_missing_parts() {
        let names = vec!["Le Scientifique".to_string(), "Le Philosophe".to_string()];
        let raw = r#"{"summary":"s","positions":{"Le Scientifique":"prudent"},"open_questions":[],"emotions":{"Scientifique":{"frustration":5},"stagnating":true}}"#;
        let (memory, emotions) = parse_turn_analyst(raw, &names).unwrap();
        assert_eq!(memory.summary, "s");
        assert_eq!(memory.positions.len(), 1);
        assert_eq!(emotions.deltas.get("Le Scientifique").unwrap().frustration, 5);
        assert_eq!(emotions.stagnating, Some(true), "flag inside the emotions object");
        // Memory only: emotions untouched; stagnating at the top level is honoured
        let (memory, emotions) = parse_turn_analyst(r#"{"summary":"only","stagnating":false}"#, &names).unwrap();
        assert_eq!(memory.summary, "only");
        assert!(emotions.deltas.is_empty());
        assert_eq!(emotions.stagnating, Some(false));
        // Emotions only: memory empty (caller keeps the previous summary)
        let (memory, emotions) = parse_turn_analyst(r#"{"emotions":{"Le Philosophe":{"accord":-3}}}"#, &names).unwrap();
        assert!(memory.summary.is_empty() && memory.positions.is_empty());
        assert_eq!(emotions.deltas.get("Le Philosophe").unwrap().accord, -3);
        assert!(parse_turn_analyst("pas du json", &names).is_none());
    }

    // ── Memory update (v1.17) ───────────────────────────────────────────

    #[test]
    fn memory_update_accepts_mixed_positions_and_open_questions() {
        let raw = r#"{"summary": "s", "positions": {"A": "prudent", "B": {"stance": "critique", "shift": "s'est radicalisé", "would_change_if": null}, "C": {"position": "neutre", "évolution": "aucune"}}, "open_questions": [{"to": "A", "from": "B", "question": "Et le coût ?"}, {"to": "", "question": "x"}]}"#;
        let parsed: MemoryUpdateResponse = parse_json_response(raw).unwrap();
        assert_eq!(parsed.summary, "s");
        let a = parsed.positions.get("A").cloned().unwrap().into_parts();
        assert_eq!(a, ("prudent".to_string(), None, None));
        let b = parsed.positions.get("B").cloned().unwrap().into_parts();
        assert_eq!(b, ("critique".to_string(), Some("s'est radicalisé".to_string()), None));
        let c = parsed.positions.get("C").cloned().unwrap().into_parts();
        assert_eq!(c, ("neutre".to_string(), Some("aucune".to_string()), None));
        assert_eq!(parsed.open_questions.len(), 2);
        assert_eq!(parsed.open_questions[0].to, "A");
        assert_eq!(parsed.open_questions[0].question, "Et le coût ?");

        // v1.16 payload (no open questions, string positions only) still parses
        let old: MemoryUpdateResponse = parse_json_response(r#"{"summary":"x","positions":{"A":"y"}}"#).unwrap();
        assert!(old.open_questions.is_empty());
        assert_eq!(old.positions.len(), 1);
    }
}

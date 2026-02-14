use std::collections::{HashMap, HashSet};

use serde::de::DeserializeOwned;

use crate::models::emotion::EmotionDelta;
use crate::models::message::ReactionType;
use crate::models::moderation::{ModerationResult, RawReaction};

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

/// Parse moderation result with fallback to "none"
pub fn parse_moderation(raw: &str) -> ModerationResult {
    parse_json_response(raw).unwrap_or_default()
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
                    reactions.push(RawReaction { speaker, reaction, justification });
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
pub struct ParsedReaction {
    pub speaker_name: String,
    pub reaction_type: ReactionType,
    pub justification: Option<String>,
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

/// Strip leading French articles: "Le ", "La ", "L'", "Les "
fn strip_french_article(name: &str) -> &str {
    let trimmed = name.trim();
    for prefix in &["le ", "la ", "l'", "les "] {
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
pub fn parse_emotion_deltas(
    raw: &str,
    known_speakers: &[String],
) -> HashMap<String, EmotionDelta> {
    let mut result = HashMap::new();

    // Try parsing as HashMap<String, EmotionDelta>
    let parsed: Option<HashMap<String, EmotionDelta>> = parse_json_response(raw).ok();
    if let Some(map) = parsed {
        for (llm_name, delta) in map {
            if let Some(matched) = match_speaker_name(&llm_name, known_speakers) {
                result.insert(matched.clone(), delta);
            }
        }
    }

    result
}

fn validate_reactions(raw: Vec<RawReaction>, known_speakers: &[String]) -> Vec<ParsedReaction> {
    let mut seen = HashSet::new();
    raw.into_iter()
        .filter_map(|r| {
            let speaker = match_speaker_name(&r.speaker, known_speakers)?;

            // Normalize the reaction value FIRST (before dedup)
            // so that "none" reactions don't consume a dedup slot
            let reaction_type = match r.reaction.to_lowercase().as_str() {
                "like" | "agree" | "d'accord" | "👍" | "positive" | "positif" => Some(ReactionType::Like),
                "dislike" | "disagree" | "pas d'accord" | "👎" | "negative" | "négatif" | "negatif" => Some(ReactionType::Dislike),
                _ => None,
            }?;

            // Deduplicate: keep only the first valid reaction per target speaker
            if !seen.insert(speaker.clone()) {
                return None;
            }

            let justification = {
                let trimmed = r.justification.trim();
                if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
            };

            Some(ParsedReaction {
                speaker_name: speaker.clone(),
                reaction_type,
                justification,
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

use crate::constants;
use crate::models::argument_map::ArgumentType;

/// Parsed argument ready for merging into the argument map.
#[derive(Debug)]
pub struct ParsedArgument {
    pub text: String,
    pub arg_type: ArgumentType,
    pub for_thesis: Option<String>,
    pub against_thesis: Option<String>,
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
    #[serde(default)]
    speaker: String,
    #[serde(default)]
    new_theses: Vec<String>,
    #[serde(default)]
    arguments: Vec<RawArgument>,
}

#[derive(Debug, serde::Deserialize)]
struct RawArgument {
    #[serde(default)]
    text: String,
    #[serde(default, alias = "type")]
    arg_type: String,
    #[serde(default)]
    for_thesis: Option<String>,
    #[serde(default)]
    against_thesis: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct WrappedExtractions {
    #[serde(default, alias = "results")]
    extractions: Vec<RawArgumentExtraction>,
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
            let speaker = match match_speaker_name(&ext.speaker, known_speakers) {
                Some(s) => s,
                None => {
                    tracing::warn!(
                        llm_speaker = %ext.speaker,
                        known = ?known_speakers,
                        "Argument extraction: speaker name not matched, skipping"
                    );
                    return None;
                }
            };

            let new_theses: Vec<String> = ext
                .new_theses
                .into_iter()
                .map(|t| {
                    super::truncate_str(&t, constants::ARGMAP_MAX_THESIS_LABEL).to_string()
                })
                .filter(|t| !t.is_empty())
                .collect();

            let arguments: Vec<ParsedArgument> = ext
                .arguments
                .into_iter()
                .filter(|a| !a.text.trim().is_empty())
                .map(|a| {
                    let arg_type = match a.arg_type.to_lowercase().as_str() {
                        "counter" | "contre" | "counterargument" | "counter-argument"
                        | "opposition" | "réfutation" | "refutation" => ArgumentType::Counter,
                        "evidence" | "preuve" | "proof" | "données" | "data" | "source" => {
                            ArgumentType::Evidence
                        }
                        _ => ArgumentType::Support,
                    };
                    ParsedArgument {
                        text: super::truncate_str(&a.text, constants::ARGMAP_MAX_ARGUMENT_LABEL)
                            .to_string(),
                        arg_type,
                        for_thesis: a.for_thesis.filter(|s| !s.is_empty()),
                        against_thesis: a.against_thesis.filter(|s| !s.is_empty()),
                    }
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
        let result = parse_moderation("This is not JSON at all");
        assert_eq!(result.action, ModerationAction::None);
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
}

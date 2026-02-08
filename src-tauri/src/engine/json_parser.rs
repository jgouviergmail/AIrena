use serde::de::DeserializeOwned;

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
                    reactions.push(RawReaction { speaker, reaction });
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

fn validate_reactions(raw: Vec<RawReaction>, known_speakers: &[String]) -> Vec<ParsedReaction> {
    raw.into_iter()
        .filter_map(|r| {
            let r_lower = r.speaker.to_lowercase().trim().to_string();
            let r_stripped = strip_french_article(&r_lower).to_lowercase();

            // 1. Exact match (case-insensitive, trimmed)
            let speaker = known_speakers
                .iter()
                .find(|s| s.to_lowercase().trim() == r_lower)
                // 2. Article-stripped match: "Scientifique" matches "Le Scientifique"
                .or_else(|| {
                    if r_stripped.len() >= 3 {
                        known_speakers
                            .iter()
                            .find(|s| strip_french_article(&s.to_lowercase()) == r_stripped)
                    } else {
                        None
                    }
                })
                // 3. Prefix match: known name starts with the LLM-provided string (min 3 chars)
                .or_else(|| {
                    if r_lower.len() >= 3 {
                        known_speakers
                            .iter()
                            .find(|s| s.to_lowercase().starts_with(&r_lower))
                    } else {
                        None
                    }
                })
                // 4. Contains match: LLM output contains the stripped known name or vice versa
                .or_else(|| {
                    if r_stripped.len() >= 4 {
                        known_speakers
                            .iter()
                            .find(|s| {
                                let s_stripped = strip_french_article(&s.to_lowercase()).to_lowercase();
                                s_stripped.contains(&r_stripped) || r_stripped.contains(&s_stripped)
                            })
                    } else {
                        None
                    }
                })?;

            // Normalize the reaction value
            let reaction_type = match r.reaction.to_lowercase().as_str() {
                "like" | "agree" | "d'accord" | "👍" | "positive" | "positif" => Some(ReactionType::Like),
                "dislike" | "disagree" | "pas d'accord" | "👎" | "negative" | "négatif" | "negatif" => Some(ReactionType::Dislike),
                _ => None,
            }?;

            Some(ParsedReaction {
                speaker_name: speaker.clone(),
                reaction_type,
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
}

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

/// Parse reactions with validation against known speakers
pub fn parse_reactions(raw: &str, known_speakers: &[String]) -> Vec<ParsedReaction> {
    parse_json_response::<Vec<RawReaction>>(raw)
        .map(|r| validate_reactions(r, known_speakers))
        .unwrap_or_default()
}

/// Validated reaction ready to be converted to a Reaction
pub struct ParsedReaction {
    pub speaker_name: String,
    pub reaction_type: ReactionType,
}

fn validate_reactions(raw: Vec<RawReaction>, known_speakers: &[String]) -> Vec<ParsedReaction> {
    raw.into_iter()
        .filter_map(|r| {
            let r_lower = r.speaker.to_lowercase().trim().to_string();
            // 1. Exact match (case-insensitive, trimmed)
            let speaker = known_speakers
                .iter()
                .find(|s| s.to_lowercase().trim() == r_lower)
                // 2. Fallback: known name starts with the LLM-provided string (min 3 chars)
                .or_else(|| {
                    if r_lower.len() >= 3 {
                        known_speakers
                            .iter()
                            .find(|s| s.to_lowercase().starts_with(&r_lower))
                    } else {
                        None
                    }
                })?;

            // Normalize the reaction value
            let reaction_type = match r.reaction.to_lowercase().as_str() {
                "like" | "agree" | "d'accord" => Some(ReactionType::Like),
                "dislike" | "disagree" | "pas d'accord" => Some(ReactionType::Dislike),
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

/// Safe string truncation that respects UTF-8 char boundaries
fn safe_truncate(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        s.to_string()
    } else {
        let mut end = max_bytes;
        while !s.is_char_boundary(end) && end > 0 {
            end -= 1;
        }
        s[..end].to_string()
    }
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
}

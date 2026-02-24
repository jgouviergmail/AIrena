pub mod directive_builder;
pub mod dynamics_parser;
pub mod emotion_engine;
pub mod json_parser;
pub mod memory_manager;
pub mod mode_prompts;
pub mod orchestrator;
pub mod prompt_builder;
pub mod token_budget;
pub mod turn_manager;

/// UTF-8–safe truncation: returns the longest prefix of `s` that fits within `max_chars`.
/// Uses `str::floor_char_boundary` to avoid splitting multi-byte characters.
pub(crate) fn truncate_str(s: &str, max_chars: usize) -> &str {
    &s[..s.floor_char_boundary(max_chars)]
}

/// UTF-8–safe tail truncation: returns the longest suffix of `s` that fits within `max_chars`.
/// Uses `str::ceil_char_boundary` to avoid splitting multi-byte characters.
pub(crate) fn truncate_tail(s: &str, max_chars: usize) -> &str {
    if s.len() <= max_chars {
        return s;
    }
    &s[s.ceil_char_boundary(s.len() - max_chars)..]
}

/// UTF-8–safe truncation at the last word boundary (space) before `max_bytes`.
/// If the string fits within `max_bytes`, returns it unchanged.
/// Otherwise, finds the last space before the byte limit and appends "…".
/// Falls back to char-boundary truncation if no suitable space is found.
pub(crate) fn truncate_at_word_boundary(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }
    let safe_end = s.floor_char_boundary(max_bytes);
    let prefix = &s[..safe_end];
    // Find last space — don't cut too short (keep at least 50% of limit)
    if let Some(last_space) = prefix.rfind(' ') {
        if last_space >= max_bytes / 2 {
            return format!("{}…", s[..last_space].trim_end());
        }
    }
    // No good word boundary — truncate at char boundary
    format!("{}…", prefix.trim_end())
}

/// Apply a signed i8 delta to a u8 value, clamping result to 0-100.
/// Used by both EmotionalProfile::apply_delta and emotion_engine::apply_contagion.
pub(crate) fn apply_i8_clamped(val: u8, delta: i8) -> u8 {
    (val as i16 + delta as i16).clamp(0, 100) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_tail_short_string() {
        // String shorter than limit — returned as-is
        assert_eq!(truncate_tail("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_tail_exact_length() {
        // String exactly at limit — returned as-is
        assert_eq!(truncate_tail("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_tail_ascii() {
        assert_eq!(truncate_tail("abcdefghij", 5), "fghij");
    }

    #[test]
    fn test_truncate_tail_french_multibyte() {
        // "Singularité" contains 'é' (2 bytes in UTF-8)
        let s = "La Singularité est proche";
        let tail = truncate_tail(s, 10);
        assert!(tail.len() <= 10);
        assert!(tail.is_char_boundary(0)); // valid UTF-8 start
        assert!(s.ends_with(tail));
    }

    #[test]
    fn test_truncate_tail_chinese() {
        // Chinese chars are 3 bytes each in UTF-8
        let s = "你好世界测试";
        let tail = truncate_tail(s, 6); // 6 bytes = 2 Chinese chars
        assert!(tail.len() <= 6);
        assert!(s.ends_with(tail));
    }

    #[test]
    fn test_truncate_tail_empty() {
        assert_eq!(truncate_tail("", 10), "");
    }

    #[test]
    fn test_truncate_str_consistency() {
        // truncate_str keeps the prefix, truncate_tail keeps the suffix
        let s = "abcdefghij";
        assert_eq!(truncate_str(s, 5), "abcde");
        assert_eq!(truncate_tail(s, 5), "fghij");
    }

    // ── truncate_at_word_boundary ───────────────────────────────────────

    #[test]
    fn test_word_boundary_short_string() {
        assert_eq!(truncate_at_word_boundary("hello world", 50), "hello world");
    }

    #[test]
    fn test_word_boundary_cuts_at_space() {
        // "hello world again" = 17 bytes. Limit 12 → "hello world" (11) + "…"
        assert_eq!(truncate_at_word_boundary("hello world again", 12), "hello world…");
    }

    #[test]
    fn test_word_boundary_french_accents() {
        let s = "L'IA ne remplace pas mais transforme les tâches existantes, laissant la créativité et la stratégie aux humains";
        let result = truncate_at_word_boundary(s, 80);
        assert!(result.ends_with('…'));
        // Must not cut mid-word
        let without_ellipsis = &result[..result.len() - 3]; // "…" is 3 bytes
        assert!(without_ellipsis.ends_with(|c: char| c.is_alphabetic() || c == ','));
        assert!(result.len() <= 85); // 80 + "…" (3 bytes) + minor tolerance
    }

    #[test]
    fn test_word_boundary_no_space_fallback() {
        // Single long word without spaces — falls back to char-boundary truncation
        let s = "supercalifragilisticexpialidocious";
        let result = truncate_at_word_boundary(s, 10);
        assert!(result.ends_with('…'));
        assert_eq!(&result[..10], "supercalif");
    }

    #[test]
    fn test_word_boundary_exact_fit() {
        assert_eq!(truncate_at_word_boundary("hello", 5), "hello");
    }

}

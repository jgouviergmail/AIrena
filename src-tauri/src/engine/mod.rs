pub mod cast;
pub mod directive_builder;
pub mod dynamics_parser;
pub mod emotion_engine;
pub mod argument_merge;
pub mod focus;
pub mod json_parser;
pub mod memory_manager;
pub mod mode_prompts;
pub mod orchestrator;
pub mod prompt_builder;
pub mod reactions;
pub mod open_loops;
pub mod relationships;
pub mod stage_directions;
pub mod tuning;
pub mod diagnostics;
pub mod dramaturgy;
pub mod scene_events;
pub mod mode_roles;
pub mod token_budget;
pub mod turn_manager;

#[cfg(test)]
mod bench;
#[cfg(test)]
pub mod bench_metrics;
#[cfg(test)]
mod engine_tests;
#[cfg(test)]
mod emotion_sim;

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

/// Sentence terminators, Latin and CJK.
const SENTENCE_ENDS: [char; 7] = ['.', '!', '?', '…', '。', '！', '？'];

/// Whole sentences only (v1.20.4): the text unchanged when it fits and ends a
/// sentence; otherwise the last complete sentence within `max_bytes` when it
/// keeps at least half of the allowance, else a word-boundary cut with "…".
/// For model output shown as is (announcements, stage directions, quoted facts),
/// which a token allowance may have cut mid-sentence.
pub(crate) fn truncate_at_sentence_boundary(s: &str, max_bytes: usize) -> String {
    let s = s.trim();
    let end = s.floor_char_boundary(max_bytes.min(s.len()));
    let prefix = &s[..end];
    let fits = prefix.len() == s.len();
    if fits && prefix.ends_with(SENTENCE_ENDS) {
        return s.to_string();
    }
    let last_end = prefix
        .char_indices()
        .rev()
        .find(|(_, c)| SENTENCE_ENDS.contains(c))
        .map(|(i, c)| i + c.len_utf8());
    match last_end {
        Some(cut) if cut >= end / 2 => prefix[..cut].trim_end().to_string(),
        _ if fits => s.to_string(),
        _ => truncate_at_word_boundary(s, max_bytes),
    }
}

/// Detect model safety refusals (e.g. "I'm sorry, but I can't help with that.",
/// "Je ne peux pas…", "抱歉…") — short answers only, a long answer that merely
/// quotes such a phrase is content. Shared by the engine and the bench metrics.
pub(crate) fn is_model_refusal(text: &str) -> bool {
    let lower = text.to_lowercase();
    let trimmed = lower.trim();
    if trimmed.len() > crate::constants::ORCH_MAX_REFUSAL_LENGTH {
        return false;
    }
    crate::constants::REFUSAL_PREFIXES.iter().any(|p| trimmed.starts_with(p))
        || crate::constants::REFUSAL_SUBSTRINGS.iter().any(|s| trimmed.contains(s))
}

/// A reaction's quoted excerpt is kept only when it really appears in the
/// target message (case-insensitive, whitespace-normalised) — models often
/// paraphrase, and a fabricated quote must never be highlighted.
pub(crate) fn validated_quote(target: &str, quote: Option<&str>) -> Option<String> {
    let quote = quote?.trim();
    if quote.chars().count() < crate::constants::REACTION_QUOTE_MIN_CHARS {
        return None;
    }
    let normalise = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase();
    let haystack = normalise(target);
    let needle = normalise(quote);
    if haystack.contains(&needle) {
        Some(truncate_at_word_boundary(quote, crate::constants::REACTION_QUOTE_MAX_CHARS))
    } else {
        None
    }
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

    // ── truncate_at_sentence_boundary (v1.20.4) ─────────────────────────

    #[test]
    fn sentence_boundary_keeps_whole_sentences_only() {
        // Fits and ends a sentence: unchanged
        assert_eq!(truncate_at_sentence_boundary("Une phrase. Une autre !", 100), "Une phrase. Une autre !");
        // Too long: the last complete sentence within the allowance
        assert_eq!(truncate_at_sentence_boundary("Une phrase. Une autre phrase. Et la fin", 30), "Une phrase. Une autre phrase.");
        // Cut by a token allowance (no terminator): back to the last sentence when half remains
        assert_eq!(truncate_at_sentence_boundary("Première phrase complète. Puis le modèle s'arrê", 100), "Première phrase complète.");
        // No usable sentence end: word boundary with an ellipsis
        assert_eq!(truncate_at_sentence_boundary("mot mot mot mot mot mot mot mot", 15), "mot mot mot…");
        // A short first sentence would drop more than half: word cut instead
        assert_eq!(truncate_at_sentence_boundary("Oui. Puis une longue explication sans fin qui continue", 30), "Oui. Puis une longue…");
        // Fits without a terminator and without any sentence end: unchanged
        assert_eq!(truncate_at_sentence_boundary("Ana — devient cassant", 100), "Ana — devient cassant");
        // CJK terminators and multibyte safety
        assert_eq!(truncate_at_sentence_boundary("第一句。第二句很长很长很长很长", 20), "第一句。");
        assert_eq!(truncate_at_sentence_boundary("Élan… suite très très longue", 8), "Élan…");
    }

    #[test]
    fn validated_quote_keeps_real_excerpts_only() {
        let msg = "Les données montrent une transformation,   pas un remplacement du métier.";
        assert_eq!(validated_quote(msg, Some("une transformation, pas un remplacement")).as_deref(), Some("une transformation, pas un remplacement"));
        assert_eq!(validated_quote(msg, Some("UNE TRANSFORMATION")).as_deref(), Some("UNE TRANSFORMATION"), "case-insensitive");
        assert!(validated_quote(msg, Some("les robots remplacent tout")).is_none(), "paraphrase rejected");
        assert!(validated_quote(msg, Some("pas")).is_none(), "too short to be a quote");
        assert!(validated_quote(msg, None).is_none());
        let long = "x".repeat(400);
        assert!(validated_quote(&long, Some(&long)).unwrap().len() <= crate::constants::REACTION_QUOTE_MAX_CHARS + 3);
    }

}

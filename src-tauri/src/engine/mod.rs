pub mod emotion_engine;
pub mod json_parser;
pub mod memory_manager;
pub mod orchestrator;
pub mod prompt_builder;
pub mod turn_manager;

/// UTF-8–safe truncation: returns the longest prefix of `s` that fits within `max_chars`.
/// Uses `str::floor_char_boundary` to avoid splitting multi-byte characters.
pub(crate) fn truncate_str(s: &str, max_chars: usize) -> &str {
    &s[..s.floor_char_boundary(max_chars)]
}

/// Apply a signed i8 delta to a u8 value, clamping result to 0-100.
/// Used by both EmotionalProfile::apply_delta and emotion_engine::apply_contagion.
pub(crate) fn apply_i8_clamped(val: u8, delta: i8) -> u8 {
    (val as i16 + delta as i16).clamp(0, 100) as u8
}

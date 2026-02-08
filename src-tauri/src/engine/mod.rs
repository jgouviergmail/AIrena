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

use std::collections::HashMap;

use crate::constants;
use crate::models::memory::{MessageSummary, ParticipantMemory, ParticipantPosition, TurnSnapshot};
use crate::models::message::Message;

/// Add a turn snapshot to immediate memory, evicting oldest if needed.
/// When `is_fiction` is true, stores full story segments for narrative continuity.
pub fn add_turn_to_memory(
    memory: &mut ParticipantMemory,
    turn_number: u32,
    messages: &[Message],
    is_fiction: bool,
) {
    let max_chars = if is_fiction { constants::MEMORY_MAX_FICTION_MESSAGE_CHARS } else { constants::MEMORY_MAX_MESSAGE_CHARS };
    let snapshot = TurnSnapshot {
        turn_number,
        messages: messages
            .iter()
            .map(|m| MessageSummary {
                speaker_name: m.speaker_name.clone(),
                content: truncate_content(&m.content, max_chars),
            })
            .collect(),
    };

    memory.immediate.push(snapshot);

    // Evict oldest turns beyond the limit
    while memory.immediate.len() > constants::MEMORY_MAX_IMMEDIATE_TURNS {
        memory.immediate.remove(0);
    }
}

/// Update contextual summary and positional map from a combined LLM response
pub fn update_from_llm_response(
    memory: &mut ParticipantMemory,
    summary: String,
    positions: HashMap<String, String>,
) {
    memory.contextual_summary = truncate_content(&summary, constants::MEMORY_MAX_SUMMARY_CHARS);

    memory.positional_map = positions
        .into_iter()
        .map(|(name, stance)| {
            (
                name.clone(),
                ParticipantPosition {
                    participant_name: name,
                    stance,
                },
            )
        })
        .collect();
}

/// Serialize the positional map to JSON for prompt injection
pub fn positional_map_to_json(memory: &ParticipantMemory) -> String {
    let map: HashMap<&str, &str> = memory
        .positional_map
        .iter()
        .map(|(k, v)| (k.as_str(), v.stance.as_str()))
        .collect();
    serde_json::to_string(&map).unwrap_or_else(|_| "{}".to_string())
}

/// Format turn messages as text for the memory update prompt.
/// When `is_fiction` is true, uses the fiction limit so the summarizer sees full story segments.
pub fn format_turn_messages(messages: &[Message], is_fiction: bool) -> String {
    let max_chars = if is_fiction { constants::MEMORY_MAX_FICTION_MESSAGE_CHARS } else { constants::MEMORY_FORMAT_TURN_CHARS };
    messages
        .iter()
        .map(|m| format!("{}: {}", m.speaker_name, truncate_content(&m.content, max_chars)))
        .collect::<Vec<_>>()
        .join("\n")
}

fn truncate_content(content: &str, max_chars: usize) -> String {
    let truncated = super::truncate_str(content, max_chars);
    if truncated.len() < content.len() {
        format!("{}...", truncated)
    } else {
        content.to_string()
    }
}

use std::collections::HashMap;

use crate::models::memory::{MessageSummary, ParticipantMemory, ParticipantPosition, TurnSnapshot};
use crate::models::message::Message;

/// Maximum number of immediate turns to keep
const MAX_IMMEDIATE_TURNS: usize = 3;

/// Maximum characters per message in immediate memory
const MAX_MESSAGE_CHARS: usize = 600;

/// Add a turn snapshot to immediate memory, evicting oldest if needed
pub fn add_turn_to_memory(memory: &mut ParticipantMemory, turn_number: u32, messages: &[Message]) {
    let snapshot = TurnSnapshot {
        turn_number,
        messages: messages
            .iter()
            .map(|m| MessageSummary {
                speaker_name: m.speaker_name.clone(),
                content: truncate_content(&m.content, MAX_MESSAGE_CHARS),
            })
            .collect(),
    };

    memory.immediate.push(snapshot);

    // Evict oldest turns beyond the limit
    while memory.immediate.len() > MAX_IMMEDIATE_TURNS {
        memory.immediate.remove(0);
    }
}

/// Maximum characters for the contextual summary to prevent unbounded growth
const MAX_SUMMARY_CHARS: usize = 1500;

/// Update contextual summary and positional map from a combined LLM response
pub fn update_from_llm_response(
    memory: &mut ParticipantMemory,
    summary: String,
    positions: HashMap<String, String>,
) {
    memory.contextual_summary = truncate_content(&summary, MAX_SUMMARY_CHARS);

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

/// Format turn messages as text for the memory update prompt
pub fn format_turn_messages(messages: &[Message]) -> String {
    messages
        .iter()
        .map(|m| format!("{}: {}", m.speaker_name, truncate_content(&m.content, 400)))
        .collect::<Vec<_>>()
        .join("\n")
}

fn truncate_content(content: &str, max_chars: usize) -> String {
    let end = content.floor_char_boundary(max_chars);
    if end < content.len() {
        format!("{}...", &content[..end])
    } else {
        content.to_string()
    }
}

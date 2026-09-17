use std::collections::HashMap;

use crate::constants;
use crate::engine::json_parser::PositionInput;
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

/// Update contextual summary and positional map from a combined LLM response.
/// `summary_max_chars` controls the maximum length of the stored summary —
/// dynamically set by the token budget instead of using a fixed constant.
///
/// Positions keep their trajectory: the first stance ever recorded for a
/// participant stays as `initial_stance`; a blank new stance keeps the previous
/// one (the model forgot someone, it did not erase them).
pub fn update_from_llm_response(
    memory: &mut ParticipantMemory,
    summary: String,
    positions: HashMap<String, PositionInput>,
    summary_max_chars: usize,
) {
    memory.contextual_summary = truncate_content(&summary, summary_max_chars);

    for (name, input) in positions {
        let (stance, shift, would_change_if) = input.into_parts();
        match memory.positional_map.get_mut(&name) {
            Some(existing) => {
                if !stance.is_empty() {
                    existing.stance = stance;
                }
                if existing.initial_stance.is_none() {
                    existing.initial_stance = Some(existing.stance.clone());
                }
                existing.shift = shift;
                existing.would_change_if = would_change_if;
            }
            None if !stance.is_empty() => {
                memory.positional_map.insert(
                    name.clone(),
                    ParticipantPosition {
                        participant_name: name,
                        initial_stance: Some(stance.clone()),
                        stance,
                        shift,
                        would_change_if,
                    },
                );
            }
            None => {}
        }
    }
}

/// Positions in a stable order (by participant name) for events and reports.
pub fn positions_sorted(memory: &ParticipantMemory) -> Vec<ParticipantPosition> {
    let mut positions: Vec<ParticipantPosition> = memory.positional_map.values().cloned().collect();
    positions.sort_by(|a, b| a.participant_name.cmp(&b.participant_name));
    positions
}

/// Serialize the positional map to JSON for prompt injection — the v1.17
/// object shape (stance + trajectory) so the model updates rather than restates.
pub fn positional_map_to_json(memory: &ParticipantMemory) -> String {
    #[derive(serde::Serialize)]
    struct Wire<'a> {
        stance: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        shift: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        would_change_if: Option<&'a str>,
    }
    let map: HashMap<&str, Wire<'_>> = memory
        .positional_map
        .iter()
        .map(|(k, v)| {
            (k.as_str(), Wire { stance: &v.stance, shift: v.shift.as_deref(), would_change_if: v.would_change_if.as_deref() })
        })
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

#[cfg(test)]
mod tests {
    use super::*;

    fn detailed(stance: &str, shift: Option<&str>, cond: Option<&str>) -> PositionInput {
        PositionInput::Detailed {
            stance: stance.to_string(),
            shift: shift.map(|s| serde_json::Value::String(s.to_string())),
            would_change_if: cond.map(|s| serde_json::Value::String(s.to_string())),
        }
    }

    #[test]
    fn positions_keep_their_trajectory_across_updates() {
        let mut memory = ParticipantMemory::default();
        let first = HashMap::from([
            ("A".to_string(), PositionInput::Text("prudent".to_string())),
            ("B".to_string(), detailed("critique", None, Some("des preuves"))),
            ("C".to_string(), PositionInput::Text("  ".to_string())),
        ]);
        update_from_llm_response(&mut memory, "résumé".to_string(), first, 100);
        assert_eq!(memory.contextual_summary, "résumé");
        assert_eq!(memory.positional_map.len(), 2, "blank newcomer ignored");
        let a = &memory.positional_map["A"];
        assert_eq!(a.initial_stance.as_deref(), Some("prudent"));
        assert!(!a.has_evolved());
        assert_eq!(memory.positional_map["B"].would_change_if.as_deref(), Some("des preuves"));

        // Second turn: A moves (string form), B forgotten by the model, C appears
        let second = HashMap::from([
            ("A".to_string(), detailed("ouvert", Some("s'est ouvert"), None)),
            ("B".to_string(), PositionInput::Text(String::new())),
            ("C".to_string(), PositionInput::Text("neutre".to_string())),
        ]);
        update_from_llm_response(&mut memory, "r2".to_string(), second, 100);
        let a = &memory.positional_map["A"];
        assert_eq!((a.stance.as_str(), a.initial_stance.as_deref(), a.shift.as_deref()), ("ouvert", Some("prudent"), Some("s'est ouvert")));
        assert!(a.has_evolved());
        let b = &memory.positional_map["B"];
        assert_eq!(b.stance, "critique", "blank update keeps the previous stance");
        assert_eq!(b.would_change_if, None, "trajectory fields follow the latest answer");
        assert_eq!(memory.positional_map["C"].initial_stance.as_deref(), Some("neutre"));

        let sorted = positions_sorted(&memory);
        assert_eq!(sorted.iter().map(|p| p.participant_name.as_str()).collect::<Vec<_>>(), vec!["A", "B", "C"]);
        let json = positional_map_to_json(&memory);
        assert!(json.contains(r#""A":{"stance":"ouvert","shift":"s'est ouvert"}"#), "{json}");
        assert!(json.contains(r#""C":{"stance":"neutre"}"#), "{json}");
    }
}

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct ParticipantMemory {
    /// Recent complete turns (last 2-3)
    pub immediate: Vec<TurnSnapshot>,
    /// Cumulative summary of older turns
    pub contextual_summary: String,
    /// Positions of each participant
    pub positional_map: HashMap<String, ParticipantPosition>,
}

impl Default for ParticipantMemory {
    fn default() -> Self {
        Self {
            immediate: Vec::new(),
            contextual_summary: String::new(),
            positional_map: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TurnSnapshot {
    pub turn_number: u32,
    pub messages: Vec<MessageSummary>,
}

#[derive(Debug, Clone)]
pub struct MessageSummary {
    pub speaker_name: String,
    /// Truncated to ~200 tokens
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParticipantPosition {
    pub participant_name: String,
    pub stance: String,
}

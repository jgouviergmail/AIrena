use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default)]
pub struct ParticipantMemory {
    /// Recent complete turns (last 2-3)
    pub immediate: Vec<TurnSnapshot>,
    /// Cumulative summary of older turns
    pub contextual_summary: String,
    /// Positions of each participant
    pub positional_map: HashMap<String, ParticipantPosition>,
}

#[derive(Debug, Clone)]
pub struct TurnSnapshot {
    pub turn_number: u32,
    pub messages: Vec<MessageSummary>,
}

#[derive(Debug, Clone)]
pub struct MessageSummary {
    pub speaker_name: String,
    /// Truncated to MAX_MESSAGE_CHARS (1500) or MAX_FICTION_MESSAGE_CHARS (3000) in fiction mode
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParticipantPosition {
    pub participant_name: String,
    pub stance: String,
}

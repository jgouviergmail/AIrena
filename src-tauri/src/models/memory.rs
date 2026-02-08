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
    /// Truncated to ~200 tokens
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParticipantPosition {
    pub participant_name: String,
    pub stance: String,
}

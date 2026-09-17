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

/// A participant's position and its trajectory (v1.17). Older discussions
/// only carried `stance`; the other fields default to `None`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParticipantPosition {
    pub participant_name: String,
    pub stance: String,
    /// Stance recorded the first time the participant was mapped
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial_stance: Option<String>,
    /// How the stance moved since the previous update (model's words)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shift: Option<String>,
    /// What would make the participant change their mind
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub would_change_if: Option<String>,
}

impl ParticipantPosition {
    /// The stance has visibly moved since it was first recorded.
    pub fn has_evolved(&self) -> bool {
        self.shift.as_deref().is_some_and(|s| !s.trim().is_empty())
            || self.initial_stance.as_deref().is_some_and(|initial| initial != self.stance)
    }
}

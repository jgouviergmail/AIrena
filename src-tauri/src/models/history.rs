use serde::{Deserialize, Serialize};

use super::llm::UsageLedger;
use super::message::Message;

fn default_provider() -> String {
    "ollama".to_string()
}

/// Participant metadata stored as JSON in the discussions table.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParticipantInfo {
    pub id: String,
    pub name: String,
    pub role: String,
    pub emoji: String,
}

/// Request payload from frontend to save a discussion.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveDiscussionRequest {
    pub id: String,
    pub topic: String,
    pub discussion_language: String,
    pub model_name: String,
    pub participants: Vec<ParticipantInfo>,
    pub total_turns: u32,
    pub synthesis: String,
    pub created_at: String,
    pub messages: Vec<Message>,
    #[serde(default)]
    pub discussion_mode: String,
    #[serde(default)]
    pub document_content: String,
    #[serde(default)]
    pub document_format: String,
    #[serde(default)]
    pub argument_map_md: String,
    #[serde(default)]
    pub argument_map_md_by_speaker: String,
    /// Serialised `ArgumentMap` (empty when the map was disabled)
    #[serde(default)]
    pub argument_map_json: String,
    #[serde(default = "default_provider")]
    pub llm_provider: String,
    #[serde(default)]
    pub usage: UsageLedger,
    #[serde(default)]
    pub estimated_cost_usd: f64,
}

/// Lightweight summary for listing discussions (no messages).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscussionSummary {
    pub id: String,
    pub topic: String,
    pub discussion_language: String,
    pub model_name: String,
    pub participants: Vec<ParticipantInfo>,
    pub total_turns: u32,
    pub has_synthesis: bool,
    pub created_at: String,
    pub discussion_mode: String,
    pub document_format: String,
    pub has_argument_map: bool,
    pub llm_provider: String,
    pub total_tokens: u32,
    pub estimated_cost_usd: f64,
}

/// Full discussion detail with all messages.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscussionDetail {
    pub id: String,
    pub topic: String,
    pub discussion_language: String,
    pub model_name: String,
    pub participants: Vec<ParticipantInfo>,
    pub total_turns: u32,
    pub synthesis: String,
    pub created_at: String,
    pub messages: Vec<Message>,
    pub discussion_mode: String,
    pub document_content: String,
    pub document_format: String,
    pub argument_map_md: String,
    pub argument_map_md_by_speaker: String,
    /// Serialised `ArgumentMap` — empty for discussions saved before v1.16
    pub argument_map_json: String,
    pub llm_provider: String,
    pub usage: UsageLedger,
    pub estimated_cost_usd: f64,
}

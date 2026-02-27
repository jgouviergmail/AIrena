use serde::{Deserialize, Serialize};

use super::message::Message;

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
}

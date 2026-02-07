use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SpeakerRole {
    #[serde(rename = "IArbitre")]
    Arbitre,
    #[serde(rename = "GladIAteur")]
    Gladiateur,
    #[serde(rename = "user")]
    User,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ReactionType {
    Like,
    Dislike,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: String,
    pub discussion_id: String,
    pub turn_number: u32,
    pub speaker_id: String,
    pub speaker_name: String,
    pub role: SpeakerRole,
    pub content: String,
    pub inner_thought: Option<String>,
    pub reactions: Vec<Reaction>,
    pub is_ban_notification: bool,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reaction {
    pub from_speaker_id: String,
    pub from_speaker_name: String,
    pub reaction_type: ReactionType,
    pub target_message_id: String,
}

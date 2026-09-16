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

/// Nature of a message's `inner_thought`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThoughtKind {
    /// In-character private reflection produced by the separate thought phase.
    #[default]
    Persona,
    /// Raw model reasoning exposed by the provider (DeepSeek `reasoning_content`).
    Reasoning,
}

impl ThoughtKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Persona => "persona",
            Self::Reasoning => "reasoning",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value {
            "reasoning" => Self::Reasoning,
            _ => Self::Persona,
        }
    }
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
    #[serde(default)]
    pub thought_kind: ThoughtKind,
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
    #[serde(default)]
    pub justification: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_without_thought_kind_deserialises_as_persona() {
        let json = r#"{"id":"m1","discussionId":"d","turnNumber":1,"speakerId":"s","speakerName":"S","role":"GladIAteur","content":"c","innerThought":null,"reactions":[],"isBanNotification":false,"timestamp":"2026-09-16T10:00:00Z"}"#;
        let m: Message = serde_json::from_str(json).unwrap();
        assert_eq!(m.thought_kind, ThoughtKind::Persona);
        assert!(serde_json::to_string(&m).unwrap().contains("\"thoughtKind\":\"persona\""));
        assert_eq!(ThoughtKind::parse("reasoning"), ThoughtKind::Reasoning);
        assert_eq!(ThoughtKind::parse("garbage"), ThoughtKind::Persona);
    }
}

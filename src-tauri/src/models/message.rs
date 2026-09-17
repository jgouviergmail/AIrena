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

/// Colour of a reaction. `Like`/`Dislike` are the historical pair; the others
/// refine them (v1.17). Positive/negative/neutral classes drive the
/// relationship graph, the emotional effects live in `emotion_engine`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum ReactionType {
    Like,
    Dislike,
    /// Strong point, worth building on (like + confidence bonus for the target)
    Insightful,
    /// Genuine question raised by the intervention (opens a loop, boosts curiosity)
    Question,
    /// Off-topic or derailing (softened dislike + moderation hint)
    OffTopic,
    /// Made the room laugh (enthusiasm for both sides)
    Laugh,
}

impl ReactionType {
    pub const ALL: [ReactionType; 6] = [
        Self::Like,
        Self::Dislike,
        Self::Insightful,
        Self::Question,
        Self::OffTopic,
        Self::Laugh,
    ];

    /// Approval: counts as a "like" for relationships and the historical rules.
    pub fn is_positive(&self) -> bool {
        matches!(self, Self::Like | Self::Insightful)
    }

    /// Disapproval: counts as a "dislike" for relationships and the historical rules.
    pub fn is_negative(&self) -> bool {
        matches!(self, Self::Dislike | Self::OffTopic)
    }

    /// Neither approval nor disapproval (ignored by the relationship graph).
    pub fn is_neutral(&self) -> bool {
        !self.is_positive() && !self.is_negative()
    }

    /// Wire name (camelCase) — the same string serde produces.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Like => "like",
            Self::Dislike => "dislike",
            Self::Insightful => "insightful",
            Self::Question => "question",
            Self::OffTopic => "offTopic",
            Self::Laugh => "laugh",
        }
    }

    /// Parse the wire name and the synonyms models tend to produce.
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_lowercase().as_str() {
            "like" | "agree" | "d'accord" | "👍" | "positive" | "positif" | "accord" => Some(Self::Like),
            "dislike" | "disagree" | "pas d'accord" | "👎" | "negative" | "négatif" | "negatif" | "désaccord" => Some(Self::Dislike),
            "insightful" | "insight" | "pertinent" | "brillant" | "strong" | "fort" | "💡" => Some(Self::Insightful),
            "question" | "curious" | "questionnement" | "❓" | "?" => Some(Self::Question),
            "offtopic" | "off_topic" | "off-topic" | "hors sujet" | "hors-sujet" | "horssujet" | "🚫" => Some(Self::OffTopic),
            "laugh" | "funny" | "rire" | "drôle" | "drole" | "humor" | "😂" | "🤣" => Some(Self::Laugh),
            _ => None,
        }
    }
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

/// What a message is: a contribution, or one of the system lines the engine
/// interleaves for the audience (v1.17). Only `Normal` and `BanNotification`
/// (and `ActAnnouncement` / `SceneEvent`, which the moderator "says") reach
/// the participants' prompts; stage directions are for the audience only.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum MessageKind {
    #[default]
    Normal,
    BanNotification,
    /// Short italic line describing a participant's visible state (no LLM call)
    StageDirection,
    /// The moderator announces the act that starts this turn
    ActAnnouncement,
    /// The moderator announces a scene event (surprise fact, duel, hot seat…)
    SceneEvent,
}

impl MessageKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::BanNotification => "banNotification",
            Self::StageDirection => "stageDirection",
            Self::ActAnnouncement => "actAnnouncement",
            Self::SceneEvent => "sceneEvent",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value {
            "banNotification" => Self::BanNotification,
            "stageDirection" => Self::StageDirection,
            "actAnnouncement" => Self::ActAnnouncement,
            "sceneEvent" => Self::SceneEvent,
            _ => Self::Normal,
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
    /// Kept for v1.16 payloads; always consistent with `kind == BanNotification`.
    pub is_ban_notification: bool,
    #[serde(default)]
    pub kind: MessageKind,
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
    /// Exact excerpt of the target message the reaction points at (validated by the engine).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quote: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_without_thought_kind_deserialises_as_persona() {
        let json = r#"{"id":"m1","discussionId":"d","turnNumber":1,"speakerId":"s","speakerName":"S","role":"GladIAteur","content":"c","innerThought":null,"reactions":[],"isBanNotification":false,"timestamp":"2026-09-16T10:00:00Z"}"#;
        let m: Message = serde_json::from_str(json).unwrap();
        assert_eq!(m.thought_kind, ThoughtKind::Persona);
        assert_eq!(m.kind, MessageKind::Normal, "v1.16 payloads have no kind");
        assert!(serde_json::to_string(&m).unwrap().contains("\"thoughtKind\":\"persona\""));
        assert_eq!(ThoughtKind::parse("reasoning"), ThoughtKind::Reasoning);
        assert_eq!(ThoughtKind::parse("garbage"), ThoughtKind::Persona);
    }

    #[test]
    fn reaction_types_classify_and_parse_synonyms() {
        assert!(ReactionType::Insightful.is_positive());
        assert!(ReactionType::OffTopic.is_negative());
        assert!(ReactionType::Question.is_neutral() && ReactionType::Laugh.is_neutral());
        assert!(!ReactionType::Like.is_neutral());
        for t in ReactionType::ALL {
            // Wire name round-trips through serde and `parse`
            let json = serde_json::to_string(&t).unwrap();
            assert_eq!(json, format!("\"{}\"", t.as_str()));
            assert_eq!(ReactionType::parse(t.as_str()), Some(t));
        }
        assert_eq!(ReactionType::parse("Hors sujet"), Some(ReactionType::OffTopic));
        assert_eq!(ReactionType::parse("D'ACCORD"), Some(ReactionType::Like));
        assert_eq!(ReactionType::parse("none"), None);
        // A v1.16 reaction without a quote still deserialises, and the quote is omitted when absent
        let json = r#"{"fromSpeakerId":"a","fromSpeakerName":"A","reactionType":"like","targetMessageId":"m"}"#;
        let r: Reaction = serde_json::from_str(json).unwrap();
        assert!(r.quote.is_none());
        assert!(!serde_json::to_string(&r).unwrap().contains("quote"));
    }

    #[test]
    fn message_kind_round_trips_and_gates_visibility() {
        for kind in [MessageKind::Normal, MessageKind::BanNotification, MessageKind::StageDirection, MessageKind::ActAnnouncement, MessageKind::SceneEvent] {
            assert_eq!(MessageKind::parse(kind.as_str()), kind);
            assert_eq!(serde_json::to_string(&kind).unwrap(), format!("\"{}\"", kind.as_str()));
        }
        assert_eq!(MessageKind::parse("unknown"), MessageKind::Normal);
    }
}

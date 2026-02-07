use serde::Serialize;

use super::emotion::EmotionalProfile;
use super::message::{Message, Reaction};

/// Events sent from the backend to the frontend via Channel<ArenaEvent>
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "type", content = "data")]
pub enum ArenaEvent {
    /// Discussion started successfully
    DiscussionStarted { discussion_id: String },
    /// Streaming message token
    MessageChunk { speaker_id: String, chunk: String },
    /// Complete message (after streaming ends)
    MessageComplete { message: Message },
    /// Reaction emitted by a participant
    ReactionEmitted {
        message_id: String,
        reaction: Reaction,
    },
    /// Streaming inner thought token
    ThoughtChunk { speaker_id: String, chunk: String },
    /// Complete inner thought
    ThoughtComplete { speaker_id: String, thought: String },
    /// New turn started
    TurnStarted {
        turn_number: u32,
        speaker_order: Vec<String>,
    },
    /// Turn skipped (all banned)
    TurnSkipped {
        reason: String,
        next_available_turn: u32,
    },
    /// Active speaker changed
    SpeakerActive { speaker_id: String },
    /// Emotions updated (rule-based, instant)
    EmotionUpdated {
        speaker_id: String,
        emotions: EmotionalProfile,
    },
    /// Ban issued by the IArbitre
    BanIssued {
        banned_id: String,
        banned_name: String,
        reason: String,
        duration: u32,
    },
    /// Ban lifted (participant returns)
    BanLifted {
        speaker_id: String,
        speaker_name: String,
    },
    /// It's the user's turn
    UserTurnReady,
    /// User intervention timed out
    UserTurnTimeout,
    /// Pause confirmed
    PauseConfirmed,
    /// Resume confirmed
    ResumeConfirmed,
    /// Streaming synthesis token
    SynthesisChunk { chunk: String },
    /// Final synthesis complete
    SynthesisComplete { summary: String },
    /// Discussion ended
    DiscussionEnded,
    /// Non-fatal error (displayed in feed)
    Error { message: String },
}

use super::message::ReactionType;

/// Commands sent from the frontend to the running engine
#[derive(Debug)]
pub enum EngineCommand {
    /// The audience (the user) reacts to a message
    AudienceReaction { message_id: String, reaction_type: ReactionType },
    /// The audience votes on the motion of an Oxford debate (`for` / `against`, v1.19)
    AudienceVote { choice: String },
    Pause,
    Resume,
    /// Soft stop: finish the current turn
    Stop,
    /// Hard stop: interrupt immediately
    ForceStop,
    UserWantsToIntervene,
    SubmitUserMessage { content: String },
    /// User cancels their intervention
    SkipUserTurn,
    /// Step mode (v1.20.1): the engine waits for the audience's cue before each speaker
    SetStepMode { enabled: bool },
    /// The audience's cue: the next speaker may talk
    NextSpeaker,
    /// Manually adjust a participant's emotion axis
    AdjustEmotion {
        speaker_id: String,
        axis: String,
        value: u8,
    },
}

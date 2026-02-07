/// Commands sent from the frontend to the running engine
#[derive(Debug)]
pub enum EngineCommand {
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
}

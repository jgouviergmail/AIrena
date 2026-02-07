use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerationResult {
    #[serde(default = "default_moderation_action")]
    pub action: ModerationAction,
    #[serde(default)]
    pub comment: String,
    #[serde(default)]
    pub ban_reason: String,
    #[serde(default)]
    pub ban_duration: u32,
}

fn default_moderation_action() -> ModerationAction {
    ModerationAction::None
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ModerationAction {
    None,
    Comment,
    Ban,
}

impl Default for ModerationResult {
    fn default() -> Self {
        Self {
            action: ModerationAction::None,
            comment: String::new(),
            ban_reason: String::new(),
            ban_duration: 0,
        }
    }
}

/// Raw reaction structure received from the LLM (before validation)
#[derive(Debug, Deserialize)]
pub struct RawReaction {
    pub speaker: String,
    pub reaction: String,
}

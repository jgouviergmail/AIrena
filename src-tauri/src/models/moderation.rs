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
/// Aliases handle common alternative field names LLMs may use.
#[derive(Debug, Deserialize)]
pub struct RawReaction {
    #[serde(alias = "name", alias = "participant", alias = "intervenant", alias = "gladiateur")]
    pub speaker: String,
    #[serde(alias = "response", alias = "opinion", alias = "avis", alias = "vote", alias = "type")]
    pub reaction: String,
    #[serde(default, alias = "reason", alias = "explication", alias = "raison", alias = "motif")]
    pub justification: String,
    /// Exact excerpt the reaction points at (optional, validated by the engine)
    #[serde(default, alias = "citation", alias = "extrait", alias = "excerpt")]
    pub quote: String,
    /// Whether the intervention deserves a reaction at all (v1.20.5): a false
    /// value — boolean or spelled out — is a silence; absent means yes.
    #[serde(default, alias = "react", alias = "reagit", alias = "réagit")]
    pub reacts: Option<serde_json::Value>,
}

impl RawReaction {
    /// The model declined to react ("reacts": false / "false" / "non" / "no").
    pub fn declined(&self) -> bool {
        match &self.reacts {
            Some(serde_json::Value::Bool(b)) => !b,
            Some(serde_json::Value::String(s)) => matches!(s.trim().to_lowercase().as_str(), "false" | "non" | "no" | "否" | "none"),
            _ => false,
        }
    }
}

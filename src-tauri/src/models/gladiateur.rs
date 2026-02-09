use serde::{Deserialize, Serialize};

use super::emotion::EmotionalProfile;
use super::memory::ParticipantMemory;
use super::settings::LlmParams;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GladIAteurConfig {
    pub id: String,
    pub name: String,
    pub intervention_number: u32,
    pub system_prompt: String,
    pub llm_params: LlmParams,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GladIAteurState {
    pub config: GladIAteurConfig,
    /// 0 = active, > 0 = banned for this many turns
    pub ban_remaining_turns: u32,
    /// Prevents off-by-one on ban decrement
    pub ban_issued_this_turn: bool,
    pub memory: ParticipantMemory,
    pub emotions: EmotionalProfile,
    pub web_searches_used_discussion: u32,
}

impl GladIAteurState {
    pub fn new(config: GladIAteurConfig) -> Self {
        Self {
            config,
            ban_remaining_turns: 0,
            ban_issued_this_turn: false,
            memory: ParticipantMemory::default(),
            emotions: EmotionalProfile::default(),
            web_searches_used_discussion: 0,
        }
    }

    /// A gladiator is banned if their ban counter is > 0
    pub fn is_banned(&self) -> bool {
        self.ban_remaining_turns > 0
    }
}

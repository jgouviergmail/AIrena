use serde::{Deserialize, Serialize};

use super::emotion::{EmotionSnapshot, EmotionalProfile};
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
    /// JSON string of initial EmotionalProfile (from predefined profile)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial_emotions: Option<String>,
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
    /// Emotion history per turn (capped at 30)
    pub emotion_history: Vec<EmotionSnapshot>,
    /// History of past search queries (for deduplication in prompts)
    pub search_queries_history: Vec<String>,
}

impl GladIAteurState {
    pub fn new(config: GladIAteurConfig, initial_emotions: Option<EmotionalProfile>) -> Self {
        let emotions = initial_emotions.unwrap_or_default();
        Self {
            config,
            ban_remaining_turns: 0,
            ban_issued_this_turn: false,
            memory: ParticipantMemory::default(),
            emotions,
            emotion_history: Vec::new(),
            search_queries_history: Vec::new(),
        }
    }

    /// A gladiator is banned if their ban counter is > 0
    pub fn is_banned(&self) -> bool {
        self.ban_remaining_turns > 0
    }
}

use serde::{Deserialize, Serialize};

use super::discussion::TurnDistribution;
use super::emotion::{EmotionSnapshot, EmotionalProfile};
use super::memory::ParticipantMemory;
use super::settings::LlmParams;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IArbitreConfig {
    pub id: String,
    pub name: String,
    pub system_prompt: String,
    pub turn_distribution: TurnDistribution,
    pub llm_params: LlmParams,
    /// If true, the IArbitre does 1 mandatory web search on the topic before introduction
    #[serde(default)]
    pub web_search_intro: bool,
    /// If true, the IArbitre does 1 mandatory Wikipedia search on the topic before introduction
    #[serde(default)]
    pub wiki_search_intro: bool,
}

#[derive(Debug, Clone)]
pub struct IArbitreState {
    pub config: IArbitreConfig,
    /// The IArbitre has its own memory
    pub memory: ParticipantMemory,
    /// Emotional profile of the IArbitre
    pub emotions: EmotionalProfile,
    /// Emotion history per turn (capped at 30)
    pub emotion_history: Vec<EmotionSnapshot>,
}

impl IArbitreState {
    pub fn new(config: IArbitreConfig) -> Self {
        Self {
            config,
            memory: ParticipantMemory::default(),
            emotions: EmotionalProfile::default(),
            emotion_history: Vec::new(),
        }
    }
}

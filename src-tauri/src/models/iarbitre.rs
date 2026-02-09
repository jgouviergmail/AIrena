use serde::{Deserialize, Serialize};

use super::discussion::TurnDistribution;
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
}

#[derive(Debug, Clone)]
pub struct IArbitreState {
    pub config: IArbitreConfig,
    /// The IArbitre has its own memory
    pub memory: ParticipantMemory,
}

impl IArbitreState {
    pub fn new(config: IArbitreConfig) -> Self {
        Self {
            config,
            memory: ParticipantMemory::default(),
        }
    }
}

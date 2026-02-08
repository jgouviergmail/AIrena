use serde::{Deserialize, Serialize};

use super::gladiateur::GladIAteurConfig;
use super::iarbitre::IArbitreConfig;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum TurnDistribution {
    Sequential,
    Random,
    Democratic,
    Authoritarian,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DiscussionStatus {
    Active,
    Paused,
    StopRequested,
    ForceStopRequested,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscussionConfig {
    pub topic: String,
    pub discussion_language: String,
    pub arbitre: IArbitreConfig,
    pub gladiateurs: Vec<GladIAteurConfig>,
    pub max_turns: Option<u32>,
    pub user_name: String,
    pub user_intervention_timeout_secs: u64,
}

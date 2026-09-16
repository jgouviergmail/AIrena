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

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum DiscussionMode {
    #[default]
    Debate,
    Ideation,
    CoConstruction,
    UserDriven,
    Socratic,
    Tutorial,
    CritiqueReview,
    CollaborativeFiction,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum DocumentFormat {
    #[default]
    None,
    Txt,
    Md,
    Csv,
}

impl DocumentFormat {
    pub fn as_extension(&self) -> &str {
        match self {
            Self::None => "none",
            Self::Txt => "txt",
            Self::Md => "md",
            Self::Csv => "csv",
        }
    }
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
    /// Global pool of web searches for the entire discussion (0 = disabled).
    /// Shared between all gladiateurs, max 1 per gladiateur per turn.
    #[serde(default)]
    pub web_search_pool: u32,
    /// Global pool of Wikipedia searches for the entire discussion (0 = disabled).
    /// Shared between all gladiateurs, max 1 per gladiateur per turn.
    #[serde(default)]
    pub wiki_search_pool: u32,
    /// Discussion mode (debate, ideation, co-construction, etc.)
    #[serde(default)]
    pub discussion_mode: DiscussionMode,
    /// Document format for co-construction (none = disabled)
    #[serde(default)]
    pub document_format: DocumentFormat,
    /// Enable real-time argument map extraction
    #[serde(default)]
    pub argument_map_enabled: bool,
    /// Whether to inject the full document or use RAG chunk search.
    #[serde(default)]
    pub document_injection_mode: DocumentInjectionMode,
    /// Co-construction: regenerate the document once per turn or after every intervention.
    #[serde(default)]
    pub document_update_granularity: DocumentUpdateGranularity,
}

/// When the co-construction document is regenerated.
#[derive(Debug, Default, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DocumentUpdateGranularity {
    /// One LLM call at the end of the turn integrating every contribution (default).
    #[default]
    Turn,
    /// One LLM call after each intervention (legacy behaviour, N× more calls).
    Intervention,
}

/// Controls how imported documents are provided to the AI during discussions.
#[derive(Debug, Default, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DocumentInjectionMode {
    /// Search-based: relevant chunks extracted via hybrid BM25 + vector search.
    #[default]
    Rag,
    /// Full injection: entire document included in each prompt (requires sufficient budget).
    FullInjection,
}

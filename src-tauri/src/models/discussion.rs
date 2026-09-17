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
    /// Adversarial trial: prosecution, defence, witnesses, jurors, a verdict (v1.19)
    Trial,
    /// Oxford-style debate on a motion, two camps, the audience votes before and after (v1.19)
    OxfordDebate,
    /// Parties with distinct interests seek an agreement (v1.19)
    Negotiation,
    /// De Bono's six thinking hats, rotating every turn (v1.19)
    SixHats,
    /// A crisis cell fed with dispatches every turn (v1.19)
    CrisisCell,
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
    /// Liveliness options (v1.17): reaction timing, audience, staging…
    #[serde(default)]
    pub features: DiscussionFeatures,
}

/// When participants react to an intervention.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReactionTiming {
    /// Everyone reacts right after each intervention (default since v1.17).
    #[default]
    Immediate,
    /// Each speaker reacts to the previous turn just before speaking (v1.16 behaviour).
    Deferred,
}

/// Optional liveliness features of a discussion. Every field has a default so
/// that v1.16 configurations deserialise; the "v1.16 equivalent" profile
/// (`DiscussionFeatures::legacy()`) reproduces the historical engine behaviour.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscussionFeatures {
    #[serde(default)]
    pub reaction_timing: ReactionTiming,
    /// The user may react to messages from the arena.
    #[serde(default = "default_true")]
    pub audience_reactions: bool,
    /// The moderator may trigger scene events (surprise fact, duel, hot seat…).
    #[serde(default = "default_true")]
    pub scene_events: bool,
    /// Each participant receives a secret agenda (mode permitting).
    #[serde(default = "default_true")]
    pub hidden_agenda: bool,
    /// Allies may relay each other within a turn.
    #[serde(default = "default_true")]
    pub coalitions: bool,
}

fn default_true() -> bool {
    true
}

impl Default for DiscussionFeatures {
    fn default() -> Self {
        Self {
            reaction_timing: ReactionTiming::Immediate,
            audience_reactions: true,
            scene_events: true,
            hidden_agenda: true,
            coalitions: true,
        }
    }
}

impl DiscussionMode {
    /// Modes where a secret objective makes sense: the debate-like ones and the
    /// relay story (an author's agenda). Teaching, questioning, reviewing and
    /// user-led sessions have none.
    pub fn supports_hidden_agenda(&self) -> bool {
        matches!(
            self,
            DiscussionMode::Debate | DiscussionMode::CollaborativeFiction | DiscussionMode::Trial | DiscussionMode::OxfordDebate | DiscussionMode::Negotiation
        )
    }

    /// Modes where answering objections on the merits is the point (v1.20.1): the
    /// argument map opens loops on the objected speakers, the moderator asks for
    /// depth and the `Deepen` speech act is available.
    pub fn rewards_depth(&self) -> bool {
        matches!(
            self,
            DiscussionMode::Debate
                | DiscussionMode::Trial
                | DiscussionMode::OxfordDebate
                | DiscussionMode::CritiqueReview
                | DiscussionMode::Socratic
                | DiscussionMode::Negotiation
                | DiscussionMode::UserDriven
        )
    }

    /// Every mode, in the order the wizard shows them (test sweeps).
    #[cfg(test)]
    pub const ALL: [DiscussionMode; 13] = [
        DiscussionMode::Debate,
        DiscussionMode::Ideation,
        DiscussionMode::CoConstruction,
        DiscussionMode::UserDriven,
        DiscussionMode::Socratic,
        DiscussionMode::Tutorial,
        DiscussionMode::CritiqueReview,
        DiscussionMode::CollaborativeFiction,
        DiscussionMode::Trial,
        DiscussionMode::OxfordDebate,
        DiscussionMode::Negotiation,
        DiscussionMode::SixHats,
        DiscussionMode::CrisisCell,
    ];
}

impl DiscussionFeatures {
    /// The v1.16 behaviour: deferred reactions, nothing staged (test profile).
    #[cfg(test)]
    pub fn legacy() -> Self {
        Self {
            reaction_timing: ReactionTiming::Deferred,
            audience_reactions: false,
            scene_events: false,
            hidden_agenda: false,
            coalitions: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn features_default_from_empty_json_and_legacy_profile() {
        let f: DiscussionFeatures = serde_json::from_str("{}").unwrap();
        assert_eq!(f, DiscussionFeatures::default());
        assert_eq!(f.reaction_timing, ReactionTiming::Immediate);
        assert!(f.audience_reactions && f.scene_events && f.hidden_agenda && f.coalitions);
        let legacy = DiscussionFeatures::legacy();
        assert_eq!(legacy.reaction_timing, ReactionTiming::Deferred);
        assert!(!legacy.scene_events);
        // Partial payload keeps the other defaults
        let partial: DiscussionFeatures = serde_json::from_str(r#"{"reactionTiming":"deferred"}"#).unwrap();
        assert_eq!(partial.reaction_timing, ReactionTiming::Deferred);
        assert!(partial.audience_reactions);
    }
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

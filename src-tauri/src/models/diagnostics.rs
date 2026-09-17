//! Measures of a discussion (v1.17): wall-clock timings per turn phase and
//! the diagnostics emitted before the end (no silent failure).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Duration of one phase of a turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhaseTiming {
    /// "speakers" | "endOfTurn" | "document" | "emotion" | "memory" | "turnAnalyst" | "argumentMap"
    pub name: String,
    pub ms: u64,
}

/// Timings of one turn (emitted at its end).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnTimings {
    pub turn: u32,
    pub phases: Vec<PhaseTiming>,
}

/// What went wrong or not during the discussion, emitted before `DiscussionEnded`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscussionDiagnostics {
    /// Structured answers that could not be parsed, by call kind (wire name)
    #[serde(default)]
    pub json_parse_failures: BTreeMap<String, u32>,
    /// Interventions detected as safety refusals
    #[serde(default)]
    pub refusals: u32,
    /// LLM calls retried (empty or unusable answers)
    #[serde(default)]
    pub retries: u32,
    /// Share of interventions that named the target their intention declared (0–1)
    #[serde(default)]
    pub intention_compliance: Option<f32>,
}

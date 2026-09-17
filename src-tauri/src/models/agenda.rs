//! Hidden agendas (v1.19): a secret objective per participant, generated at the
//! start, never revealed by the persona, unveiled with the synthesis.

use serde::{Deserialize, Serialize};

/// What a participant secretly pursues (fields already trimmed and bounded).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Agenda {
    /// The goal they push for
    pub objective: String,
    /// What they will never concede
    pub red_line: String,
    /// What would make them consider they won
    pub victory: String,
}

impl Agenda {
    pub fn is_empty(&self) -> bool {
        self.objective.is_empty() && self.red_line.is_empty() && self.victory.is_empty()
    }
}

/// An agenda unveiled at the end, with the synthesis' verdict when it gave one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgendaReveal {
    pub speaker_id: String,
    pub speaker_name: String,
    #[serde(flatten)]
    pub agenda: Agenda,
    /// Whether the objective was reached, when the synthesis said so
    pub achieved: Option<bool>,
}

/// Casting suggestion for a topic (v1.19): profiles chosen by the model.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CastingSuggestion {
    pub gladiateurs: Vec<CastingPick>,
    /// Suggested moderator profile id, when one was named
    pub arbitre: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CastingPick {
    pub id: String,
    pub reason: String,
}

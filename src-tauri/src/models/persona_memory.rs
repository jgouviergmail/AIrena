//! Long-term persona memory (v1.20): what a gladiateur keeps of a discussion,
//! written by a `Recap` call at the end and recalled, ranked by topic
//! similarity, when the same profile joins a later discussion.

use serde::{Deserialize, Serialize};

/// The structured recap of one participant (fields already trimmed and bounded).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonaRecap {
    /// Positions defended, in the persona's words
    #[serde(default)]
    pub positions: Vec<String>,
    /// Lines they are proud of
    #[serde(default)]
    pub best_lines: Vec<String>,
    #[serde(default)]
    pub allies: Vec<String>,
    #[serde(default)]
    pub rivals: Vec<String>,
    /// What they take away
    #[serde(default)]
    pub lesson: String,
}

impl PersonaRecap {
    pub fn is_empty(&self) -> bool {
        self.positions.is_empty() && self.best_lines.is_empty() && self.allies.is_empty() && self.rivals.is_empty() && self.lesson.is_empty()
    }

    /// Plain text of the recap (what the full-text ranking and the prompt block read).
    pub fn as_text(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        parts.extend(self.positions.iter().cloned());
        parts.push(self.lesson.clone());
        parts.extend(self.best_lines.iter().cloned());
        parts.into_iter().filter(|p| !p.is_empty()).collect::<Vec<_>>().join(" ")
    }
}

/// A recap emitted by the engine, attached to the profile the speaker came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonaRecapRecord {
    pub speaker_id: String,
    pub speaker_name: String,
    pub profile_id: String,
    pub recap: PersonaRecap,
}

/// A stored memory, as recalled for a new discussion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonaMemory {
    pub id: String,
    pub profile_id: String,
    pub discussion_id: String,
    pub topic: String,
    pub created_at: String,
    pub recap: PersonaRecap,
}

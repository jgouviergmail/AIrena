//! Pre-speech contract of a participant (v1.17): whom they address, what they
//! aim for and which angle they take — decided before the intervention so the
//! spoken text can be checked against it (bench, diagnostics).

use serde::{Deserialize, Serialize};

/// What the speaker tries to achieve with the coming intervention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum IntentionGoal {
    Convince,
    Nuance,
    Contest,
    Question,
    Concede,
    /// Bring the discussion back to life / move it forward (also the fallback)
    #[default]
    Relaunch,
}

impl IntentionGoal {
    pub const ALL: [IntentionGoal; 6] = [
        IntentionGoal::Convince,
        IntentionGoal::Nuance,
        IntentionGoal::Contest,
        IntentionGoal::Question,
        IntentionGoal::Concede,
        IntentionGoal::Relaunch,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Convince => "convince",
            Self::Nuance => "nuance",
            Self::Contest => "contest",
            Self::Question => "question",
            Self::Concede => "concede",
            Self::Relaunch => "relaunch",
        }
    }

    /// Parse the model's value (trilingual synonyms, case-insensitive); unknown → `Relaunch`.
    pub fn parse(value: &str) -> Self {
        let v = value.trim().to_lowercase();
        match v.as_str() {
            "convince" | "convaincre" | "persuade" | "说服" => Self::Convince,
            "nuance" | "nuancer" | "qualify" | "细化" | "补充" => Self::Nuance,
            "contest" | "contester" | "challenge" | "refute" | "réfuter" | "反驳" | "质疑" => Self::Contest,
            "question" | "questionner" | "interroger" | "ask" | "提问" => Self::Question,
            "concede" | "concéder" | "conceder" | "acknowledge" | "让步" | "承认" => Self::Concede,
            _ => Self::Relaunch,
        }
    }

    /// Verb shown to the model / the user, in the discussion language.
    pub fn label(&self, lang: &str) -> &'static str {
        match (self, lang) {
            (Self::Convince, "en") => "convince",
            (Self::Convince, "zh") => "说服",
            (Self::Convince, _) => "convaincre",
            (Self::Nuance, "en") => "nuance",
            (Self::Nuance, "zh") => "细化",
            (Self::Nuance, _) => "nuancer",
            (Self::Contest, "en") => "contest",
            (Self::Contest, "zh") => "反驳",
            (Self::Contest, _) => "contester",
            (Self::Question, "en") => "question",
            (Self::Question, "zh") => "提问",
            (Self::Question, _) => "questionner",
            (Self::Concede, "en") => "concede",
            (Self::Concede, "zh") => "让步",
            (Self::Concede, _) => "concéder",
            (Self::Relaunch, "en") => "relaunch",
            (Self::Relaunch, "zh") => "推进",
            (Self::Relaunch, _) => "relancer",
        }
    }
}

/// A parsed, validated intention. Text fields are already trimmed and bounded.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Intention {
    /// Participant addressed (exact display name); `None` = the topic itself
    pub target: Option<String>,
    pub goal: IntentionGoal,
    /// The angle / main idea of the coming intervention
    pub angle: String,
    /// A point the speaker is ready to grant
    pub concession: Option<String>,
    /// A question the speaker wants answered (opens a loop on the target)
    pub question: Option<String>,
    /// 1-based index of the open loop the speaker answers, when declared
    pub answers: Option<usize>,
    /// In-character private reflection (1–2 sentences) — becomes the persona thought
    pub thought: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn goal_parsing_is_trilingual_and_falls_back_to_relaunch() {
        assert_eq!(IntentionGoal::parse(" Convaincre "), IntentionGoal::Convince);
        assert_eq!(IntentionGoal::parse("challenge"), IntentionGoal::Contest);
        assert_eq!(IntentionGoal::parse("让步"), IntentionGoal::Concede);
        assert_eq!(IntentionGoal::parse("interroger"), IntentionGoal::Question);
        assert_eq!(IntentionGoal::parse("whatever"), IntentionGoal::Relaunch);
        assert_eq!(IntentionGoal::parse(""), IntentionGoal::Relaunch);
        for goal in IntentionGoal::ALL {
            assert_eq!(IntentionGoal::parse(goal.as_str()), goal, "round trip {goal:?}");
            assert_eq!(IntentionGoal::parse(goal.label("fr")), goal, "fr label {goal:?}");
            assert!(!goal.label("zh").is_empty());
        }
        assert_eq!(serde_json::to_string(&IntentionGoal::Relaunch).unwrap(), "\"relaunch\"");
    }
}

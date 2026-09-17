//! Mode-specific result of a discussion (v1.19): the jury's verdict of a
//! trial, the agreement of a negotiation, the audience swing of an Oxford
//! debate. Persisted by the frontend in `report_json.outcome`.

use serde::{Deserialize, Serialize};

/// Sides of a trial verdict (serialised as the frontend and the parser expect).
pub const VERDICT_PROSECUTION: &str = "prosecution";
pub const VERDICT_DEFENSE: &str = "defense";
/// Sides of the Oxford motion, as the audience votes them.
pub const VOTE_FOR: &str = "for";
pub const VOTE_AGAINST: &str = "against";

/// One juror's verdict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerdictVote {
    pub voter_id: String,
    pub voter_name: String,
    /// `VERDICT_PROSECUTION` or `VERDICT_DEFENSE`
    pub choice: String,
    pub reason: String,
}

/// One negotiating party's decision on the deal on the table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartyDecision {
    pub party_id: String,
    pub party_name: String,
    pub accepts: bool,
    pub reason: String,
}

/// When the audience votes in an Oxford debate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VotePhase {
    /// Before the first turn
    Before,
    /// After the last turn
    After,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ModeOutcome {
    /// Trial: the jurors' votes (or the moderator's when there is no jury), the majority side
    #[serde(rename_all = "camelCase")]
    Verdict { votes: Vec<VerdictVote>, winner: Option<String>, by_arbitre: bool },
    /// Negotiation: every party's decision; reached when all accept
    #[serde(rename_all = "camelCase")]
    Agreement { parties: Vec<PartyDecision>, reached: bool },
    /// Oxford debate: the audience's vote before and after, the side that moved it
    #[serde(rename_all = "camelCase")]
    AudienceSwing { before: Option<String>, after: Option<String>, winner: Option<String> },
}

impl ModeOutcome {
    /// Majority of the votes; `None` on a tie or without votes.
    pub fn majority(votes: &[VerdictVote]) -> Option<String> {
        let prosecution = votes.iter().filter(|v| v.choice == VERDICT_PROSECUTION).count();
        let defense = votes.iter().filter(|v| v.choice == VERDICT_DEFENSE).count();
        match prosecution.cmp(&defense) {
            std::cmp::Ordering::Greater => Some(VERDICT_PROSECUTION.to_string()),
            std::cmp::Ordering::Less => Some(VERDICT_DEFENSE.to_string()),
            std::cmp::Ordering::Equal => None,
        }
    }

    /// The side that moved the audience: the "after" vote when it differs from
    /// the "before" one; nobody when the audience did not move or did not vote twice.
    pub fn swing_winner(before: Option<&str>, after: Option<&str>) -> Option<String> {
        match (before, after) {
            (Some(b), Some(a)) if a != b => Some(a.to_string()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vote(choice: &str) -> VerdictVote {
        VerdictVote { voter_id: "j".into(), voter_name: "J".into(), choice: choice.into(), reason: String::new() }
    }

    #[test]
    fn majority_and_swing_rules() {
        assert_eq!(ModeOutcome::majority(&[vote(VERDICT_PROSECUTION), vote(VERDICT_DEFENSE), vote(VERDICT_PROSECUTION)]).as_deref(), Some(VERDICT_PROSECUTION));
        assert_eq!(ModeOutcome::majority(&[vote(VERDICT_PROSECUTION), vote(VERDICT_DEFENSE)]), None);
        assert_eq!(ModeOutcome::majority(&[]), None);
        assert_eq!(ModeOutcome::swing_winner(Some(VOTE_FOR), Some(VOTE_AGAINST)).as_deref(), Some(VOTE_AGAINST));
        assert_eq!(ModeOutcome::swing_winner(Some(VOTE_FOR), Some(VOTE_FOR)), None);
        assert_eq!(ModeOutcome::swing_winner(None, Some(VOTE_FOR)), None);
        let json = serde_json::to_string(&ModeOutcome::AudienceSwing { before: Some(VOTE_FOR.into()), after: None, winner: None }).unwrap();
        assert!(json.contains(r#""kind":"audienceSwing""#));
    }
}

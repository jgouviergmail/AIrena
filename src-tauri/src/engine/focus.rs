//! Rotating conversational focus — who a speaker should primarily address.
//!
//! Without it, every gladiateur is nudged to "react to what was just said",
//! which makes the discussion converge on the first speaker of each turn
//! ("everyone against X"). The focus is drawn at random from weighted
//! candidates so that, within a turn, attention rotates across participants
//! and sometimes leaves the person-to-person axis altogether to push the topic.

use std::collections::HashSet;

use rand::distributions::WeightedIndex;
use rand::prelude::*;

use crate::constants;

/// Who the speaker should address in priority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Focus {
    /// Address this participant (name as displayed).
    Speaker(String),
    /// Advance the topic without answering anyone in particular.
    Topic,
}

impl Focus {
    pub fn speaker_name(&self) -> Option<&str> {
        match self {
            Self::Speaker(n) => Some(n.as_str()),
            Self::Topic => None,
        }
    }
}

/// Everything the weighting needs, precomputed by the engine.
pub struct FocusInputs<'a> {
    /// Other active participants (names), excluding the speaker.
    pub candidates: &'a [String],
    /// Participants who spoke during the previous turn.
    pub spoke_previous_turn: &'a HashSet<String>,
    /// Participants who already spoke during the current turn.
    pub spoke_this_turn: &'a HashSet<String>,
    /// Participants already chosen as focus by someone this turn.
    pub targeted_this_turn: &'a HashSet<String>,
    /// Participants with an established relationship (ally / rival / tense).
    pub related: &'a HashSet<String>,
    /// Participants the audience reacted to this turn (the room wants to hear more about them).
    pub audience_favoured: &'a HashSet<String>,
}

/// Weighted focus candidates. Deterministic — the random draw is separate.
///
/// - a participant who spoke recently and nobody targeted yet: strongly favoured
/// - a participant with an established relationship: favoured
/// - a participant who has not spoken recently: not a valid focus
/// - the topic itself: always an option
pub fn focus_weights(inputs: &FocusInputs<'_>) -> Vec<(Focus, u32)> {
    let mut out: Vec<(Focus, u32)> = Vec::with_capacity(inputs.candidates.len() + 1);
    for name in inputs.candidates {
        let spoke_recently = inputs.spoke_previous_turn.contains(name) || inputs.spoke_this_turn.contains(name);
        if !spoke_recently {
            continue;
        }
        let mut weight = constants::FOCUS_WEIGHT_BASE;
        if !inputs.targeted_this_turn.contains(name) {
            weight += constants::FOCUS_WEIGHT_UNTARGETED;
        }
        if inputs.related.contains(name) {
            weight += constants::FOCUS_WEIGHT_RELATIONSHIP;
        }
        if inputs.audience_favoured.contains(name) {
            weight += constants::FOCUS_WEIGHT_AUDIENCE;
        }
        out.push((Focus::Speaker(name.clone()), weight));
    }
    out.push((Focus::Topic, constants::FOCUS_WEIGHT_TOPIC));
    out
}

/// Draw one focus from weighted candidates.
pub fn draw_focus(weights: &[(Focus, u32)], rng: &mut impl Rng) -> Focus {
    let dist = WeightedIndex::new(weights.iter().map(|(_, w)| *w)).ok();
    match dist {
        Some(d) => weights[d.sample(rng)].0.clone(),
        None => Focus::Topic,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(names: &[&str]) -> HashSet<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn recent_untargeted_speakers_are_strongly_favoured() {
        let candidates = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let prev = set(&["A", "B", "C"]);
        let now = set(&[]);
        let targeted = set(&["A"]);
        let related = set(&[]);
        let audience = set(&["C"]);
        let w = focus_weights(&FocusInputs { candidates: &candidates, spoke_previous_turn: &prev, spoke_this_turn: &now, targeted_this_turn: &targeted, related: &related, audience_favoured: &audience });
        let get = |n: &str| w.iter().find(|(f, _)| f.speaker_name() == Some(n)).map(|(_, x)| *x).unwrap();
        assert_eq!(get("A"), constants::FOCUS_WEIGHT_BASE);
        assert_eq!(get("B"), constants::FOCUS_WEIGHT_BASE + constants::FOCUS_WEIGHT_UNTARGETED);
        assert_eq!(get("C"), get("B") + constants::FOCUS_WEIGHT_AUDIENCE, "the audience's favourite gets a bonus");
        assert!(get("B") > get("A") * 2, "already-targeted speakers must be much less likely");
        assert!(w.iter().any(|(f, x)| *f == Focus::Topic && *x == constants::FOCUS_WEIGHT_TOPIC));
    }

    #[test]
    fn silent_participants_are_not_candidates_and_relationships_add_weight() {
        let candidates = vec!["A".to_string(), "Silent".to_string()];
        let prev = set(&["A"]);
        let now = set(&[]);
        let targeted = set(&[]);
        let related = set(&["A"]);
        let w = focus_weights(&FocusInputs { candidates: &candidates, spoke_previous_turn: &prev, spoke_this_turn: &now, targeted_this_turn: &targeted, related: &related, audience_favoured: &set(&[]) });
        assert!(!w.iter().any(|(f, _)| f.speaker_name() == Some("Silent")));
        let a = w.iter().find(|(f, _)| f.speaker_name() == Some("A")).unwrap().1;
        assert_eq!(a, constants::FOCUS_WEIGHT_BASE + constants::FOCUS_WEIGHT_UNTARGETED + constants::FOCUS_WEIGHT_RELATIONSHIP);
    }

    #[test]
    fn draw_respects_weights_and_rotates() {
        // Over many draws, the untargeted speaker must dominate the targeted one,
        // and the topic must still come up.
        let candidates = vec!["A".to_string(), "B".to_string()];
        let prev = set(&["A", "B"]);
        let empty = set(&[]);
        let targeted = set(&["A"]);
        let w = focus_weights(&FocusInputs { candidates: &candidates, spoke_previous_turn: &prev, spoke_this_turn: &empty, targeted_this_turn: &targeted, related: &empty, audience_favoured: &empty });
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let (mut a, mut b, mut topic) = (0, 0, 0);
        for _ in 0..1000 {
            match draw_focus(&w, &mut rng) {
                Focus::Speaker(n) if n == "A" => a += 1,
                Focus::Speaker(_) => b += 1,
                Focus::Topic => topic += 1,
            }
        }
        assert!(b > a * 2, "B={b} A={a}");
        assert!(topic > 100 && topic < 500, "topic={topic}");
    }

    #[test]
    fn empty_candidates_fall_back_to_topic() {
        let w = focus_weights(&FocusInputs { candidates: &[], spoke_previous_turn: &set(&[]), spoke_this_turn: &set(&[]), targeted_this_turn: &set(&[]), related: &set(&[]), audience_favoured: &set(&[]) });
        assert_eq!(w.len(), 1);
        assert_eq!(draw_focus(&w, &mut rand::thread_rng()), Focus::Topic);
        assert_eq!(draw_focus(&[], &mut rand::thread_rng()), Focus::Topic);
    }
}

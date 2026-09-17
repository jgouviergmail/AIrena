//! Weighted relationship graph (v1.17): every approval or disapproval between
//! two participants adds one point to a directed score that decays at the end
//! of each turn, so an old rivalry fades unless it is fed again. The integer
//! counts are kept next to the scores for display.

use std::collections::{HashMap, HashSet};

use crate::engine::directive_builder::RelationshipLean;
use crate::engine::tuning::Tuning;
use crate::models::relationship::{RelationshipEdge, RelationshipKind, RelationshipTrend};

/// Directed `(from, to)` → `(positive, negative)` weighted scores, plus the net
/// score of every undirected edge as last emitted (for the trend).
#[derive(Debug, Default)]
pub struct RelationshipScores {
    scores: HashMap<(String, String), (f32, f32)>,
    counts: HashMap<(String, String), (u32, u32)>,
    last_net: HashMap<(String, String), f32>,
}

impl RelationshipScores {
    /// One approval (`positive`) or disapproval from `from` to `to`.
    pub fn record(&mut self, from: &str, to: &str, positive: bool) {
        let key = (from.to_string(), to.to_string());
        let score = self.scores.entry(key.clone()).or_insert((0.0, 0.0));
        let count = self.counts.entry(key).or_insert((0, 0));
        if positive {
            score.0 += 1.0;
            count.0 += 1;
        } else {
            score.1 += 1.0;
            count.1 += 1;
        }
    }

    /// End of turn: every score fades by `factor` (counts are untouched).
    pub fn decay(&mut self, factor: f32) {
        for (p, n) in self.scores.values_mut() {
            *p *= factor;
            *n *= factor;
        }
    }

    /// Weighted `(positive, negative)` from `from` to `to`.
    pub fn directed(&self, from: &str, to: &str) -> (f32, f32) {
        self.scores.get(&(from.to_string(), to.to_string())).copied().unwrap_or((0.0, 0.0))
    }

    /// Integer `(likes, dislikes)` from `from` to `to` (display).
    pub fn counts(&self, from: &str, to: &str) -> (u32, u32) {
        self.counts.get(&(from.to_string(), to.to_string())).copied().unwrap_or((0, 0))
    }

    /// Classified relationship of a pair from both directed scores.
    pub fn classify_pair(&self, a: &str, b: &str, tuning: &Tuning) -> Option<RelationshipKind> {
        let (ab_pos, ab_neg) = self.directed(a, b);
        let (ba_pos, ba_neg) = self.directed(b, a);
        classify_relationship(ab_pos, ab_neg, ba_pos, ba_neg, tuning)
    }

    /// Who carries the coldness between `a` and `b`, from `a`'s point of view
    /// (v1.20.5): the critic is the direction cold by `RELATIONSHIP_MUTUAL_MIN`
    /// while the other is not.
    pub fn lean_of(&self, a: &str, b: &str) -> RelationshipLean {
        let min_lean = crate::constants::RELATIONSHIP_MUTUAL_MIN;
        let (ab_pos, ab_neg) = self.directed(a, b);
        let (ba_pos, ba_neg) = self.directed(b, a);
        let (mine, theirs) = (ab_pos - ab_neg, ba_pos - ba_neg);
        if mine <= -min_lean && theirs > -min_lean {
            RelationshipLean::IAmTheCritic
        } else if theirs <= -min_lean && mine > -min_lean {
            RelationshipLean::TheyAreTheCritic
        } else {
            RelationshipLean::Mutual
        }
    }

    /// Undirected edge list for the UI graph, with the classified kind, the net
    /// warmth score and its trend since the previous call (which this updates).
    pub fn edges(&mut self, tuning: &Tuning) -> Vec<RelationshipEdge> {
        let mut seen: HashSet<(String, String)> = HashSet::new();
        let mut keys: Vec<(String, String)> = self.scores.keys().cloned().collect();
        keys.sort();
        let mut edges = Vec::new();
        for (from, to) in keys {
            let key = if from <= to { (from.clone(), to.clone()) } else { (to.clone(), from.clone()) };
            if !seen.insert(key.clone()) {
                continue;
            }
            let (a, b) = key.clone();
            let (ab_likes, ab_dislikes) = self.counts(&a, &b);
            let (ba_likes, ba_dislikes) = self.counts(&b, &a);
            let (ab_pos, ab_neg) = self.directed(&a, &b);
            let (ba_pos, ba_neg) = self.directed(&b, &a);
            let score = ab_pos - ab_neg + ba_pos - ba_neg;
            let trend = match self.last_net.get(&key) {
                Some(prev) if score - prev > crate::constants::RELATIONSHIP_TREND_EPSILON => RelationshipTrend::Warming,
                Some(prev) if prev - score > crate::constants::RELATIONSHIP_TREND_EPSILON => RelationshipTrend::Cooling,
                _ => RelationshipTrend::Stable,
            };
            self.last_net.insert(key, score);
            let kind = classify_relationship(ab_pos, ab_neg, ba_pos, ba_neg, tuning).map(|k| k.as_str().to_string());
            edges.push(RelationshipEdge { a, b, ab_likes, ab_dislikes, ba_likes, ba_dislikes, kind, score, trend: Some(trend) });
        }
        edges
    }
}

/// Classification on the **net** warmth of each direction (approvals minus
/// disapprovals). v1.20.5: the pair is judged on the sum of both directions,
/// each direction only needing to lean the right way (`RELATIONSHIP_MUTUAL_MIN`)
/// — mutual warmth → ally, mutual coldness with no fresh approval either way →
/// rival; one warm and one cold, or one direction cold on its own by
/// `relationship_tense_score` → tense; else nothing. A kind word from a rival
/// thaws the rivalry into a tension (the step toward reconciliation); an
/// alliance survives an occasional disapproval.
pub fn classify_relationship(my_pos: f32, my_neg: f32, their_pos: f32, their_neg: f32, tuning: &Tuning) -> Option<RelationshipKind> {
    let (mine, theirs) = (my_pos - my_neg, their_pos - their_neg);
    let min_lean = crate::constants::RELATIONSHIP_MUTUAL_MIN;
    let (low, high) = (mine.min(theirs), mine.max(theirs));
    let sum = mine + theirs;
    if low >= min_lean && sum >= tuning.relationship_ally_score {
        Some(RelationshipKind::Ally)
    } else if high <= -min_lean && sum <= -tuning.relationship_rival_score && my_pos.max(their_pos) < min_lean {
        Some(RelationshipKind::Rival)
    } else if (low <= -min_lean && high >= min_lean && high - low >= tuning.relationship_tense_score) || low <= -tuning.relationship_tense_score {
        Some(RelationshipKind::Tense)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scores_classify_decay_and_trend() {
        let tuning = Tuning::default();
        let mut rel = RelationshipScores::default();
        for _ in 0..2 {
            rel.record("g1", "g2", false);
            rel.record("g2", "g1", false);
        }
        assert_eq!(rel.classify_pair("g1", "g2", &tuning), Some(RelationshipKind::Rival));
        let edges = rel.edges(&tuning);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].kind.as_deref(), Some("rival"));
        assert_eq!((edges[0].ab_dislikes, edges[0].ba_dislikes), (2, 2));
        assert_eq!(edges[0].score, -4.0);
        assert_eq!(edges[0].trend, Some(RelationshipTrend::Stable), "first emission has no history");

        // Four quiet turns: the rivalry fades (0.85⁴ × 2 ≈ 1.04 per direction, sum 2.09 < 2.5), counts stay
        for _ in 0..4 {
            rel.decay(tuning.relationship_decay_per_turn);
        }
        assert_eq!(rel.classify_pair("g1", "g2", &tuning), None);
        let edges = rel.edges(&tuning);
        assert_eq!(edges[0].kind, None);
        assert_eq!(edges[0].ab_dislikes, 2, "integer counts are history");
        assert_eq!(edges[0].trend, Some(RelationshipTrend::Warming), "less negative = warming");

        // Feeding it again cools it down
        rel.record("g1", "g2", false);
        rel.record("g2", "g1", false);
        assert_eq!(rel.edges(&tuning)[0].trend, Some(RelationshipTrend::Cooling));
        // A small move is stable
        rel.decay(0.99);
        assert_eq!(rel.edges(&tuning)[0].trend, Some(RelationshipTrend::Stable));
    }

    /// v1.20.5 — judged on the sum of both directions, each merely leaning the
    /// right way: the sincere-reaction regime (half of the reactions neutral)
    /// still forms allies and rivals within a few turns, and a persistent critic
    /// makes a pair tense on their own.
    /// v1.20.5 — the lean names the critic of a tense pair.
    #[test]
    fn lean_names_the_critic() {
        let mut rel = RelationshipScores::default();
        for _ in 0..3 {
            rel.record("g1", "g2", false);
        }
        assert_eq!(rel.lean_of("g1", "g2"), RelationshipLean::IAmTheCritic);
        assert_eq!(rel.lean_of("g2", "g1"), RelationshipLean::TheyAreTheCritic);
        rel.record("g2", "g1", false);
        assert_eq!(rel.lean_of("g1", "g2"), RelationshipLean::Mutual, "both cold");
        assert_eq!(RelationshipScores::default().lean_of("a", "b"), RelationshipLean::Mutual);
    }

    #[test]
    fn classification_uses_net_warmth_of_the_pair() {
        let t = Tuning::default();
        assert_eq!(classify_relationship(2.0, 0.0, 2.0, 0.0, &t), Some(RelationshipKind::Ally));
        assert_eq!(classify_relationship(3.0, 1.0, 2.0, 0.0, &t), Some(RelationshipKind::Ally), "net 2 + 2 still allies");
        assert_eq!(classify_relationship(2.8, 0.0, 1.2, 0.0, &t), Some(RelationshipKind::Ally), "the real debate's 8 approvals for 1 disapproval");
        assert_eq!(classify_relationship(1.9, 0.0, 0.7, 0.0, &t), None, "sum above 2.5 but one side barely leans");
        assert_eq!(classify_relationship(3.5, 0.0, 0.2, 0.0, &t), None, "one-sided admiration is not an alliance");
        assert_eq!(classify_relationship(0.0, 2.0, 0.0, 2.0, &t), Some(RelationshipKind::Rival));
        assert_eq!(classify_relationship(0.0, 1.23, 0.0, 1.06, &t), None, "mutual criticism below the combined threshold");
        assert_eq!(classify_relationship(0.0, 1.4, 0.0, 1.2, &t), Some(RelationshipKind::Rival), "two disapprovals each way in memory");
        assert_eq!(classify_relationship(2.0, 0.0, 0.0, 2.0, &t), Some(RelationshipKind::Tense));
        assert_eq!(classify_relationship(0.0, 2.0, 2.0, 0.0, &t), Some(RelationshipKind::Tense));
        assert_eq!(classify_relationship(2.71, 0.0, 0.0, 0.52, &t), None, "one skeptical dislike against three approvals: not tense");
        assert_eq!(classify_relationship(0.0, 2.0, 0.0, 0.0, &t), Some(RelationshipKind::Tense), "a persistent critic alone makes the pair tense");
        assert_eq!(classify_relationship(0.0, 1.9, 0.0, 0.44, &t), None, "not yet");
        assert_eq!(classify_relationship(2.0, 2.0, 2.0, 2.0, &t), None, "as many approvals as disapprovals: nothing");
        // A kind word from a rival thaws the rivalry into a tension (the other is still the critic)
        assert_eq!(classify_relationship(1.0, 2.0, 0.0, 2.0, &t), Some(RelationshipKind::Tense));
        assert_eq!(classify_relationship(1.0, 2.5, 0.0, 2.5, &t), Some(RelationshipKind::Tense), "still cold on both sides, but an approval is in memory: not a rivalry");
        assert_eq!(classify_relationship(1.0, 2.0, 0.0, 1.5, &t), None, "both relent: nothing");
        // An alliance survives an occasional disapproval
        assert_eq!(classify_relationship(3.0, 0.85, 2.0, 0.0, &t), Some(RelationshipKind::Ally));
    }
}

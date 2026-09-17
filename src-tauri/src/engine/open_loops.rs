//! Open loops (v1.17): questions asked to a participant and commitments they
//! made, kept in front of them until they deal with them or the loop expires.
//!
//! Lifetime is counted in the *owner's own interventions* (`OPEN_LOOPS_TTL_TURNS`):
//! a banned participant does not speak, so their loops wait for their return.
//! Each speaker keeps at most `OPEN_LOOPS_MAX_PER_SPEAKER` loops (FIFO).

use std::collections::{HashMap, VecDeque};

use crate::constants;
use crate::engine::truncate_str;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenLoopKind {
    /// Someone asked the owner something
    Question,
    /// The owner granted a point / promised something
    Commitment,
    /// Someone countered the owner's thesis or argument (argument map, v1.20.1): an answer on the merits is expected
    Objection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenLoop {
    pub kind: OpenLoopKind,
    pub text: String,
    /// Display name of who opened the loop (the owner for a commitment)
    pub from_name: String,
    pub turn: u32,
    /// Own interventions left before the loop is dropped
    pub ttl: u8,
}

impl OpenLoop {
    pub fn new(kind: OpenLoopKind, text: &str, from_name: &str, turn: u32) -> Self {
        Self {
            kind,
            text: truncate_str(text.trim(), constants::OPEN_LOOP_TEXT_MAX_CHARS).to_string(),
            from_name: from_name.to_string(),
            turn,
            ttl: constants::OPEN_LOOPS_TTL_TURNS,
        }
    }
}

/// Open loops of every participant, keyed by speaker id.
#[derive(Debug, Default)]
pub struct OpenLoopRegistry {
    loops: HashMap<String, VecDeque<OpenLoop>>,
}

impl OpenLoopRegistry {
    /// Append a loop for `owner_id` (empty texts and exact duplicates are ignored;
    /// the oldest loop is dropped beyond the per-speaker cap).
    pub fn push(&mut self, owner_id: &str, open_loop: OpenLoop) {
        if open_loop.text.is_empty() {
            return;
        }
        let queue = self.loops.entry(owner_id.to_string()).or_default();
        if queue.iter().any(|l| l.kind == open_loop.kind && l.text == open_loop.text) {
            return;
        }
        queue.push_back(open_loop);
        while queue.len() > constants::OPEN_LOOPS_MAX_PER_SPEAKER {
            queue.pop_front();
        }
    }

    /// Loops currently in front of `owner_id`, oldest first.
    pub fn for_speaker(&self, owner_id: &str) -> Vec<&OpenLoop> {
        self.loops.get(owner_id).map(|q| q.iter().collect()).unwrap_or_default()
    }

    /// Drop the loop the owner declared answered (`index` is 1-based, as listed in the prompt).
    pub fn resolve(&mut self, owner_id: &str, index: usize) {
        if let Some(queue) = self.loops.get_mut(owner_id) {
            if index >= 1 && index <= queue.len() {
                queue.remove(index - 1);
            }
        }
    }

    /// The owner just spoke: every loop they were shown loses one life; expired ones go.
    pub fn tick(&mut self, owner_id: &str) {
        if let Some(queue) = self.loops.get_mut(owner_id) {
            for l in queue.iter_mut() {
                l.ttl = l.ttl.saturating_sub(1);
            }
            queue.retain(|l| l.ttl > 0);
        }
    }

    /// Number of loops kept for `owner_id`.
    #[cfg(test)]
    pub fn len(&self, owner_id: &str) -> usize {
        self.loops.get(owner_id).map_or(0, VecDeque::len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn question(text: &str, turn: u32) -> OpenLoop {
        OpenLoop::new(OpenLoopKind::Question, text, "Le Philosophe", turn)
    }

    #[test]
    fn loops_are_fifo_capped_and_deduplicated() {
        let mut reg = OpenLoopRegistry::default();
        reg.push("g1", question("", 1));
        assert_eq!(reg.len("g1"), 0, "empty text ignored");
        for i in 0..5 {
            reg.push("g1", question(&format!("Q{i} ?"), 1));
        }
        reg.push("g1", question("Q4 ?", 2));
        let texts: Vec<&str> = reg.for_speaker("g1").iter().map(|l| l.text.as_str()).collect();
        assert_eq!(texts, vec!["Q2 ?", "Q3 ?", "Q4 ?"], "oldest dropped, duplicate ignored");
        assert!(reg.for_speaker("unknown").is_empty());
        // Same text, other kind: kept (a commitment is not a question)
        reg.push("g1", OpenLoop::new(OpenLoopKind::Commitment, "Q4 ?", "Le Scientifique", 2));
        assert_eq!(reg.len("g1"), 3);
        assert_eq!(reg.for_speaker("g1")[2].kind, OpenLoopKind::Commitment);
    }

    #[test]
    fn ttl_counts_own_interventions_only_and_resolve_is_one_based() {
        let mut reg = OpenLoopRegistry::default();
        reg.push("g1", question("Pourquoi ?", 1));
        reg.push("g1", question("Comment ?", 1));
        // g2 speaking (ticking g2) never touches g1's loops — a banned g1 keeps them
        reg.tick("g2");
        assert_eq!(reg.len("g1"), 2);
        reg.resolve("g1", 1);
        assert_eq!(reg.for_speaker("g1")[0].text, "Comment ?");
        reg.resolve("g1", 0);
        reg.resolve("g1", 9);
        assert_eq!(reg.len("g1"), 1, "out-of-range indices are ignored");
        for _ in 0..constants::OPEN_LOOPS_TTL_TURNS {
            assert_eq!(reg.len("g1"), 1);
            reg.tick("g1");
        }
        assert_eq!(reg.len("g1"), 0, "gone after TTL own interventions");
    }

    #[test]
    fn texts_are_trimmed_and_bounded() {
        let long = "x".repeat(constants::OPEN_LOOP_TEXT_MAX_CHARS + 50);
        let l = OpenLoop::new(OpenLoopKind::Question, &format!("  {long}  "), "A", 3);
        assert_eq!(l.text.len(), constants::OPEN_LOOP_TEXT_MAX_CHARS);
        assert_eq!(l.ttl, constants::OPEN_LOOPS_TTL_TURNS);
    }
}

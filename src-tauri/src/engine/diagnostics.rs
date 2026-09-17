//! Engine-side counters behind `DiscussionDiagnostics` and the per-turn timer
//! behind `TurnTimings` (v1.17). Pure bookkeeping, no LLM.

use std::collections::BTreeMap;
use std::time::Instant;

use crate::models::diagnostics::{DiscussionDiagnostics, PhaseTiming, TurnTimings};
use crate::models::llm::CallKind;

/// Counters of what went wrong (or not) during a discussion.
#[derive(Debug, Default)]
pub struct DiagnosticsCounters {
    parse_failures: BTreeMap<String, u32>,
    refusals: u32,
    retries: u32,
    intentions_checked: u32,
    intentions_kept: u32,
}

impl DiagnosticsCounters {
    /// A structured answer of `kind` could not be parsed.
    pub fn note_parse_failure(&mut self, kind: CallKind) {
        *self.parse_failures.entry(call_kind_name(kind)).or_insert(0) += 1;
    }

    pub fn note_refusal(&mut self) {
        self.refusals += 1;
    }

    pub fn note_retry(&mut self) {
        self.retries += 1;
    }

    /// An intervention was checked against its declared target (`kept`: the target was named).
    pub fn note_intention(&mut self, kept: bool) {
        self.intentions_checked += 1;
        if kept {
            self.intentions_kept += 1;
        }
    }

    pub fn report(&self) -> DiscussionDiagnostics {
        DiscussionDiagnostics {
            json_parse_failures: self.parse_failures.clone(),
            refusals: self.refusals,
            retries: self.retries,
            intention_compliance: (self.intentions_checked > 0)
                .then(|| self.intentions_kept as f32 / self.intentions_checked as f32),
        }
    }
}

/// Wire name of a call kind ("argumentMap", "memory", …) — the same as the usage ledger keys.
fn call_kind_name(kind: CallKind) -> String {
    serde_json::to_value(kind)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| format!("{kind:?}"))
}

/// Wall-clock phases of the current turn.
#[derive(Debug, Default)]
pub struct TurnTimer {
    phases: Vec<PhaseTiming>,
    open: Option<(String, Instant)>,
}

impl TurnTimer {
    /// Forget the previous turn.
    pub fn reset(&mut self) {
        self.phases.clear();
        self.open = None;
    }

    /// Start timing a phase (closing any phase still open).
    pub fn start(&mut self, name: &str) {
        self.stop();
        self.open = Some((name.to_string(), Instant::now()));
    }

    /// Close the open phase, if any.
    pub fn stop(&mut self) {
        if let Some((name, started)) = self.open.take() {
            self.record(&name, started.elapsed().as_millis() as u64);
        }
    }

    /// Record a measured duration (phases run concurrently measure themselves).
    pub fn record(&mut self, name: &str, ms: u64) {
        self.phases.push(PhaseTiming { name: name.to_string(), ms });
    }

    pub fn finish(&mut self, turn: u32) -> TurnTimings {
        self.stop();
        TurnTimings { turn, phases: std::mem::take(&mut self.phases) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_report_wire_names_and_a_compliance_ratio() {
        let mut d = DiagnosticsCounters::default();
        assert_eq!(d.report(), DiscussionDiagnostics::default());
        d.note_parse_failure(CallKind::ArgumentMap);
        d.note_parse_failure(CallKind::ArgumentMap);
        d.note_parse_failure(CallKind::Memory);
        d.note_refusal();
        d.note_retry();
        d.note_retry();
        d.note_intention(true);
        d.note_intention(true);
        d.note_intention(false);
        let r = d.report();
        assert_eq!(r.json_parse_failures.get("argumentMap"), Some(&2));
        assert_eq!(r.json_parse_failures.get("memory"), Some(&1));
        assert_eq!((r.refusals, r.retries), (1, 2));
        assert!((r.intention_compliance.unwrap() - 2.0 / 3.0).abs() < 1e-6);
        assert!(serde_json::to_string(&r).unwrap().contains("\"jsonParseFailures\":{\"argumentMap\":2"));
    }

    #[test]
    fn timer_closes_open_phases_and_takes_measured_ones() {
        let mut t = TurnTimer::default();
        t.start("speakers");
        t.start("endOfTurn");
        t.record("memory", 42);
        let timings = t.finish(3);
        assert_eq!(timings.turn, 3);
        let names: Vec<&str> = timings.phases.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["speakers", "memory", "endOfTurn"]);
        assert_eq!(timings.phases[1].ms, 42);
        assert!(t.finish(4).phases.is_empty(), "taken");
    }
}

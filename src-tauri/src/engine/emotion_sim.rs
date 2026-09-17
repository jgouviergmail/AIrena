//! Deterministic simulations of the emotion model (v1.20.5): scripted streams of
//! reactions, analyst deltas and contagion over several turns, without any LLM,
//! asserting the balance properties the model promises — no saturation, real
//! sensitivity, variability, recovery, persona differentiation, bounded theatre.
//! `emotion_sim_report` (ignored) prints the trajectories for the audit journal.

use crate::constants;
use crate::engine::emotion_engine::*;
use crate::engine::tuning::Tuning;
use crate::models::emotion::{EmotionDelta, EmotionalProfile};

const AXES: [&str; 6] = ["engagement", "accord", "confiance", "frustration", "curiosite", "enthousiasme"];

fn axis(p: &EmotionalProfile, name: &str) -> u8 {
    match name {
        "engagement" => p.engagement,
        "accord" => p.accord,
        "confiance" => p.confiance,
        "frustration" => p.frustration,
        "curiosite" => p.curiosite,
        _ => p.enthousiasme,
    }
}

struct Speaker {
    name: String,
    baseline: EmotionalProfile,
    gains: PersonaGains,
    profile: EmotionalProfile,
    /// One snapshot after every step (intervention, analyst, contagion)
    trajectory: Vec<EmotionalProfile>,
    /// Absolute threshold crossings (85 / 15), as (axis, direction)
    crossings: Vec<(String, String)>,
    /// Relative crossings (notable zone entered, with hysteresis), as (axis, direction)
    movements: Vec<(String, String)>,
}

impl Speaker {
    fn theatre(&self) -> usize {
        self.crossings.len() + self.movements.len()
    }

    /// Times one axis fired (absolute or relative), the most fired axis.
    fn max_per_axis(&self) -> usize {
        AXES.iter().map(|a| self.crossings.iter().chain(self.movements.iter()).filter(|(axis, _)| axis == a).count()).max().unwrap_or(0)
    }
}

/// The engine's emotional pipeline over a cast, immediate timing: every
/// intervention receives one reaction round and applies the turn drift; the end
/// of each turn applies the analyst's deltas then the contagion.
struct Simulation {
    speakers: Vec<Speaker>,
    zones: ShiftZones,
}

impl Simulation {
    fn new(cast: &[(&str, EmotionalProfile, Option<[u8; 5]>)]) -> Self {
        let tuning = Tuning::default();
        Self {
            speakers: cast
                .iter()
                .map(|(name, baseline, ocean)| Speaker {
                    name: name.to_string(),
                    baseline: baseline.clone(),
                    gains: gains_from_ocean(*ocean, &tuning),
                    profile: baseline.clone(),
                    trajectory: vec![baseline.clone()],
                    crossings: Vec::new(),
                    movements: Vec::new(),
                })
                .collect(),
            zones: ShiftZones::default(),
        }
    }

    fn record(&mut self, i: usize, prev: &EmotionalProfile) {
        let current = self.speakers[i].profile.clone();
        self.speakers[i].crossings.extend(detect_thresholds(prev, &current).into_iter().map(|(a, d, _)| (a, d)));
        let name = self.speakers[i].name.clone();
        let baseline = self.speakers[i].baseline.clone();
        let moved = self.zones.crossings(&name, &current, &baseline);
        self.speakers[i].movements.extend(moved.into_iter().map(|(a, d, _)| (a, d)));
        self.speakers[i].trajectory.push(current);
    }

    /// One intervention of speaker `i`: the reactions it receives (one round), the
    /// ones it gave, then the per-intervention drift.
    fn intervention(&mut self, i: usize, received: ReactionTally, given: GivenReactions, stagnating: bool) {
        let prev = self.speakers[i].profile.clone();
        let gains = self.speakers[i].gains;
        let baseline = self.speakers[i].baseline.clone();
        let p = &mut self.speakers[i].profile;
        apply_received_reactions(p, &received, &gains);
        apply_given_reactions(p, &given);
        apply_turn_effects(p, &baseline, stagnating);
        self.record(i, &prev);
    }

    /// End of turn: the analyst's deltas (bounded and elastic), then the contagion.
    fn end_of_turn(&mut self, deltas: &[EmotionDelta]) {
        for (i, d) in deltas.iter().enumerate() {
            let prev = self.speakers[i].profile.clone();
            apply_llm_delta(&mut self.speakers[i].profile, &clamp_delta(d, constants::EMOTION_LLM_DELTA_CAP));
            self.record(i, &prev);
        }
        let profiles: Vec<&EmotionalProfile> = self.speakers.iter().map(|s| &s.profile).collect();
        if profiles.len() >= 2 {
            let avg = compute_average(&profiles);
            for s in &mut self.speakers {
                apply_contagion(&avg, &mut s.profile);
                s.trajectory.push(s.profile.clone());
            }
        }
    }

    /// A whole turn where every speaker intervenes once with the same script.
    fn turn(&mut self, received: impl Fn(usize, usize) -> ReactionTally, given: GivenReactions, delta: EmotionDelta, stagnating: bool) {
        let turn_index = self.speakers[0].trajectory.len();
        for i in 0..self.speakers.len() {
            self.intervention(i, received(i, turn_index), given, stagnating);
        }
        let deltas = vec![delta; self.speakers.len()];
        self.end_of_turn(&deltas);
    }

    fn series(&self, i: usize, name: &str) -> Vec<u8> {
        self.speakers[i].trajectory.iter().map(|p| axis(p, name)).collect()
    }

    fn peak(&self) -> u8 {
        self.speakers.iter().flat_map(|s| s.trajectory.iter()).flat_map(|p| AXES.iter().map(move |a| axis(p, a))).max().unwrap_or(0)
    }

    /// Lowest value over time on the axes that start at 50 (frustration starts at 10).
    fn floor(&self) -> u8 {
        self.speakers.iter().flat_map(|s| s.trajectory.iter()).flat_map(|p| AXES.iter().filter(|a| **a != "frustration").map(move |a| axis(p, a))).min().unwrap_or(0)
    }
}

/// Number of times a series changes direction (rises after falling or the reverse).
fn direction_changes(series: &[u8]) -> usize {
    let mut last: i8 = 0;
    let mut changes = 0;
    for w in series.windows(2) {
        let sign = (i16::from(w[1]) - i16::from(w[0])).signum() as i8;
        if sign != 0 {
            if last != 0 && sign != last {
                changes += 1;
            }
            last = sign;
        }
    }
    changes
}

fn tally(likes: u32, dislikes: u32, insightful: u32, questions: u32, laughs: u32) -> ReactionTally {
    ReactionTally { likes, dislikes, insightful, questions, off_topic: 0, laughs }
}

/// The polite chorus of the real debate before v1.20.4: three 💡 per intervention.
fn chorus() -> ReactionTally {
    tally(3, 0, 3, 0, 0)
}

/// Three opponents disapproving every intervention.
fn hostile() -> ReactionTally {
    tally(0, 3, 0, 0, 0)
}

/// Sincere reactions: mostly nothing, sometimes one opinion.
fn sincere(step: usize) -> ReactionTally {
    match step % 8 {
        0 => tally(1, 0, 0, 0, 0),
        2 => tally(0, 1, 0, 0, 0),
        3 => tally(0, 0, 0, 1, 0),
        5 => tally(1, 0, 1, 0, 0),
        6 => tally(0, 0, 0, 0, 1),
        7 => tally(0, 2, 0, 0, 0),
        _ => ReactionTally::default(),
    }
}

fn delta(engagement: i8, accord: i8, confiance: i8, frustration: i8, curiosite: i8, enthousiasme: i8) -> EmotionDelta {
    EmotionDelta { engagement, accord, confiance, frustration, curiosite, enthousiasme }
}

fn neutral_cast(n: usize) -> Vec<(&'static str, EmotionalProfile, Option<[u8; 5]>)> {
    ["A", "B", "C", "D"].iter().take(n).map(|name| (*name, EmotionalProfile::default(), None)).collect()
}

const TURNS: usize = 8;

/// The pathology of the real debate (three 💡 per intervention, a flattering
/// analyst, approvals given every turn) no longer saturates: the peak stays
/// under the ceiling, it takes turns to get high, and the theatre stays bounded.
#[test]
fn polite_chorus_no_longer_saturates() {
    let mut sim = Simulation::new(&neutral_cast(4));
    for _ in 0..TURNS {
        sim.turn(|_, _| chorus(), GivenReactions { net: 3, laughs: 0 }, delta(3, 2, 3, 0, 0, 2), false);
    }
    assert!(sim.peak() <= 88, "peak {}", sim.peak());
    let confiance = sim.series(0, "confiance");
    let first_above_80 = confiance.iter().position(|v| *v > 80).unwrap_or(usize::MAX);
    assert!(first_above_80 >= 6, "reaching 80 takes at least two whole turns (3 steps each): step {first_above_80} of {confiance:?}");
    assert!(*confiance.last().unwrap() >= 75, "eight turns of unanimous praise are felt: {confiance:?}");
    for s in &sim.speakers {
        // Four axes climb: each enters its zone once (hysteresis), plus at most one absolute crossing
        assert!(s.theatre() <= 6 && s.max_per_axis() <= 2, "{}: {:?} {:?}", s.name, s.crossings, s.movements);
    }
}

/// Three opponents disapproving every intervention for eight turns: the
/// frustration is felt fast, plateaus well under the ceiling, the confiance
/// never collapses, and the theatre fires — once or twice, not every turn.
#[test]
fn hostile_crowd_plateaus_and_reaches_the_theatre() {
    let mut sim = Simulation::new(&neutral_cast(4));
    for _ in 0..TURNS {
        sim.turn(|_, _| hostile(), GivenReactions { net: -2, laughs: 0 }, delta(0, -2, -3, 3, 0, -2), false);
    }
    let frustration = sim.series(0, "frustration");
    assert!(frustration[3 * 3] >= 40, "felt within three turns: {frustration:?}");
    assert!(sim.peak() <= 82, "peak {} — {frustration:?}", sim.peak());
    assert!(sim.floor() >= 18, "floor {} — confiance {:?}", sim.floor(), sim.series(0, "confiance"));
    // Plateau: the last two turns move by less than 5 points
    let n = frustration.len();
    assert!(frustration[n - 1].abs_diff(frustration[n - 7]) <= 5, "plateau: {frustration:?}");
    for s in &sim.speakers {
        assert!(s.theatre() >= 1, "{}: the storm shows on stage", s.name);
        assert!(s.theatre() <= 6 && s.max_per_axis() <= 2, "{}: {:?} {:?}", s.name, s.crossings, s.movements);
    }
}

/// Sincere reactions (mostly none, one opinion now and then) with a balanced
/// analyst: every axis stays mid-range, keeps moving both ways, and the theatre
/// stays rare.
#[test]
fn sincere_reactions_keep_axes_mid_range_and_alive() {
    let mut sim = Simulation::new(&neutral_cast(4));
    for t in 0..TURNS {
        let d = if t % 2 == 0 { delta(3, -2, 2, 1, 3, 2) } else { delta(-2, 3, -3, -1, -2, -3) };
        sim.turn(|i, step| sincere(step + i), GivenReactions { net: if t % 3 == 0 { 1 } else { -1 }, laughs: 0 }, d, t % 4 == 3);
    }
    assert!(sim.peak() <= 75 && sim.floor() >= 30, "range {}..={}", sim.floor(), sim.peak());
    for (i, s) in sim.speakers.iter().enumerate() {
        let confiance = sim.series(i, "confiance");
        assert!(direction_changes(&confiance) >= 3, "{}: confiance must go both ways: {confiance:?}", s.name);
        assert!(s.theatre() <= 3, "{}: {:?} {:?}", s.name, s.crossings, s.movements);
    }
}

/// The same storm hurts a neurotic persona more than a stable one at every step,
/// and the same praise lifts an agreeable one more — the differences survive
/// the caps and the resistance.
#[test]
fn ocean_sensitivity_orders_the_trajectories() {
    let cast = [
        ("nerveux", EmotionalProfile::default(), Some([5, 5, 5, 5, 9])),
        ("neutre", EmotionalProfile::default(), None),
        ("stable", EmotionalProfile::default(), Some([5, 5, 5, 5, 3])),
    ];
    let mut sim = Simulation::new(&cast);
    for _ in 0..TURNS {
        sim.turn(|_, _| hostile(), GivenReactions::default(), delta(0, 0, 0, 0, 0, 0), false);
    }
    let (nervous, neutral, stable) = (sim.series(0, "frustration"), sim.series(1, "frustration"), sim.series(2, "frustration"));
    for step in 1..nervous.len() {
        assert!(nervous[step] >= neutral[step] && neutral[step] >= stable[step], "step {step}: {nervous:?} {neutral:?} {stable:?}");
    }
    assert!(nervous.last().unwrap() - stable.last().unwrap() >= 5, "{nervous:?} vs {stable:?}");

    let cast = [("agréable", EmotionalProfile::default(), Some([5, 5, 5, 9, 5])), ("neutre", EmotionalProfile::default(), None)];
    let mut sim = Simulation::new(&cast);
    for _ in 0..4 {
        sim.turn(|_, _| tally(2, 0, 1, 0, 0), GivenReactions::default(), delta(0, 0, 0, 0, 0, 0), false);
    }
    let (agreeable, neutral) = (sim.series(0, "confiance"), sim.series(1, "confiance"));
    assert!(agreeable.last().unwrap() > neutral.last().unwrap(), "{agreeable:?} vs {neutral:?}");
}

/// After four hostile turns, six quiet ones bring everyone most of the way back
/// toward their baseline — grudges fade, they do not vanish at once.
#[test]
fn recovery_after_a_storm() {
    let mut sim = Simulation::new(&neutral_cast(3));
    for _ in 0..4 {
        sim.turn(|_, _| hostile(), GivenReactions { net: -2, laughs: 0 }, delta(0, -2, -3, 3, 0, 0), false);
    }
    let peak_frustration = *sim.series(0, "frustration").iter().max().unwrap();
    let low_confiance = *sim.series(0, "confiance").iter().min().unwrap();
    for _ in 0..6 {
        sim.turn(|_, _| ReactionTally::default(), GivenReactions::default(), delta(0, 0, 0, 0, 0, 0), false);
    }
    let end = sim.speakers[0].profile.clone();
    let baseline = &sim.speakers[0].baseline;
    assert!(end.frustration <= baseline.frustration + (peak_frustration - baseline.frustration) / 2, "frustration {} after a peak of {peak_frustration}", end.frustration);
    assert!(end.frustration > baseline.frustration, "not forgotten either");
    assert!(end.confiance >= baseline.confiance - (baseline.confiance - low_confiance) / 2, "confiance {} after a low of {low_confiance}", end.confiance);
}

/// A persona confident and hot-headed by nature keeps its temperament: praise
/// lifts it a little, quiet turns bring it back to ITS baseline, never to 50.
#[test]
fn extreme_persona_keeps_its_temperament() {
    let baseline = EmotionalProfile { confiance: 85, frustration: 60, enthousiasme: 30, ..Default::default() };
    let mut sim = Simulation::new(&[("sanguin", baseline.clone(), None), ("témoin", EmotionalProfile::default(), None)]);
    for _ in 0..6 {
        sim.turn(|i, _| if i == 0 { chorus() } else { ReactionTally::default() }, GivenReactions::default(), delta(0, 0, 0, 0, 0, 0), false);
    }
    let confiance = sim.series(0, "confiance");
    assert!(*confiance.iter().max().unwrap() <= 95, "{confiance:?}");
    let peak_enthusiasm = *sim.series(0, "enthousiasme").iter().max().unwrap();
    for _ in 0..6 {
        sim.turn(|_, _| ReactionTally::default(), GivenReactions::default(), delta(0, 0, 0, 0, 0, 0), false);
    }
    let end = &sim.speakers[0].profile;
    assert!(end.confiance.abs_diff(baseline.confiance) <= 3, "back to 85, not 50: {}", end.confiance);
    assert!(end.frustration >= 55, "hot-headed stays hot-headed: {}", end.frustration);
    assert!(end.enthousiasme <= baseline.enthousiasme + (peak_enthusiasm - baseline.enthousiasme) / 2, "calm comes back to calm: {} after a peak of {peak_enthusiasm}", end.enthousiasme);
}

/// The moderator's path (analyst deltas + homeostasis toward neutral, no
/// reactions): twelve turns of a flattering analyst stay bounded.
#[test]
fn moderator_stays_bounded_under_a_flattering_analyst() {
    let mut p = EmotionalProfile::default();
    for _ in 0..12 {
        apply_llm_delta(&mut p, &delta(4, 0, 3, 0, 2, 0));
        apply_turn_effects(&mut p, &EmotionalProfile::default(), false);
    }
    assert!(p.engagement <= 72 && p.confiance <= 68, "{p:?} (raw: 98 / 86)");
    assert!(p.engagement >= 58, "still felt: {p:?}");
}

/// Trajectories for the audit journal (`cargo test --lib emotion_sim_report -- --ignored --nocapture`).
#[test]
#[ignore]
fn emotion_sim_report() {
    let print = |title: &str, sim: &Simulation, axes: &[&str]| {
        println!("\n### {title}\n");
        for a in axes {
            let s = sim.series(0, a);
            let per_turn: Vec<String> = s.iter().step_by(3).map(|v| v.to_string()).collect();
            println!("- {a} (par tour) : {}", per_turn.join(" → "));
        }
        for s in &sim.speakers {
            println!("- {} : {} franchissement(s) absolu(s) {:?}, {} mouvement(s) {:?}", s.name, s.crossings.len(), s.crossings, s.movements.len(), s.movements);
        }
    };
    let mut sim = Simulation::new(&neutral_cast(4));
    for _ in 0..TURNS {
        sim.turn(|_, _| chorus(), GivenReactions { net: 3, laughs: 0 }, delta(3, 2, 3, 0, 0, 2), false);
    }
    print("Chœur poli (3 💡 par intervention, analyste flatteur)", &sim, &["confiance", "engagement", "accord", "enthousiasme"]);
    let mut sim = Simulation::new(&neutral_cast(4));
    for _ in 0..TURNS {
        sim.turn(|_, _| hostile(), GivenReactions { net: -2, laughs: 0 }, delta(0, -2, -3, 3, 0, -2), false);
    }
    print("Foule hostile (3 👎 par intervention)", &sim, &["frustration", "confiance", "accord"]);
    let mut sim = Simulation::new(&neutral_cast(4));
    for t in 0..TURNS {
        let d = if t % 2 == 0 { delta(3, -2, 2, 1, 3, 2) } else { delta(-2, 3, -3, -1, -2, -3) };
        sim.turn(|i, step| sincere(step + i), GivenReactions { net: if t % 3 == 0 { 1 } else { -1 }, laughs: 0 }, d, t % 4 == 3);
    }
    print("Réactions sincères (surtout aucune, une opinion de temps en temps)", &sim, &["confiance", "frustration", "engagement", "curiosite"]);
}

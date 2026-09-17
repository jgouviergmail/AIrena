use std::collections::HashMap;

use crate::constants;
use crate::engine::tuning::Tuning;
use crate::models::emotion::RoomMood;
use crate::models::settings::LlmParams;
use crate::models::emotion::{EmotionDelta, EmotionalProfile};
use crate::models::message::ReactionType;

/// Reactions received on one intervention, by colour. `likes` / `dislikes`
/// are the positive / negative classes (like + insightful, dislike + off-topic);
/// the typed counters add their specific effects on top.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReactionTally {
    pub likes: u32,
    pub dislikes: u32,
    pub insightful: u32,
    pub questions: u32,
    pub off_topic: u32,
    pub laughs: u32,
}

impl ReactionTally {
    pub fn add(&mut self, kind: ReactionType) {
        if kind.is_positive() {
            self.likes += 1;
        } else if kind.is_negative() {
            self.dislikes += 1;
        }
        match kind {
            ReactionType::Insightful => self.insightful += 1,
            ReactionType::Question => self.questions += 1,
            ReactionType::OffTopic => self.off_topic += 1,
            ReactionType::Laugh => self.laughs += 1,
            ReactionType::Like | ReactionType::Dislike => {}
        }
    }

    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// What was received since `before` (a reset in between counts as nothing).
    pub fn since(&self, before: &Self) -> Self {
        Self {
            likes: self.likes.saturating_sub(before.likes),
            dislikes: self.dislikes.saturating_sub(before.dislikes),
            insightful: self.insightful.saturating_sub(before.insightful),
            questions: self.questions.saturating_sub(before.questions),
            off_topic: self.off_topic.saturating_sub(before.off_topic),
            laughs: self.laughs.saturating_sub(before.laughs),
        }
    }

    /// Heuristic: contradiction = dislikes >= threshold
    pub fn was_contradicted(&self) -> bool {
        self.dislikes >= constants::EMOTION_CONTRADICTION_THRESHOLD
    }

    /// Heuristic: support = likes >= threshold
    pub fn was_supported(&self) -> bool {
        self.likes >= constants::EMOTION_SUPPORT_THRESHOLD
    }
}

/// Reactions a speaker just gave (their own stance shows in `accord` and `enthousiasme`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GivenReactions {
    /// likes − dislikes
    pub net: i32,
    pub laughs: u32,
}

impl GivenReactions {
    pub fn add(&mut self, kind: ReactionType) {
        if kind.is_positive() {
            self.net += 1;
        } else if kind.is_negative() {
            self.net -= 1;
        }
        if kind == ReactionType::Laugh {
            self.laughs += 1;
        }
    }
}

/// Context for the rule-based emotion update run after each intervention
/// (deferred reactions: received on the previous message, given just now).
pub struct EmotionContext {
    pub received: ReactionTally,
    pub given: GivenReactions,
    /// Real stagnation signal computed by the engine (summary similarity,
    /// reaction drought or LLM flag) — never a turn-number heuristic.
    pub is_discussion_stagnating: bool,
}

/// Diminishing returns (v1.20.4): the n-th reaction of a kind weighs 1/n of the
/// first (harmonic sum — 1, 1.5, 1.83, 2.08…), then the cap. Three polite
/// "insightful" no longer count as three standing ovations.
fn scaled(factor: u16, count: u32, cap: u16) -> u8 {
    let harmonic: f32 = (1..=count.min(64)).map(|n| 1.0 / n as f32).sum();
    ((f32::from(factor) * harmonic).round() as u16).min(cap) as u8
}

/// Share of a delta that applies at `value` (v1.20.4): 1.0 inside the comfort
/// band, then down to `EMOTION_EXTREME_RESISTANCE` at the extreme the delta
/// pushes toward, quadratically (v1.20.5: the brake bites early) — an axis near
/// 100 barely rises, an axis near 0 barely falls, and both come back easily.
pub fn resistance(value: u8, increasing: bool) -> f32 {
    let high = f32::from(constants::EMOTION_COMFORT_HIGH);
    let low = 100.0 - high;
    let v = f32::from(value);
    let span = 100.0 - high;
    let excess = if increasing { (v - high).max(0.0) } else { (low - v).max(0.0) };
    if excess <= 0.0 {
        return 1.0;
    }
    let remaining = 1.0 - (excess / span).min(1.0);
    constants::EMOTION_EXTREME_RESISTANCE + (1.0 - constants::EMOTION_EXTREME_RESISTANCE) * remaining * remaining
}

/// Apply a signed delta to one axis through the resistance (rounded; a delta of at
/// least one point always moves the axis by at least one point).
pub fn elastic(value: u8, delta: i16) -> u8 {
    if delta == 0 {
        return value;
    }
    let increasing = delta > 0;
    let effective = (f32::from(delta) * resistance(value, increasing)).round() as i16;
    let effective = if effective == 0 { delta.signum() } else { effective };
    (i16::from(value) + effective).clamp(0, 100) as u8
}

/// Net deltas of one event on the six axes, applied together: capped per axis
/// (`EMOTION_ROUND_AXIS_CAP`) then passed through the resistance.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct AxisDeltas {
    pub engagement: i16,
    pub accord: i16,
    pub confiance: i16,
    pub frustration: i16,
    pub curiosite: i16,
    pub enthousiasme: i16,
}

impl AxisDeltas {
    /// Apply to `profile`, each axis capped to ±cap then made elastic.
    pub fn apply(&self, profile: &mut EmotionalProfile, cap: u8) {
        self.apply_felt(self, profile, cap);
    }

    /// Apply `felt` (these deltas scaled by the persona's gains) to `profile`:
    /// the round cap bounds the raw event (`self`) and scales with the persona's
    /// sensitivity, so a nervous persona still feels a capped blow harder than
    /// a stable one — and a neutral persona feels exactly the raw event.
    pub fn apply_felt(&self, felt: &AxisDeltas, profile: &mut EmotionalProfile, cap: u8) {
        let cap = i16::from(cap);
        let bounded = |raw: i16, felt: i16| -> i16 {
            if raw == 0 {
                return 0;
            }
            let capped = raw.clamp(-cap, cap);
            (f32::from(capped) * f32::from(felt) / f32::from(raw)).round() as i16
        };
        profile.engagement = elastic(profile.engagement, bounded(self.engagement, felt.engagement));
        profile.accord = elastic(profile.accord, bounded(self.accord, felt.accord));
        profile.confiance = elastic(profile.confiance, bounded(self.confiance, felt.confiance));
        profile.frustration = elastic(profile.frustration, bounded(self.frustration, felt.frustration));
        profile.curiosite = elastic(profile.curiosite, bounded(self.curiosite, felt.curiosite));
        profile.enthousiasme = elastic(profile.enthousiasme, bounded(self.enthousiasme, felt.enthousiasme));
    }
}

/// The largest move away from the persona's baseline, when it reaches
/// `EMOTION_NOTABLE_SHIFT`: the axis name (as in `detect_thresholds`) and the
/// signed shift. What the debate did to the speaker, as opposed to who they are.
pub fn dominant_shift(current: &EmotionalProfile, baseline: &EmotionalProfile) -> Option<(&'static str, i16)> {
    axes_with_baseline(current, baseline)
        .into_iter()
        .map(|(name, c, b)| (name, i16::from(c) - i16::from(b)))
        .filter(|(_, shift)| shift.unsigned_abs() >= u16::from(constants::EMOTION_NOTABLE_SHIFT))
        .max_by_key(|(_, shift)| shift.unsigned_abs())
}

/// How strongly a persona feels the rule effects (v1.17). All 1.0 = the v1.16
/// deltas exactly; derived from OCEAN by `gains_from_ocean`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PersonaGains {
    /// Neuroticism: how much a disapproval or a contradiction hurts
    pub dislike_sensitivity: f32,
    /// Agreeableness: how much an approval or support pleases
    pub like_sensitivity: f32,
    /// Openness: how much a question stirs curiosity
    pub curiosity_gain: f32,
    /// Extraversion: how much attention raises engagement
    pub engagement_gain: f32,
}

impl Default for PersonaGains {
    fn default() -> Self {
        Self { dislike_sensitivity: 1.0, like_sensitivity: 1.0, curiosity_gain: 1.0, engagement_gain: 1.0 }
    }
}

/// One OCEAN trait → gain: neutral (1.0) between the extremes, then linear to the bounds.
fn trait_gain(value: u8, tuning: &Tuning) -> f32 {
    let (low, high) = (constants::OCEAN_EXTREME_LOW, constants::OCEAN_EXTREME_HIGH);
    if value >= high {
        let steps = (10 - high + 1) as f32; // 8, 9, 10 → 3 steps up to the max
        1.0 + (tuning.ocean_gain_max - 1.0) * (value - high + 1) as f32 / steps
    } else if value <= low {
        let steps = low as f32; // 3, 2, 1 → 3 steps down to the min
        1.0 - (1.0 - tuning.ocean_gain_min) * (low - value + 1) as f32 / steps
    } else {
        1.0
    }
}

/// Gains of a persona from its OCEAN matrix (`None` or middling traits → 1.0).
pub fn gains_from_ocean(ocean: Option<[u8; 5]>, tuning: &Tuning) -> PersonaGains {
    let Some([o, _, e, a, n]) = ocean else { return PersonaGains::default() };
    PersonaGains {
        dislike_sensitivity: trait_gain(n, tuning),
        like_sensitivity: trait_gain(a, tuning),
        curiosity_gain: trait_gain(o, tuning),
        engagement_gain: trait_gain(e, tuning),
    }
}

/// A rule delta scaled by a gain (rounded; a gain of 1.0 is the identity).
fn gained(delta: u8, gain: f32) -> u8 {
    (f32::from(delta) * gain).round().clamp(0.0, 100.0) as u8
}

/// Net deltas of the reactions RECEIVED on an intervention (diminishing per kind,
/// scaled by the persona's gains). Applied through `AxisDeltas::apply`: capped
/// per axis and elastic near the extremes.
pub fn received_deltas(r: &ReactionTally, gains: &PersonaGains) -> AxisDeltas {
    let mut d = AxisDeltas::default();
    // Likes -> confiance up, engagement up
    if r.likes > 0 {
        d.confiance += i16::from(gained(scaled(constants::EMOTION_LIKE_CONF_FACTOR, r.likes, constants::EMOTION_LIKE_CONF_CAP), gains.like_sensitivity));
        d.engagement += i16::from(gained(scaled(constants::EMOTION_LIKE_ENG_FACTOR, r.likes, constants::EMOTION_LIKE_ENG_CAP), gains.engagement_gain));
    }
    // Dislikes -> frustration up (off-topic softer), confiance down
    if r.dislikes > 0 {
        let plain = r.dislikes - r.off_topic.min(r.dislikes);
        let frust = (scaled(constants::EMOTION_DISLIKE_FRUST_FACTOR, plain, constants::EMOTION_DISLIKE_FRUST_CAP) as u16
            + scaled(constants::EMOTION_OFFTOPIC_FRUST_FACTOR, r.off_topic, constants::EMOTION_DISLIKE_FRUST_CAP) as u16)
            .min(constants::EMOTION_DISLIKE_FRUST_CAP) as u8;
        d.frustration += i16::from(gained(frust, gains.dislike_sensitivity));
        d.confiance -= i16::from(gained(scaled(constants::EMOTION_DISLIKE_CONF_FACTOR, r.dislikes, constants::EMOTION_DISLIKE_CONF_CAP), gains.dislike_sensitivity));
    }
    // Contradiction -> frustration up, engagement up
    if r.was_contradicted() {
        d.frustration += i16::from(gained(constants::EMOTION_CONTRADICTION_FRUST, gains.dislike_sensitivity));
        d.engagement += i16::from(gained(constants::EMOTION_CONTRADICTION_ENG, gains.engagement_gain));
    }
    // Support -> enthousiasme up, confiance up
    if r.was_supported() {
        d.enthousiasme += i16::from(gained(constants::EMOTION_SUPPORT_ENTHOUSIASME, gains.like_sensitivity));
        d.confiance += i16::from(gained(constants::EMOTION_SUPPORT_CONF, gains.like_sensitivity));
    }
    // Typed nuances
    if r.insightful > 0 {
        d.confiance += i16::from(gained(scaled(constants::EMOTION_INSIGHTFUL_CONF_BONUS as u16, r.insightful, constants::EMOTION_LIKE_CONF_CAP), gains.like_sensitivity));
    }
    if r.questions > 0 {
        d.curiosite += i16::from(gained(scaled(constants::EMOTION_QUESTION_CURIOSITY, r.questions, constants::EMOTION_QUESTION_CURIOSITY_CAP), gains.curiosity_gain));
    }
    if r.laughs > 0 {
        d.enthousiasme += i16::from(scaled(constants::EMOTION_LAUGH_ENTHUSIASM, r.laughs, constants::EMOTION_LAUGH_ENTHUSIASM_CAP));
    }
    d
}

/// Effects of the reactions RECEIVED on an intervention: one round, capped per axis, elastic.
pub fn apply_received_reactions(profile: &mut EmotionalProfile, r: &ReactionTally, gains: &PersonaGains) {
    let raw = received_deltas(r, &PersonaGains::default());
    let felt = received_deltas(r, gains);
    raw.apply_felt(&felt, profile, constants::EMOTION_ROUND_AXIS_CAP);
}

/// Effects of the reactions a speaker GAVE: accord follows what they approve or reject, laughing lifts them.
pub fn apply_given_reactions(profile: &mut EmotionalProfile, g: &GivenReactions) {
    let mut d = AxisDeltas::default();
    if g.net != 0 {
        let magnitude = (g.net.unsigned_abs() as u16 * u16::from(constants::EMOTION_ACCORD_GIVEN_FACTOR)).min(u16::from(constants::EMOTION_ACCORD_GIVEN_CAP)) as i16;
        d.accord = if g.net > 0 { magnitude } else { -magnitude };
    }
    if g.laughs > 0 {
        d.enthousiasme = i16::from(scaled(constants::EMOTION_LAUGH_ENTHUSIASM, g.laughs, constants::EMOTION_LAUGH_ENTHUSIASM_CAP));
    }
    d.apply(profile, constants::EMOTION_ROUND_AXIS_CAP);
}

/// The analyst's tone deltas (already bounded by `clamp_delta`), elastic like every other effect.
pub fn apply_llm_delta(profile: &mut EmotionalProfile, delta: &EmotionDelta) {
    AxisDeltas {
        engagement: i16::from(delta.engagement),
        accord: i16::from(delta.accord),
        confiance: i16::from(delta.confiance),
        frustration: i16::from(delta.frustration),
        curiosite: i16::from(delta.curiosite),
        enthousiasme: i16::from(delta.enthousiasme),
    }
    .apply(profile, constants::EMOTION_LLM_DELTA_CAP as u8);
}

/// Per-intervention drift: stagnation penalty, then homeostasis — every axis
/// returns toward the persona's OWN baseline (not a neutral 50), so a serene
/// persona stays serene and a hot-headed one stays hot-headed absent new events.
/// Each axis recovers a share of its distance (`EMOTION_HOMEOSTASIS_PERCENT`), one
/// point at least; frustration and enthousiasme never recover slower than their
/// explicit rates (v1.20.5: far from the baseline the share wins, so a storm
/// plateaus instead of creeping to the ceiling).
pub fn apply_turn_effects(profile: &mut EmotionalProfile, initial: &EmotionalProfile, stagnating: bool) {
    if stagnating {
        profile.engagement = sub_clamped(profile.engagement, constants::EMOTION_STAGNATION_ENG);
        profile.curiosite = sub_clamped(profile.curiosite, constants::EMOTION_STAGNATION_CURIOSITE);
    }
    let frustration_step = homeostasis_step(profile.frustration, initial.frustration).max(constants::EMOTION_DECAY_FRUSTRATION_RATE);
    profile.frustration = decay_toward(profile.frustration, initial.frustration, frustration_step);
    let enthusiasm_step = homeostasis_step(profile.enthousiasme, initial.enthousiasme).max(constants::EMOTION_DECAY_ENTHUSIASM_RATE);
    profile.enthousiasme = decay_toward(profile.enthousiasme, initial.enthousiasme, enthusiasm_step);
    profile.engagement = decay_toward(profile.engagement, initial.engagement, homeostasis_step(profile.engagement, initial.engagement));
    profile.accord = decay_toward(profile.accord, initial.accord, homeostasis_step(profile.accord, initial.accord));
    profile.confiance = decay_toward(profile.confiance, initial.confiance, homeostasis_step(profile.confiance, initial.confiance));
    profile.curiosite = decay_toward(profile.curiosite, initial.curiosite, homeostasis_step(profile.curiosite, initial.curiosite));
}

/// Points recovered toward the baseline this intervention: a share of the distance, one at least.
fn homeostasis_step(value: u8, baseline: u8) -> u8 {
    let distance = value.abs_diff(baseline);
    if distance == 0 {
        return 0;
    }
    ((u16::from(distance) * u16::from(constants::EMOTION_HOMEOSTASIS_PERCENT)).div_ceil(100) as u8).max(1)
}

/// Full rule-based update of the deferred path (received, given, then turn effects).
pub fn update_emotions(
    current: &EmotionalProfile,
    initial: &EmotionalProfile,
    ctx: &EmotionContext,
    gains: &PersonaGains,
) -> EmotionalProfile {
    let mut new = current.clone();
    apply_received_reactions(&mut new, &ctx.received, gains);
    apply_given_reactions(&mut new, &ctx.given);
    apply_turn_effects(&mut new, initial, ctx.is_discussion_stagnating);
    new
}

/// Sampling parameters of a spoken intervention shaped by the speaker's state
/// (v1.17): enthusiasm moves the temperature, engagement the length. Never
/// applied to JSON calls. Providers that ignore the temperature while thinking
/// only get the length change.
pub fn modulate_sampling(params: &LlmParams, emotions: &EmotionalProfile, tuning: &Tuning) -> LlmParams {
    let mut out = params.clone();
    let enthusiasm = (f32::from(emotions.enthousiasme) - 50.0) / 50.0;
    out.temperature = (params.temperature + enthusiasm * tuning.emotion_temp_span).clamp(tuning.emotion_temp_min, tuning.emotion_temp_max);
    let engagement = f32::from(emotions.engagement) / 100.0;
    let factor = tuning.emotion_len_min + (tuning.emotion_len_max - tuning.emotion_len_min) * engagement;
    out.num_predict = ((params.num_predict as f32) * factor).round().max(1.0) as i32;
    out
}

/// Temperature of the room from the active gladiateurs (`None` below two).
pub fn room_mood(profiles: &[&EmotionalProfile]) -> Option<(EmotionalProfile, RoomMood)> {
    if profiles.len() < 2 {
        return None;
    }
    let avg = compute_average(profiles);
    let label = if avg.frustration > constants::ROOM_MOOD_TENSE_FRUSTRATION {
        RoomMood::Tense
    } else if avg.engagement < constants::ROOM_MOOD_FLAT_ENGAGEMENT {
        RoomMood::Flat
    } else if avg.enthousiasme > constants::ROOM_MOOD_LIVELY_ENTHUSIASM {
        RoomMood::Lively
    } else {
        RoomMood::Serene
    };
    Some((avg, label))
}

/// Immediate emotional impact of being banned by the moderator.
pub fn apply_ban_penalty(profile: &mut EmotionalProfile) {
    profile.frustration = add_clamped(profile.frustration, constants::EMOTION_BAN_FRUST);
    profile.engagement = sub_clamped(profile.engagement, constants::EMOTION_BAN_ENG);
}

/// Bound every axis of an LLM-provided delta to ±cap (the model is asked for
/// tone/content nuance only — reactions and bans are already rule-applied).
pub fn clamp_delta(delta: &EmotionDelta, cap: i8) -> EmotionDelta {
    EmotionDelta {
        engagement: delta.engagement.clamp(-cap, cap),
        accord: delta.accord.clamp(-cap, cap),
        confiance: delta.confiance.clamp(-cap, cap),
        frustration: delta.frustration.clamp(-cap, cap),
        curiosite: delta.curiosite.clamp(-cap, cap),
        enthousiasme: delta.enthousiasme.clamp(-cap, cap),
    }
}

/// Saturating addition clamped to 100
pub fn add_clamped(val: u8, delta: u8) -> u8 {
    val.saturating_add(delta).min(100)
}

/// Saturating subtraction (floors at 0)
pub fn sub_clamped(val: u8, delta: u8) -> u8 {
    val.saturating_sub(delta)
}

/// Progressive return toward a target value (never overshoots it)
fn decay_toward(val: u8, target: u8, rate: u8) -> u8 {
    if val > target {
        val.saturating_sub(rate).max(target)
    } else if val < target {
        val.saturating_add(rate).min(target)
    } else {
        val
    }
}

/// Jaccard similarity between the word sets of two texts (lowercased, ≥ 3 chars).
/// Used to detect a discussion that no longer produces new ideas.
pub fn text_similarity(a: &str, b: &str) -> f32 {
    let words = |s: &str| -> std::collections::HashSet<String> {
        s.split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.chars().count() >= 3)
            .map(|w| w.to_lowercase())
            .collect()
    };
    let wa = words(a);
    let wb = words(b);
    if wa.is_empty() && wb.is_empty() {
        return 1.0;
    }
    if wa.is_empty() || wb.is_empty() {
        return 0.0;
    }
    let inter = wa.intersection(&wb).count() as f32;
    let union = wa.union(&wb).count() as f32;
    inter / union
}

/// Detect thresholds that were **newly** crossed between prev and current.
/// Returns `[(axis_name, "high"|"low", value)]`.
pub fn detect_thresholds(
    prev: &EmotionalProfile,
    current: &EmotionalProfile,
) -> Vec<(String, String, u8)> {
    let mut crossed = Vec::new();
    let axes: [(&str, u8, u8); 6] = [
        ("engagement", prev.engagement, current.engagement),
        ("accord", prev.accord, current.accord),
        ("confiance", prev.confiance, current.confiance),
        ("frustration", prev.frustration, current.frustration),
        ("curiosite", prev.curiosite, current.curiosite),
        ("enthousiasme", prev.enthousiasme, current.enthousiasme),
    ];
    for (name, p, c) in axes {
        if p < constants::EMOTION_HIGH_THRESHOLD && c >= constants::EMOTION_HIGH_THRESHOLD {
            crossed.push((name.to_string(), "high".to_string(), c));
        }
        if p > constants::EMOTION_LOW_THRESHOLD && c <= constants::EMOTION_LOW_THRESHOLD {
            crossed.push((name.to_string(), "low".to_string(), c));
        }
    }
    crossed
}

/// The six axes of a profile next to a baseline: `(axis, value, baseline)`.
fn axes_with_baseline(current: &EmotionalProfile, baseline: &EmotionalProfile) -> [(&'static str, u8, u8); 6] {
    [
        ("engagement", current.engagement, baseline.engagement),
        ("accord", current.accord, baseline.accord),
        ("confiance", current.confiance, baseline.confiance),
        ("frustration", current.frustration, baseline.frustration),
        ("curiosite", current.curiosite, baseline.curiosite),
        ("enthousiasme", current.enthousiasme, baseline.enthousiasme),
    ]
}

/// Which notable zone each speaker's axes occupy (+1 above the baseline, −1
/// below, 0 none), with hysteresis (v1.20.5): a zone is entered at
/// `EMOTION_NOTABLE_SHIFT` and left only below `EMOTION_NOTABLE_SHIFT −
/// EMOTION_SHIFT_REARM`. Movements rather than levels — the theatre of what the
/// debate did to a speaker — announced once per entry, never at every
/// intervention of a speaker hovering around the boundary.
#[derive(Debug, Default, Clone)]
pub struct ShiftZones {
    zones: HashMap<(String, &'static str), i8>,
}

impl ShiftZones {
    /// Axes of `speaker_id` that just entered a notable zone, as `(axis,
    /// direction, value)` like `detect_thresholds` — "high" when the move is upward.
    pub fn crossings(&mut self, speaker_id: &str, current: &EmotionalProfile, baseline: &EmotionalProfile) -> Vec<(String, String, u8)> {
        let notable = i16::from(constants::EMOTION_NOTABLE_SHIFT);
        let rearm = notable - i16::from(constants::EMOTION_SHIFT_REARM);
        let mut crossed = Vec::new();
        for (axis, value, base) in axes_with_baseline(current, baseline) {
            let shift = i16::from(value) - i16::from(base);
            let zone = self.zones.entry((speaker_id.to_string(), axis)).or_insert(0);
            let next = if shift >= notable {
                1
            } else if shift <= -notable {
                -1
            } else if shift.abs() < rearm {
                0
            } else {
                *zone
            };
            if next != 0 && next != *zone {
                crossed.push((axis.to_string(), if next > 0 { "high" } else { "low" }.to_string(), value));
            }
            *zone = next;
        }
        crossed
    }
}

/// Compute the contagion delta for a single axis.
fn contagion_delta(avg_val: u8, current_val: u8, rate: f32, max_delta: f32) -> i8 {
    let diff = avg_val as f32 - current_val as f32;
    (diff * rate).round().clamp(-max_delta, max_delta) as i8
}

/// Apply emotional contagion: move target toward the group average.
pub fn apply_contagion(avg: &EmotionalProfile, target: &mut EmotionalProfile) {
    use super::apply_i8_clamped;
    let rate = constants::EMOTION_CONTAGION_RATE;
    let max = constants::EMOTION_CONTAGION_MAX_DELTA;
    target.engagement = apply_i8_clamped(target.engagement, contagion_delta(avg.engagement, target.engagement, rate, max));
    target.accord = apply_i8_clamped(target.accord, contagion_delta(avg.accord, target.accord, rate, max));
    target.confiance = apply_i8_clamped(target.confiance, contagion_delta(avg.confiance, target.confiance, rate, max));
    target.frustration = apply_i8_clamped(target.frustration, contagion_delta(avg.frustration, target.frustration, rate, max));
    target.curiosite = apply_i8_clamped(target.curiosite, contagion_delta(avg.curiosite, target.curiosite, rate, max));
    target.enthousiasme = apply_i8_clamped(target.enthousiasme, contagion_delta(avg.enthousiasme, target.enthousiasme, rate, max));
}

/// Compute the average emotional profile from a slice of profiles
pub fn compute_average(profiles: &[&EmotionalProfile]) -> EmotionalProfile {
    if profiles.is_empty() {
        return EmotionalProfile::default();
    }
    let n = profiles.len() as u32;
    EmotionalProfile {
        engagement: (profiles.iter().map(|p| p.engagement as u32).sum::<u32>() / n) as u8,
        accord: (profiles.iter().map(|p| p.accord as u32).sum::<u32>() / n) as u8,
        confiance: (profiles.iter().map(|p| p.confiance as u32).sum::<u32>() / n) as u8,
        frustration: (profiles.iter().map(|p| p.frustration as u32).sum::<u32>() / n) as u8,
        curiosite: (profiles.iter().map(|p| p.curiosite as u32).sum::<u32>() / n) as u8,
        enthousiasme: (profiles.iter().map(|p| p.enthousiasme as u32).sum::<u32>() / n) as u8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quiet() -> EmotionContext {
        EmotionContext { received: ReactionTally::default(), given: GivenReactions::default(), is_discussion_stagnating: false }
    }

    fn received(likes: u32, dislikes: u32) -> ReactionTally {
        ReactionTally { likes, dislikes, ..Default::default() }
    }

    #[test]
    fn default_emotions_unchanged_without_events() {
        // Persona baseline frustration 10: no drift toward 50 anymore.
        let profile = EmotionalProfile::default();
        let result = update_emotions(&profile, &profile, &quiet(), &PersonaGains::default());
        assert_eq!(result, profile);
    }

    #[test]
    fn decay_returns_toward_persona_baseline_not_neutral() {
        let initial = EmotionalProfile { frustration: 70, enthousiasme: 20, ..Default::default() };
        // Near the baseline the explicit rate is the floor (a 12 % share would be one point)
        let warm = EmotionalProfile { frustration: 75, enthousiasme: 25, ..initial.clone() };
        let r = update_emotions(&warm, &initial, &quiet(), &PersonaGains::default());
        assert_eq!(r.frustration, 75 - constants::EMOTION_DECAY_FRUSTRATION_RATE);
        assert_eq!(r.enthousiasme, 25 - constants::EMOTION_DECAY_ENTHUSIASM_RATE);
        // Far from it the proportional share wins (v1.20.5): 40 points away → 5
        let heated = EmotionalProfile { frustration: 90, enthousiasme: 60, ..initial.clone() };
        let r = update_emotions(&heated, &initial, &quiet(), &PersonaGains::default());
        assert_eq!(r.frustration, 90 - 3, "20 away → ceil(2.4)");
        assert_eq!(r.enthousiasme, 60 - 5, "40 away → ceil(4.8)");
        // Below baseline: climbs back up, never overshoots
        let calm = EmotionalProfile { frustration: 69, ..initial.clone() };
        assert_eq!(update_emotions(&calm, &initial, &quiet(), &PersonaGains::default()).frustration, 70);
    }

    /// v1.20.4 — likes lift confiance and engagement, with diminishing returns:
    /// the third like weighs less than the first, and one round never moves an
    /// axis by more than the round cap.
    #[test]
    fn likes_increase_confiance_and_engagement_with_diminishing_returns() {
        let profile = EmotionalProfile::default();
        let after = |likes: u32| update_emotions(&profile, &profile, &EmotionContext { received: received(likes, 0), ..quiet() }, &PersonaGains::default());
        let (one, two, three) = (after(1), after(2), after(3));
        assert!(one.confiance > 50 && one.engagement > 50, "{one:?}");
        assert!(two.confiance > one.confiance, "support bonus at two likes: {one:?} {two:?}");
        assert!(three.confiance - two.confiance < two.confiance - one.confiance, "diminishing: {one:?} {two:?} {three:?}");
        let raw = received_deltas(&received(3, 0), &PersonaGains::default());
        assert!(raw.confiance > i16::from(constants::EMOTION_ROUND_AXIS_CAP), "three likes exceed the cap before it applies: {raw:?}");
        assert!(three.confiance <= 50 + constants::EMOTION_ROUND_AXIS_CAP, "{three:?}");
        // The harmonic sum: 1, 1.5, 1.83… of the first reaction, then the kind's cap
        assert_eq!((scaled(5, 1, 15), scaled(5, 2, 15), scaled(5, 3, 15), scaled(5, 100, 15)), (5, 8, 9, 15));
    }

    /// v1.20.4 — no saturation: near an extreme an axis barely moves further
    /// (never zero), moves back easily, and always stays within 0..=100.
    #[test]
    fn extremes_resist_and_values_stay_in_bounds() {
        assert_eq!(resistance(50, true), 1.0);
        assert_eq!(resistance(constants::EMOTION_COMFORT_HIGH, true), 1.0);
        assert!((resistance(100, true) - constants::EMOTION_EXTREME_RESISTANCE).abs() < 1e-6);
        assert!((resistance(0, false) - constants::EMOTION_EXTREME_RESISTANCE).abs() < 1e-6);
        assert_eq!(resistance(100, false), 1.0, "coming back from the top is free");
        assert_eq!(resistance(0, true), 1.0, "coming back from the bottom is free");
        let mid = resistance(82, true);
        assert!(mid < 1.0 && mid > constants::EMOTION_EXTREME_RESISTANCE, "in between: {mid}");
        assert!(resistance(75, true) > resistance(85, true) && resistance(85, true) > resistance(95, true));
        assert!((resistance(85, true) - 0.31).abs() < 0.02, "quadratic: the brake bites early — {}", resistance(85, true));
        // The same +10 moves a mid axis by 10 and a saturated one by a couple of points, never zero
        assert_eq!(elastic(50, 10), 60);
        let near_top = elastic(95, 10);
        assert!(near_top > 95 && near_top <= 98, "{near_top}");
        assert_eq!(elastic(99, 10), 100, "a delta always moves at least one point");
        assert_eq!(elastic(100, 10), 100);
        assert_eq!(elastic(100, -10), 90);
        assert_eq!(elastic(0, -10), 0);
        assert_eq!(elastic(50, 0), 50);
        // Through the whole model: a confident speaker showered with likes stays below 100
        let profile = EmotionalProfile { confiance: 95, ..Default::default() };
        let ctx = EmotionContext { received: received(3, 0), ..quiet() };
        let r = update_emotions(&profile, &profile, &ctx, &PersonaGains::default());
        assert!(r.confiance > 95 && r.confiance <= 100, "{r:?}");
    }

    /// v1.20.4 — the analyst's deltas are elastic too: +10 on a saturated axis
    /// is a nudge, −10 on it is a real drop.
    #[test]
    fn llm_deltas_are_elastic() {
        let mut high = EmotionalProfile { engagement: 96, accord: 50, ..Default::default() };
        apply_llm_delta(&mut high, &EmotionDelta { engagement: 10, accord: 10, confiance: 0, frustration: 0, curiosite: 0, enthousiasme: 0 });
        assert!(high.engagement > 96 && high.engagement < 100, "{high:?}");
        assert_eq!(high.accord, 60);
        apply_llm_delta(&mut high, &EmotionDelta { engagement: -10, accord: 0, confiance: 0, frustration: 0, curiosite: 0, enthousiasme: 0 });
        assert!(high.engagement <= 89, "{high:?}");
    }

    /// v1.20.4 — homeostasis: every axis drifts back toward the persona's own
    /// baseline at each intervention, faster when far, one point at least.
    #[test]
    fn all_axes_return_toward_the_baseline() {
        let initial = EmotionalProfile { engagement: 40, accord: 60, confiance: 70, curiosite: 55, ..Default::default() };
        let moved = EmotionalProfile { engagement: 90, accord: 10, confiance: 72, curiosite: 55, ..initial.clone() };
        let r = update_emotions(&moved, &initial, &quiet(), &PersonaGains::default());
        assert!(r.engagement < 90 && r.engagement > 40, "{r:?}");
        assert!(r.accord > 10 && r.accord < 60, "{r:?}");
        assert_eq!(r.confiance, 71, "one point at least, never past the baseline");
        assert_eq!(r.curiosite, 55, "at the baseline: nothing");
        let step_far = 90 - r.engagement;
        let step_near = r.accord - 10;
        assert!(step_far > 1 && step_near > 1 && step_far == step_near, "proportional to the distance (50 both): {step_far} {step_near}");
        // Repeated quiet interventions converge to the baseline without overshooting
        let mut p = moved.clone();
        for _ in 0..60 {
            p = update_emotions(&p, &initial, &quiet(), &PersonaGains::default());
        }
        assert_eq!((p.engagement, p.accord, p.confiance), (40, 60, 70), "{p:?}");
    }

    /// v1.20.5 — a stage cue fires when an axis enters the notable zone, once:
    /// hovering around the boundary (homeostasis back, reaction forward) never
    /// re-fires, leaving the zone re-arms only below the hysteresis, a swing to
    /// the opposite zone fires again, and speakers are tracked apart.
    #[test]
    fn shift_zones_fire_once_per_entry_with_hysteresis() {
        let base = EmotionalProfile { confiance: 70, ..Default::default() };
        let mut zones = ShiftZones::default();
        let at = |confiance: u8| EmotionalProfile { confiance, ..Default::default() };
        assert!(zones.crossings("g1", &at(60), &base).is_empty(), "−10: not notable");
        assert_eq!(zones.crossings("g1", &at(54), &base), vec![("confiance".to_string(), "low".to_string(), 54)]);
        // Hovering: −13 (inside the hysteresis band) then −16 again → silent
        assert!(zones.crossings("g1", &at(57), &base).is_empty());
        assert!(zones.crossings("g1", &at(54), &base).is_empty());
        assert!(zones.crossings("g1", &at(40), &base).is_empty(), "deeper: still the same zone");
        // Back within the re-arm distance (< 10): the zone is left, silently
        assert!(zones.crossings("g1", &at(62), &base).is_empty());
        // A second real drop fires again
        assert_eq!(zones.crossings("g1", &at(50), &base).len(), 1);
        // A swing straight to the opposite zone fires the other direction
        assert_eq!(zones.crossings("g1", &at(90), &base), vec![("confiance".to_string(), "high".to_string(), 90)]);
        // Another speaker has its own zones
        assert_eq!(zones.crossings("g2", &at(54), &base).len(), 1);
        // Exactly at the notable distance counts; one below does not (fresh tracker)
        let mut fresh = ShiftZones::default();
        assert!(fresh.crossings("g3", &at(70 - constants::EMOTION_NOTABLE_SHIFT + 1), &base).is_empty());
        assert_eq!(fresh.crossings("g3", &at(70 - constants::EMOTION_NOTABLE_SHIFT), &base).len(), 1);
    }

    /// v1.20.4 — the dominant shift names what the debate did to the speaker.
    #[test]
    fn dominant_shift_is_the_largest_notable_move_from_the_baseline() {
        let base = EmotionalProfile::default();
        assert_eq!(dominant_shift(&base, &base), None);
        let small = EmotionalProfile { confiance: 50 + constants::EMOTION_NOTABLE_SHIFT - 1, ..Default::default() };
        assert_eq!(dominant_shift(&small, &base), None);
        let shaken = EmotionalProfile { confiance: 30, accord: 66, ..Default::default() };
        assert_eq!(dominant_shift(&shaken, &base), Some(("confiance", -20)));
        let convinced = EmotionalProfile { accord: 80, frustration: 26, ..Default::default() };
        assert_eq!(dominant_shift(&convinced, &base), Some(("accord", 30)));
    }

    #[test]
    fn ban_penalty_is_immediate_and_explicit() {
        let mut profile = EmotionalProfile::default();
        apply_ban_penalty(&mut profile);
        assert_eq!(profile.frustration, 10 + constants::EMOTION_BAN_FRUST);
        assert_eq!(profile.engagement, 50 - constants::EMOTION_BAN_ENG);
    }

    /// Accord follows what the speaker approves or rejects, symmetrically, gently
    /// (v1.20.4: factor 2, cap 6 — approving three times a turn no longer makes an ally).
    #[test]
    fn accord_follows_reactions_given_with_cap() {
        let mut up = EmotionalProfile::default();
        apply_given_reactions(&mut up, &GivenReactions { net: 2, laughs: 0 });
        assert_eq!(up.accord, 50 + 2 * constants::EMOTION_ACCORD_GIVEN_FACTOR);
        let mut down = EmotionalProfile::default();
        apply_given_reactions(&mut down, &GivenReactions { net: -2, laughs: 0 });
        assert_eq!(down.accord, 50 - 2 * constants::EMOTION_ACCORD_GIVEN_FACTOR, "symmetric");
        let mut capped = EmotionalProfile::default();
        apply_given_reactions(&mut capped, &GivenReactions { net: 50, laughs: 0 });
        assert_eq!(capped.accord, 50 + constants::EMOTION_ACCORD_GIVEN_CAP);
    }

    #[test]
    fn stagnation_lowers_engagement_and_curiosity_only_when_flagged() {
        let profile = EmotionalProfile::default();
        let flagged = update_emotions(&profile, &profile, &EmotionContext { is_discussion_stagnating: true, ..quiet() }, &PersonaGains::default());
        let mut direct = profile.clone();
        apply_turn_effects(&mut direct, &profile, true);
        assert_eq!(direct, flagged, "the composed update equals the split steps");
        // Penalty, then the homeostasis recovers one point of it
        assert!(flagged.engagement < 50 && flagged.engagement >= 50 - constants::EMOTION_STAGNATION_ENG, "{flagged:?}");
        assert!(flagged.curiosite < 50 && flagged.curiosite >= 50 - constants::EMOTION_STAGNATION_CURIOSITE, "{flagged:?}");
        let lively = update_emotions(&profile, &profile, &quiet(), &PersonaGains::default());
        assert_eq!(lively.engagement, 50);
    }

    #[test]
    fn no_u8_overflow_with_many_likes() {
        let profile = EmotionalProfile::default();
        let ctx = EmotionContext { received: received(100, 0), ..quiet() };
        let result = update_emotions(&profile, &profile, &ctx, &PersonaGains::default());
        assert!(result.confiance <= 100);
        assert!(result.engagement <= 100);
    }

    #[test]
    fn typed_reactions_add_their_nuance_on_top_of_the_classes() {
        let base = EmotionalProfile::default();
        let mut tally = ReactionTally::default();
        for k in [ReactionType::Insightful, ReactionType::Question, ReactionType::OffTopic, ReactionType::Laugh, ReactionType::Like] {
            tally.add(k);
        }
        assert_eq!((tally.likes, tally.dislikes, tally.insightful, tally.questions, tally.off_topic, tally.laughs), (2, 1, 1, 1, 1, 1));
        let mut p = base.clone();
        apply_received_reactions(&mut p, &tally, &PersonaGains::default());
        // Likes and the insightful lift confiance (the off-topic costs a little), the
        // support bonus and the laugh lift enthousiasme, the question feeds curiosity
        assert!(p.confiance > 50 && p.confiance <= 50 + constants::EMOTION_ROUND_AXIS_CAP, "{p:?}");
        assert_eq!(p.frustration, 10 + constants::EMOTION_OFFTOPIC_FRUST_FACTOR as u8, "off-topic is softer than a dislike");
        assert_eq!(p.curiosite, 50 + constants::EMOTION_QUESTION_CURIOSITY as u8);
        assert_eq!(p.enthousiasme, 50 + constants::EMOTION_SUPPORT_ENTHOUSIASME + constants::EMOTION_LAUGH_ENTHUSIASM as u8);
        let mut plain_dislike = base.clone();
        apply_received_reactions(&mut plain_dislike, &received(0, 1), &PersonaGains::default());
        assert!(plain_dislike.frustration > p.frustration, "{plain_dislike:?}");
        // Given: a laugh lifts the reactor, net follows the classes
        let mut g = GivenReactions::default();
        g.add(ReactionType::Laugh);
        g.add(ReactionType::OffTopic);
        assert_eq!((g.net, g.laughs), (-1, 1));
        let mut q = base.clone();
        apply_given_reactions(&mut q, &g);
        assert_eq!((q.accord, q.enthousiasme), (50 - constants::EMOTION_ACCORD_GIVEN_FACTOR, 50 + constants::EMOTION_LAUGH_ENTHUSIASM as u8));
        // A neutral reaction alone changes no class counter
        let mut n = ReactionTally::default();
        n.add(ReactionType::Question);
        assert!(!n.is_empty() && n.likes == 0 && n.dislikes == 0);
    }

    #[test]
    fn llm_deltas_are_bounded() {
        let d = EmotionDelta { engagement: 40, accord: -40, confiance: 3, frustration: 127, curiosite: -128, enthousiasme: 0 };
        let c = clamp_delta(&d, 10);
        assert_eq!((c.engagement, c.accord, c.confiance, c.frustration, c.curiosite, c.enthousiasme), (10, -10, 3, 10, -10, 0));
    }

    #[test]
    fn text_similarity_detects_identical_and_disjoint_summaries() {
        let a = "Les participants discutent de l'impact de l'automatisation sur l'emploi";
        assert!((text_similarity(a, a) - 1.0).abs() < 1e-6);
        assert!(text_similarity(a, "Le climat change rapidement selon les modèles") < 0.2);
        let b = "Les participants discutent encore de l'impact de l'automatisation sur l'emploi et les salaires";
        assert!(text_similarity(a, b) > 0.6);
        assert_eq!(text_similarity("", ""), 1.0);
        assert_eq!(text_similarity("abc def", ""), 0.0);
    }

    #[test]
    fn detect_thresholds_high_low_and_already_above() {
        let prev = EmotionalProfile { frustration: 80, ..Default::default() };
        let curr = EmotionalProfile { frustration: 90, ..Default::default() };
        let t = detect_thresholds(&prev, &curr);
        assert_eq!(t, vec![("frustration".to_string(), "high".to_string(), 90)]);

        let prev = EmotionalProfile { engagement: 20, ..Default::default() };
        let curr = EmotionalProfile { engagement: 10, ..Default::default() };
        assert_eq!(detect_thresholds(&prev, &curr)[0].1, "low");

        let prev = EmotionalProfile { frustration: 90, ..Default::default() };
        let curr = EmotionalProfile { frustration: 95, ..Default::default() };
        assert!(detect_thresholds(&prev, &curr).is_empty());
    }

    #[test]
    fn contagion_moves_toward_average_and_is_capped() {
        let avg = EmotionalProfile { engagement: 80, ..Default::default() };
        let mut target = EmotionalProfile { engagement: 40, ..Default::default() };
        apply_contagion(&avg, &mut target);
        assert_eq!(target.engagement, 42); // diff 40 × 0.05 = 2

        let avg = EmotionalProfile { engagement: 100, ..Default::default() };
        let mut target = EmotionalProfile { engagement: 0, ..Default::default() };
        apply_contagion(&avg, &mut target);
        assert_eq!(target.engagement, 3); // 5 clamped to 3
    }

    #[test]
    fn compute_average_of_two_profiles() {
        let a = EmotionalProfile { engagement: 80, accord: 60, confiance: 40, frustration: 20, curiosite: 90, enthousiasme: 50 };
        let b = EmotionalProfile { engagement: 40, accord: 80, confiance: 60, frustration: 40, curiosite: 30, enthousiasme: 70 };
        let avg = compute_average(&[&a, &b]);
        assert_eq!(avg, EmotionalProfile { engagement: 60, accord: 70, confiance: 50, frustration: 30, curiosite: 60, enthousiasme: 60 });
    }

    // ── Émotions incarnées (v1.17) ──────────────────────────────────────

    /// S22 — the same event hurts a neurotic persona more than a stable one; an
    /// absent or middling OCEAN reproduces the v1.16 deltas exactly (golden).
    #[test]
    fn ocean_gains_scale_the_rule_deltas_and_neutral_is_the_golden_path() {
        let tuning = Tuning::default();
        assert_eq!(gains_from_ocean(None, &tuning), PersonaGains::default());
        assert_eq!(gains_from_ocean(Some([5, 5, 5, 5, 5]), &tuning), PersonaGains::default());
        assert_eq!(gains_from_ocean(Some([4, 7, 7, 4, 7]), &tuning), PersonaGains::default(), "the whole 4..=7 band is neutral");
        let nervous = gains_from_ocean(Some([5, 5, 5, 5, 9]), &tuning);
        let stable = gains_from_ocean(Some([5, 5, 5, 5, 3]), &tuning);
        assert!((nervous.dislike_sensitivity - 1.2667).abs() < 1e-3, "{nervous:?}");
        assert!((stable.dislike_sensitivity - 0.8667).abs() < 1e-3, "{stable:?}");
        let extreme = gains_from_ocean(Some([10, 1, 1, 10, 10]), &tuning);
        assert_eq!((extreme.curiosity_gain, extreme.engagement_gain, extreme.like_sensitivity, extreme.dislike_sensitivity), (1.4, 0.6, 1.4, 1.4));

        let profile = EmotionalProfile::default();
        let ctx = EmotionContext { received: received(0, 2), given: GivenReactions::default(), is_discussion_stagnating: false };
        let golden = update_emotions(&profile, &profile, &ctx, &PersonaGains::default());
        let neurotic = update_emotions(&profile, &profile, &ctx, &nervous);
        let calm = update_emotions(&profile, &profile, &ctx, &stable);
        assert!(neurotic.frustration > golden.frustration && golden.frustration > calm.frustration, "{neurotic:?} {golden:?} {calm:?}");
        assert!(neurotic.confiance < golden.confiance && golden.confiance < calm.confiance);
        // Unrelated axes untouched by the neuroticism gain
        assert_eq!(neurotic.curiosite, golden.curiosite);
        // Gains scale the capped deltas (never reverse them) and stay within 0..=100
        let ctx = EmotionContext { received: received(50, 0), given: GivenReactions::default(), is_discussion_stagnating: false };
        let golden = update_emotions(&profile, &profile, &ctx, &PersonaGains::default()).confiance;
        let boosted = update_emotions(&profile, &profile, &ctx, &extreme).confiance;
        assert!(boosted > golden && boosted <= 100, "{golden} → {boosted}");
    }

    /// S27 — enthusiasm moves the temperature, engagement the length, within bounds.
    #[test]
    fn sampling_follows_enthusiasm_and_engagement_within_bounds() {
        let tuning = Tuning::default();
        let params = LlmParams { temperature: 0.8, num_predict: 1000, ..Default::default() };
        let hot = EmotionalProfile { enthousiasme: 90, engagement: 20, ..Default::default() };
        let out = modulate_sampling(&params, &hot, &tuning);
        assert!((out.temperature - 0.92).abs() < 1e-6, "{}", out.temperature);
        assert_eq!(out.num_predict, 880, "engagement 20 → ×0.88");
        let flat = EmotionalProfile { enthousiasme: 0, engagement: 100, ..Default::default() };
        let out = modulate_sampling(&params, &flat, &tuning);
        assert!((out.temperature - 0.65).abs() < 1e-6);
        assert_eq!(out.num_predict, 1200);
        // Neutral state = original parameters; bounds hold at the extremes
        let neutral = modulate_sampling(&params, &EmotionalProfile::default(), &tuning);
        assert_eq!((neutral.temperature, neutral.num_predict), (0.8, 1000));
        let cold = LlmParams { temperature: 0.3, ..params.clone() };
        assert_eq!(modulate_sampling(&cold, &flat, &tuning).temperature, tuning.emotion_temp_min);
        let boiling = LlmParams { temperature: 1.2, ..params };
        assert_eq!(modulate_sampling(&boiling, &hot, &tuning).temperature, tuning.emotion_temp_max);
    }

    /// S30 / S31 — the room mood needs two active profiles and follows the thresholds.
    #[test]
    fn room_mood_needs_two_profiles_and_follows_thresholds() {
        let a = EmotionalProfile { frustration: 80, ..Default::default() };
        assert!(room_mood(&[&a]).is_none());
        let b = EmotionalProfile { frustration: 60, ..Default::default() };
        let (avg, label) = room_mood(&[&a, &b]).unwrap();
        assert_eq!((avg.frustration, label), (70, RoomMood::Tense));
        let low = EmotionalProfile { engagement: 20, ..Default::default() };
        assert_eq!(room_mood(&[&low, &low]).unwrap().1, RoomMood::Flat);
        let high = EmotionalProfile { enthousiasme: 80, ..Default::default() };
        assert_eq!(room_mood(&[&high, &high]).unwrap().1, RoomMood::Lively);
        let d = EmotionalProfile::default();
        assert_eq!(room_mood(&[&d, &d]).unwrap().1, RoomMood::Serene);
        assert_eq!(serde_json::to_string(&RoomMood::Tense).unwrap(), "\"tense\"");
        assert!(RoomMood::Flat.moderator_hint("zh").contains("沉闷"));
    }
}

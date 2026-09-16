use crate::constants;
use crate::models::emotion::{EmotionDelta, EmotionalProfile};

/// Context for the rule-based emotion update run after each intervention.
pub struct EmotionContext {
    /// Likes received this turn on the speaker's previous intervention.
    pub likes_received: u32,
    /// Dislikes received this turn on the speaker's previous intervention.
    pub dislikes_received: u32,
    /// Net reactions the speaker just GAVE (likes − dislikes) — drives `accord`.
    pub net_reactions_given: i32,
    /// Real stagnation signal computed by the engine (summary similarity,
    /// reaction drought or LLM flag) — never a turn-number heuristic.
    pub is_discussion_stagnating: bool,
}

impl EmotionContext {
    /// Heuristic: contradiction = dislikes >= threshold
    pub fn was_contradicted(&self) -> bool {
        self.dislikes_received >= constants::EMOTION_CONTRADICTION_THRESHOLD
    }

    /// Heuristic: support = likes >= threshold
    pub fn was_supported(&self) -> bool {
        self.likes_received >= constants::EMOTION_SUPPORT_THRESHOLD
    }
}

/// Rule-based update. `initial` is the persona's starting profile: extremes decay
/// back toward it (not toward a neutral 50), so a serene persona stays serene and
/// a hot-headed one stays hot-headed absent new events.
pub fn update_emotions(
    current: &EmotionalProfile,
    initial: &EmotionalProfile,
    ctx: &EmotionContext,
) -> EmotionalProfile {
    let mut new = current.clone();

    // Likes -> confiance up, engagement up (compute in u16 to avoid u8 overflow)
    if ctx.likes_received > 0 {
        let delta_conf = ((constants::EMOTION_LIKE_CONF_FACTOR * ctx.likes_received as u16).min(constants::EMOTION_LIKE_CONF_CAP)) as u8;
        let delta_eng = ((constants::EMOTION_LIKE_ENG_FACTOR * ctx.likes_received as u16).min(constants::EMOTION_LIKE_ENG_CAP)) as u8;
        new.confiance = add_clamped(new.confiance, delta_conf);
        new.engagement = add_clamped(new.engagement, delta_eng);
    }

    // Dislikes -> frustration up, confiance down
    if ctx.dislikes_received > 0 {
        let delta_frust = ((constants::EMOTION_DISLIKE_FRUST_FACTOR * ctx.dislikes_received as u16).min(constants::EMOTION_DISLIKE_FRUST_CAP)) as u8;
        let delta_conf = ((constants::EMOTION_DISLIKE_CONF_FACTOR * ctx.dislikes_received as u16).min(constants::EMOTION_DISLIKE_CONF_CAP)) as u8;
        new.frustration = add_clamped(new.frustration, delta_frust);
        new.confiance = sub_clamped(new.confiance, delta_conf);
    }

    // Contradiction -> frustration up, engagement up
    if ctx.was_contradicted() {
        new.frustration = add_clamped(new.frustration, constants::EMOTION_CONTRADICTION_FRUST);
        new.engagement = add_clamped(new.engagement, constants::EMOTION_CONTRADICTION_ENG);
    }

    // Support -> enthousiasme up, confiance up
    if ctx.was_supported() {
        new.enthousiasme = add_clamped(new.enthousiasme, constants::EMOTION_SUPPORT_ENTHOUSIASME);
        new.confiance = add_clamped(new.confiance, constants::EMOTION_SUPPORT_CONF);
    }

    // Reactions given -> accord follows what the speaker approves or rejects
    if ctx.net_reactions_given != 0 {
        let magnitude = (ctx.net_reactions_given.unsigned_abs() as u16 * constants::EMOTION_ACCORD_GIVEN_FACTOR as u16)
            .min(constants::EMOTION_ACCORD_GIVEN_CAP as u16) as u8;
        new.accord = if ctx.net_reactions_given > 0 {
            add_clamped(new.accord, magnitude)
        } else {
            sub_clamped(new.accord, magnitude)
        };
    }

    // Stagnation -> engagement down, curiosite down
    if ctx.is_discussion_stagnating {
        new.engagement = sub_clamped(new.engagement, constants::EMOTION_STAGNATION_ENG);
        new.curiosite = sub_clamped(new.curiosite, constants::EMOTION_STAGNATION_CURIOSITE);
    }

    // Natural decay: extremes return toward the persona's own baseline
    new.frustration = decay_toward(new.frustration, initial.frustration, constants::EMOTION_DECAY_FRUSTRATION_RATE);
    new.enthousiasme = decay_toward(new.enthousiasme, initial.enthousiasme, constants::EMOTION_DECAY_ENTHUSIASM_RATE);

    new
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
        EmotionContext { likes_received: 0, dislikes_received: 0, net_reactions_given: 0, is_discussion_stagnating: false }
    }

    #[test]
    fn default_emotions_unchanged_without_events() {
        // Persona baseline frustration 10: no drift toward 50 anymore.
        let profile = EmotionalProfile::default();
        let result = update_emotions(&profile, &profile, &quiet());
        assert_eq!(result, profile);
    }

    #[test]
    fn decay_returns_toward_persona_baseline_not_neutral() {
        let initial = EmotionalProfile { frustration: 70, enthousiasme: 20, ..Default::default() };
        let heated = EmotionalProfile { frustration: 90, enthousiasme: 60, ..initial.clone() };
        let r = update_emotions(&heated, &initial, &quiet());
        assert_eq!(r.frustration, 90 - constants::EMOTION_DECAY_FRUSTRATION_RATE);
        assert_eq!(r.enthousiasme, 60 - constants::EMOTION_DECAY_ENTHUSIASM_RATE);
        // Below baseline: climbs back up, never overshoots
        let calm = EmotionalProfile { frustration: 69, ..initial.clone() };
        assert_eq!(update_emotions(&calm, &initial, &quiet()).frustration, 70);
    }

    #[test]
    fn likes_increase_confiance_and_engagement() {
        let profile = EmotionalProfile::default();
        let ctx = EmotionContext { likes_received: 2, ..quiet() };
        let result = update_emotions(&profile, &profile, &ctx);
        // 50 + 10 (likes) + 5 (support bonus, 2 likes >= 2) = 65
        assert_eq!(result.confiance, 65);
        assert_eq!(result.engagement, 56); // 50 + 6
    }

    #[test]
    fn values_clamp_at_100() {
        let profile = EmotionalProfile { confiance: 95, ..Default::default() };
        let ctx = EmotionContext { likes_received: 3, ..quiet() };
        assert_eq!(update_emotions(&profile, &profile, &ctx).confiance, 100);
    }

    #[test]
    fn ban_penalty_is_immediate_and_explicit() {
        let mut profile = EmotionalProfile::default();
        apply_ban_penalty(&mut profile);
        assert_eq!(profile.frustration, 10 + constants::EMOTION_BAN_FRUST);
        assert_eq!(profile.engagement, 50 - constants::EMOTION_BAN_ENG);
    }

    #[test]
    fn accord_follows_reactions_given_with_cap() {
        let profile = EmotionalProfile::default();
        let up = update_emotions(&profile, &profile, &EmotionContext { net_reactions_given: 2, ..quiet() });
        assert_eq!(up.accord, 50 + 2 * constants::EMOTION_ACCORD_GIVEN_FACTOR);
        let down = update_emotions(&profile, &profile, &EmotionContext { net_reactions_given: -1, ..quiet() });
        assert_eq!(down.accord, 50 - constants::EMOTION_ACCORD_GIVEN_FACTOR);
        let capped = update_emotions(&profile, &profile, &EmotionContext { net_reactions_given: 50, ..quiet() });
        assert_eq!(capped.accord, 50 + constants::EMOTION_ACCORD_GIVEN_CAP);
    }

    #[test]
    fn stagnation_lowers_engagement_and_curiosity_only_when_flagged() {
        let profile = EmotionalProfile::default();
        let flagged = update_emotions(&profile, &profile, &EmotionContext { is_discussion_stagnating: true, ..quiet() });
        assert_eq!(flagged.engagement, 50 - constants::EMOTION_STAGNATION_ENG);
        assert_eq!(flagged.curiosite, 50 - constants::EMOTION_STAGNATION_CURIOSITE);
        let lively = update_emotions(&profile, &profile, &quiet());
        assert_eq!(lively.engagement, 50);
    }

    #[test]
    fn no_u8_overflow_with_many_likes() {
        let profile = EmotionalProfile::default();
        let ctx = EmotionContext { likes_received: 100, ..quiet() };
        let result = update_emotions(&profile, &profile, &ctx);
        assert!(result.confiance <= 100);
        assert!(result.engagement <= 100);
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
}

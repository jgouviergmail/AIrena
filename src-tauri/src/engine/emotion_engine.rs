use crate::constants;
use crate::models::emotion::EmotionalProfile;

/// Context for rule-based emotion update
pub struct EmotionContext {
    pub likes_received: u32,
    pub dislikes_received: u32,
    pub was_recently_banned: bool,
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

pub fn update_emotions(current: &EmotionalProfile, ctx: &EmotionContext) -> EmotionalProfile {
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

    // Ban -> frustration way up, engagement down
    if ctx.was_recently_banned {
        new.frustration = add_clamped(new.frustration, constants::EMOTION_BAN_FRUST);
        new.engagement = sub_clamped(new.engagement, constants::EMOTION_BAN_ENG);
    }

    // Stagnation -> engagement down, curiosite down
    if ctx.is_discussion_stagnating {
        new.engagement = sub_clamped(new.engagement, constants::EMOTION_STAGNATION_ENG);
        new.curiosite = sub_clamped(new.curiosite, constants::EMOTION_STAGNATION_CURIOSITE);
    }

    // Natural decay: extremes return toward target
    new.frustration = decay_toward(
        new.frustration,
        constants::EMOTION_DECAY_FRUSTRATION_TARGET,
        constants::EMOTION_DECAY_FRUSTRATION_RATE,
    );
    new.enthousiasme = decay_toward(
        new.enthousiasme,
        constants::EMOTION_DECAY_ENTHUSIASM_TARGET,
        constants::EMOTION_DECAY_ENTHUSIASM_RATE,
    );

    new
}

/// Saturating addition clamped to 100
pub fn add_clamped(val: u8, delta: u8) -> u8 {
    val.saturating_add(delta).min(100)
}

/// Saturating subtraction (floors at 0)
pub fn sub_clamped(val: u8, delta: u8) -> u8 {
    val.saturating_sub(delta)
}

/// Progressive return toward a target value
fn decay_toward(val: u8, target: u8, rate: u8) -> u8 {
    if val > target {
        val.saturating_sub(rate)
    } else if val < target {
        val.saturating_add(rate).min(100)
    } else {
        val
    }
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

    #[test]
    fn test_default_emotions_unchanged_with_empty_context() {
        let profile = EmotionalProfile::default();
        let ctx = EmotionContext {
            likes_received: 0,
            dislikes_received: 0,
            was_recently_banned: false,
            is_discussion_stagnating: false,
        };
        let result = update_emotions(&profile, &ctx);
        // Only decay should apply
        assert_eq!(result.engagement, 50);
        assert_eq!(result.confiance, 50);
        // Frustration starts at 10, decays toward 50 by 2
        assert_eq!(result.frustration, 12);
        // Enthousiasme starts at 50, no change
        assert_eq!(result.enthousiasme, 50);
    }

    #[test]
    fn test_likes_increase_confiance_and_engagement() {
        let profile = EmotionalProfile::default();
        let ctx = EmotionContext {
            likes_received: 2,
            dislikes_received: 0,
            was_recently_banned: false,
            is_discussion_stagnating: false,
        };
        let result = update_emotions(&profile, &ctx);
        // 50 + 10 (likes) + 5 (support bonus, 2 likes >= 2) = 65
        assert_eq!(result.confiance, 65);
        assert_eq!(result.engagement, 56); // 50 + 6
    }

    #[test]
    fn test_values_clamp_at_100() {
        let mut profile = EmotionalProfile::default();
        profile.confiance = 95;
        let ctx = EmotionContext {
            likes_received: 3,
            dislikes_received: 0,
            was_recently_banned: false,
            is_discussion_stagnating: false,
        };
        let result = update_emotions(&profile, &ctx);
        assert_eq!(result.confiance, 100); // 95 + 15, clamped at 100
    }

    #[test]
    fn test_ban_increases_frustration() {
        let profile = EmotionalProfile::default();
        let ctx = EmotionContext {
            likes_received: 0,
            dislikes_received: 0,
            was_recently_banned: true,
            is_discussion_stagnating: false,
        };
        let result = update_emotions(&profile, &ctx);
        // frustration: 10 + 15 = 25, then decay toward 50 by 2 = 27
        assert_eq!(result.frustration, 27);
        assert_eq!(result.engagement, 40); // 50 - 10
    }

    #[test]
    fn test_no_u8_overflow_with_many_likes() {
        let profile = EmotionalProfile::default();
        let ctx = EmotionContext {
            likes_received: 100,
            dislikes_received: 0,
            was_recently_banned: false,
            is_discussion_stagnating: false,
        };
        // Should not panic
        let result = update_emotions(&profile, &ctx);
        assert!(result.confiance <= 100);
        assert!(result.engagement <= 100);
    }

    #[test]
    fn test_detect_thresholds_high() {
        let prev = EmotionalProfile { frustration: 80, ..Default::default() };
        let curr = EmotionalProfile { frustration: 90, ..Default::default() };
        let thresholds = detect_thresholds(&prev, &curr);
        assert_eq!(thresholds.len(), 1);
        assert_eq!(thresholds[0].0, "frustration");
        assert_eq!(thresholds[0].1, "high");
        assert_eq!(thresholds[0].2, 90);
    }

    #[test]
    fn test_detect_thresholds_low() {
        let prev = EmotionalProfile { engagement: 20, ..Default::default() };
        let curr = EmotionalProfile { engagement: 10, ..Default::default() };
        let thresholds = detect_thresholds(&prev, &curr);
        assert_eq!(thresholds.len(), 1);
        assert_eq!(thresholds[0].0, "engagement");
        assert_eq!(thresholds[0].1, "low");
    }

    #[test]
    fn test_detect_thresholds_already_above() {
        // Already above threshold → no new crossing
        let prev = EmotionalProfile { frustration: 90, ..Default::default() };
        let curr = EmotionalProfile { frustration: 95, ..Default::default() };
        let thresholds = detect_thresholds(&prev, &curr);
        assert!(thresholds.is_empty());
    }

    #[test]
    fn test_contagion_moves_toward_average() {
        let avg = EmotionalProfile { engagement: 80, ..Default::default() };
        let mut target = EmotionalProfile { engagement: 40, ..Default::default() };
        apply_contagion(&avg, &mut target);
        // engagement: diff = 40, * 0.05 = 2.0, so target goes 40 → 42
        assert_eq!(target.engagement, 42);
    }

    #[test]
    fn test_contagion_capped_at_3() {
        let avg = EmotionalProfile { engagement: 100, ..Default::default() };
        let mut target = EmotionalProfile { engagement: 0, ..Default::default() };
        apply_contagion(&avg, &mut target);
        // diff = 100 * 0.05 = 5.0, clamped to 3
        assert_eq!(target.engagement, 3);
    }

    #[test]
    fn test_compute_average() {
        let a = EmotionalProfile { engagement: 80, accord: 60, confiance: 40, frustration: 20, curiosite: 90, enthousiasme: 50 };
        let b = EmotionalProfile { engagement: 40, accord: 80, confiance: 60, frustration: 40, curiosite: 30, enthousiasme: 70 };
        let avg = compute_average(&[&a, &b]);
        assert_eq!(avg.engagement, 60);
        assert_eq!(avg.accord, 70);
        assert_eq!(avg.confiance, 50);
        assert_eq!(avg.frustration, 30);
        assert_eq!(avg.curiosite, 60);
        assert_eq!(avg.enthousiasme, 60);
    }
}

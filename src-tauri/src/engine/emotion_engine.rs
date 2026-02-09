use crate::models::emotion::EmotionalProfile;

/// Context for rule-based emotion update
pub struct EmotionContext {
    pub likes_received: u32,
    pub dislikes_received: u32,
    pub was_recently_banned: bool,
    pub is_discussion_stagnating: bool,
}

impl EmotionContext {
    /// Heuristic: contradiction = >= 2 dislikes received
    pub fn was_contradicted(&self) -> bool {
        self.dislikes_received >= 2
    }

    /// Heuristic: support = >= 2 likes received
    pub fn was_supported(&self) -> bool {
        self.likes_received >= 2
    }
}

pub fn update_emotions(current: &EmotionalProfile, ctx: &EmotionContext) -> EmotionalProfile {
    let mut new = current.clone();

    // Likes -> confiance up, engagement up (compute in u16 to avoid u8 overflow)
    if ctx.likes_received > 0 {
        let delta_conf = ((5u16 * ctx.likes_received as u16).min(15)) as u8;
        let delta_eng = ((3u16 * ctx.likes_received as u16).min(10)) as u8;
        new.confiance = add_clamped(new.confiance, delta_conf);
        new.engagement = add_clamped(new.engagement, delta_eng);
    }

    // Dislikes -> frustration up, confiance down
    if ctx.dislikes_received > 0 {
        let delta_frust = ((5u16 * ctx.dislikes_received as u16).min(15)) as u8;
        let delta_conf = ((3u16 * ctx.dislikes_received as u16).min(10)) as u8;
        new.frustration = add_clamped(new.frustration, delta_frust);
        new.confiance = sub_clamped(new.confiance, delta_conf);
    }

    // Contradiction -> frustration up, engagement up
    if ctx.was_contradicted() {
        new.frustration = add_clamped(new.frustration, 8);
        new.engagement = add_clamped(new.engagement, 5);
    }

    // Support -> enthousiasme up, confiance up
    if ctx.was_supported() {
        new.enthousiasme = add_clamped(new.enthousiasme, 8);
        new.confiance = add_clamped(new.confiance, 5);
    }

    // Ban -> frustration way up, engagement down
    if ctx.was_recently_banned {
        new.frustration = add_clamped(new.frustration, 15);
        new.engagement = sub_clamped(new.engagement, 10);
    }

    // Stagnation -> engagement down, curiosite down
    if ctx.is_discussion_stagnating {
        new.engagement = sub_clamped(new.engagement, 5);
        new.curiosite = sub_clamped(new.curiosite, 5);
    }

    // Natural decay: extremes return toward 50
    new.frustration = decay_toward(new.frustration, 50, 2);
    new.enthousiasme = decay_toward(new.enthousiasme, 50, 1);

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

pub const HIGH_THRESHOLD: u8 = 85;
pub const LOW_THRESHOLD: u8 = 15;

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
        if p < HIGH_THRESHOLD && c >= HIGH_THRESHOLD {
            crossed.push((name.to_string(), "high".to_string(), c));
        }
        if p > LOW_THRESHOLD && c <= LOW_THRESHOLD {
            crossed.push((name.to_string(), "low".to_string(), c));
        }
    }
    crossed
}

/// Apply emotional contagion: move target toward the group average.
/// The contagion is weak (±3 max per axis, 5% of distance).
pub fn apply_contagion(avg: &EmotionalProfile, target: &mut EmotionalProfile) {
    use super::apply_i8_clamped;
    fn contagion_delta(avg_val: u8, current_val: u8) -> i8 {
        let diff = avg_val as f32 - current_val as f32;
        (diff * 0.05).round().clamp(-3.0, 3.0) as i8
    }
    target.engagement = apply_i8_clamped(target.engagement, contagion_delta(avg.engagement, target.engagement));
    target.accord = apply_i8_clamped(target.accord, contagion_delta(avg.accord, target.accord));
    target.confiance = apply_i8_clamped(target.confiance, contagion_delta(avg.confiance, target.confiance));
    target.frustration = apply_i8_clamped(target.frustration, contagion_delta(avg.frustration, target.frustration));
    target.curiosite = apply_i8_clamped(target.curiosite, contagion_delta(avg.curiosite, target.curiosite));
    target.enthousiasme = apply_i8_clamped(target.enthousiasme, contagion_delta(avg.enthousiasme, target.enthousiasme));
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

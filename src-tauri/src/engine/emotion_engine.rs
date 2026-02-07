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
fn add_clamped(val: u8, delta: u8) -> u8 {
    val.saturating_add(delta).min(100)
}

/// Saturating subtraction (floors at 0)
fn sub_clamped(val: u8, delta: u8) -> u8 {
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
}

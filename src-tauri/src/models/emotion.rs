use serde::{Deserialize, Serialize};

/// Emotional profile with 6 axes (0-100 each)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmotionalProfile {
    pub engagement: u8,
    pub accord: u8,
    pub confiance: u8,
    pub frustration: u8,
    pub curiosite: u8,
    pub enthousiasme: u8,
}

impl Default for EmotionalProfile {
    fn default() -> Self {
        Self {
            engagement: 50,
            accord: 50,
            confiance: 50,
            frustration: 10,
            curiosite: 50,
            enthousiasme: 50,
        }
    }
}

impl EmotionalProfile {
    /// Parse from an optional JSON string, falling back to default
    pub fn from_json_opt(json: Option<&str>) -> Self {
        json.and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default()
    }

    /// Apply a signed delta to each axis, clamping to 0-100
    pub fn apply_delta(&mut self, delta: &EmotionDelta) {
        use crate::engine::apply_i8_clamped;
        self.engagement = apply_i8_clamped(self.engagement, delta.engagement);
        self.accord = apply_i8_clamped(self.accord, delta.accord);
        self.confiance = apply_i8_clamped(self.confiance, delta.confiance);
        self.frustration = apply_i8_clamped(self.frustration, delta.frustration);
        self.curiosite = apply_i8_clamped(self.curiosite, delta.curiosite);
        self.enthousiasme = apply_i8_clamped(self.enthousiasme, delta.enthousiasme);
    }
}

/// Snapshot of emotions at a given turn
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmotionSnapshot {
    pub turn: u32,
    pub emotions: EmotionalProfile,
}

/// Signed deltas returned by LLM emotion analysis (all fields default to 0)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmotionDelta {
    #[serde(default)]
    pub engagement: i8,
    #[serde(default)]
    pub accord: i8,
    #[serde(default)]
    pub confiance: i8,
    #[serde(default)]
    pub frustration: i8,
    #[serde(default)]
    pub curiosite: i8,
    #[serde(default)]
    pub enthousiasme: i8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_delta_clamp() {
        let mut profile = EmotionalProfile {
            engagement: 95,
            accord: 5,
            confiance: 50,
            frustration: 50,
            curiosite: 50,
            enthousiasme: 50,
        };
        let delta = EmotionDelta {
            engagement: 10,  // 95+10 = 105 → clamped to 100
            accord: -10,     // 5-10 = -5 → clamped to 0
            confiance: 0,
            frustration: -5,
            curiosite: 5,
            enthousiasme: 0,
        };
        profile.apply_delta(&delta);
        assert_eq!(profile.engagement, 100);
        assert_eq!(profile.accord, 0);
        assert_eq!(profile.confiance, 50);
        assert_eq!(profile.frustration, 45);
        assert_eq!(profile.curiosite, 55);
    }

    #[test]
    fn test_from_json_opt() {
        let json = r#"{"engagement":80,"accord":30,"confiance":70,"frustration":20,"curiosite":90,"enthousiasme":60}"#;
        let profile = EmotionalProfile::from_json_opt(Some(json));
        assert_eq!(profile.engagement, 80);
        assert_eq!(profile.curiosite, 90);

        // Invalid JSON → default
        let profile = EmotionalProfile::from_json_opt(Some("not json"));
        assert_eq!(profile.engagement, 50);

        // None → default
        let profile = EmotionalProfile::from_json_opt(None);
        assert_eq!(profile.frustration, 10);
    }
}

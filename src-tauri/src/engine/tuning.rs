//! Tunable dynamics of the engine (v1.17, editable since v1.20). Defaults are
//! the constants; the advanced settings persist a JSON override
//! (`advanced_tuning_json`) that `validate` clamps into safe bounds before the
//! engine reads it — the functions that consume a value never see one out of
//! range.

use serde::{Deserialize, Serialize};

use crate::constants;

/// Inclusive bounds of one knob (shown by the advanced settings as slider limits).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bounds {
    pub min: f64,
    pub max: f64,
}

const fn b(min: f64, max: f64) -> Bounds {
    Bounds { min, max }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Tuning {
    /// Bounds of the persona gains derived from OCEAN (1.0 = the v1.16 rule deltas)
    pub ocean_gain_min: f32,
    pub ocean_gain_max: f32,
    /// Sampling modulation by emotions (interventions only)
    pub emotion_temp_span: f32,
    pub emotion_temp_min: f32,
    pub emotion_temp_max: f32,
    pub emotion_len_min: f32,
    pub emotion_len_max: f32,
    /// Weighted relationship scores
    pub relationship_decay_per_turn: f32,
    pub relationship_ally_score: f32,
    pub relationship_rival_score: f32,
    pub relationship_tense_score: f32,
    /// Scene events (v1.18): probability per turn and its boost while the discussion stagnates
    pub scene_event_base_probability: f64,
    pub scene_event_stagnation_boost: f64,
    /// Coalitions (v1.18): probability per turn when an ally pair is active
    pub coalition_probability: f64,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            ocean_gain_min: constants::OCEAN_GAIN_MIN,
            ocean_gain_max: constants::OCEAN_GAIN_MAX,
            emotion_temp_span: constants::EMOTION_TEMP_SPAN,
            emotion_temp_min: constants::EMOTION_TEMP_MIN,
            emotion_temp_max: constants::EMOTION_TEMP_MAX,
            emotion_len_min: constants::EMOTION_LEN_MIN,
            emotion_len_max: constants::EMOTION_LEN_MAX,
            relationship_decay_per_turn: constants::RELATIONSHIP_DECAY_PER_TURN,
            relationship_ally_score: constants::RELATIONSHIP_ALLY_SCORE,
            relationship_rival_score: constants::RELATIONSHIP_RIVAL_SCORE,
            relationship_tense_score: constants::RELATIONSHIP_TENSE_SCORE,
            scene_event_base_probability: constants::SCENE_EVENT_BASE_PROBABILITY,
            scene_event_stagnation_boost: constants::SCENE_EVENT_STAGNATION_BOOST,
            coalition_probability: constants::COALITION_PROBABILITY,
        }
    }
}

/// Bounds of every knob, in the field order of [`Tuning`] (the advanced settings mirror them).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TuningBounds {
    pub ocean_gain_min: Bounds,
    pub ocean_gain_max: Bounds,
    pub emotion_temp_span: Bounds,
    pub emotion_temp_min: Bounds,
    pub emotion_temp_max: Bounds,
    pub emotion_len_min: Bounds,
    pub emotion_len_max: Bounds,
    pub relationship_decay_per_turn: Bounds,
    pub relationship_ally_score: Bounds,
    pub relationship_rival_score: Bounds,
    pub relationship_tense_score: Bounds,
    pub scene_event_base_probability: Bounds,
    pub scene_event_stagnation_boost: Bounds,
    pub coalition_probability: Bounds,
}

impl Tuning {
    pub const BOUNDS: TuningBounds = TuningBounds {
        ocean_gain_min: b(0.2, 1.0),
        ocean_gain_max: b(1.0, 3.0),
        emotion_temp_span: b(0.0, 0.5),
        emotion_temp_min: b(0.0, 1.0),
        emotion_temp_max: b(0.5, 2.0),
        emotion_len_min: b(0.3, 1.0),
        emotion_len_max: b(1.0, 2.0),
        relationship_decay_per_turn: b(0.5, 1.0),
        relationship_ally_score: b(0.5, 10.0),
        relationship_rival_score: b(0.5, 10.0),
        relationship_tense_score: b(0.5, 10.0),
        scene_event_base_probability: b(0.0, 1.0),
        scene_event_stagnation_boost: b(0.0, 1.0),
        coalition_probability: b(0.0, 1.0),
    };

    /// Parse the persisted override; unreadable JSON yields the defaults.
    pub fn from_json(json: &str) -> Self {
        if json.trim().is_empty() {
            return Self::default();
        }
        match serde_json::from_str::<Self>(json) {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!(error = %e, "Advanced tuning unreadable — defaults used");
                Self::default()
            }
        }
    }

    /// Clamp every knob into its bounds, keep min ≤ max pairs consistent, and
    /// report what was corrected (S43). Non-finite values fall back to the default.
    pub fn validate(&mut self) -> Vec<String> {
        let defaults = Self::default();
        let bounds = &Self::BOUNDS;
        let mut fixes = Vec::new();
        macro_rules! clamp {
            ($field:ident) => {{
                let value = f64::from(self.$field);
                let fixed = if !value.is_finite() { f64::from(defaults.$field) } else { value.clamp(bounds.$field.min, bounds.$field.max) };
                if (fixed - value).abs() > f64::EPSILON || !value.is_finite() {
                    fixes.push(format!("{} {} → {}", stringify!($field), value, fixed));
                    self.$field = fixed as _;
                }
            }};
        }
        clamp!(ocean_gain_min);
        clamp!(ocean_gain_max);
        clamp!(emotion_temp_span);
        clamp!(emotion_temp_min);
        clamp!(emotion_temp_max);
        clamp!(emotion_len_min);
        clamp!(emotion_len_max);
        clamp!(relationship_decay_per_turn);
        clamp!(relationship_ally_score);
        clamp!(relationship_rival_score);
        clamp!(relationship_tense_score);
        clamp!(scene_event_base_probability);
        clamp!(scene_event_stagnation_boost);
        clamp!(coalition_probability);
        // Ordered pairs: a crossed pair is reset to its defaults
        if self.emotion_temp_min > self.emotion_temp_max {
            fixes.push(format!("emotion_temp_min {} > emotion_temp_max {} → defaults", self.emotion_temp_min, self.emotion_temp_max));
            self.emotion_temp_min = defaults.emotion_temp_min;
            self.emotion_temp_max = defaults.emotion_temp_max;
        }
        if self.emotion_len_min > self.emotion_len_max {
            fixes.push(format!("emotion_len_min {} > emotion_len_max {} → defaults", self.emotion_len_min, self.emotion_len_max));
            self.emotion_len_min = defaults.emotion_len_min;
            self.emotion_len_max = defaults.emotion_len_max;
        }
        if self.ocean_gain_min > self.ocean_gain_max {
            fixes.push(format!("ocean_gain_min {} > ocean_gain_max {} → defaults", self.ocean_gain_min, self.ocean_gain_max));
            self.ocean_gain_min = defaults.ocean_gain_min;
            self.ocean_gain_max = defaults.ocean_gain_max;
        }
        for fix in &fixes {
            tracing::warn!(fix, "Advanced tuning corrected");
        }
        fixes
    }

    /// The validated tuning of the settings (defaults when the override is empty).
    pub fn from_settings(json: &str) -> Self {
        let mut tuning = Self::from_json(json);
        tuning.validate();
        tuning
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// S43 — defaults are the constants; out-of-bound values are clamped and reported; crossed pairs reset.
    #[test]
    fn defaults_are_the_constants_and_validation_corrects_the_rest() {
        let mut t = Tuning::default();
        assert!(t.validate().is_empty());
        assert_eq!(t.scene_event_base_probability, constants::SCENE_EVENT_BASE_PROBABILITY);
        assert_eq!(t.coalition_probability, constants::COALITION_PROBABILITY);

        let mut wild: Tuning = serde_json::from_str(r#"{"sceneEventBaseProbability": 4.0, "emotionTempMin": 0.9, "emotionTempMax": 0.5, "relationshipDecayPerTurn": -1}"#).unwrap();
        let fixes = wild.validate();
        assert_eq!(wild.scene_event_base_probability, 1.0);
        assert_eq!(wild.relationship_decay_per_turn, 0.5);
        assert_eq!((wild.emotion_temp_min, wild.emotion_temp_max), (constants::EMOTION_TEMP_MIN, constants::EMOTION_TEMP_MAX));
        assert!(fixes.iter().any(|f| f.starts_with("scene_event_base_probability")) && fixes.iter().any(|f| f.contains("emotion_temp_min")), "{fixes:?}");
        // Untouched knobs keep their defaults (partial JSON)
        assert_eq!(wild.ocean_gain_max, constants::OCEAN_GAIN_MAX);
        // Unreadable JSON → defaults; empty → defaults; NaN → default
        assert_eq!(Tuning::from_settings("nope"), Tuning::default());
        assert_eq!(Tuning::from_settings(""), Tuning::default());
        let mut nan = Tuning { emotion_len_max: f32::NAN, ..Tuning::default() };
        nan.validate();
        assert_eq!(nan.emotion_len_max, constants::EMOTION_LEN_MAX);
        // Round trip in camelCase
        let json = serde_json::to_string(&Tuning::default()).unwrap();
        assert!(json.contains("\"oceanGainMin\""));
        assert_eq!(Tuning::from_settings(&json), Tuning::default());
    }
}

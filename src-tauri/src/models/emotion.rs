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

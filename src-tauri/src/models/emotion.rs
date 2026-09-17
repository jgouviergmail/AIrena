use serde::{Deserialize, Serialize};

/// Emotional profile with 6 axes (0-100 each)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
}

/// Overall temperature of the room, from the average profile of the active gladiateurs (v1.17).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RoomMood {
    Tense,
    Flat,
    Lively,
    Serene,
}

impl RoomMood {
    /// Hint for the moderator prompt ("ambiance : tendue — calme le jeu").
    pub fn moderator_hint(&self, lang: &str) -> &'static str {
        match (self, lang) {
            (Self::Tense, "en") => "Room mood: tense — calm things down, defuse before it turns personal.",
            (Self::Tense, "zh") => "现场气氛：紧张——平息局面，在演变成人身攻击前化解。",
            (Self::Tense, _) => "Ambiance de la salle : tendue — calme le jeu, désamorce avant que cela devienne personnel.",
            (Self::Flat, "en") => "Room mood: flat — a sharper question or a challenge would wake the room up.",
            (Self::Flat, "zh") => "现场气氛：沉闷——一个更尖锐的问题或挑战能唤醒全场。",
            (Self::Flat, _) => "Ambiance de la salle : molle — une question plus tranchante ou un défi réveillerait la salle.",
            (Self::Lively, "en") => "Room mood: lively — keep the energy, just keep it fair.",
            (Self::Lively, "zh") => "现场气氛：活跃——保持这股劲，只需保证公平。",
            (Self::Lively, _) => "Ambiance de la salle : vive — garde cette énergie, veille seulement à l'équité.",
            (Self::Serene, "en") => "Room mood: serene — no intervention needed on tone.",
            (Self::Serene, "zh") => "现场气氛：平和——语气方面无需干预。",
            (Self::Serene, _) => "Ambiance de la salle : sereine — aucune intervention nécessaire sur le ton.",
        }
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

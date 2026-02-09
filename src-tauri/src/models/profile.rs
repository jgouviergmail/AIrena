use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PredefinedProfile {
    pub id: String,
    pub name: String,
    pub personality: String,
    pub system_prompt: String,
    pub is_builtin: bool,
    #[serde(default = "default_profile_type")]
    pub profile_type: String,
    #[serde(default = "default_category")]
    pub category: String,
    /// JSON string of initial EmotionalProfile (personality-specific starting emotions)
    #[serde(default)]
    pub initial_emotions: Option<String>,
}

fn default_profile_type() -> String {
    "gladiateur".to_string()
}

fn default_category() -> String {
    "autres".to_string()
}

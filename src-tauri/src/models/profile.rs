use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PredefinedProfile {
    pub id: String,
    pub name: String,
    pub personality: String,
    pub system_prompt: String,
    pub is_builtin: bool,
}

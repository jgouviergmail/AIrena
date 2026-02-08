use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmParams {
    pub temperature: f32,
    pub top_p: f32,
    pub top_k: u32,
    pub num_predict: i32,
    pub num_ctx: u32,
    pub repeat_penalty: f32,
}

impl Default for LlmParams {
    fn default() -> Self {
        Self {
            temperature: 0.7,
            top_p: 0.9,
            top_k: 40,
            num_predict: 1024,
            num_ctx: 8192,
            repeat_penalty: 1.1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub username: String,
    pub language: String,
    pub theme: String,
    pub ollama_url: String,
    pub ollama_model: String,
    pub emotion_driven: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            username: String::new(),
            language: "fr".to_string(),
            theme: "dark".to_string(),
            ollama_url: "http://localhost:11434".to_string(),
            ollama_model: String::new(),
            emotion_driven: false,
        }
    }
}

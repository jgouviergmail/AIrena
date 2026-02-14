use serde::{Deserialize, Serialize};

use crate::constants;

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
            temperature: constants::LLM_DEFAULT_TEMPERATURE,
            top_p: constants::LLM_DEFAULT_TOP_P,
            top_k: constants::LLM_DEFAULT_TOP_K,
            num_predict: constants::LLM_DEFAULT_NUM_PREDICT,
            num_ctx: constants::LLM_DEFAULT_NUM_CTX,
            repeat_penalty: constants::LLM_DEFAULT_REPEAT_PENALTY,
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
    pub tavily_api_key: String,
    pub tavily_period_start: String,
    pub tavily_usage_count: u32,
    pub tavily_usage_history: String,
    #[serde(default)]
    pub embedding_model: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            username: String::new(),
            language: "fr".to_string(),
            theme: "dark".to_string(),
            ollama_url: constants::DEFAULT_OLLAMA_URL.to_string(),
            ollama_model: String::new(),
            emotion_driven: false,
            tavily_api_key: String::new(),
            tavily_period_start: String::new(),
            tavily_usage_count: 0,
            tavily_usage_history: "[]".to_string(),
            embedding_model: String::new(),
        }
    }
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<ChatOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub think: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_predict: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_ctx: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repeat_penalty: Option<f32>,
}

#[derive(Debug, Deserialize)]
pub struct ChatResponse {
    pub message: ChatResponseMessage,
    pub done: bool,
    /// Ollama sets this to "length" when the model hit num_predict token limit,
    /// or "stop" for normal EOS completion.
    #[serde(default)]
    pub done_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChatResponseMessage {
    #[allow(dead_code)]
    pub role: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub thinking: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub digest: String,
}

// ── /api/show response ──────────────────────────────────────────────────

/// Response from Ollama's POST `/api/show` endpoint.
/// Contains model architecture details, parameters, and quantization info.
#[derive(Debug, Clone, Deserialize)]
pub struct ShowResponse {
    /// Template used for prompt formatting.
    #[serde(default)]
    pub template: String,
    /// Model metadata (family, parameter_size, quantization_level).
    #[serde(default)]
    pub details: ShowDetails,
    /// Architecture-specific model info. Keys are prefixed by model family
    /// (e.g. `"llama.block_count"`, `"qwen2.embedding_length"`).
    #[serde(default)]
    pub model_info: std::collections::HashMap<String, serde_json::Value>,
}

/// Model details from the `details` field of `/api/show`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ShowDetails {
    #[serde(default)]
    pub parent_model: String,
    #[serde(default)]
    pub format: String,
    #[serde(default)]
    pub family: String,
    #[serde(default)]
    pub parameter_size: String,
    #[serde(default)]
    pub quantization_level: String,
}

// ── /api/ps response ────────────────────────────────────────────────────

/// Response from Ollama's GET `/api/ps` endpoint.
/// Lists currently loaded models with VRAM usage.
#[derive(Debug, Clone, Deserialize)]
pub struct PsResponse {
    #[serde(default)]
    pub models: Vec<PsModel>,
}

/// A single running model from `/api/ps`.
#[derive(Debug, Clone, Deserialize)]
pub struct PsModel {
    /// Full model name (e.g. "llama3.1:8b-instruct-q4_K_M").
    #[serde(default)]
    pub name: String,
    /// Short model identifier.
    #[serde(default)]
    pub model: String,
    /// Total model size in bytes.
    #[serde(default)]
    pub size: u64,
    /// VRAM occupied by this model in bytes.
    #[serde(default)]
    pub size_vram: u64,
    /// Expiry timestamp for keep-alive.
    #[serde(default)]
    pub expires_at: String,
}

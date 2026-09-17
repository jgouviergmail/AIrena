use serde::{Deserialize, Serialize};

use super::llm::{ProviderKind, ReasoningLevel, ReasoningPace};
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
    /// Per-speaker reasoning level; `None` inherits the global setting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_level: Option<ReasoningLevel>,
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
            reasoning_level: None,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
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
    #[serde(default)]
    pub license_key: String,
    /// JSON-serialized token budget priorities (user-customizable ordering).
    /// Empty string means "use defaults".
    #[serde(default)]
    pub token_budget_priorities: String,
    /// Global context window size in tokens (shared by all speakers).
    /// Ollama: auto-detected from GPU VRAM. DeepSeek: a cost-driven context budget.
    #[serde(default = "default_num_ctx")]
    pub num_ctx: u32,

    // ── LLM provider ────────────────────────────────────────────────
    /// Backend serving discussions.
    #[serde(default)]
    pub llm_provider: ProviderKind,
    /// Global default reasoning level (per-speaker override in `LlmParams`).
    #[serde(default = "default_reasoning_level")]
    pub reasoning_level: ReasoningLevel,
    /// Show the model's raw reasoning in the "thoughts" panel when displayable.
    #[serde(default = "default_true")]
    pub show_model_reasoning: bool,
    /// Wall-clock pace of the reasoning (`Fast` caps `Auto` and shrinks allowances).
    #[serde(default)]
    pub reasoning_pace: ReasoningPace,

    // ── Audio (v1.18) ───────────────────────────────────────────────
    /// Read the interventions aloud (browser speech synthesis).
    #[serde(default)]
    pub tts_enabled: bool,
    /// `Follow`: drop the backlog when the speaker changes; `Full`: read everything.
    #[serde(default)]
    pub tts_mode: TtsMode,
    #[serde(default = "default_tts_volume")]
    pub tts_volume: f32,
    /// Procedural sounds (gong at each turn, applause, whistle…).
    #[serde(default)]
    pub sound_enabled: bool,
    #[serde(default = "default_sound_volume")]
    pub sound_volume: f32,

    // ── DeepSeek ────────────────────────────────────────────────────
    #[serde(default)]
    pub deepseek_api_key: String,
    #[serde(default = "default_deepseek_model")]
    pub deepseek_model: String,
    /// Monthly spending cap in USD (0 = unlimited).
    #[serde(default)]
    pub deepseek_monthly_budget_usd: f64,
    /// Start date (YYYY-MM-DD) of the rolling monthly period.
    #[serde(default)]
    pub deepseek_period_start: String,
    /// JSON `PeriodUsage` for the current period.
    #[serde(default = "default_period_usage_json")]
    pub deepseek_period_usage_json: String,
    /// JSON array of archived periods.
    #[serde(default = "default_json_array")]
    pub deepseek_usage_history: String,

    // ── OpenAI-compatible server (v1.20) ────────────────────────────
    #[serde(default = "default_openai_compat_base_url")]
    pub openai_compat_base_url: String,
    /// Empty for a local server without authentication
    #[serde(default)]
    pub openai_compat_api_key: String,
    #[serde(default)]
    pub openai_compat_model: String,
    /// Manual model list (JSON array) used when the server publishes no catalogue
    #[serde(default = "default_json_array")]
    pub openai_compat_models: String,

    // ── Advanced tuning and long memory (v1.20) ─────────────────────
    /// JSON override of `engine::tuning::Tuning` (empty / "{}" = the constants)
    #[serde(default = "default_json_object")]
    pub advanced_tuning_json: String,
    /// Personas remember their past discussions (recaps written and recalled)
    #[serde(default = "default_true")]
    pub persona_memory_enabled: bool,
}

fn default_json_object() -> String {
    "{}".to_string()
}

fn default_openai_compat_base_url() -> String {
    constants::OPENAI_COMPAT_DEFAULT_BASE_URL.to_string()
}

/// Secrets are masked so that a `{:?}` in a log line can never leak a key.
impl std::fmt::Debug for AppSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mask = |s: &str| if s.is_empty() { "<empty>" } else { "<redacted>" };
        f.debug_struct("AppSettings")
            .field("username", &self.username)
            .field("language", &self.language)
            .field("theme", &self.theme)
            .field("ollama_url", &self.ollama_url)
            .field("ollama_model", &self.ollama_model)
            .field("emotion_driven", &self.emotion_driven)
            .field("tavily_api_key", &mask(&self.tavily_api_key))
            .field("tavily_period_start", &self.tavily_period_start)
            .field("tavily_usage_count", &self.tavily_usage_count)
            .field("embedding_model", &self.embedding_model)
            .field("license_key", &mask(&self.license_key))
            .field("num_ctx", &self.num_ctx)
            .field("llm_provider", &self.llm_provider)
            .field("reasoning_level", &self.reasoning_level)
            .field("show_model_reasoning", &self.show_model_reasoning)
            .field("reasoning_pace", &self.reasoning_pace)
            .field("tts_enabled", &self.tts_enabled)
            .field("tts_mode", &self.tts_mode)
            .field("sound_enabled", &self.sound_enabled)
            .field("deepseek_api_key", &mask(&self.deepseek_api_key))
            .field("deepseek_model", &self.deepseek_model)
            .field("deepseek_monthly_budget_usd", &self.deepseek_monthly_budget_usd)
            .field("deepseek_period_start", &self.deepseek_period_start)
            .field("openai_compat_base_url", &self.openai_compat_base_url)
            .field("openai_compat_api_key", &mask(&self.openai_compat_api_key))
            .field("openai_compat_model", &self.openai_compat_model)
            .finish_non_exhaustive()
    }
}

fn default_num_ctx() -> u32 {
    constants::LLM_DEFAULT_NUM_CTX
}

fn default_reasoning_level() -> ReasoningLevel {
    ReasoningLevel::Auto
}

fn default_tts_volume() -> f32 {
    constants::AUDIO_DEFAULT_TTS_VOLUME
}

fn default_sound_volume() -> f32 {
    constants::AUDIO_DEFAULT_SOUND_VOLUME
}

/// How the voice keeps up with the discussion (v1.18).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TtsMode {
    /// The backlog is dropped when the speaker changes (stays live)
    #[default]
    Follow,
    /// Everything is read, however late
    Full,
}

impl TtsMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Follow => "follow",
            Self::Full => "full",
        }
    }

    /// Parse a stored value; unknown values fall back to `Follow`.
    pub fn parse(value: &str) -> Self {
        match value.trim().to_lowercase().as_str() {
            "full" => Self::Full,
            _ => Self::Follow,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_deepseek_model() -> String {
    constants::DEEPSEEK_DEFAULT_MODEL.to_string()
}

fn default_period_usage_json() -> String {
    "{}".to_string()
}

fn default_json_array() -> String {
    "[]".to_string()
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
            license_key: String::new(),
            token_budget_priorities: String::new(),
            num_ctx: constants::LLM_DEFAULT_NUM_CTX,
            llm_provider: ProviderKind::Ollama,
            reasoning_level: ReasoningLevel::Auto,
            show_model_reasoning: true,
            reasoning_pace: ReasoningPace::Normal,
            tts_enabled: false,
            tts_mode: TtsMode::Follow,
            tts_volume: constants::AUDIO_DEFAULT_TTS_VOLUME,
            sound_enabled: false,
            sound_volume: constants::AUDIO_DEFAULT_SOUND_VOLUME,
            deepseek_api_key: String::new(),
            deepseek_model: constants::DEEPSEEK_DEFAULT_MODEL.to_string(),
            deepseek_monthly_budget_usd: 0.0,
            deepseek_period_start: String::new(),
            deepseek_period_usage_json: "{}".to_string(),
            deepseek_usage_history: "[]".to_string(),
            openai_compat_base_url: constants::OPENAI_COMPAT_DEFAULT_BASE_URL.to_string(),
            openai_compat_api_key: String::new(),
            openai_compat_model: String::new(),
            openai_compat_models: "[]".to_string(),
            advanced_tuning_json: "{}".to_string(),
            persona_memory_enabled: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llm_params_backward_compatible_without_reasoning_level() {
        let json = r#"{"temperature":0.5,"topP":0.9,"topK":40,"numPredict":512,"numCtx":4096,"repeatPenalty":1.1}"#;
        let p: LlmParams = serde_json::from_str(json).unwrap();
        assert!(p.reasoning_level.is_none());
        // Not serialized when None (keeps frontend payloads unchanged)
        assert!(!serde_json::to_string(&p).unwrap().contains("reasoningLevel"));
        let p2 = LlmParams { reasoning_level: Some(ReasoningLevel::High), ..p };
        assert!(serde_json::to_string(&p2).unwrap().contains("\"reasoningLevel\":\"high\""));
    }

    #[test]
    fn app_settings_defaults_for_new_fields() {
        // A v1.15 payload without any provider field must deserialize to Ollama defaults.
        let json = r#"{"username":"u","language":"fr","theme":"dark","ollamaUrl":"http://x","ollamaModel":"m","emotionDriven":false,"tavilyApiKey":"","tavilyPeriodStart":"","tavilyUsageCount":0,"tavilyUsageHistory":"[]"}"#;
        let s: AppSettings = serde_json::from_str(json).unwrap();
        assert_eq!(s.llm_provider, ProviderKind::Ollama);
        assert_eq!(s.reasoning_level, ReasoningLevel::Auto);
        assert!(s.show_model_reasoning);
        assert_eq!(s.reasoning_pace, ReasoningPace::Normal);
        assert!(!s.tts_enabled && !s.sound_enabled);
        assert_eq!(s.tts_mode, TtsMode::Follow);
        assert_eq!(s.tts_volume, constants::AUDIO_DEFAULT_TTS_VOLUME);
        assert_eq!(TtsMode::parse("FULL"), TtsMode::Full);
        assert_eq!(TtsMode::parse("?"), TtsMode::Follow);
        assert_eq!(s.deepseek_model, constants::DEEPSEEK_DEFAULT_MODEL);
        assert_eq!(s.deepseek_period_usage_json, "{}");
        assert_eq!(s.deepseek_usage_history, "[]");
        assert_eq!(s.deepseek_monthly_budget_usd, 0.0);
    }
}

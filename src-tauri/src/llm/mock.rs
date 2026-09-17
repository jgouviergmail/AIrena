//! Scripted in-memory provider for engine tests (no network).

use std::sync::Mutex;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use super::{LlmCapabilities, LlmError, LlmProvider, LlmRequest, LlmResponse, TokenCallback};
use crate::constants;
use crate::models::llm::ProviderKind;

type Script = Box<dyn Fn(&LlmRequest) -> Result<LlmResponse, LlmError> + Send + Sync>;

pub struct MockLlmProvider {
    script: Script,
    caps: LlmCapabilities,
    model_name: String,
    /// What `validate` answers (`None` = success)
    validation_error: Option<LlmError>,
    /// Every request received, in order (for assertions).
    pub calls: Mutex<Vec<LlmRequest>>,
    /// Simulated streaming chunk size (chars); 0 = deliver content in one callback.
    pub chunk_chars: usize,
}

impl MockLlmProvider {
    pub fn scripted(script: impl Fn(&LlmRequest) -> Result<LlmResponse, LlmError> + Send + Sync + 'static) -> Self {
        Self {
            script: Box::new(script),
            caps: LlmCapabilities {
                supports_reasoning: false,
                reasoning_levels: false,
                reasoning_displayable: false,
                supports_json_mode: true,
                context_tokens: constants::LLM_DEFAULT_NUM_CTX,
                chars_per_token_latin: constants::CHARS_PER_TOKEN_LATIN,
                chars_per_token_cjk: constants::CHARS_PER_TOKEN_CJK,
                reports_usage: true,
                billable: false,
                // Two: the end-of-turn calls stay separate (memory, emotion…); set 1
                // with `with_capabilities` to exercise the fused turn analyst.
                max_parallel_calls: 2,
            },
            model_name: "mock-model".to_string(),
            validation_error: None,
            calls: Mutex::new(Vec::new()),
            chunk_chars: 4,
        }
    }

    pub fn with_capabilities(mut self, caps: LlmCapabilities) -> Self {
        self.caps = caps;
        self
    }

    pub fn with_model_name(mut self, name: &str) -> Self {
        self.model_name = name.to_string();
        self
    }

    /// Make `validate` fail (the error is cloned by kind on each call).
    pub fn failing_validation(mut self, error: LlmError) -> Self {
        self.validation_error = Some(error);
        self
    }

    pub fn recorded_calls(&self) -> Vec<LlmRequest> {
        self.calls.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

#[async_trait]
impl LlmProvider for MockLlmProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Ollama
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }

    fn capabilities(&self) -> &LlmCapabilities {
        &self.caps
    }

    async fn chat_stream(
        &self,
        request: &LlmRequest,
        on_content: TokenCallback<'_>,
        on_reasoning: TokenCallback<'_>,
        cancel: CancellationToken,
    ) -> Result<LlmResponse, LlmError> {
        if cancel.is_cancelled() {
            return Err(LlmError::Cancelled);
        }
        self.calls.lock().unwrap_or_else(|e| e.into_inner()).push(request.clone());
        let response = (self.script)(request)?;

        if let Some(reasoning) = &response.reasoning {
            stream_chunks(reasoning, self.chunk_chars, on_reasoning);
        }
        stream_chunks(&response.content, self.chunk_chars, on_content);
        Ok(response)
    }

    async fn validate(&self) -> Result<(), LlmError> {
        match &self.validation_error {
            None => Ok(()),
            Some(LlmError::ModelNotFound(m)) => Err(LlmError::ModelNotFound(m.clone())),
            Some(LlmError::Auth) => Err(LlmError::Auth),
            Some(other) => Err(LlmError::Connection(other.to_string())),
        }
    }
}

fn stream_chunks(text: &str, chunk_chars: usize, cb: TokenCallback<'_>) {
    if text.is_empty() {
        return;
    }
    if chunk_chars == 0 {
        cb(text);
        return;
    }
    let chars: Vec<char> = text.chars().collect();
    for piece in chars.chunks(chunk_chars) {
        let s: String = piece.iter().collect();
        cb(&s);
    }
}

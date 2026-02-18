use std::time::Duration;

use futures_util::StreamExt;
use tokio_util::sync::CancellationToken;

use super::error::OllamaError;
use super::types::{ChatRequest, ChatResponse, ModelInfo, PsResponse, ShowResponse};
use crate::constants;
use crate::models::settings::LlmParams;

/// Result of a streaming chat with think mode
pub struct ChatStreamResult {
    pub content: String,
    pub thinking: Option<String>,
}

#[derive(Clone)]
pub struct OllamaClient {
    client: reqwest::Client,
    base_url: String,
    model: String,
}

impl OllamaClient {
    pub fn new(base_url: &str, model: &str) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(constants::OLLAMA_HTTP_TIMEOUT_SECS))
                .build()
                .expect("Failed to build HTTP client"),
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
        }
    }

    /// Validate that the model exists BEFORE starting a discussion
    pub async fn validate_model(&self) -> Result<(), OllamaError> {
        let models = self.list_models().await?;
        if !models
            .iter()
            .any(|m| m.name == self.model || m.name.starts_with(&self.model))
        {
            return Err(OllamaError::ModelNotFound(self.model.clone()));
        }
        Ok(())
    }

    /// Chat streaming with retry and timeout
    /// NOTE: on_token must be Send to cross .await boundaries
    pub async fn chat_streaming(
        &self,
        request: &ChatRequest,
        on_token: impl Fn(&str) + Send,
        cancel: CancellationToken,
    ) -> Result<String, OllamaError> {
        let result = self
            .chat_streaming_with_think(request, on_token, |_| {}, cancel)
            .await?;
        Ok(result.content)
    }

    /// Chat streaming with think mode — separate callbacks for content and thinking tokens.
    /// Includes retry with exponential backoff (up to 3 attempts).
    pub async fn chat_streaming_with_think(
        &self,
        request: &ChatRequest,
        on_content_token: impl Fn(&str) + Send,
        on_thinking_token: impl Fn(&str) + Send,
        cancel: CancellationToken,
    ) -> Result<ChatStreamResult, OllamaError> {
        for attempt in 0..=constants::OLLAMA_MAX_RETRIES {
            match self
                .stream_ndjson(request, &on_content_token, &on_thinking_token, &cancel)
                .await
            {
                Ok(result) => return Ok(result),
                Err(OllamaError::Cancelled) => return Err(OllamaError::Cancelled),
                Err(e) if e.is_connection_error() && attempt < constants::OLLAMA_MAX_RETRIES => {
                    tracing::warn!(
                        "Ollama connection error (attempt {}): {}",
                        attempt + 1,
                        e
                    );
                    tokio::time::sleep(Duration::from_secs(constants::OLLAMA_RETRY_BACKOFF_BASE_SECS.pow(attempt))).await;
                }
                Err(e) => return Err(e),
            }
        }
        Err(OllamaError::ConnectionLost)
    }

    /// Chat without streaming (for JSON responses like reactions, moderation)
    pub async fn chat(
        &self,
        request: &ChatRequest,
        cancel: CancellationToken,
    ) -> Result<String, OllamaError> {
        self.chat_streaming(request, |_| {}, cancel).await
    }

    /// Unified NDJSON streaming — buffered parsing with Vec<u8>.
    /// Handles both content and thinking tokens via separate callbacks.
    async fn stream_ndjson(
        &self,
        request: &ChatRequest,
        on_content_token: &(impl Fn(&str) + Send),
        on_thinking_token: &(impl Fn(&str) + Send),
        cancel: &CancellationToken,
    ) -> Result<ChatStreamResult, OllamaError> {
        let url = format!("{}/api/chat", self.base_url);
        let response = self.client.post(&url).json(request).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let msg = format!("HTTP {}", status);
            return Err(if status.is_client_error() {
                OllamaError::ClientError(msg)
            } else {
                OllamaError::ConnectionFailed(msg)
            });
        }

        let mut stream = response.bytes_stream();
        let mut buf = Vec::<u8>::new();
        let mut accumulated_content = String::new();
        let mut accumulated_thinking = String::new();

        loop {
            tokio::select! {
                chunk = stream.next() => {
                    match chunk {
                        Some(Ok(bytes)) => {
                            buf.extend_from_slice(&bytes);
                            while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
                                let line: Vec<u8> = buf.drain(..=pos).collect();
                                let line = String::from_utf8_lossy(&line);
                                let line = line.trim();
                                if !line.is_empty() {
                                    let resp: ChatResponse = serde_json::from_str(line)?;
                                    if resp.done {
                                        if resp.done_reason.as_deref() == Some("length") {
                                            tracing::warn!(
                                                chars = accumulated_content.len(),
                                                "Response truncated: model hit num_predict token limit"
                                            );
                                        }
                                        return Ok(ChatStreamResult {
                                            content: accumulated_content,
                                            thinking: if accumulated_thinking.is_empty() {
                                                None
                                            } else {
                                                Some(accumulated_thinking)
                                            },
                                        });
                                    }
                                    if let Some(thinking) = &resp.message.thinking {
                                        if !thinking.is_empty() {
                                            on_thinking_token(thinking);
                                            accumulated_thinking.push_str(thinking);
                                        }
                                    }
                                    if !resp.message.content.is_empty() {
                                        on_content_token(&resp.message.content);
                                        accumulated_content.push_str(&resp.message.content);
                                    }
                                }
                            }
                        }
                        Some(Err(e)) => return Err(OllamaError::RequestFailed(e)),
                        None => {
                            // Process any leftover data in buffer (last line without trailing \n)
                            if !buf.is_empty() {
                                let line = String::from_utf8_lossy(&buf);
                                let line = line.trim();
                                if !line.is_empty() {
                                    if let Ok(resp) = serde_json::from_str::<ChatResponse>(line) {
                                        if !resp.message.content.is_empty() {
                                            on_content_token(&resp.message.content);
                                            accumulated_content.push_str(&resp.message.content);
                                        }
                                        if resp.done_reason.as_deref() == Some("length") {
                                            tracing::warn!(
                                                chars = accumulated_content.len(),
                                                "Response truncated: model hit num_predict token limit"
                                            );
                                        }
                                    }
                                }
                            }
                            return Ok(ChatStreamResult {
                                content: accumulated_content,
                                thinking: if accumulated_thinking.is_empty() {
                                    None
                                } else {
                                    Some(accumulated_thinking)
                                },
                            });
                        }
                    }
                }
                _ = cancel.cancelled() => {
                    return Err(OllamaError::Cancelled);
                }
            }
        }
    }

    /// Unload a specific model from Ollama VRAM (keep_alive: 0).
    /// Uses POST /api/generate with keep_alive:0 — the documented Ollama unload method.
    pub async fn unload_model(
        base_url: &str,
        model_name: &str,
        client: &reqwest::Client,
    ) -> Result<(), OllamaError> {
        let url = format!("{base_url}/api/generate");
        let body = serde_json::json!({
            "model": model_name,
            "keep_alive": 0
        });
        let resp = client
            .post(&url)
            .json(&body)
            .timeout(Duration::from_secs(constants::OLLAMA_CHECK_TIMEOUT_SECS))
            .send()
            .await?;
        if !resp.status().is_success() {
            tracing::warn!("Failed to unload model {model_name}: HTTP {}", resp.status());
        }
        Ok(())
    }

    /// Unload all currently loaded models from Ollama VRAM.
    pub async fn unload_all_models(&self) -> Result<usize, OllamaError> {
        let ps = self.list_running_models().await?;
        let count = ps.models.len();
        for model in &ps.models {
            if let Err(e) =
                Self::unload_model(&self.base_url, &model.name, &self.client).await
            {
                tracing::warn!("Failed to unload {}: {e}", model.name);
            }
        }
        tracing::info!("Unloaded {count} model(s) from VRAM");
        Ok(count)
    }

    /// Preload a model into Ollama's memory by sending a minimal chat request.
    /// This avoids cold-start delays when the discussion starts.
    ///
    /// # Arguments
    /// - `num_ctx` — Context size to allocate. **Critical**: without this, Ollama
    ///   uses the model's native context length (often 128K), which can consume
    ///   all available VRAM just for the KV cache.
    pub async fn preload_model(&self, num_ctx: Option<u32>) -> Result<(), OllamaError> {
        let url = format!("{}/api/chat", self.base_url);
        let mut body = serde_json::json!({
            "model": self.model,
            "messages": [{"role": "user", "content": "hello"}],
            "stream": false,
            "keep_alive": "5m"
        });
        if let Some(ctx) = num_ctx {
            body["options"] = serde_json::json!({ "num_ctx": ctx });
        }
        let resp = self.client
            .post(&url)
            .json(&body)
            .timeout(Duration::from_secs(constants::OLLAMA_PRELOAD_TIMEOUT_SECS))
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(OllamaError::ConnectionFailed(format!(
                "Preload failed: HTTP {}",
                resp.status()
            )));
        }
        Ok(())
    }

    pub async fn check_connection(&self) -> bool {
        self.client
            .get(format!("{}/api/tags", self.base_url))
            .timeout(Duration::from_secs(constants::OLLAMA_CHECK_TIMEOUT_SECS))
            .send()
            .await
            .is_ok()
    }

    pub async fn list_models(&self) -> Result<Vec<ModelInfo>, OllamaError> {
        let url = format!("{}/api/tags", self.base_url);
        let resp = self.client.get(&url).send().await?;
        let body: serde_json::Value = resp.json().await
            .map_err(|e| OllamaError::ConnectionFailed(e.to_string()))?;
        let models = body["models"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|m| serde_json::from_value(m.clone()).ok())
            .collect();
        Ok(models)
    }

    /// Fetch model architecture and metadata via POST `/api/show`.
    pub async fn show_model(&self, model_name: &str) -> Result<ShowResponse, OllamaError> {
        let url = format!("{}/api/show", self.base_url);
        let body = serde_json::json!({ "name": model_name });
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .timeout(Duration::from_secs(constants::OLLAMA_CHECK_TIMEOUT_SECS))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            return Err(if status.as_u16() == 404 {
                OllamaError::ModelNotFound(model_name.to_string())
            } else {
                OllamaError::ConnectionFailed(format!("show_model: HTTP {status}"))
            });
        }

        resp.json::<ShowResponse>()
            .await
            .map_err(|e| OllamaError::ConnectionFailed(format!("show_model parse error: {e}")))
    }

    /// List currently loaded/running models via GET `/api/ps`.
    pub async fn list_running_models(&self) -> Result<PsResponse, OllamaError> {
        let url = format!("{}/api/ps", self.base_url);
        let resp = self
            .client
            .get(&url)
            .timeout(Duration::from_secs(constants::OLLAMA_CHECK_TIMEOUT_SECS))
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(OllamaError::ConnectionFailed(format!(
                "list_running_models: HTTP {}",
                resp.status()
            )));
        }

        resp.json::<PsResponse>()
            .await
            .map_err(|e| OllamaError::ConnectionFailed(format!("list_running_models parse error: {e}")))
    }

    /// Build a ChatRequest from LlmParams
    pub fn build_request(
        &self,
        system_prompt: &str,
        user_message: &str,
        params: &LlmParams,
        json_format: bool,
    ) -> ChatRequest {
        use super::types::{ChatMessage, ChatOptions};

        let mut messages = vec![];
        if !system_prompt.is_empty() {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            });
        }
        messages.push(ChatMessage {
            role: "user".to_string(),
            content: user_message.to_string(),
        });

        ChatRequest {
            model: self.model.clone(),
            messages,
            format: if json_format {
                Some("json".to_string())
            } else {
                None
            },
            stream: true,
            options: Some(ChatOptions {
                temperature: Some(params.temperature),
                top_p: Some(params.top_p),
                top_k: Some(params.top_k),
                num_predict: Some(params.num_predict),
                num_ctx: Some(params.num_ctx),
                repeat_penalty: Some(params.repeat_penalty),
            }),
            think: None,
        }
    }
}

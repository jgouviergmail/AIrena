use std::time::Duration;

use futures_util::StreamExt;
use tokio_util::sync::CancellationToken;

use super::error::OllamaError;
use super::types::{ChatRequest, ChatResponse, ModelInfo};
use crate::models::settings::LlmParams;

pub struct OllamaClient {
    client: reqwest::Client,
    base_url: String,
    model: String,
}

impl OllamaClient {
    pub fn new(base_url: &str, model: &str) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(120))
                .build()
                .expect("Failed to build HTTP client"),
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
        }
    }

    pub fn model(&self) -> &str {
        &self.model
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
        for attempt in 0..=2u32 {
            match self
                .chat_streaming_inner(request, &on_token, &cancel)
                .await
            {
                Ok(content) => return Ok(content),
                Err(OllamaError::Cancelled) => return Err(OllamaError::Cancelled),
                Err(e) if e.is_connection_error() && attempt < 2 => {
                    tracing::warn!(
                        "Ollama connection error (attempt {}): {}",
                        attempt + 1,
                        e
                    );
                    tokio::time::sleep(Duration::from_secs(2u64.pow(attempt))).await;
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

    /// Streaming NDJSON — buffered parsing with Vec<u8> (no String reallocation)
    async fn chat_streaming_inner(
        &self,
        request: &ChatRequest,
        on_token: &(impl Fn(&str) + Send),
        cancel: &CancellationToken,
    ) -> Result<String, OllamaError> {
        let url = format!("{}/api/chat", self.base_url);
        let response = self.client.post(&url).json(request).send().await?;

        if !response.status().is_success() {
            return Err(OllamaError::ConnectionFailed(format!(
                "HTTP {}",
                response.status()
            )));
        }

        let mut stream = response.bytes_stream();
        let mut buf = Vec::<u8>::new();
        let mut accumulated = String::new();

        loop {
            tokio::select! {
                chunk = stream.next() => {
                    match chunk {
                        Some(Ok(bytes)) => {
                            buf.extend_from_slice(&bytes);
                            // NDJSON: process complete lines
                            while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
                                let line: Vec<u8> = buf.drain(..=pos).collect();
                                let line = String::from_utf8_lossy(&line);
                                let line = line.trim();
                                if !line.is_empty() {
                                    let resp: ChatResponse = serde_json::from_str(line)?;
                                    if resp.done {
                                        return Ok(accumulated);
                                    }
                                    on_token(&resp.message.content);
                                    accumulated.push_str(&resp.message.content);
                                }
                            }
                        }
                        Some(Err(e)) => return Err(OllamaError::RequestFailed(e)),
                        None => return Ok(accumulated),
                    }
                }
                _ = cancel.cancelled() => {
                    return Err(OllamaError::Cancelled);
                }
            }
        }
    }

    /// Preload a model into Ollama's memory by sending a minimal chat request.
    /// This avoids cold-start delays when the discussion starts.
    pub async fn preload_model(&self) -> Result<(), OllamaError> {
        let url = format!("{}/api/chat", self.base_url);
        let body = serde_json::json!({
            "model": self.model,
            "messages": [{"role": "user", "content": "hello"}],
            "stream": false,
            "keep_alive": "5m"
        });
        let resp = self.client
            .post(&url)
            .json(&body)
            .timeout(Duration::from_secs(300)) // Model loading can take a while
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
            .get(&format!("{}/api/tags", self.base_url))
            .timeout(Duration::from_secs(5))
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
        }
    }
}

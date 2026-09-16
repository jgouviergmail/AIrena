use std::time::Duration;

use futures_util::StreamExt;
use tokio_util::sync::CancellationToken;

use super::error::OllamaError;
use super::types::{ChatRequest, ChatResponse, ModelInfo, PsResponse, ShowResponse};
use crate::constants;

/// Strip `<think>...</think>` blocks from text.
/// Some thinking models leak these tags into the content field; this ensures clean output.
pub fn strip_think_tags(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut remaining = text;
    while let Some(start) = remaining.find("<think>") {
        result.push_str(&remaining[..start]);
        if let Some(end) = remaining[start..].find("</think>") {
            remaining = &remaining[start + end + "</think>".len()..];
        } else {
            // Unclosed <think> tag — strip everything from here
            remaining = "";
            break;
        }
    }
    result.push_str(remaining);
    result.trim().to_string()
}

/// Result of a streaming chat (content + optional thinking + token counts).
#[derive(Debug, Default)]
pub struct ChatStreamResult {
    pub content: String,
    pub thinking: Option<String>,
    /// From the final chunk's `prompt_eval_count`.
    pub prompt_eval_count: Option<u32>,
    /// From the final chunk's `eval_count` (thinking tokens included).
    pub eval_count: Option<u32>,
    /// The model hit `num_predict` (`done_reason == "length"`).
    pub truncated: bool,
}

/// Low-level HTTP client for the Ollama API. Chat requests go through
/// `llm::ollama::OllamaProvider`, which owns the wire mapping.
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

    /// Get the configured model name.
    pub fn model_name(&self) -> &str {
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

    /// Chat streaming with think mode — separate callbacks for content and thinking tokens.
    /// Includes retry with exponential backoff (up to 3 attempts).
    /// NOTE: callbacks must be Send to cross .await boundaries
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
        let mut result = ChatStreamResult::default();
        let mut accumulated_thinking = String::new();

        let finish = |mut result: ChatStreamResult, thinking: String| {
            result.thinking = if thinking.is_empty() { None } else { Some(thinking) };
            result
        };

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
                                if line.is_empty() {
                                    continue;
                                }
                                let resp: ChatResponse = serde_json::from_str(line)?;
                                Self::absorb_chunk(&resp, &mut result, &mut accumulated_thinking, on_content_token, on_thinking_token);
                                if resp.done {
                                    return Ok(finish(result, accumulated_thinking));
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
                                        Self::absorb_chunk(&resp, &mut result, &mut accumulated_thinking, on_content_token, on_thinking_token);
                                    }
                                }
                            }
                            return Ok(finish(result, accumulated_thinking));
                        }
                    }
                }
                _ = cancel.cancelled() => {
                    return Err(OllamaError::Cancelled);
                }
            }
        }
    }

    /// Fold one NDJSON chunk into the accumulated result (tokens, counts, truncation).
    fn absorb_chunk(
        resp: &ChatResponse,
        result: &mut ChatStreamResult,
        thinking: &mut String,
        on_content_token: &(impl Fn(&str) + Send),
        on_thinking_token: &(impl Fn(&str) + Send),
    ) {
        if let Some(t) = &resp.message.thinking {
            if !t.is_empty() {
                on_thinking_token(t);
                thinking.push_str(t);
            }
        }
        if !resp.message.content.is_empty() {
            on_content_token(&resp.message.content);
            result.content.push_str(&resp.message.content);
        }
        if resp.done {
            result.prompt_eval_count = resp.prompt_eval_count;
            result.eval_count = resp.eval_count;
            if resp.done_reason.as_deref() == Some("length") {
                result.truncated = true;
                tracing::warn!(
                    chars = result.content.len(),
                    "Response truncated: model hit num_predict token limit"
                );
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

    /// Preload a model into Ollama's memory without generating any tokens.
    /// Uses POST `/api/generate` with no prompt — Ollama loads the model and
    /// returns immediately with `done_reason: "load"`.
    ///
    /// # Arguments
    /// - `num_ctx` — Context size to allocate. **Critical**: without this, Ollama
    ///   uses the model's native context length (often 128K), which can consume
    ///   all available VRAM just for the KV cache.
    pub async fn preload_model(&self, num_ctx: Option<u32>) -> Result<(), OllamaError> {
        let url = format!("{}/api/generate", self.base_url);
        let mut body = serde_json::json!({
            "model": self.model,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ollama::types::ChatResponseMessage;

    #[test]
    fn test_strip_think_tags_no_tags() {
        assert_eq!(strip_think_tags("Hello world"), "Hello world");
    }

    #[test]
    fn test_strip_think_tags_simple() {
        assert_eq!(
            strip_think_tags("<think>reasoning here</think>The answer is 42."),
            "The answer is 42."
        );
    }

    #[test]
    fn test_strip_think_tags_multiline() {
        let input = "<think>\nLet me think about this...\nOk I got it.\n</think>\nHere is my response.";
        assert_eq!(strip_think_tags(input), "Here is my response.");
    }

    #[test]
    fn test_strip_think_tags_multiple() {
        let input = "<think>first</think>Hello <think>second</think>world";
        assert_eq!(strip_think_tags(input), "Hello world");
    }

    #[test]
    fn test_strip_think_tags_unclosed() {
        // Unclosed think tag — strip from <think> to end
        let input = "Before <think>reasoning without close";
        assert_eq!(strip_think_tags(input), "Before");
    }

    #[test]
    fn test_strip_think_tags_empty_after_strip() {
        assert_eq!(strip_think_tags("<think>only thinking</think>"), "");
    }

    #[test]
    fn test_strip_think_tags_no_content() {
        assert_eq!(strip_think_tags(""), "");
    }

    #[test]
    fn absorb_chunk_accumulates_tokens_counts_and_truncation() {
        let mut result = ChatStreamResult::default();
        let mut thinking = String::new();
        let content_seen = std::sync::Mutex::new(Vec::new());
        let thinking_seen = std::sync::Mutex::new(Vec::new());
        let on_c = |t: &str| content_seen.lock().unwrap().push(t.to_string());
        let on_t = |t: &str| thinking_seen.lock().unwrap().push(t.to_string());

        let mid = ChatResponse {
            message: ChatResponseMessage { role: "assistant".into(), content: "Bon".into(), thinking: Some("hmm".into()) },
            done: false,
            done_reason: None,
            prompt_eval_count: None,
            eval_count: None,
        };
        OllamaClient::absorb_chunk(&mid, &mut result, &mut thinking, &on_c, &on_t);
        let last = ChatResponse {
            message: ChatResponseMessage { role: "assistant".into(), content: "jour".into(), thinking: None },
            done: true,
            done_reason: Some("length".into()),
            prompt_eval_count: Some(120),
            eval_count: Some(45),
        };
        OllamaClient::absorb_chunk(&last, &mut result, &mut thinking, &on_c, &on_t);

        assert_eq!(result.content, "Bonjour");
        assert_eq!(thinking, "hmm");
        assert_eq!(result.prompt_eval_count, Some(120));
        assert_eq!(result.eval_count, Some(45));
        assert!(result.truncated);
        assert_eq!(content_seen.lock().unwrap().len(), 2);
        assert_eq!(thinking_seen.lock().unwrap().len(), 1);
    }

    #[test]
    fn chat_response_parses_final_chunk_counts() {
        let json = r#"{"model":"m","message":{"role":"assistant","content":""},"done":true,"done_reason":"stop","prompt_eval_count":321,"eval_count":87}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.prompt_eval_count, Some(321));
        assert_eq!(resp.eval_count, Some(87));
        // Older Ollama without counts still parses
        let json = r#"{"message":{"role":"assistant","content":"x"},"done":false}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert!(resp.eval_count.is_none());
    }
}

use std::time::Duration;

use crate::constants;
use crate::ollama::error::OllamaError;

/// Ollama embedding client — wraps the `/api/embed` endpoint.
///
/// Designed for zero-config usage: works with any Ollama model (chat or embedding).
/// The `/api/embed` endpoint does not check model capabilities — any loaded model
/// can generate embeddings from its last hidden layer.
///
/// `reqwest::Client` is `Arc`-based, so `Clone` is cheap.
#[derive(Clone)]
pub struct EmbeddingClient {
    client: reqwest::Client,
    base_url: String,
    model: String,
}

impl EmbeddingClient {
    pub fn new(base_url: &str, model: &str) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(constants::RAG_EMBED_TIMEOUT_SECS))
                .build()
                .expect("Failed to build embedding HTTP client"),
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
        }
    }

    /// Validate that the model exists via `/api/show`.
    pub async fn validate_model(&self) -> Result<(), OllamaError> {
        let url = format!("{}/api/show", self.base_url);
        let body = serde_json::json!({ "name": &self.model });
        let resp = self.client.post(&url).json(&body).send().await?;
        if resp.status().is_success() {
            Ok(())
        } else if resp.status().as_u16() == 404 {
            Err(OllamaError::ModelNotFound(self.model.clone()))
        } else {
            Err(OllamaError::ConnectionFailed(format!(
                "Model validation failed: HTTP {}",
                resp.status()
            )))
        }
    }

    /// Embed a batch of texts. Splits into sub-batches of `RAG_EMBED_BATCH_SIZE`.
    pub async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, OllamaError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let url = format!("{}/api/embed", self.base_url);
        let mut all_embeddings = Vec::with_capacity(texts.len());

        for batch in texts.chunks(constants::RAG_EMBED_BATCH_SIZE) {
            let body = serde_json::json!({
                "model": &self.model,
                "input": batch,
            });

            let resp = self.client.post(&url).json(&body).send().await?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body_text = resp.text().await.unwrap_or_default();
                return Err(if status.is_client_error() {
                    OllamaError::ClientError(format!(
                        "Embedding failed (HTTP {status}): {body_text}"
                    ))
                } else {
                    OllamaError::ConnectionFailed(format!(
                        "Embedding failed (HTTP {status}): {body_text}"
                    ))
                });
            }

            let json: serde_json::Value = resp.json().await.map_err(|e| {
                OllamaError::ConnectionFailed(format!("Invalid embedding response: {e}"))
            })?;

            let embeddings = json["embeddings"]
                .as_array()
                .ok_or_else(|| {
                    OllamaError::ConnectionFailed(
                        "Missing 'embeddings' field in response".to_string(),
                    )
                })?;

            for emb in embeddings {
                let vec: Vec<f32> = emb
                    .as_array()
                    .ok_or_else(|| {
                        OllamaError::ConnectionFailed(
                            "Invalid embedding format: expected array of numbers".to_string(),
                        )
                    })?
                    .iter()
                    .filter_map(|v| v.as_f64().map(|f| f as f32))
                    .collect();

                if vec.is_empty() {
                    return Err(OllamaError::ConnectionFailed(
                        "Empty embedding vector returned".to_string(),
                    ));
                }

                all_embeddings.push(vec);
            }
        }

        Ok(all_embeddings)
    }

    /// Embed a single text (convenience wrapper).
    pub async fn embed_one(&self, text: &str) -> Result<Vec<f32>, OllamaError> {
        let results = self.embed_batch(&[text.to_string()]).await?;
        results.into_iter().next().ok_or_else(|| {
            OllamaError::ConnectionFailed("No embedding returned for single text".to_string())
        })
    }
}

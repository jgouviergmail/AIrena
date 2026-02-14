use std::time::Duration;

use tokio_util::sync::CancellationToken;

use super::error::TavilyError;
use super::TavilySearchResponse;
use crate::constants;

const TAVILY_API_URL: &str = "https://api.tavily.com/search";

#[derive(Clone)]
pub struct TavilyClient {
    http_client: reqwest::Client,
    api_key: String,
}

impl TavilyClient {
    pub fn new(api_key: &str) -> Self {
        Self {
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(constants::TAVILY_HTTP_TIMEOUT_SECS))
                .build()
                .expect("reqwest client builder should not fail with basic timeout config"),
            api_key: api_key.to_string(),
        }
    }

    pub async fn search(
        &self,
        query: &str,
        cancel: CancellationToken,
    ) -> Result<TavilySearchResponse, TavilyError> {
        let body = serde_json::json!({
            "query": query,
            "search_depth": "basic",
            "max_results": constants::TAVILY_MAX_RESULTS,
            "include_answer": true,
        });

        let future = self
            .http_client
            .post(TAVILY_API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send();

        // Race the HTTP call against cancellation
        let response = tokio::select! {
            result = future => {
                result.map_err(|e| TavilyError::Network(e.to_string()))?
            }
            _ = cancel.cancelled() => {
                return Err(TavilyError::Cancelled);
            }
        };

        // Check HTTP status BEFORE parsing JSON body
        let status = response.status().as_u16();
        if status != 200 {
            let body_text = response
                .text()
                .await
                .unwrap_or_else(|_| String::from("<no body>"));
            return Err(match status {
                401 => TavilyError::InvalidKey,
                429 => TavilyError::RateLimit,
                432 => TavilyError::QuotaExceeded,
                _ => TavilyError::Http(status, body_text),
            });
        }

        let parsed: TavilySearchResponse = response
            .json()
            .await
            .map_err(|e| TavilyError::Network(format!("JSON parse error: {e}")))?;

        Ok(parsed)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OllamaError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Model not found: {0}")]
    ModelNotFound(String),
    #[error("Request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),
    #[error("JSON parse error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Connection lost after retries")]
    ConnectionLost,
    #[error("Request cancelled")]
    Cancelled,
    #[error("Empty response from model")]
    EmptyResponse,
}

impl OllamaError {
    pub fn is_connection_error(&self) -> bool {
        match self {
            Self::ConnectionFailed(_) | Self::ConnectionLost => true,
            Self::RequestFailed(e) => e.is_connect() || e.is_timeout(),
            _ => false,
        }
    }
}

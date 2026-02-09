#[derive(Debug, thiserror::Error)]
pub enum TavilyError {
    #[error("Invalid API key")]
    InvalidKey,
    #[error("Rate limit exceeded")]
    RateLimit,
    #[error("Monthly quota exceeded")]
    QuotaExceeded,
    #[error("HTTP error {0}: {1}")]
    Http(u16, String),
    #[error("Network error: {0}")]
    Network(String),
    #[error("Cancelled")]
    Cancelled,
}

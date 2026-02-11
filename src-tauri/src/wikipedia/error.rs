#[derive(Debug, thiserror::Error)]
pub enum WikiError {
    #[error("HTTP error {0}: {1}")]
    Http(u16, String),
    #[error("Network error: {0}")]
    Network(String),
    #[error("Cancelled")]
    Cancelled,
}

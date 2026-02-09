pub mod client;
pub mod error;

#[derive(Debug, serde::Deserialize)]
pub struct TavilySearchResponse {
    #[serde(default)]
    pub answer: Option<String>,
    #[serde(default)]
    pub results: Vec<TavilyResult>,
}

#[derive(Debug, serde::Deserialize)]
pub struct TavilyResult {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    #[allow(dead_code)] // Deserialized from API for completeness, not read
    pub score: f64,
}

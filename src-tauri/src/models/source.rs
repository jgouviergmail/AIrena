use serde::{Deserialize, Serialize};

use crate::constants;
use crate::engine::truncate_at_word_boundary;

/// Where a reference injected into a prompt came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceKind {
    Web,
    Wiki,
    Rag,
}

impl SourceKind {
    pub fn label(&self, lang: &str) -> &'static str {
        match (self, lang) {
            (Self::Web, "en") => "web",
            (Self::Web, "zh") => "网络",
            (Self::Web, _) => "web",
            (Self::Wiki, "en") => "Wikipedia",
            (Self::Wiki, "zh") => "维基百科",
            (Self::Wiki, _) => "Wikipédia",
            (Self::Rag, "en") => "document",
            (Self::Rag, "zh") => "文档",
            (Self::Rag, _) => "document",
        }
    }
}

/// One web result exposed to the frontend (title, link, short excerpt).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSourceInfo {
    pub title: String,
    pub url: String,
    pub domain: String,
    pub snippet: String,
}

/// One Wikipedia article exposed to the frontend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WikiSourceInfo {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

/// A reference the engine injected into a speaker's prompt, remembered for the synthesis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceRecord {
    pub kind: SourceKind,
    pub turn: u32,
    pub speaker_name: String,
    pub title: String,
    /// http(s) URL for web/wiki; `file#index` for document chunks
    pub url: String,
}

/// Host part of a URL ("https://www.example.com/a/b" → "www.example.com"); the URL itself when malformed.
pub fn domain_of(url: &str) -> String {
    url.split("//")
        .nth(1)
        .and_then(|rest| rest.split('/').next())
        .filter(|d| !d.is_empty())
        .unwrap_or(url)
        .to_string()
}

/// Short, word-bounded excerpt kept for display (never the full extract).
pub fn snippet_of(text: &str) -> String {
    truncate_at_word_boundary(text.trim(), constants::SOURCE_SNIPPET_CHARS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_and_snippet_helpers() {
        assert_eq!(domain_of("https://www.example.com/a/b?c=1"), "www.example.com");
        assert_eq!(domain_of("http://example.org"), "example.org");
        assert_eq!(domain_of("garbage"), "garbage");
        assert_eq!(domain_of("https:///path"), "https:///path");
        let long = "mot ".repeat(200);
        let s = snippet_of(&long);
        assert!(s.len() <= constants::SOURCE_SNIPPET_CHARS + 3 && s.ends_with('…'));
        assert_eq!(snippet_of("  court  "), "court");
        assert_eq!(serde_json::to_string(&SourceKind::Wiki).unwrap(), "\"wiki\"");
        assert_eq!(SourceKind::Rag.label("fr"), "document");
    }
}

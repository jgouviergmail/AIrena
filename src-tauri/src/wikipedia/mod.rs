pub mod client;
pub mod error;

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct WikiSearchResponse {
    pub query: Option<WikiQuery>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct WikiQuery {
    pub pages: Vec<WikiPage>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct WikiPage {
    pub title: String,
    pub pageid: u64,
    /// Relevance rank from the search generator (for sorting)
    pub index: i32,
    pub extract: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_real_response() {
        let json = r#"{
            "batchcomplete": true,
            "continue": { "gsroffset": 3, "continue": "gsroffset||" },
            "query": {
                "pages": [
                    { "pageid": 12345, "ns": 0, "title": "Intelligence artificielle", "index": 1, "extract": "L'intelligence artificielle est..." },
                    { "pageid": 67890, "ns": 0, "title": "Apprentissage automatique", "index": 2, "extract": "L'apprentissage automatique..." }
                ]
            }
        }"#;
        let resp: WikiSearchResponse = serde_json::from_str(json).unwrap();
        let query = resp.query.unwrap();
        assert_eq!(query.pages.len(), 2);
        assert_eq!(query.pages[0].title, "Intelligence artificielle");
        assert_eq!(query.pages[0].index, 1);
        assert_eq!(query.pages[1].pageid, 67890);
    }

    #[test]
    fn test_deserialize_empty_response() {
        // Wikipedia returns this when no results found
        let json = r#"{"batchcomplete": true}"#;
        let resp: WikiSearchResponse = serde_json::from_str(json).unwrap();
        assert!(resp.query.is_none());
    }

    #[test]
    fn test_deserialize_maxlag_error() {
        // Wikipedia returns this when server is overloaded (maxlag)
        let json = r#"{"error":{"code":"maxlag","info":"Waiting for ...","host":"..."}}"#;
        let resp: WikiSearchResponse = serde_json::from_str(json).unwrap();
        assert!(resp.query.is_none());
    }

    #[test]
    fn test_deserialize_missing_fields() {
        let json = r#"{"query": {"pages": [{"title": "Test"}]}}"#;
        let resp: WikiSearchResponse = serde_json::from_str(json).unwrap();
        let page = &resp.query.unwrap().pages[0];
        assert_eq!(page.title, "Test");
        assert_eq!(page.pageid, 0);
        assert_eq!(page.index, 0);
        assert!(page.extract.is_empty());
    }
}

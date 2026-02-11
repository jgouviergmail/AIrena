use std::time::Duration;

use tokio_util::sync::CancellationToken;

use super::error::WikiError;
use super::WikiSearchResponse;

/// Max results per search query (3 to allow disambiguation filtering)
const WIKI_RESULTS_LIMIT: u8 = 3;
/// Max characters for the plain-text extract (intro only)
const WIKI_EXTRACT_CHARS: u16 = 500;
/// Wikipedia maxlag parameter (seconds) — request is retried server-side if lag exceeds this
const WIKI_MAX_LAG_SECS: u8 = 5;
/// HTTP timeout for Wikipedia API calls
const WIKI_TIMEOUT_SECS: u64 = 15;

#[derive(Clone)]
pub struct WikiClient {
    http_client: reqwest::Client,
}

impl WikiClient {
    pub fn new() -> Self {
        Self {
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(WIKI_TIMEOUT_SECS))
                .user_agent(format!(
                    "AIrena/{} (https://github.com/AIrena) reqwest",
                    env!("CARGO_PKG_VERSION")
                ))
                .build()
                .expect("reqwest client builder should not fail with basic config"),
        }
    }

    /// Map discussion language to Wikipedia subdomain
    fn wiki_lang(discussion_language: &str) -> &str {
        match discussion_language {
            "fr" => "fr",
            "en" => "en",
            "zh" => "zh",
            _ => "en",
        }
    }

    fn build_url(lang: &str, query: &str) -> String {
        format!(
            "https://{}.wikipedia.org/w/api.php?action=query&generator=search&gsrsearch={}&gsrlimit={}&prop=extracts&exintro=1&explaintext=1&exchars={}&format=json&formatversion=2&maxlag={}",
            lang,
            urlencoding::encode(query),
            WIKI_RESULTS_LIMIT,
            WIKI_EXTRACT_CHARS,
            WIKI_MAX_LAG_SECS,
        )
    }

    async fn fetch(
        &self,
        lang: &str,
        query: &str,
        cancel: &CancellationToken,
    ) -> Result<WikiSearchResponse, WikiError> {
        let url = Self::build_url(lang, query);

        let future = self.http_client.get(&url).send();

        let response = tokio::select! {
            result = future => {
                result.map_err(|e| WikiError::Network(e.to_string()))?
            }
            _ = cancel.cancelled() => {
                return Err(WikiError::Cancelled);
            }
        };

        let status = response.status().as_u16();
        if status != 200 {
            let body_text = response
                .text()
                .await
                .unwrap_or_else(|_| String::from("<no body>"));
            return Err(WikiError::Http(status, body_text));
        }

        let parsed: WikiSearchResponse = response
            .json()
            .await
            .map_err(|e| WikiError::Network(format!("JSON parse error: {e}")))?;

        Ok(parsed)
    }

    /// Returns true if the response has actual pages
    fn has_results(resp: &WikiSearchResponse) -> bool {
        resp.query
            .as_ref()
            .is_some_and(|q| !q.pages.is_empty())
    }

    /// Search Wikipedia. Returns (response, actual_lang_used) — `actual_lang_used`
    /// may differ from `discussion_language` when zh→en fallback triggers.
    ///
    /// Fetches up to 3 results and picks the most relevant one based on query
    /// keyword overlap with the article title and extract. This avoids
    /// disambiguation false positives (e.g. "GLUE" → "Colle" instead of the ML benchmark).
    pub async fn search(
        &self,
        query: &str,
        discussion_language: &str,
        cancel: CancellationToken,
    ) -> Result<(WikiSearchResponse, String), WikiError> {
        let lang = Self::wiki_lang(discussion_language);
        let resp = self.fetch(lang, query, &cancel).await?;

        // Fallback zh→en: Chinese Wikipedia is smaller, retry on English if no results
        if lang == "zh" && !Self::has_results(&resp) {
            tracing::info!(
                "zh Wikipedia returned no results for '{}', falling back to en",
                query
            );
            let fallback = self.fetch("en", query, &cancel).await?;
            let filtered = Self::pick_best_result(query, fallback);
            return Ok((filtered, "en".to_string()));
        }

        let filtered = Self::pick_best_result(query, resp);
        Ok((filtered, lang.to_string()))
    }

    /// Among multiple results, pick the one whose title + extract best matches the query.
    /// Returns a WikiSearchResponse with only the best page (or empty if none had content).
    fn pick_best_result(query: &str, resp: WikiSearchResponse) -> WikiSearchResponse {
        let pages = match resp.query {
            Some(ref q) if !q.pages.is_empty() => &q.pages,
            _ => return resp,
        };

        // Single result — nothing to filter
        if pages.len() == 1 {
            return resp;
        }

        // Extract query keywords (lowercased, min 2 chars)
        let query_lower = query.to_lowercase();
        let keywords: Vec<&str> = query_lower
            .split(|c: char| !c.is_alphanumeric() && c != '-' && c != '\'')
            .filter(|w| w.len() >= 2)
            .collect();

        // Score each page by keyword overlap with title + extract
        let mut best_idx = 0;
        let mut best_score: i32 = -1;

        for (i, page) in pages.iter().enumerate() {
            // Skip pages with empty extracts (likely disambiguation stubs)
            if page.extract.trim().is_empty() {
                continue;
            }

            let title_lower = page.title.to_lowercase();
            let extract_lower = page.extract.to_lowercase();
            let haystack = format!("{} {}", title_lower, extract_lower);

            let mut score: i32 = 0;

            // Exact query match in title → strong signal
            if title_lower.contains(&query_lower) {
                score += 10;
            }

            // Keyword overlap
            for kw in &keywords {
                if title_lower.contains(kw) {
                    score += 3;
                }
                if extract_lower.contains(kw) {
                    score += 1;
                }
            }

            // Prefer Wikipedia's own ranking (lower index = higher relevance)
            // Small bonus for top-ranked result
            if page.index == 1 {
                score += 2;
            }

            // Penalize very short extracts (likely stubs or disambig)
            if page.extract.len() < 50 {
                score -= 3;
            }

            // Check for disambiguation signals in extract
            if haystack.contains("peut désigner")
                || haystack.contains("peut faire référence")
                || haystack.contains("may refer to")
                || haystack.contains("disambiguation")
                || haystack.contains("homonymie")
            {
                score -= 10;
            }

            if score > best_score {
                best_score = score;
                best_idx = i;
            }
        }

        let best_page = pages[best_idx].clone();

        if pages.len() > 1 {
            tracing::debug!(
                query = %query,
                picked = %best_page.title,
                score = best_score,
                candidates = pages.len(),
                "Wikipedia disambiguation: picked best result"
            );
        }

        WikiSearchResponse {
            query: Some(super::WikiQuery {
                pages: vec![best_page],
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wiki_lang_mapping() {
        assert_eq!(WikiClient::wiki_lang("fr"), "fr");
        assert_eq!(WikiClient::wiki_lang("en"), "en");
        assert_eq!(WikiClient::wiki_lang("zh"), "zh");
        assert_eq!(WikiClient::wiki_lang("de"), "en"); // default
        assert_eq!(WikiClient::wiki_lang(""), "en");
    }

    #[test]
    fn test_build_url() {
        let url = WikiClient::build_url("fr", "intelligence artificielle");
        assert!(url.starts_with("https://fr.wikipedia.org/w/api.php?"));
        assert!(url.contains("intelligence%20artificielle"));
        assert!(url.contains("gsrlimit=3"));
        assert!(url.contains("formatversion=2"));
    }

    #[test]
    fn test_has_results() {
        let empty = WikiSearchResponse { query: None };
        assert!(!WikiClient::has_results(&empty));

        let empty_pages = WikiSearchResponse {
            query: Some(super::super::WikiQuery {
                pages: Vec::new(),
            }),
        };
        assert!(!WikiClient::has_results(&empty_pages));

        let with_pages = WikiSearchResponse {
            query: Some(super::super::WikiQuery {
                pages: vec![super::super::WikiPage {
                    title: "Test".to_string(),
                    pageid: 1,
                    index: 1,
                    extract: "content".to_string(),
                }],
            }),
        };
        assert!(WikiClient::has_results(&with_pages));
    }

    fn make_page(title: &str, extract: &str, index: i32) -> super::super::WikiPage {
        super::super::WikiPage {
            title: title.to_string(),
            pageid: index as u64,
            index,
            extract: extract.to_string(),
        }
    }

    fn make_response(pages: Vec<super::super::WikiPage>) -> WikiSearchResponse {
        WikiSearchResponse {
            query: Some(super::super::WikiQuery { pages }),
        }
    }

    #[test]
    fn test_pick_best_result_glue_disambiguation() {
        // Simulates: query "GLUE" returns "Colle" (French for glue) + an AI benchmark article
        let resp = make_response(vec![
            make_page("Colle", "La colle est un produit de consistance liquide ou gélatineuse servant à lier des pièces entre elles.", 1),
            make_page("GLUE (benchmark)", "Le General Language Understanding Evaluation (GLUE) est un benchmark pour évaluer les modèles de traitement du langage naturel.", 2),
        ]);
        let filtered = WikiClient::pick_best_result("GLUE", resp);
        let pages = filtered.query.unwrap().pages;
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].title, "GLUE (benchmark)");
    }

    #[test]
    fn test_pick_best_result_exact_title_match() {
        let resp = make_response(vec![
            make_page("Singularité", "En mathématiques, une singularité est un point remarquable d'un objet mathématique.", 1),
            make_page("Singularité technologique", "La singularité technologique est l'hypothèse selon laquelle l'intelligence artificielle dépassera l'intelligence humaine.", 2),
        ]);
        let filtered = WikiClient::pick_best_result("Singularité technologique", resp);
        let pages = filtered.query.unwrap().pages;
        assert_eq!(pages[0].title, "Singularité technologique");
    }

    #[test]
    fn test_pick_best_result_single_result_unchanged() {
        let resp = make_response(vec![
            make_page("Intelligence artificielle", "L'intelligence artificielle est un ensemble de théories et techniques.", 1),
        ]);
        let filtered = WikiClient::pick_best_result("Intelligence artificielle", resp);
        let pages = filtered.query.unwrap().pages;
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].title, "Intelligence artificielle");
    }

    #[test]
    fn test_pick_best_result_disambiguation_page_penalized() {
        let resp = make_response(vec![
            make_page("Transformer", "Transformer peut désigner : un film, un jouet, une architecture de réseau de neurones.", 1),
            make_page("Transformer (architecture)", "Le Transformer est une architecture de réseau de neurones artificiels introduite par Google en 2017.", 2),
        ]);
        let filtered = WikiClient::pick_best_result("Transformer architecture réseau neurones", resp);
        let pages = filtered.query.unwrap().pages;
        assert_eq!(pages[0].title, "Transformer (architecture)");
    }

    #[test]
    fn test_pick_best_result_empty_extract_skipped() {
        let resp = make_response(vec![
            make_page("SuperGLUE", "", 1),
            make_page("Cyanoacrylate", "La cyanoacrylate est un adhésif puissant à base de monomère de cyanoacrylate.", 2),
            make_page("SuperGLUE (benchmark)", "SuperGLUE est un benchmark d'évaluation des modèles de traitement du langage naturel.", 3),
        ]);
        let filtered = WikiClient::pick_best_result("SuperGLUE", resp);
        let pages = filtered.query.unwrap().pages;
        assert_eq!(pages[0].title, "SuperGLUE (benchmark)");
    }
}

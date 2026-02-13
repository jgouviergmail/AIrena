use std::collections::HashMap;

use crate::constants;

/// In-memory BM25 index for keyword-based document retrieval.
///
/// Standard BM25 implementation with:
/// - k1 = 1.2 (term frequency saturation)
/// - b = 0.75 (document length normalization)
pub struct Bm25Index {
    /// Inverted index: token → Vec<(doc_id, term_frequency)>
    postings: HashMap<String, Vec<(usize, u32)>>,
    /// Document lengths (in tokens): doc_id → length
    doc_lengths: HashMap<usize, u32>,
    /// Total number of documents indexed
    doc_count: u32,
    /// Average document length
    avg_doc_len: f32,
}

impl Bm25Index {
    pub fn new() -> Self {
        Self {
            postings: HashMap::new(),
            doc_lengths: HashMap::new(),
            doc_count: 0,
            avg_doc_len: 0.0,
        }
    }

    /// Index a document (chunk) by its tokenized form.
    pub fn add_document(&mut self, doc_id: usize, tokens: &[String]) {
        let doc_len = tokens.len() as u32;
        self.doc_lengths.insert(doc_id, doc_len);
        self.doc_count += 1;

        // Update average document length
        let total: u32 = self.doc_lengths.values().sum();
        self.avg_doc_len = total as f32 / self.doc_count as f32;

        // Count term frequencies for this document
        let mut tf: HashMap<&str, u32> = HashMap::new();
        for token in tokens {
            *tf.entry(token.as_str()).or_insert(0) += 1;
        }

        // Update inverted index
        for (term, freq) in tf {
            self.postings
                .entry(term.to_string())
                .or_default()
                .push((doc_id, freq));
        }
    }

    /// Search the index and return the top_k results as (doc_id, BM25_score).
    pub fn search(&self, query_tokens: &[String], top_k: usize) -> Vec<(usize, f32)> {
        if self.doc_count == 0 || query_tokens.is_empty() {
            return Vec::new();
        }

        let k1 = constants::RAG_BM25_K1;
        let b = constants::RAG_BM25_B;

        let mut scores: HashMap<usize, f32> = HashMap::new();

        for token in query_tokens {
            let postings = match self.postings.get(token) {
                Some(p) => p,
                None => continue,
            };

            // IDF: log((N - n + 0.5) / (n + 0.5) + 1)
            let n = postings.len() as f32;
            let idf = ((self.doc_count as f32 - n + 0.5) / (n + 0.5) + 1.0).ln();

            for &(doc_id, tf) in postings {
                let doc_len = self.doc_lengths.get(&doc_id).copied().unwrap_or(1) as f32;
                let tf_f = tf as f32;
                // BM25 score: IDF * (tf * (k1 + 1)) / (tf + k1 * (1 - b + b * dl/avgdl))
                let numerator = tf_f * (k1 + 1.0);
                let denominator = tf_f + k1 * (1.0 - b + b * doc_len / self.avg_doc_len);
                *scores.entry(doc_id).or_insert(0.0) += idf * numerator / denominator;
            }
        }

        // Sort by score descending and return top_k
        let mut results: Vec<(usize, f32)> = scores.into_iter().collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(top_k);
        results
    }

    /// Rebuild the entire index from scratch.
    /// Used after removing documents to ensure clean state.
    pub fn clear(&mut self) {
        self.postings.clear();
        self.doc_lengths.clear();
        self.doc_count = 0;
        self.avg_doc_len = 0.0;
    }
}

/// Simple tokenizer: lowercase, split on whitespace, strip punctuation.
pub fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split_whitespace()
        .map(|w| w.trim_matches(|c: char| c.is_ascii_punctuation()).to_string())
        .filter(|w| !w.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_basic() {
        let tokens = tokenize("Hello, World! This is a TEST.");
        assert_eq!(tokens, vec!["hello", "world", "this", "is", "a", "test"]);
    }

    #[test]
    fn test_tokenize_empty() {
        let tokens = tokenize("");
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_tokenize_punctuation_only() {
        let tokens = tokenize("..., !!!");
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_empty_index_search() {
        let index = Bm25Index::new();
        let results = index.search(&tokenize("hello"), 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_single_document() {
        let mut index = Bm25Index::new();
        let tokens = tokenize("the quick brown fox jumps over the lazy dog");
        index.add_document(0, &tokens);

        let results = index.search(&tokenize("fox"), 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 0);
        assert!(results[0].1 > 0.0);
    }

    #[test]
    fn test_multiple_documents_ranking() {
        let mut index = Bm25Index::new();
        index.add_document(0, &tokenize("cat sat on the mat"));
        index.add_document(1, &tokenize("the dog played in the park"));
        index.add_document(2, &tokenize("cat and dog are friends cat cat"));

        // "cat" appears most in doc 2 (3 times), then doc 0 (1 time)
        let results = index.search(&tokenize("cat"), 10);
        assert!(results.len() >= 2);
        assert_eq!(results[0].0, 2, "Doc 2 should rank first (most 'cat' occurrences)");
        assert_eq!(results[1].0, 0, "Doc 0 should rank second");
    }

    #[test]
    fn test_top_k_limit() {
        let mut index = Bm25Index::new();
        for i in 0..20 {
            index.add_document(i, &tokenize(&format!("document number {i} with word")));
        }

        let results = index.search(&tokenize("word"), 5);
        assert_eq!(results.len(), 5);
    }

    #[test]
    fn test_idf_scoring() {
        let mut index = Bm25Index::new();
        // "rare" appears only in doc 0, "common" appears in all
        index.add_document(0, &tokenize("rare common word"));
        index.add_document(1, &tokenize("common word here"));
        index.add_document(2, &tokenize("common word there"));

        let results = index.search(&tokenize("rare"), 10);
        assert_eq!(results.len(), 1);
        // "rare" has high IDF because it appears in only 1 of 3 docs
        assert!(results[0].1 > 0.5);
    }

    #[test]
    fn test_no_match() {
        let mut index = Bm25Index::new();
        index.add_document(0, &tokenize("hello world"));

        let results = index.search(&tokenize("xyz"), 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_clear() {
        let mut index = Bm25Index::new();
        index.add_document(0, &tokenize("hello world"));
        index.clear();

        let results = index.search(&tokenize("hello"), 10);
        assert!(results.is_empty());
    }
}

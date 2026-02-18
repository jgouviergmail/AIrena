use std::collections::HashMap;

use serde::Serialize;
use tokio_util::sync::CancellationToken;

use crate::constants;
use crate::engine::truncate_str;
use crate::models::settings::LlmParams;
use crate::ollama::client::OllamaClient;
use crate::ollama::error::OllamaError;

use super::bm25::{self, Bm25Index};
use super::chunker::TextChunk;
use super::embedder::EmbeddingClient;
use super::parser::ParsedDocument;

/// Metadata about an imported RAG document
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagDocumentInfo {
    pub doc_id: String,
    pub file_name: String,
    pub format: String,
    pub chunk_count: usize,
    pub char_count: usize,
}

/// Information about a retrieved chunk (sent to frontend via events)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagChunkInfo {
    pub file_name: String,
    pub chunk_index: usize,
    pub preview: String,
    pub relevance_score: f32,
}

/// Internal chunk storage
struct StoredChunk {
    doc_id: String,
    file_name: String,
    chunk_index: usize,
    text: String,
    embedding: Vec<f32>,
    bm25_tokens: Vec<String>,
}

/// In-memory vector store + BM25 index with hybrid retrieval pipeline.
///
/// Lifecycle: created during import (Setup) → taken by DiscussionEngine → dropped at end.
pub struct RagStore {
    embedding_client: EmbeddingClient,
    chunks: Vec<StoredChunk>,
    bm25_index: Bm25Index,
    documents: HashMap<String, RagDocumentInfo>,
    /// Original full text of each imported document (doc_id → text).
    /// Used for full document injection when the budget allows it.
    full_texts: HashMap<String, String>,
    /// Global chunk counter (used as BM25 doc_id, monotonically increasing)
    next_chunk_id: usize,
}

impl RagStore {
    pub fn new(embedding_client: EmbeddingClient) -> Self {
        Self {
            embedding_client,
            chunks: Vec::new(),
            bm25_index: Bm25Index::new(),
            documents: HashMap::new(),
            full_texts: HashMap::new(),
            next_chunk_id: 0,
        }
    }

    /// Get a reference to the embedding client (for cloning outside the Mutex lock).
    pub fn embedding_client(&self) -> &EmbeddingClient {
        &self.embedding_client
    }

    /// Add a document with pre-computed embeddings.
    /// Parsing, chunking, and embedding are done OUTSIDE the Mutex lock.
    pub fn add_document_with_embeddings(
        &mut self,
        doc: &ParsedDocument,
        doc_id: &str,
        chunks: Vec<TextChunk>,
        embeddings: Vec<Vec<f32>>,
    ) -> RagDocumentInfo {
        let char_count = doc.text.len();
        let chunk_count = chunks.len();

        for (chunk, embedding) in chunks.into_iter().zip(embeddings.into_iter()) {
            let tokens = bm25::tokenize(&chunk.text);
            let global_id = self.next_chunk_id;
            self.next_chunk_id += 1;

            self.bm25_index.add_document(global_id, &tokens);

            self.chunks.push(StoredChunk {
                doc_id: doc_id.to_string(),
                file_name: doc.file_name.clone(),
                chunk_index: chunk.chunk_index,
                text: chunk.text,
                embedding,
                bm25_tokens: tokens,
            });
        }

        let info = RagDocumentInfo {
            doc_id: doc_id.to_string(),
            file_name: doc.file_name.clone(),
            format: doc.format.as_str().to_string(),
            chunk_count,
            char_count,
        };
        self.documents.insert(doc_id.to_string(), info.clone());
        self.full_texts.insert(doc_id.to_string(), doc.text.clone());
        info
    }

    /// Remove a document and rebuild the BM25 index.
    pub fn remove_document(&mut self, doc_id: &str) -> bool {
        if self.documents.remove(doc_id).is_none() {
            return false;
        }
        self.full_texts.remove(doc_id);

        self.chunks.retain(|c| c.doc_id != doc_id);

        // Rebuild BM25 index from remaining chunks
        self.bm25_index.clear();
        for (i, chunk) in self.chunks.iter().enumerate() {
            self.bm25_index.add_document(i, &chunk.bm25_tokens);
        }
        // Reset next_chunk_id to match current index
        self.next_chunk_id = self.chunks.len();

        true
    }

    /// Get status of all imported documents.
    pub fn get_status(&self) -> Vec<RagDocumentInfo> {
        self.documents.values().cloned().collect()
    }

    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    /// Total character count across all imported documents.
    pub fn total_char_count(&self) -> usize {
        self.documents.values().map(|d| d.char_count).sum()
    }

    /// Concatenate the full text of all imported documents, separated by headers.
    /// Returns `None` if no documents are stored.
    pub fn get_full_text(&self) -> Option<String> {
        if self.full_texts.is_empty() {
            return None;
        }
        // Sort by doc_id for deterministic ordering
        let mut entries: Vec<_> = self.full_texts.iter().collect();
        entries.sort_by_key(|(id, _)| *id);
        let mut result = String::new();
        for (doc_id, text) in &entries {
            if let Some(info) = self.documents.get(*doc_id) {
                if !result.is_empty() {
                    result.push_str("\n\n---\n\n");
                }
                result.push_str(&format!("[{}]\n{}", info.file_name, text));
            }
        }
        Some(result)
    }

    /// Dimension of the stored embeddings (None if no chunks).
    pub fn embedding_dim(&self) -> Option<usize> {
        self.chunks.first().map(|c| c.embedding.len())
    }

    /// Full hybrid retrieval pipeline:
    /// 1. Vector search (cosine similarity) → top-30
    /// 2. BM25 keyword search → top-30
    /// 3. RRF fusion → top-10
    /// 4. LLM selects 3-5 most relevant chunks
    /// 5. Anti "Lost in the Middle" reordering
    /// 6. Format context string
    pub async fn query(
        &self,
        context: &str,
        language: &str,
        ollama_client: &OllamaClient,
        llm_params: &LlmParams,
        cancel: CancellationToken,
    ) -> Result<(String, Vec<RagChunkInfo>), OllamaError> {
        if self.is_empty() {
            return Ok((String::new(), Vec::new()));
        }

        // 1a. Vector search: embed context → cosine similarity vs all chunks
        let query_embedding = self.embedding_client.embed_one(context).await?;
        let vector_results = self.vector_search(&query_embedding, constants::RAG_RETRIEVAL_TOP_K);

        // 1b. BM25 keyword search
        let query_tokens = bm25::tokenize(context);
        let bm25_results = self.bm25_index.search(&query_tokens, constants::RAG_RETRIEVAL_TOP_K);

        // 1c. RRF fusion → top-10
        let fused = rrf_fuse(&vector_results, &bm25_results, constants::RAG_RRF_TOP_K);

        if fused.is_empty() {
            return Ok((String::new(), Vec::new()));
        }

        // 2. LLM selection (3-5 chunks) — with fallback to top-3 RRF on failure
        let selected_indices = self
            .llm_select(&fused, context, language, ollama_client, llm_params, &cancel)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(error = %e, "LLM chunk selection failed, using top-{} RRF", constants::RAG_FALLBACK_TOP_K);
                fused.iter().take(constants::RAG_FALLBACK_TOP_K).map(|&(idx, _)| idx).collect()
            });

        if selected_indices.is_empty() {
            return Ok((String::new(), Vec::new()));
        }

        // 3. Collect selected chunks with scores
        let mut selected: Vec<(usize, f32)> = selected_indices
            .iter()
            .filter_map(|&idx| {
                fused.iter().find(|&&(i, _)| i == idx).copied()
            })
            .collect();

        // 4. Anti "Lost in the Middle" reordering
        order_anti_lost_in_middle(&mut selected);

        // 5. Build result
        let chunk_infos: Vec<RagChunkInfo> = selected
            .iter()
            .filter_map(|&(idx, score)| {
                self.chunks.get(idx).map(|chunk| RagChunkInfo {
                    file_name: chunk.file_name.clone(),
                    chunk_index: chunk.chunk_index,
                    preview: truncate_str(&chunk.text, 100).to_string(),
                    relevance_score: score,
                })
            })
            .collect();

        let context_text = self.build_context_text(&selected, language);

        Ok((context_text, chunk_infos))
    }

    /// Cosine similarity brute-force search (sufficient for <10K chunks).
    fn vector_search(&self, query_embedding: &[f32], top_k: usize) -> Vec<(usize, f32)> {
        let mut scores: Vec<(usize, f32)> = self
            .chunks
            .iter()
            .enumerate()
            .map(|(i, chunk)| (i, cosine_similarity(query_embedding, &chunk.embedding)))
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(top_k);
        scores
    }

    /// LLM selects the most relevant chunks from the fused candidates.
    async fn llm_select(
        &self,
        candidates: &[(usize, f32)],
        context: &str,
        language: &str,
        ollama_client: &OllamaClient,
        llm_params: &LlmParams,
        cancel: &CancellationToken,
    ) -> Result<Vec<usize>, OllamaError> {
        // Build numbered previews for the LLM
        let mut previews = String::new();
        for (rank, &(idx, _score)) in candidates.iter().enumerate() {
            if let Some(chunk) = self.chunks.get(idx) {
                let preview = truncate_str(&chunk.text, 200);
                previews.push_str(&format!(
                    "{}. [{}#{}] {}\n",
                    rank + 1,
                    chunk.file_name,
                    chunk.chunk_index,
                    preview
                ));
            }
        }

        let system_prompt = build_selection_system_prompt(language);
        let user_prompt = format!(
            "Context: {}\n\nCandidate fragments:\n{}",
            truncate_str(context, 500),
            previews
        );

        let mut selection_params = llm_params.clone();
        selection_params.temperature = constants::TEMP_VOTING;

        let request =
            ollama_client.build_request(&system_prompt, &user_prompt, &selection_params, true);

        let response = ollama_client.chat(&request, cancel.clone()).await?;

        // Parse JSON response: {"selected": [1, 3, 5]}
        let parsed: serde_json::Value =
            serde_json::from_str(&response).map_err(|_| {
                OllamaError::ConnectionFailed("Invalid JSON from LLM selection".to_string())
            })?;

        let selected_numbers: Vec<usize> = parsed["selected"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_u64().map(|n| n as usize))
                    .filter(|&n| n >= 1 && n <= candidates.len())
                    .map(|n| n - 1) // Convert from 1-indexed to 0-indexed rank
                    .collect()
            })
            .unwrap_or_default();

        if selected_numbers.is_empty() {
            return Err(OllamaError::ConnectionFailed(
                "LLM selected no chunks".to_string(),
            ));
        }

        // Convert rank indices back to chunk indices, limited by RAG_LLM_SELECT_MAX
        let result: Vec<usize> = selected_numbers
            .into_iter()
            .take(constants::RAG_LLM_SELECT_MAX)
            .filter_map(|rank| candidates.get(rank).map(|&(idx, _)| idx))
            .collect();

        Ok(result)
    }

    /// Build the formatted context text for injection into prompts.
    fn build_context_text(&self, selected: &[(usize, f32)], language: &str) -> String {
        let header = match language {
            "en" => "[Knowledge base — imported documents]",
            "zh" => "[知识库 — 导入的文档]",
            _ => "[Base de connaissances — documents importés]",
        };

        let mut output = format!("{header}\n\n");
        let mut total_len = output.len();

        for (rank, &(idx, _)) in selected.iter().enumerate() {
            if let Some(chunk) = self.chunks.get(idx) {
                let entry = format!(
                    "{}. \"{}\" (#{}) :\n{}\n\n",
                    rank + 1,
                    chunk.file_name,
                    chunk.chunk_index,
                    chunk.text
                );

                if total_len + entry.len() > constants::RAG_MAX_CONTEXT_LEN {
                    // Truncate this entry to fit
                    let remaining = constants::RAG_MAX_CONTEXT_LEN.saturating_sub(total_len);
                    if remaining > 50 {
                        let boundary = entry.floor_char_boundary(remaining);
                        output.push_str(&entry[..boundary]);
                        output.push_str("...\n");
                    }
                    break;
                }

                output.push_str(&entry);
                total_len += entry.len();
            }
        }

        output.trim().to_string()
    }
}

/// Cosine similarity between two vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let mut dot = 0.0_f32;
    let mut norm_a = 0.0_f32;
    let mut norm_b = 0.0_f32;

    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }

    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom < f32::EPSILON {
        0.0
    } else {
        dot / denom
    }
}

/// Reciprocal Rank Fusion: combine two ranked lists.
/// `RRF_score(d) = Σ 1/(k + rank_i(d))` with k = RAG_RRF_K (60).
fn rrf_fuse(
    vec_results: &[(usize, f32)],
    bm25_results: &[(usize, f32)],
    top_k: usize,
) -> Vec<(usize, f32)> {
    let k = constants::RAG_RRF_K;
    let mut scores: HashMap<usize, f32> = HashMap::new();

    for (rank, &(idx, _)) in vec_results.iter().enumerate() {
        *scores.entry(idx).or_insert(0.0) += 1.0 / (k + rank as f32 + 1.0);
    }
    for (rank, &(idx, _)) in bm25_results.iter().enumerate() {
        *scores.entry(idx).or_insert(0.0) += 1.0 / (k + rank as f32 + 1.0);
    }

    let mut results: Vec<(usize, f32)> = scores.into_iter().collect();
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(top_k);
    results
}

/// Anti "Lost in the Middle" ordering:
/// Most relevant first, 2nd most relevant last, rest in the middle.
fn order_anti_lost_in_middle(items: &mut Vec<(usize, f32)>) {
    if items.len() <= 2 {
        return;
    }

    // Items are already sorted by relevance (highest first from RRF)
    // We want: #1, #3, #5, ..., #4, #2
    let mut reordered = Vec::with_capacity(items.len());
    let first = items[0];
    let second = items[1];
    let rest = &items[2..];

    reordered.push(first);
    reordered.extend_from_slice(rest);
    reordered.push(second);

    *items = reordered;
}

/// Build the LLM selection system prompt (trilingual).
fn build_selection_system_prompt(language: &str) -> String {
    match language {
        "en" => {
            "You are a document retrieval assistant. Given a discussion context and numbered document fragments, \
             select the 3-5 most relevant fragments that would enrich a speaker's response.\n\
             Reply ONLY with JSON: {\"selected\": [1, 3, 5]}\n\
             Rules:\n\
             - Select 3 to 5 fragments maximum\n\
             - Only select fragments directly relevant to the current discussion topic\n\
             - Prefer fragments with facts, data, or specific references\n\
             - Use the fragment numbers as shown (1-indexed)".to_string()
        }
        "zh" => {
            "你是一个文档检索助手。给定讨论上下文和编号的文档片段，\
             选择3-5个最相关的片段来丰富发言者的回答。\n\
             仅回复JSON：{\"selected\": [1, 3, 5]}\n\
             规则：\n\
             - 最多选择3到5个片段\n\
             - 仅选择与当前讨论主题直接相关的片段\n\
             - 优先选择包含事实、数据或具体引用的片段\n\
             - 使用显示的片段编号（从1开始）".to_string()
        }
        _ => {
            "Tu es un assistant de recherche documentaire. À partir du contexte de discussion et des fragments numérotés, \
             sélectionne les 3-5 fragments les plus pertinents pour enrichir la réponse du locuteur.\n\
             Réponds UNIQUEMENT en JSON : {\"selected\": [1, 3, 5]}\n\
             Règles :\n\
             - Sélectionne 3 à 5 fragments maximum\n\
             - Ne sélectionne que les fragments directement pertinents pour le sujet de discussion actuel\n\
             - Privilégie les fragments contenant des faits, données ou références spécifiques\n\
             - Utilise les numéros de fragments tels qu'affichés (indexés à partir de 1)".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 1e-6, "Identical vectors should have similarity 1.0");
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-6, "Orthogonal vectors should have similarity 0.0");
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![-1.0, -2.0, -3.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim + 1.0).abs() < 1e-6, "Opposite vectors should have similarity -1.0");
    }

    #[test]
    fn test_cosine_similarity_different_lengths() {
        let a = vec![1.0, 2.0];
        let b = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&a, &b);
        assert_eq!(sim, 0.0, "Different-length vectors should return 0.0");
    }

    #[test]
    fn test_cosine_similarity_empty() {
        let sim = cosine_similarity(&[], &[]);
        assert_eq!(sim, 0.0);
    }

    #[test]
    fn test_cosine_similarity_zero_vector() {
        let a = vec![0.0, 0.0, 0.0];
        let b = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&a, &b);
        assert_eq!(sim, 0.0, "Zero vector should return 0.0");
    }

    #[test]
    fn test_rrf_fuse_basic() {
        let vec_results = vec![(0, 0.9), (1, 0.8), (2, 0.7)];
        let bm25_results = vec![(1, 5.0), (2, 4.0), (3, 3.0)];
        let fused = rrf_fuse(&vec_results, &bm25_results, 10);

        // Doc 1 appears in both lists → highest RRF score
        assert!(!fused.is_empty());
        // Doc 1 should be near the top (appears in rank 2 of vec and rank 1 of bm25)
        let doc1_pos = fused.iter().position(|&(idx, _)| idx == 1);
        assert!(doc1_pos.is_some());
    }

    #[test]
    fn test_rrf_fuse_disjoint() {
        let vec_results = vec![(0, 0.9), (1, 0.8)];
        let bm25_results = vec![(2, 5.0), (3, 4.0)];
        let fused = rrf_fuse(&vec_results, &bm25_results, 10);

        // All 4 docs should appear
        assert_eq!(fused.len(), 4);
    }

    #[test]
    fn test_rrf_fuse_top_k() {
        let vec_results: Vec<(usize, f32)> = (0..20).map(|i| (i, 1.0 - i as f32 * 0.05)).collect();
        let bm25_results: Vec<(usize, f32)> = (10..30).map(|i| (i, 1.0 - (i - 10) as f32 * 0.05)).collect();
        let fused = rrf_fuse(&vec_results, &bm25_results, 5);
        assert_eq!(fused.len(), 5);
    }

    #[test]
    fn test_anti_lost_in_middle_basic() {
        let mut items = vec![(0, 1.0), (1, 0.9), (2, 0.8), (3, 0.7), (4, 0.6)];
        order_anti_lost_in_middle(&mut items);
        // Expected: #1(0), #3(2), #4(3), #5(4), #2(1)
        assert_eq!(items[0].0, 0, "Most relevant should be first");
        assert_eq!(items[items.len() - 1].0, 1, "2nd most relevant should be last");
    }

    #[test]
    fn test_anti_lost_in_middle_two_items() {
        let mut items = vec![(0, 1.0), (1, 0.9)];
        order_anti_lost_in_middle(&mut items);
        // No reordering for ≤2 items
        assert_eq!(items[0].0, 0);
        assert_eq!(items[1].0, 1);
    }

    #[test]
    fn test_anti_lost_in_middle_single() {
        let mut items = vec![(0, 1.0)];
        order_anti_lost_in_middle(&mut items);
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn test_build_context_text_french() {
        let store = RagStore {
            embedding_client: EmbeddingClient::new("http://localhost:11434", "test"),
            chunks: vec![StoredChunk {
                doc_id: "doc1".to_string(),
                file_name: "contrat.pdf".to_string(),
                chunk_index: 0,
                text: "Article 1: Le vendeur cède au preneur...".to_string(),
                embedding: vec![1.0],
                bm25_tokens: vec![],
            }],
            bm25_index: Bm25Index::new(),
            documents: HashMap::new(),
            full_texts: HashMap::new(),
            next_chunk_id: 1,
        };

        let selected = vec![(0, 0.95)];
        let context = store.build_context_text(&selected, "fr");
        assert!(context.contains("Base de connaissances"));
        assert!(context.contains("contrat.pdf"));
        assert!(context.contains("Article 1"));
    }

    #[test]
    fn test_build_context_text_english() {
        let store = RagStore {
            embedding_client: EmbeddingClient::new("http://localhost:11434", "test"),
            chunks: vec![StoredChunk {
                doc_id: "doc1".to_string(),
                file_name: "contract.pdf".to_string(),
                chunk_index: 0,
                text: "Section 1: The seller hereby transfers...".to_string(),
                embedding: vec![1.0],
                bm25_tokens: vec![],
            }],
            bm25_index: Bm25Index::new(),
            documents: HashMap::new(),
            full_texts: HashMap::new(),
            next_chunk_id: 1,
        };

        let selected = vec![(0, 0.95)];
        let context = store.build_context_text(&selected, "en");
        assert!(context.contains("Knowledge base"));
    }

    #[test]
    fn test_build_context_text_chinese() {
        let store = RagStore {
            embedding_client: EmbeddingClient::new("http://localhost:11434", "test"),
            chunks: vec![StoredChunk {
                doc_id: "doc1".to_string(),
                file_name: "合同.pdf".to_string(),
                chunk_index: 0,
                text: "第一条：卖方将...".to_string(),
                embedding: vec![1.0],
                bm25_tokens: vec![],
            }],
            bm25_index: Bm25Index::new(),
            documents: HashMap::new(),
            full_texts: HashMap::new(),
            next_chunk_id: 1,
        };

        let selected = vec![(0, 0.95)];
        let context = store.build_context_text(&selected, "zh");
        assert!(context.contains("知识库"));
    }

    #[test]
    fn test_selection_system_prompt_trilingual() {
        let fr = build_selection_system_prompt("fr");
        assert!(fr.contains("assistant de recherche documentaire"));

        let en = build_selection_system_prompt("en");
        assert!(en.contains("document retrieval assistant"));

        let zh = build_selection_system_prompt("zh");
        assert!(zh.contains("文档检索助手"));
    }

    #[test]
    fn test_total_char_count_empty() {
        let store = RagStore {
            embedding_client: EmbeddingClient::new("http://localhost:11434", "test"),
            chunks: Vec::new(),
            bm25_index: Bm25Index::new(),
            documents: HashMap::new(),
            full_texts: HashMap::new(),
            next_chunk_id: 0,
        };
        assert_eq!(store.total_char_count(), 0);
    }

    #[test]
    fn test_total_char_count_multiple_docs() {
        let mut docs = HashMap::new();
        docs.insert("d1".to_string(), RagDocumentInfo {
            doc_id: "d1".to_string(),
            file_name: "a.txt".to_string(),
            format: "txt".to_string(),
            chunk_count: 1,
            char_count: 100,
        });
        docs.insert("d2".to_string(), RagDocumentInfo {
            doc_id: "d2".to_string(),
            file_name: "b.txt".to_string(),
            format: "txt".to_string(),
            chunk_count: 2,
            char_count: 250,
        });
        let store = RagStore {
            embedding_client: EmbeddingClient::new("http://localhost:11434", "test"),
            chunks: Vec::new(),
            bm25_index: Bm25Index::new(),
            documents: docs,
            full_texts: HashMap::new(),
            next_chunk_id: 0,
        };
        assert_eq!(store.total_char_count(), 350);
    }

    #[test]
    fn test_get_full_text_empty() {
        let store = RagStore {
            embedding_client: EmbeddingClient::new("http://localhost:11434", "test"),
            chunks: Vec::new(),
            bm25_index: Bm25Index::new(),
            documents: HashMap::new(),
            full_texts: HashMap::new(),
            next_chunk_id: 0,
        };
        assert!(store.get_full_text().is_none());
    }

    #[test]
    fn test_get_full_text_single_doc() {
        let mut docs = HashMap::new();
        docs.insert("d1".to_string(), RagDocumentInfo {
            doc_id: "d1".to_string(),
            file_name: "notes.md".to_string(),
            format: "md".to_string(),
            chunk_count: 1,
            char_count: 11,
        });
        let mut full_texts = HashMap::new();
        full_texts.insert("d1".to_string(), "Hello World".to_string());
        let store = RagStore {
            embedding_client: EmbeddingClient::new("http://localhost:11434", "test"),
            chunks: Vec::new(),
            bm25_index: Bm25Index::new(),
            documents: docs,
            full_texts,
            next_chunk_id: 0,
        };
        let text = store.get_full_text().unwrap();
        assert!(text.contains("[notes.md]"));
        assert!(text.contains("Hello World"));
        assert!(!text.contains("---"), "Single doc should have no separator");
    }

    #[test]
    fn test_get_full_text_multiple_docs_sorted() {
        let mut docs = HashMap::new();
        docs.insert("b".to_string(), RagDocumentInfo {
            doc_id: "b".to_string(),
            file_name: "second.txt".to_string(),
            format: "txt".to_string(),
            chunk_count: 1,
            char_count: 5,
        });
        docs.insert("a".to_string(), RagDocumentInfo {
            doc_id: "a".to_string(),
            file_name: "first.txt".to_string(),
            format: "txt".to_string(),
            chunk_count: 1,
            char_count: 5,
        });
        let mut full_texts = HashMap::new();
        full_texts.insert("a".to_string(), "Alpha".to_string());
        full_texts.insert("b".to_string(), "Bravo".to_string());
        let store = RagStore {
            embedding_client: EmbeddingClient::new("http://localhost:11434", "test"),
            chunks: Vec::new(),
            bm25_index: Bm25Index::new(),
            documents: docs,
            full_texts,
            next_chunk_id: 0,
        };
        let text = store.get_full_text().unwrap();
        let first_pos = text.find("[first.txt]").unwrap();
        let second_pos = text.find("[second.txt]").unwrap();
        assert!(first_pos < second_pos, "Documents should be sorted by doc_id");
        assert!(text.contains("---"), "Multiple docs should have separator");
    }
}

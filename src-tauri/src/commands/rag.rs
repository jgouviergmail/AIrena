use std::path::Path;

use tauri::State;

use crate::constants;
use crate::error::CommandError;
use crate::rag::chunker;
use crate::rag::parser;
use crate::rag::{EmbeddingClient, RagDocumentInfo, RagStore};
use crate::state::AppState;

#[tauri::command]
pub async fn import_rag_document(
    file_path: String,
    skip_embeddings: Option<bool>,
    state: State<'_, AppState>,
) -> Result<RagDocumentInfo, CommandError> {
    let skip_emb = skip_embeddings.unwrap_or(false);

    // 1. Determine effective embedding model (only required when computing embeddings)
    let settings = state.get_settings().await?;
    let effective_model = if settings.embedding_model.is_empty() {
        settings.ollama_model.clone()
    } else {
        settings.embedding_model.clone()
    };
    if !skip_emb && effective_model.is_empty() {
        return Err(CommandError::Rag(
            "No Ollama model configured. Please configure a model in Settings.".to_string(),
        ));
    }

    // 2. Validate file exists and size
    let path = Path::new(&file_path);
    let metadata = std::fs::metadata(path)
        .map_err(|e| CommandError::Rag(format!("Cannot access file: {e}")))?;
    if metadata.len() as usize > constants::RAG_MAX_FILE_SIZE_BYTES {
        return Err(CommandError::Rag(format!(
            "File exceeds maximum size of {} MB",
            constants::RAG_MAX_FILE_SIZE_BYTES / (1024 * 1024)
        )));
    }

    // 3. Ensure RagStore exists + get embedding client (brief lock)
    let emb_client = {
        let mut guard = AppState::lock_or_recover(&state.rag_store);
        if guard.is_none() {
            let client = EmbeddingClient::new(&settings.ollama_url, &effective_model);
            *guard = Some(RagStore::new(client));
        }
        guard.as_ref().unwrap().embedding_client().clone()
    };

    // 4. Validate embedding model exists (skip when deferring embeddings)
    if !skip_emb {
        emb_client
            .validate_model()
            .await
            .map_err(|e| CommandError::Rag(format!("Embedding model error: {e}")))?;
    }

    // 5. Get existing embedding dimension for consistency check
    let existing_dim = {
        let guard = AppState::lock_or_recover(&state.rag_store);
        guard.as_ref().and_then(|s| s.embedding_dim())
    };

    // 6. Parse file (in spawn_blocking for PDF catch_unwind safety)
    let path_clone = path.to_path_buf();
    let parsed = tokio::task::spawn_blocking(move || parser::parse_file(&path_clone))
        .await
        .map_err(|e| CommandError::Rag(format!("Parse task failed: {e}")))?
        ?;

    tracing::info!(
        file = %parsed.file_name,
        format = %parsed.format.as_str(),
        chars = parsed.text.len(),
        "RAG document parsed"
    );

    // 7. Chunk text (CPU-bound but fast, no need for spawn_blocking)
    let doc_id = uuid::Uuid::new_v4().to_string();
    let chunks = chunker::chunk_text(
        &parsed.text,
        0, // doc_index within this import (always 0 for single-doc)
        constants::RAG_CHUNK_TARGET_CHARS,
        constants::RAG_CHUNK_OVERLAP_CHARS,
    );

    if chunks.is_empty() {
        return Err(CommandError::Rag(
            "Document produced no chunks after splitting".to_string(),
        ));
    }

    tracing::info!(
        file = %parsed.file_name,
        chunk_count = chunks.len(),
        skip_embeddings = skip_emb,
        "RAG document chunked"
    );

    if skip_emb {
        // Full injection mode: store text + BM25 only, defer embeddings
        let info = {
            let mut guard = AppState::lock_or_recover(&state.rag_store);
            let store = guard.as_mut().ok_or_else(|| {
                CommandError::Rag("RAG store disappeared unexpectedly".to_string())
            })?;
            store.add_document_text_only(&parsed, &doc_id, chunks)
        };

        tracing::info!(
            doc_id = %info.doc_id,
            file = %info.file_name,
            chunks = info.chunk_count,
            chars = info.char_count,
            "RAG document imported (text-only, embeddings deferred)"
        );

        return Ok(info);
    }

    // 8. Embed all chunks (async, may take time for first call loading the model)
    let texts: Vec<String> = chunks.iter().map(|c| c.text.clone()).collect();
    let embeddings = emb_client
        .embed_batch(&texts)
        .await
        .map_err(|e| CommandError::Rag(format!("Embedding failed: {e}")))?;

    // 9. Verify embedding dimension consistency
    if let Some(expected_dim) = existing_dim {
        if let Some(first_emb) = embeddings.first() {
            if first_emb.len() != expected_dim {
                return Err(CommandError::Rag(format!(
                    "Embedding dimension mismatch: expected {expected_dim}, got {}. \
                     This happens when mixing models. Clear the RAG store and re-import all documents.",
                    first_emb.len()
                )));
            }
        }
    }

    // 10. Add to store (brief lock)
    let info = {
        let mut guard = AppState::lock_or_recover(&state.rag_store);
        let store = guard.as_mut().ok_or_else(|| {
            CommandError::Rag("RAG store disappeared unexpectedly".to_string())
        })?;
        store.add_document_with_embeddings(&parsed, &doc_id, chunks, embeddings)
    };

    tracing::info!(
        doc_id = %info.doc_id,
        file = %info.file_name,
        chunks = info.chunk_count,
        chars = info.char_count,
        "RAG document imported successfully"
    );

    Ok(info)
}

#[tauri::command]
pub async fn remove_rag_document(
    doc_id: String,
    state: State<'_, AppState>,
) -> Result<bool, CommandError> {
    let mut guard = AppState::lock_or_recover(&state.rag_store);
    if let Some(store) = guard.as_mut() {
        let removed = store.remove_document(&doc_id);
        // If store is now empty, drop it entirely
        if store.is_empty() {
            *guard = None;
        }
        Ok(removed)
    } else {
        Ok(false)
    }
}

#[tauri::command]
pub async fn get_rag_status(
    state: State<'_, AppState>,
) -> Result<Vec<RagDocumentInfo>, CommandError> {
    let guard = AppState::lock_or_recover(&state.rag_store);
    Ok(guard.as_ref().map(|s| s.get_status()).unwrap_or_default())
}

#[tauri::command]
pub async fn clear_rag_store(state: State<'_, AppState>) -> Result<(), CommandError> {
    let mut guard = AppState::lock_or_recover(&state.rag_store);
    *guard = None;
    Ok(())
}

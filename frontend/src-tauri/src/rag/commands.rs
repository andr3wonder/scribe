use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime};
use tokio::sync::RwLock;
use tracing::{error, info};

use super::embeddings::EmbeddingEngine;
use super::indexer;
use super::model_manager::EmbeddingModelManager;
use super::search;
use super::vector_index::VectorIndex;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Managed state
// ---------------------------------------------------------------------------

/// Holds the optional embedding engine and the in-memory vector index.
pub struct RagState {
    pub engine: Option<EmbeddingEngine>,
    pub index: VectorIndex,
}

/// Wrapper for Tauri `.manage()` registration.
pub struct RagManagedState(pub Arc<RwLock<RagState>>);

// ---------------------------------------------------------------------------
// Response types returned to the frontend
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultResponse {
    pub chunk_id: String,
    pub meeting_id: String,
    pub content: String,
    pub score: f32,
    pub meeting_title: String,
    pub source_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStatusResponse {
    pub total_meetings: u64,
    pub indexed_meetings: u64,
    pub total_chunks: u64,
}

// ---------------------------------------------------------------------------
// Helper: ensure the embedding engine is loaded (lazy init)
// ---------------------------------------------------------------------------

/// Ensure the `EmbeddingEngine` is initialized inside `RagState`.
/// Returns an error string if the model files are missing.
async fn ensure_engine<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let rag = app.state::<RagManagedState>();
    let needs_init = {
        let state = rag.0.read().await;
        state.engine.is_none()
    };

    if needs_init {
        let app_data_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("Failed to get app data dir: {}", e))?;

        if !EmbeddingModelManager::is_model_ready(&app_data_dir) {
            return Err(
                "Embedding model not downloaded yet. Call rag_download_model first.".to_string(),
            );
        }

        let model_dir = EmbeddingModelManager::model_dir(&app_data_dir);

        // Build the engine on a blocking thread since ONNX loading is CPU-heavy
        let engine = tokio::task::spawn_blocking(move || EmbeddingEngine::new(&model_dir))
            .await
            .map_err(|e| format!("Engine init join error: {}", e))?
            .map_err(|e| format!("Failed to load embedding engine: {}", e))?;

        let mut state = rag.0.write().await;
        state.engine = Some(engine);
        info!("RAG embedding engine initialized");
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

/// Perform hybrid (vector + keyword) search across all indexed meetings.
#[tauri::command]
pub async fn rag_search<R: Runtime>(
    app: AppHandle<R>,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<SearchResultResponse>, String> {
    let limit = limit.unwrap_or(10);

    ensure_engine(&app).await?;

    let rag = app.state::<RagManagedState>();

    // Embed the query
    let query_embedding = {
        let state = rag.0.read().await;
        let engine = state
            .engine
            .as_ref()
            .ok_or("Embedding engine not available")?;
        engine
            .embed(&query)
            .map_err(|e| format!("Failed to embed query: {}", e))?
    };

    // Get db pool
    let app_state = app.state::<AppState>();
    let pool = app_state.db_manager.pool();

    // Perform hybrid search (needs read access to index)
    let state = rag.0.read().await;
    let results = search::hybrid_search(pool, &state.index, &query_embedding, &query, limit)
        .await
        .map_err(|e| format!("Search failed: {}", e))?;

    Ok(results
        .into_iter()
        .map(|r| SearchResultResponse {
            chunk_id: r.chunk_id,
            meeting_id: r.meeting_id,
            content: r.content,
            score: r.score,
            meeting_title: r.meeting_title,
            source_type: r.source_type,
        })
        .collect())
}

/// Index a single meeting's transcript and summary.
/// Returns the number of chunks created (0 if already indexed).
#[tauri::command]
pub async fn rag_index_meeting<R: Runtime>(
    app: AppHandle<R>,
    meeting_id: String,
) -> Result<usize, String> {
    ensure_engine(&app).await?;

    let rag = app.state::<RagManagedState>();
    let app_state = app.state::<AppState>();
    let pool = app_state.db_manager.pool();

    let mut state = rag.0.write().await;

    // Split borrow: get references to both fields simultaneously
    let RagState { engine, index } = &mut *state;
    let engine_ref = engine
        .as_ref()
        .ok_or("Embedding engine not available")?;

    let count = indexer::index_meeting(pool, engine_ref, index, &meeting_id)
        .await
        .map_err(|e| format!("Failed to index meeting {}: {}", meeting_id, e))?;

    Ok(count)
}

/// Index all meetings that haven't been indexed yet.
/// Returns total new chunks created.
#[tauri::command]
pub async fn rag_index_all<R: Runtime>(app: AppHandle<R>) -> Result<usize, String> {
    ensure_engine(&app).await?;

    let rag = app.state::<RagManagedState>();
    let app_state = app.state::<AppState>();
    let pool = app_state.db_manager.pool();

    let mut state = rag.0.write().await;

    // Split borrow: get references to both fields simultaneously
    let RagState { engine, index } = &mut *state;
    let engine_ref = engine
        .as_ref()
        .ok_or("Embedding engine not available")?;

    let count = indexer::index_all_unindexed(pool, engine_ref, index)
        .await
        .map_err(|e| format!("Failed to index all meetings: {}", e))?;

    Ok(count)
}

/// Get the current RAG index status: how many meetings are indexed, total chunks, etc.
#[tauri::command]
pub async fn rag_get_index_status<R: Runtime>(
    app: AppHandle<R>,
) -> Result<IndexStatusResponse, String> {
    let app_state = app.state::<AppState>();
    let pool = app_state.db_manager.pool();

    let total_meetings: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM meetings")
            .fetch_one(pool)
            .await
            .map_err(|e| format!("DB error: {}", e))?;

    let indexed_meetings: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM rag_index_status")
            .fetch_one(pool)
            .await
            .map_err(|e| format!("DB error: {}", e))?;

    let total_chunks: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM rag_chunks")
            .fetch_one(pool)
            .await
            .map_err(|e| format!("DB error: {}", e))?;

    Ok(IndexStatusResponse {
        total_meetings: total_meetings.0 as u64,
        indexed_meetings: indexed_meetings.0 as u64,
        total_chunks: total_chunks.0 as u64,
    })
}

/// Download the embedding model (snowflake-arctic-embed-s) from HuggingFace.
#[tauri::command]
pub async fn rag_download_model<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    if EmbeddingModelManager::is_model_ready(&app_data_dir) {
        info!("Embedding model already downloaded");
        return Ok(());
    }

    info!("Starting embedding model download...");

    EmbeddingModelManager::download_model(&app_data_dir, |progress| {
        info!("Embedding model download: {:.0}%", progress * 100.0);
    })
    .await
    .map_err(|e| {
        error!("Embedding model download failed: {}", e);
        format!("Download failed: {}", e)
    })?;

    info!("Embedding model download complete");

    // Eagerly initialize the engine after download
    if let Err(e) = ensure_engine(&app).await {
        error!("Failed to initialize engine after download: {}", e);
        // Non-fatal: engine will be lazy-initialized on first use
    }

    Ok(())
}

/// Check whether the embedding model files are present on disk.
#[tauri::command]
pub async fn rag_is_model_ready<R: Runtime>(app: AppHandle<R>) -> Result<bool, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    Ok(EmbeddingModelManager::is_model_ready(&app_data_dir))
}

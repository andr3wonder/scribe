use anyhow::{anyhow, Result};
use sqlx::SqlitePool;
use tracing::{info, warn};
use uuid::Uuid;

use super::chunker::{chunk_summary, chunk_transcript};
use super::embeddings::EmbeddingEngine;
use super::vector_index::VectorIndex;

/// The model version string stored in rag_index_status.
/// Bump this when switching to a different embedding model.
const MODEL_VERSION: &str = "snowflake-arctic-embed-s-v1";

/// Index a single meeting: chunk its transcript and summary, compute embeddings,
/// store everything in the database, and update the in-memory VectorIndex.
///
/// Returns the number of chunks created. Returns Ok(0) if the meeting is already indexed.
pub async fn index_meeting(
    pool: &SqlitePool,
    engine: &EmbeddingEngine,
    vector_index: &mut VectorIndex,
    meeting_id: &str,
) -> Result<usize> {
    // Check if already indexed
    let already: Option<(String,)> =
        sqlx::query_as("SELECT meeting_id FROM rag_index_status WHERE meeting_id = ?1")
            .bind(meeting_id)
            .fetch_optional(pool)
            .await?;

    if already.is_some() {
        info!("Meeting {} is already indexed, skipping", meeting_id);
        return Ok(0);
    }

    // Fetch transcript segments and concatenate
    let transcript_rows: Vec<(String,)> = sqlx::query_as(
        "SELECT transcript FROM transcripts WHERE meeting_id = ?1 ORDER BY timestamp ASC",
    )
    .bind(meeting_id)
    .fetch_all(pool)
    .await?;

    let transcript_text: String = transcript_rows
        .iter()
        .map(|(t,)| t.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    // Fetch summary (from summary_processes.result)
    let summary_row: Option<(Option<String>,)> =
        sqlx::query_as("SELECT result FROM summary_processes WHERE meeting_id = ?1")
            .bind(meeting_id)
            .fetch_optional(pool)
            .await?;

    let summary_text = summary_row
        .and_then(|(r,)| r)
        .unwrap_or_default();

    // Chunk both sources
    let mut chunks = chunk_transcript(&transcript_text);
    let summary_chunks = chunk_summary(&summary_text);
    chunks.extend(summary_chunks);

    if chunks.is_empty() {
        warn!(
            "Meeting {} has no indexable content (empty transcript and summary)",
            meeting_id
        );
        return Ok(0);
    }

    info!(
        "Indexing meeting {}: {} chunks ({} transcript, rest summary)",
        meeting_id,
        chunks.len(),
        chunks.iter().filter(|c| c.source_type == "transcript").count()
    );

    // Embed all chunks in one batch
    let texts: Vec<&str> = chunks.iter().map(|c| c.content.as_str()).collect();
    let embeddings = engine.embed_batch(&texts)?;

    if embeddings.len() != chunks.len() {
        return Err(anyhow!(
            "Embedding count mismatch: got {} embeddings for {} chunks",
            embeddings.len(),
            chunks.len()
        ));
    }

    // Insert into rag_chunks and rag_chunks_fts, and append to VectorIndex
    for (chunk, embedding) in chunks.iter().zip(embeddings.iter()) {
        let chunk_id = Uuid::new_v4().to_string();
        let embedding_blob: &[u8] = bytemuck::cast_slice(embedding.as_slice());

        sqlx::query(
            "INSERT INTO rag_chunks (id, meeting_id, source_type, content, speaker, chunk_index, token_count, embedding) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind(&chunk_id)
        .bind(meeting_id)
        .bind(&chunk.source_type)
        .bind(&chunk.content)
        .bind(&chunk.speaker)
        .bind(chunk.chunk_index as i64)
        .bind(chunk.token_count as i64)
        .bind(embedding_blob)
        .execute(pool)
        .await?;

        // Sync to FTS5 table
        sqlx::query(
            "INSERT INTO rag_chunks_fts (content, meeting_id, chunk_id) VALUES (?1, ?2, ?3)",
        )
        .bind(&chunk.content)
        .bind(meeting_id)
        .bind(&chunk_id)
        .execute(pool)
        .await?;

        // Append to in-memory index
        vector_index.append(&chunk_id, embedding);
    }

    // Record indexing status
    let chunk_count = chunks.len() as i64;
    sqlx::query(
        "INSERT INTO rag_index_status (meeting_id, chunk_count, model_version) VALUES (?1, ?2, ?3)",
    )
    .bind(meeting_id)
    .bind(chunk_count)
    .bind(MODEL_VERSION)
    .execute(pool)
    .await?;

    info!(
        "Successfully indexed meeting {} with {} chunks",
        meeting_id, chunk_count
    );

    Ok(chunks.len())
}

/// Index all meetings that are not yet in `rag_index_status`.
/// Returns the total number of new chunks created.
pub async fn index_all_unindexed(
    pool: &SqlitePool,
    engine: &EmbeddingEngine,
    vector_index: &mut VectorIndex,
) -> Result<usize> {
    let unindexed: Vec<(String,)> = sqlx::query_as(
        "SELECT m.id FROM meetings m \
         WHERE m.id NOT IN (SELECT meeting_id FROM rag_index_status)",
    )
    .fetch_all(pool)
    .await?;

    if unindexed.is_empty() {
        info!("All meetings are already indexed");
        return Ok(0);
    }

    info!("{} unindexed meetings found, starting batch indexing", unindexed.len());

    let mut total_chunks = 0usize;

    for (meeting_id,) in &unindexed {
        match index_meeting(pool, engine, vector_index, meeting_id).await {
            Ok(count) => {
                total_chunks += count;
                info!("Indexed meeting {}: {} chunks", meeting_id, count);
            }
            Err(e) => {
                warn!("Failed to index meeting {}: {}", meeting_id, e);
                // Continue indexing remaining meetings
            }
        }
    }

    info!(
        "Batch indexing complete: {} total new chunks across {} meetings",
        total_chunks,
        unindexed.len()
    );

    Ok(total_chunks)
}

/// Remove all RAG data for a meeting from the database and the in-memory index.
pub async fn remove_meeting_index(
    pool: &SqlitePool,
    vector_index: &mut VectorIndex,
    meeting_id: &str,
) -> Result<()> {
    // Get chunk IDs for this meeting so we can remove from in-memory index
    let chunk_rows: Vec<(String,)> =
        sqlx::query_as("SELECT id FROM rag_chunks WHERE meeting_id = ?1")
            .bind(meeting_id)
            .fetch_all(pool)
            .await?;

    let chunk_ids: Vec<String> = chunk_rows.into_iter().map(|(id,)| id).collect();

    // Delete from FTS5
    sqlx::query("DELETE FROM rag_chunks_fts WHERE meeting_id = ?1")
        .bind(meeting_id)
        .execute(pool)
        .await?;

    // Delete from rag_chunks
    sqlx::query("DELETE FROM rag_chunks WHERE meeting_id = ?1")
        .bind(meeting_id)
        .execute(pool)
        .await?;

    // Delete from rag_index_status
    sqlx::query("DELETE FROM rag_index_status WHERE meeting_id = ?1")
        .bind(meeting_id)
        .execute(pool)
        .await?;

    // Remove from in-memory index
    vector_index.remove_meeting(&chunk_ids);

    info!(
        "Removed RAG index for meeting {} ({} chunks)",
        meeting_id,
        chunk_ids.len()
    );

    Ok(())
}

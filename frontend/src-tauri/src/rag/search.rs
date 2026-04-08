use std::collections::HashMap;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tracing::info;

use super::vector_index::VectorIndex;

/// A single search result combining vector and keyword scores.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub chunk_id: String,
    pub meeting_id: String,
    pub content: String,
    pub score: f32,
    pub meeting_title: String,
    pub source_type: String,
}

/// Row returned from the FTS5 query.
#[derive(Debug)]
struct FtsRow {
    chunk_id: String,
    _meeting_id: String,
    _rank: f64,
}

/// Perform hybrid search combining vector similarity and FTS5 keyword search,
/// fused via Reciprocal Rank Fusion (RRF, k=60).
pub async fn hybrid_search(
    pool: &SqlitePool,
    vector_index: &VectorIndex,
    query_embedding: &[f32],
    query_text: &str,
    limit: usize,
) -> Result<Vec<SearchResult>> {
    // --- 1. Vector search ---
    let vector_k = limit * 3; // retrieve extra candidates for fusion
    let vector_hits = vector_index.search(query_embedding, vector_k);

    // --- 2. FTS5 keyword search ---
    // Sanitize the query for FTS5: remove special characters that could break the MATCH syntax
    let sanitized_query = sanitize_fts_query(query_text);
    let fts_rows: Vec<FtsRow> = if sanitized_query.is_empty() {
        vec![]
    } else {
        let raw_rows: Vec<(String, String, f64)> = sqlx::query_as(
            "SELECT chunk_id, meeting_id, rank \
             FROM rag_chunks_fts \
             WHERE rag_chunks_fts MATCH ?1 \
             ORDER BY rank \
             LIMIT ?2",
        )
        .bind(&sanitized_query)
        .bind(vector_k as i64)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        raw_rows
            .into_iter()
            .map(|(chunk_id, meeting_id, rank)| FtsRow {
                chunk_id,
                _meeting_id: meeting_id,
                _rank: rank,
            })
            .collect()
    };

    info!(
        "Hybrid search: {} vector hits, {} FTS hits for query '{}'",
        vector_hits.len(),
        fts_rows.len(),
        &query_text[..query_text.len().min(60)]
    );

    // --- 3. Reciprocal Rank Fusion (k = 60) ---
    let rrf_k: f32 = 60.0;
    let mut rrf_scores: HashMap<String, f32> = HashMap::new();

    // Vector hits: rank position is 1-based
    for (rank_pos, (chunk_id, _sim_score)) in vector_hits.iter().enumerate() {
        let rrf = 1.0 / (rrf_k + (rank_pos + 1) as f32);
        *rrf_scores.entry(chunk_id.clone()).or_default() += rrf;
    }

    // FTS hits: rank position is 1-based
    for (rank_pos, fts_row) in fts_rows.iter().enumerate() {
        let rrf = 1.0 / (rrf_k + (rank_pos + 1) as f32);
        *rrf_scores.entry(fts_row.chunk_id.clone()).or_default() += rrf;
    }

    // Sort by combined RRF score descending
    let mut scored: Vec<(String, f32)> = rrf_scores.into_iter().collect();
    scored.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Take top `limit`
    let top_ids: Vec<(String, f32)> = scored.into_iter().take(limit).collect();

    if top_ids.is_empty() {
        return Ok(vec![]);
    }

    // --- 4. Fetch full content + meeting title for the winning chunk IDs ---
    // Build a comma-separated list of ?-placeholders
    let placeholders: String = top_ids
        .iter()
        .enumerate()
        .map(|(i, _)| if i == 0 { "?".to_string() } else { ", ?".to_string() })
        .collect();

    let query_str = format!(
        "SELECT c.id, c.meeting_id, c.content, c.source_type, COALESCE(m.title, '') as title \
         FROM rag_chunks c \
         LEFT JOIN meetings m ON c.meeting_id = m.id \
         WHERE c.id IN ({})",
        placeholders
    );

    let mut query = sqlx::query_as::<_, (String, String, String, String, String)>(&query_str);
    for (chunk_id, _) in &top_ids {
        query = query.bind(chunk_id);
    }

    let detail_rows = query.fetch_all(pool).await?;

    // Build results preserving RRF score order
    let detail_map: HashMap<String, (String, String, String, String)> = detail_rows
        .into_iter()
        .map(|(id, meeting_id, content, source_type, title)| {
            (id, (meeting_id, content, source_type, title))
        })
        .collect();

    let mut results: Vec<SearchResult> = Vec::with_capacity(top_ids.len());
    for (chunk_id, rrf_score) in &top_ids {
        if let Some((meeting_id, content, source_type, title)) = detail_map.get(chunk_id) {
            results.push(SearchResult {
                chunk_id: chunk_id.clone(),
                meeting_id: meeting_id.clone(),
                content: content.clone(),
                score: *rrf_score,
                meeting_title: title.clone(),
                source_type: source_type.clone(),
            });
        }
    }

    // Results are already in RRF-score order (we iterate top_ids in sorted order)
    Ok(results)
}

/// FTS5-only search fallback (no embedding model needed).
/// Used when the embedding model is not downloaded yet.
pub async fn fts_only_search(
    pool: &SqlitePool,
    query_text: &str,
    limit: usize,
) -> Result<Vec<SearchResult>> {
    let sanitized = sanitize_fts_query(query_text);
    if sanitized.is_empty() {
        return Ok(vec![]);
    }

    let rows: Vec<(String, String, String, String, String)> = sqlx::query_as(
        "SELECT c.id, c.meeting_id, c.content, c.source_type, COALESCE(m.title, '') \
         FROM rag_chunks_fts f \
         JOIN rag_chunks c ON f.chunk_id = c.id \
         LEFT JOIN meetings m ON c.meeting_id = m.id \
         WHERE rag_chunks_fts MATCH ?1 \
         ORDER BY f.rank \
         LIMIT ?2",
    )
    .bind(&sanitized)
    .bind(limit as i64)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    Ok(rows
        .into_iter()
        .enumerate()
        .map(|(i, (id, meeting_id, content, source_type, title))| SearchResult {
            chunk_id: id,
            meeting_id,
            content,
            score: 1.0 / (60.0 + (i + 1) as f32), // RRF-style score for consistency
            meeting_title: title,
            source_type,
        })
        .collect())
}

/// Sanitize a user query for FTS5 MATCH syntax.
/// Keeps alphanumeric words and joins them with spaces (implicit AND in FTS5).
fn sanitize_fts_query(query: &str) -> String {
    query
        .split_whitespace()
        .filter_map(|word| {
            let cleaned: String = word.chars().filter(|c| c.is_alphanumeric()).collect();
            if cleaned.is_empty() {
                None
            } else {
                Some(cleaned)
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

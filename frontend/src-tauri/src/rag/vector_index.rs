use anyhow::Result;
use ndarray::{Array1, Array2, Axis};
use sqlx::SqlitePool;
use tracing::{info, warn};

/// In-memory vector index for fast cosine-similarity search over RAG chunk embeddings.
pub struct VectorIndex {
    /// Stored embeddings, shape [n, dim]. Each row is L2-normalized.
    embeddings: Array2<f32>,
    /// Parallel vec of chunk IDs matching each row in `embeddings`.
    chunk_ids: Vec<String>,
}

impl VectorIndex {
    /// Create an empty index.
    pub fn new() -> Self {
        Self {
            embeddings: Array2::zeros((0, 384)),
            chunk_ids: Vec::new(),
        }
    }

    /// Load all embeddings from the `rag_chunks` table. Rows with NULL embeddings are skipped.
    pub async fn load_from_db(pool: &SqlitePool) -> Result<Self> {
        let rows: Vec<(String, Vec<u8>)> = sqlx::query_as(
            "SELECT id, embedding FROM rag_chunks WHERE embedding IS NOT NULL",
        )
        .fetch_all(pool)
        .await?;

        if rows.is_empty() {
            info!("VectorIndex: no embeddings in database, starting empty");
            return Ok(Self::new());
        }

        let dim = 384usize;
        let n = rows.len();
        let mut data = Vec::with_capacity(n * dim);
        let mut chunk_ids = Vec::with_capacity(n);

        for (id, blob) in &rows {
            let floats: &[f32] = bytemuck::cast_slice(blob);
            if floats.len() != dim {
                warn!(
                    "VectorIndex: chunk {} has embedding dim {} (expected {}), skipping",
                    id,
                    floats.len(),
                    dim
                );
                continue;
            }
            data.extend_from_slice(floats);
            chunk_ids.push(id.clone());
        }

        let actual_n = chunk_ids.len();
        let embeddings = Array2::from_shape_vec((actual_n, dim), data)?;

        info!("VectorIndex: loaded {} embeddings from database", actual_n);

        Ok(Self {
            embeddings,
            chunk_ids,
        })
    }

    /// Search for the top-k most similar vectors to `query`.
    ///
    /// Since all vectors are L2-normalized, dot product equals cosine similarity.
    /// Returns `(chunk_id, score)` pairs sorted by descending score.
    pub fn search(&self, query: &[f32], k: usize) -> Vec<(String, f32)> {
        let n = self.chunk_ids.len();
        if n == 0 || query.is_empty() {
            return vec![];
        }

        let query_arr = Array1::from_vec(query.to_vec());

        // Compute dot products: embeddings @ query -> [n]
        let scores = self.embeddings.dot(&query_arr);

        // Collect (index, score) and partial-sort for top-k
        let mut indexed: Vec<(usize, f32)> = scores.iter().copied().enumerate().collect();
        // Sort descending by score
        indexed.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let top_k = indexed.into_iter().take(k);

        top_k
            .map(|(idx, score)| (self.chunk_ids[idx].clone(), score))
            .collect()
    }

    /// Append a single embedding to the index.
    pub fn append(&mut self, chunk_id: &str, embedding: &[f32]) {
        let dim = 384;
        if embedding.len() != dim {
            warn!(
                "VectorIndex::append: chunk {} has dim {} (expected {}), skipping",
                chunk_id,
                embedding.len(),
                dim
            );
            return;
        }

        let row = Array1::from_vec(embedding.to_vec())
            .into_shape_with_order((1, dim))
            .expect("reshape to (1, dim)");

        if self.embeddings.nrows() == 0 {
            self.embeddings = row;
        } else {
            self.embeddings = ndarray::concatenate(Axis(0), &[self.embeddings.view(), row.view()])
                .expect("concatenate embedding rows");
        }

        self.chunk_ids.push(chunk_id.to_string());
    }

    /// Remove all chunks for a given meeting from the in-memory index.
    /// This rebuilds the arrays excluding any chunk IDs that belong to the meeting.
    pub fn remove_meeting(&mut self, chunk_ids_to_remove: &[String]) {
        if chunk_ids_to_remove.is_empty() {
            return;
        }

        let remove_set: std::collections::HashSet<&str> =
            chunk_ids_to_remove.iter().map(|s| s.as_str()).collect();

        let dim = 384;
        let mut new_data: Vec<f32> = Vec::new();
        let mut new_ids: Vec<String> = Vec::new();

        for (i, id) in self.chunk_ids.iter().enumerate() {
            if !remove_set.contains(id.as_str()) {
                let row = self.embeddings.row(i);
                new_data.extend_from_slice(row.as_slice().unwrap_or(&[]));
                new_ids.push(id.clone());
            }
        }

        let n = new_ids.len();
        self.embeddings = if n > 0 {
            Array2::from_shape_vec((n, dim), new_data).unwrap_or_else(|_| Array2::zeros((0, dim)))
        } else {
            Array2::zeros((0, dim))
        };
        self.chunk_ids = new_ids;
    }

    /// Number of vectors currently in the index.
    pub fn len(&self) -> usize {
        self.chunk_ids.len()
    }

    /// Whether the index is empty.
    pub fn is_empty(&self) -> bool {
        self.chunk_ids.is_empty()
    }
}

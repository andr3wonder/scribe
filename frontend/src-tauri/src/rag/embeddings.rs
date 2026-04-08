use std::path::Path;
use std::sync::Mutex;

use anyhow::{anyhow, Result};
use ndarray::{Array2, Axis};
use ort::execution_providers::CPUExecutionProvider;
use ort::inputs;
use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use ort::value::TensorRef;
use tokenizers::Tokenizer;
use tracing::{info, warn};

/// Maximum sequence length for the snowflake-arctic-embed-s model.
const MAX_SEQ_LEN: usize = 512;

/// Embedding engine wrapping an ONNX session and a HuggingFace tokenizer.
///
/// The `Session` is wrapped in a `Mutex` because `ort::Session::run` requires `&mut self`.
/// This allows `embed` / `embed_batch` to take `&self` while still being safe for shared use.
pub struct EmbeddingEngine {
    session: Mutex<Session>,
    tokenizer: Tokenizer,
}

impl EmbeddingEngine {
    /// Create a new `EmbeddingEngine` from a directory containing `model.onnx` and `tokenizer.json`.
    pub fn new(model_dir: &Path) -> Result<Self> {
        let model_path = model_dir.join("model.onnx");
        let tokenizer_path = model_dir.join("tokenizer.json");

        if !model_path.exists() {
            return Err(anyhow!(
                "model.onnx not found at {}",
                model_path.display()
            ));
        }
        if !tokenizer_path.exists() {
            return Err(anyhow!(
                "tokenizer.json not found at {}",
                tokenizer_path.display()
            ));
        }

        info!("Loading embedding ONNX model from {}", model_path.display());

        let providers = vec![CPUExecutionProvider::default().build()];

        let session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_execution_providers(providers)?
            .commit_from_file(&model_path)?;

        info!("Embedding model loaded — inputs:");
        for input in &session.inputs {
            info!("  input: name={}, type={:?}", input.name, input.input_type);
        }

        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| anyhow!("Failed to load tokenizer: {}", e))?;

        info!("Embedding tokenizer loaded from {}", tokenizer_path.display());

        Ok(Self {
            session: Mutex::new(session),
            tokenizer,
        })
    }

    /// Embed a single text string. Returns a 384-dim L2-normalized vector.
    pub fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let batch = self.embed_batch(&[text])?;
        batch
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("Empty embedding result"))
    }

    /// Embed a batch of texts. Returns one 384-dim L2-normalized vector per input.
    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        let batch_size = texts.len();

        // Tokenize all texts
        let encodings = self
            .tokenizer
            .encode_batch(texts.to_vec(), true)
            .map_err(|e| anyhow!("Tokenization failed: {}", e))?;

        // Find the max length in this batch (clamped to MAX_SEQ_LEN)
        let max_len = encodings
            .iter()
            .map(|enc| enc.get_ids().len().min(MAX_SEQ_LEN))
            .max()
            .unwrap_or(0);

        if max_len == 0 {
            warn!("All texts tokenized to zero length");
            return Ok(vec![vec![0.0; 384]; batch_size]);
        }

        // Build padded tensors: input_ids, attention_mask, token_type_ids
        let mut input_ids_flat: Vec<i64> = Vec::with_capacity(batch_size * max_len);
        let mut attention_mask_flat: Vec<i64> = Vec::with_capacity(batch_size * max_len);
        let mut token_type_ids_flat: Vec<i64> = Vec::with_capacity(batch_size * max_len);

        for encoding in &encodings {
            let ids = encoding.get_ids();
            let mask = encoding.get_attention_mask();
            let type_ids = encoding.get_type_ids();

            let seq_len = ids.len().min(max_len);

            for i in 0..max_len {
                if i < seq_len {
                    input_ids_flat.push(ids[i] as i64);
                    attention_mask_flat.push(mask[i] as i64);
                    token_type_ids_flat.push(type_ids[i] as i64);
                } else {
                    // Padding
                    input_ids_flat.push(0);
                    attention_mask_flat.push(0);
                    token_type_ids_flat.push(0);
                }
            }
        }

        let shape = [batch_size, max_len];

        let input_ids =
            Array2::from_shape_vec(shape, input_ids_flat)?;
        let attention_mask =
            Array2::from_shape_vec(shape, attention_mask_flat)?;
        let token_type_ids =
            Array2::from_shape_vec(shape, token_type_ids_flat)?;

        // Run inference (acquire mutex for &mut Session)
        // Extract hidden state within the scope of the session guard
        let hidden = {
            let mut session_guard = self
                .session
                .lock()
                .map_err(|e| anyhow!("Session mutex poisoned: {}", e))?;
            let outputs = session_guard.run(inputs![
                "input_ids" => TensorRef::from_array_view(input_ids.view().into_dyn())?,
                "attention_mask" => TensorRef::from_array_view(attention_mask.view().into_dyn())?,
                "token_type_ids" => TensorRef::from_array_view(token_type_ids.view().into_dyn())?,
            ])?;

            // Extract last_hidden_state: [batch, seq_len, 384]
            let hidden_state = outputs
                .get("last_hidden_state")
                .ok_or_else(|| anyhow!("Model output 'last_hidden_state' not found"))?
                .try_extract_array::<f32>()?;

            hidden_state.to_owned().into_dimensionality::<ndarray::Ix3>()?
        }; // session_guard dropped here

        // Mean-pool with attention mask and L2-normalize
        let attention_mask_f32 =
            Array2::from_shape_vec(shape, attention_mask.iter().map(|&v| v as f32).collect())?;

        let mut results = Vec::with_capacity(batch_size);

        for i in 0..batch_size {
            let hidden_i = hidden.index_axis(Axis(0), i); // [seq_len, 384]
            let mask_i = attention_mask_f32.row(i); // [seq_len]

            // Expand mask to [seq_len, 1] for broadcasting
            let mask_expanded = mask_i
                .to_owned()
                .into_shape_with_order((max_len, 1))?;

            // Multiply hidden states by mask
            let masked = &hidden_i.to_owned() * &mask_expanded;

            // Sum along seq_len axis -> [384]
            let summed = masked.sum_axis(Axis(0));

            // Sum of mask for averaging
            let mask_sum = mask_i.sum().max(1e-9);

            // Mean pool
            let mean_pooled = &summed / mask_sum;

            // L2 normalize
            let norm = mean_pooled.dot(&mean_pooled).sqrt().max(1e-12);
            let normalized = &mean_pooled / norm;

            results.push(normalized.to_vec());
        }

        Ok(results)
    }
}

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use reqwest::Client;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::{info, warn};

const MODEL_ONNX_URL: &str =
    "https://huggingface.co/Snowflake/snowflake-arctic-embed-s/resolve/main/onnx/model.onnx";
const TOKENIZER_URL: &str =
    "https://huggingface.co/Snowflake/snowflake-arctic-embed-s/resolve/main/tokenizer.json";

const MODEL_DIR_NAME: &str = "models/embeddings/snowflake-arctic-embed-s";

/// Manages downloading and locating the embedding model files.
pub struct EmbeddingModelManager;

impl EmbeddingModelManager {
    /// Returns the directory where the embedding model files are stored.
    pub fn model_dir(app_data_dir: &Path) -> PathBuf {
        app_data_dir.join(MODEL_DIR_NAME)
    }

    /// Check whether both `model.onnx` and `tokenizer.json` exist.
    pub fn is_model_ready(app_data_dir: &Path) -> bool {
        let dir = Self::model_dir(app_data_dir);
        dir.join("model.onnx").exists() && dir.join("tokenizer.json").exists()
    }

    /// Download both model files from HuggingFace with a progress callback.
    ///
    /// `progress_callback` receives a value in [0.0, 1.0] indicating overall progress.
    pub async fn download_model(
        app_data_dir: &Path,
        progress_callback: impl Fn(f32),
    ) -> Result<()> {
        let dir = Self::model_dir(app_data_dir);
        fs::create_dir_all(&dir).await?;

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(600))
            .connect_timeout(std::time::Duration::from_secs(30))
            .build()?;

        // Download model.onnx (accounts for 95% of progress) and tokenizer.json (5%)
        let model_path = dir.join("model.onnx");
        let tokenizer_path = dir.join("tokenizer.json");

        // --- model.onnx ---
        if !model_path.exists() {
            info!("Downloading embedding model from {}", MODEL_ONNX_URL);
            download_file_with_progress(
                &client,
                MODEL_ONNX_URL,
                &model_path,
                |frac| progress_callback(frac * 0.95),
            )
            .await?;
            info!("model.onnx downloaded successfully");
        } else {
            info!("model.onnx already exists, skipping download");
            progress_callback(0.95);
        }

        // --- tokenizer.json ---
        if !tokenizer_path.exists() {
            info!("Downloading tokenizer from {}", TOKENIZER_URL);
            download_file_with_progress(
                &client,
                TOKENIZER_URL,
                &tokenizer_path,
                |frac| progress_callback(0.95 + frac * 0.05),
            )
            .await?;
            info!("tokenizer.json downloaded successfully");
        } else {
            info!("tokenizer.json already exists, skipping download");
            progress_callback(1.0);
        }

        progress_callback(1.0);

        Ok(())
    }
}

/// Stream-download a file from `url` to `dest`, calling `progress` with fraction [0.0, 1.0].
async fn download_file_with_progress(
    client: &Client,
    url: &str,
    dest: &Path,
    progress: impl Fn(f32),
) -> Result<()> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| anyhow!("Failed to start download from {}: {}", url, e))?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Download failed with status {} for {}",
            response.status(),
            url
        ));
    }

    let total_size = response.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;

    // Write to a temporary file first, then rename (atomic-ish)
    let tmp_path = dest.with_extension("tmp");
    let mut file = fs::File::create(&tmp_path).await?;

    let mut stream = response.bytes_stream();
    use futures_util::StreamExt;

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| anyhow!("Download stream error: {}", e))?;
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;

        if total_size > 0 {
            let frac = (downloaded as f32 / total_size as f32).min(1.0);
            progress(frac);
        }
    }

    file.flush().await?;
    drop(file);

    // Rename tmp -> final
    fs::rename(&tmp_path, dest).await?;

    if total_size > 0 {
        info!(
            "Downloaded {} ({:.1} MB)",
            dest.display(),
            total_size as f64 / (1024.0 * 1024.0)
        );
    } else {
        warn!(
            "Downloaded {} (size unknown)",
            dest.display()
        );
    }

    Ok(())
}

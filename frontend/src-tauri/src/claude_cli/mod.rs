//! Claude CLI provider — shells out to `claude -p` for LLM inference.
//! No API key needed (claude CLI handles its own OAuth).

use anyhow::{anyhow, Result};
use log::{info, warn};
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

/// Find the claude CLI binary on the system.
fn find_claude_binary() -> Result<String> {
    let home = std::env::var("HOME").unwrap_or_default();
    let candidates = [
        format!("{}/.local/bin/claude", home),
        "/usr/local/bin/claude".to_string(),
        format!("{}/.claude/local/claude", home),
    ];

    for path in &candidates {
        if std::path::Path::new(path).exists() {
            return Ok(path.clone());
        }
    }

    // Try `which claude`
    let output = std::process::Command::new("which")
        .arg("claude")
        .output()
        .ok();

    if let Some(output) = output {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Ok(path);
            }
        }
    }

    Err(anyhow!(
        "claude CLI not found. Install it from https://claude.ai/code"
    ))
}

/// Generate text using `claude -p` with optional system prompt.
/// Pipes the user prompt via stdin to avoid shell escaping issues with long transcripts.
pub async fn generate(system_prompt: &str, user_prompt: &str) -> Result<String> {
    let claude_path = find_claude_binary()?;
    info!("[ClaudeCLI] Using binary: {}", claude_path);

    let mut cmd = Command::new(&claude_path);
    cmd.arg("-p");

    if !system_prompt.is_empty() {
        cmd.arg("--system-prompt").arg(system_prompt);
    }

    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| {
        anyhow!(
            "Failed to spawn claude CLI at {}: {}. Is it installed?",
            claude_path,
            e
        )
    })?;

    // Write prompt to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(user_prompt.as_bytes()).await?;
        stdin.shutdown().await?;
    }

    // Wait for completion
    let output = child.wait_with_output().await?;

    if output.status.success() {
        let response = String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_string();
        info!(
            "[ClaudeCLI] Response: {} chars",
            response.len()
        );
        Ok(response)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!("[ClaudeCLI] Error: {}", stderr);
        Err(anyhow!("claude -p failed: {}", stderr.trim()))
    }
}

/// Check if claude CLI is available on this system.
pub fn is_available() -> bool {
    find_claude_binary().is_ok()
}

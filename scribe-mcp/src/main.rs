mod db;
mod tools;

use anyhow::Result;
use rmcp::{ServiceExt, transport::stdio};
use tracing_subscriber::{self, EnvFilter};

use crate::db::ScribeDb;
use crate::tools::ScribeMcpServer;

#[tokio::main]
async fn main() -> Result<()> {
    // Log to stderr so stdout stays clean for MCP JSON-RPC protocol
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("Starting scribe-mcp server");

    // Open the database (read-only)
    let db = match ScribeDb::open() {
        Ok(db) => {
            tracing::info!("Database opened successfully");
            db
        }
        Err(e) => {
            tracing::error!("Failed to open database: {}", e);
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    // Create the MCP server and serve over stdio
    let server = ScribeMcpServer::new(db);
    let service = server
        .serve(stdio())
        .await
        .inspect_err(|e| {
            tracing::error!("Server error: {:?}", e);
        })?;

    tracing::info!("scribe-mcp server running");
    service.waiting().await?;
    tracing::info!("scribe-mcp server shutting down");

    Ok(())
}

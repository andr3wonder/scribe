use crate::chat::service::{self, LlmConfig};
use crate::database::repositories::chat_message::ChatMessageRow;
use crate::rag::commands::RagManagedState;
use crate::state::AppState;
use log::info as log_info;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime};

/// Response returned to the frontend for a chat question.
#[derive(Debug, Serialize, Deserialize)]
pub struct ChatResponse {
    pub answer: String,
    pub meeting_id: Option<String>,
}

/// A single chat message, as returned to the frontend.
#[derive(Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub meeting_id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

impl From<ChatMessageRow> for ChatMessage {
    fn from(row: ChatMessageRow) -> Self {
        ChatMessage {
            id: row.id,
            meeting_id: row.meeting_id,
            role: row.role,
            content: row.content,
            created_at: row.created_at,
        }
    }
}

/// Ask a question about a specific meeting's transcript and get an LLM-powered answer.
/// If meeting_id is "__live__", uses the provided liveTranscript instead of fetching from DB.
#[tauri::command]
pub async fn ask_meeting_question<R: Runtime>(
    app: AppHandle<R>,
    meeting_id: String,
    question: String,
    model_provider: String,
    model_name: String,
    live_transcript: Option<String>,
) -> Result<ChatResponse, String> {
    log_info!(
        "ask_meeting_question: meeting_id={}, provider={}, model={}, question={}",
        meeting_id,
        model_provider,
        model_name,
        &question[..question.len().min(80)]
    );

    let state = app.state::<AppState>();
    let pool = state.db_manager.pool();
    let app_data_dir = app.path().app_data_dir().ok();

    let config =
        LlmConfig::from_db(pool, &model_provider, &model_name, app_data_dir).await?;

    let answer = if meeting_id == "__live__" {
        // Live recording mode — use provided transcript, don't touch DB
        let transcript = live_transcript.unwrap_or_default();
        if transcript.is_empty() {
            return Err("No transcript available yet.".to_string());
        }
        service::ask_with_transcript(&transcript, &question, &config).await?
    } else {
        service::ask_question(pool, &meeting_id, &question, &config).await?
    };

    Ok(ChatResponse {
        answer,
        meeting_id: Some(meeting_id),
    })
}

/// Get the full chat history for a meeting.
#[tauri::command]
pub async fn get_chat_history<R: Runtime>(
    app: AppHandle<R>,
    meeting_id: String,
) -> Result<Vec<ChatMessage>, String> {
    log_info!("get_chat_history: meeting_id={}", meeting_id);

    let state = app.state::<AppState>();
    let pool = state.db_manager.pool();

    let rows = service::get_chat_history(pool, &meeting_id).await?;
    let messages: Vec<ChatMessage> = rows.into_iter().map(ChatMessage::from).collect();

    log_info!(
        "get_chat_history: returning {} messages for meeting_id={}",
        messages.len(),
        meeting_id
    );

    Ok(messages)
}

/// Ask a question across all meetings (global Q&A).
/// Uses RAG hybrid search when the embedding model is available, otherwise falls
/// back to the original two-LLM-call approach.
#[tauri::command]
pub async fn ask_global_question<R: Runtime>(
    app: AppHandle<R>,
    question: String,
    model_provider: String,
    model_name: String,
) -> Result<ChatResponse, String> {
    log_info!(
        "ask_global_question: provider={}, model={}, question={}",
        model_provider,
        model_name,
        &question[..question.len().min(80)]
    );

    let state = app.state::<AppState>();
    let pool = state.db_manager.pool();
    let app_data_dir = app.path().app_data_dir().ok();

    let config =
        LlmConfig::from_db(pool, &model_provider, &model_name, app_data_dir).await?;

    // Try to get RAG state — it's optional (may not be initialized yet)
    let rag_state = app.try_state::<RagManagedState>();
    let rag_lock = rag_state.as_ref().map(|s| &s.0 as &tokio::sync::RwLock<_>);

    let answer = service::ask_global(pool, &question, &config, rag_lock).await?;

    Ok(ChatResponse {
        answer,
        meeting_id: None,
    })
}

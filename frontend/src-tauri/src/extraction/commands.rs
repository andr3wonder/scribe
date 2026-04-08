use crate::chat::service::LlmConfig;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use tauri::{AppHandle, Manager, Runtime};
use tracing::info;

use super::extractor;

// ─── Response types ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct ExtractionSummary {
    pub meeting_id: String,
    pub decision_count: usize,
    pub action_item_count: usize,
    pub question_count: usize,
    pub topic_count: usize,
    pub people_count: usize,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct DecisionResponse {
    pub id: String,
    pub meeting_id: String,
    pub description: String,
    pub rationale: Option<String>,
    pub alternatives_considered: Option<String>,
    pub decided_by: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct ActionItemResponse {
    pub id: String,
    pub meeting_id: String,
    pub description: String,
    pub owner: Option<String>,
    pub due_date: Option<String>,
    pub status: String,
    pub completed_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct OpenQuestionResponse {
    pub id: String,
    pub meeting_id: String,
    pub question: String,
    pub asked_by: Option<String>,
    pub context: Option<String>,
    pub resolved: i32,
    pub resolved_in_meeting: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExtractionStatusResponse {
    pub total_meetings: i64,
    pub extracted_meetings: i64,
    pub pending_meetings: i64,
    pub total_decisions: i64,
    pub total_action_items: i64,
    pub total_open_questions: i64,
    pub total_topics: i64,
    pub total_people: i64,
}

// ─── Tauri commands ─────────────────────────────────────────────────────────

/// Extract structured entities (decisions, action items, questions, topics, people) from a single meeting.
#[tauri::command]
pub async fn extract_meeting_entities<R: Runtime>(
    app: AppHandle<R>,
    meeting_id: String,
    model_provider: Option<String>,
    model_name: Option<String>,
) -> Result<ExtractionSummary, String> {
    info!("extract_meeting_entities: meeting_id={}", meeting_id);

    let state = app.state::<AppState>();
    let pool = state.db_manager.pool();
    let app_data_dir = app.path().app_data_dir().ok();

    // Use provided provider/model or read from DB settings
    let (provider_str, model_str) = match (model_provider, model_name) {
        (Some(p), Some(m)) => (p, m),
        _ => extractor::get_llm_config_from_db(pool).await?,
    };

    let config = LlmConfig::from_db(pool, &provider_str, &model_str, app_data_dir).await?;
    let result = extractor::extract_from_meeting(pool, &meeting_id, &config).await?;

    Ok(ExtractionSummary {
        meeting_id,
        decision_count: result.decisions.len(),
        action_item_count: result.action_items.len(),
        question_count: result.open_questions.len(),
        topic_count: result.topics.len(),
        people_count: result.people_mentioned.len(),
    })
}

/// Extract entities from all meetings that haven't been processed yet.
#[tauri::command]
pub async fn extract_all_meetings<R: Runtime>(
    app: AppHandle<R>,
) -> Result<usize, String> {
    info!("extract_all_meetings: starting batch extraction");

    let state = app.state::<AppState>();
    let pool = state.db_manager.pool();

    extractor::extract_all_unextracted(pool).await
}

/// Get all decisions for a specific meeting.
#[tauri::command]
pub async fn get_meeting_decisions<R: Runtime>(
    app: AppHandle<R>,
    meeting_id: String,
) -> Result<Vec<DecisionResponse>, String> {
    let state = app.state::<AppState>();
    let pool = state.db_manager.pool();

    sqlx::query_as::<_, DecisionResponse>(
        "SELECT id, meeting_id, description, rationale, alternatives_considered, decided_by, created_at FROM decisions WHERE meeting_id = $1 ORDER BY created_at",
    )
    .bind(&meeting_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to fetch decisions: {}", e))
}

/// Get action items, optionally filtered by meeting, owner, and/or status.
#[tauri::command]
pub async fn get_meeting_action_items<R: Runtime>(
    app: AppHandle<R>,
    meeting_id: Option<String>,
    owner: Option<String>,
    status: Option<String>,
) -> Result<Vec<ActionItemResponse>, String> {
    let state = app.state::<AppState>();
    let pool = state.db_manager.pool();

    // Build dynamic query based on filters
    let mut conditions: Vec<String> = Vec::new();
    let mut bind_index = 1;

    if meeting_id.is_some() {
        conditions.push(format!("meeting_id = ${}", bind_index));
        bind_index += 1;
    }
    if owner.is_some() {
        conditions.push(format!("owner = ${}", bind_index));
        bind_index += 1;
    }
    if status.is_some() {
        conditions.push(format!("status = ${}", bind_index));
        // bind_index not needed after this
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let query_str = format!(
        "SELECT id, meeting_id, description, owner, due_date, status, completed_at, created_at FROM action_items {} ORDER BY created_at",
        where_clause
    );

    let mut query = sqlx::query_as::<_, ActionItemResponse>(&query_str);

    if let Some(ref mid) = meeting_id {
        query = query.bind(mid);
    }
    if let Some(ref o) = owner {
        query = query.bind(o);
    }
    if let Some(ref s) = status {
        query = query.bind(s);
    }

    query
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Failed to fetch action items: {}", e))
}

/// Get open questions, optionally filtered by meeting.
#[tauri::command]
pub async fn get_open_questions<R: Runtime>(
    app: AppHandle<R>,
    meeting_id: Option<String>,
) -> Result<Vec<OpenQuestionResponse>, String> {
    let state = app.state::<AppState>();
    let pool = state.db_manager.pool();

    if let Some(mid) = meeting_id {
        sqlx::query_as::<_, OpenQuestionResponse>(
            "SELECT id, meeting_id, question, asked_by, context, resolved, resolved_in_meeting, created_at FROM open_questions WHERE meeting_id = $1 ORDER BY created_at",
        )
        .bind(&mid)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Failed to fetch open questions: {}", e))
    } else {
        sqlx::query_as::<_, OpenQuestionResponse>(
            "SELECT id, meeting_id, question, asked_by, context, resolved, resolved_in_meeting, created_at FROM open_questions WHERE resolved = 0 ORDER BY created_at",
        )
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Failed to fetch open questions: {}", e))
    }
}

/// Update the status of an action item (open -> completed, etc).
#[tauri::command]
pub async fn update_action_item_status<R: Runtime>(
    app: AppHandle<R>,
    item_id: String,
    status: String,
) -> Result<(), String> {
    let state = app.state::<AppState>();
    let pool = state.db_manager.pool();

    // Validate status
    if !["open", "completed", "overdue"].contains(&status.as_str()) {
        return Err(format!("Invalid status '{}'. Must be 'open', 'completed', or 'overdue'.", status));
    }

    if status == "completed" {
        sqlx::query(
            "UPDATE action_items SET status = $1, completed_at = datetime('now') WHERE id = $2",
        )
        .bind(&status)
        .bind(&item_id)
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to update action item: {}", e))?;
    } else {
        sqlx::query(
            "UPDATE action_items SET status = $1, completed_at = NULL WHERE id = $2",
        )
        .bind(&status)
        .bind(&item_id)
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to update action item: {}", e))?;
    }

    info!("Updated action item {} to status '{}'", item_id, status);

    Ok(())
}

/// Get overall extraction status: how many meetings extracted, totals, etc.
#[tauri::command]
pub async fn get_extraction_status<R: Runtime>(
    app: AppHandle<R>,
) -> Result<ExtractionStatusResponse, String> {
    let state = app.state::<AppState>();
    let pool = state.db_manager.pool();

    let total_meetings: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM summary_processes WHERE status = 'completed'",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Failed to count meetings: {}", e))?;

    let extracted_meetings: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM extraction_status",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Failed to count extracted meetings: {}", e))?;

    let total_decisions: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM decisions",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Failed to count decisions: {}", e))?;

    let total_action_items: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM action_items",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Failed to count action items: {}", e))?;

    let total_open_questions: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM open_questions WHERE resolved = 0",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Failed to count open questions: {}", e))?;

    let total_topics: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM topics",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Failed to count topics: {}", e))?;

    let total_people: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM people",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Failed to count people: {}", e))?;

    Ok(ExtractionStatusResponse {
        total_meetings,
        extracted_meetings,
        pending_meetings: total_meetings - extracted_meetings,
        total_decisions,
        total_action_items,
        total_open_questions,
        total_topics,
        total_people,
    })
}

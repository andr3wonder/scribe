use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use tracing::info;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ChatMessageRow {
    pub id: String,
    pub meeting_id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

pub struct ChatMessagesRepository;

impl ChatMessagesRepository {
    /// Insert a single chat message (user or assistant).
    pub async fn insert_message(
        pool: &SqlitePool,
        meeting_id: &str,
        role: &str,
        content: &str,
    ) -> Result<String, sqlx::Error> {
        let id = format!("{:032x}", uuid::Uuid::new_v4().as_u128());
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO chat_messages (id, meeting_id, role, content, created_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(meeting_id)
        .bind(role)
        .bind(content)
        .bind(&now)
        .execute(pool)
        .await?;

        info!(
            "Inserted chat message id={} role={} for meeting={}",
            id, role, meeting_id
        );
        Ok(id)
    }

    /// Fetch all chat messages for a meeting, ordered by creation time ascending.
    pub async fn get_messages_for_meeting(
        pool: &SqlitePool,
        meeting_id: &str,
    ) -> Result<Vec<ChatMessageRow>, sqlx::Error> {
        let messages = sqlx::query_as::<_, ChatMessageRow>(
            "SELECT id, meeting_id, role, content, created_at FROM chat_messages WHERE meeting_id = ? ORDER BY created_at ASC",
        )
        .bind(meeting_id)
        .fetch_all(pool)
        .await?;

        Ok(messages)
    }

    /// Fetch the last N chat messages for a meeting (most recent, ordered ascending).
    pub async fn get_recent_messages(
        pool: &SqlitePool,
        meeting_id: &str,
        limit: i64,
    ) -> Result<Vec<ChatMessageRow>, sqlx::Error> {
        // Sub-select to get the last N rows, then re-order ascending.
        let messages = sqlx::query_as::<_, ChatMessageRow>(
            r#"
            SELECT id, meeting_id, role, content, created_at
            FROM (
                SELECT id, meeting_id, role, content, created_at
                FROM chat_messages
                WHERE meeting_id = ?
                ORDER BY created_at DESC
                LIMIT ?
            )
            ORDER BY created_at ASC
            "#,
        )
        .bind(meeting_id)
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(messages)
    }

    /// Delete all chat messages for a meeting (used when deleting a meeting).
    pub async fn delete_messages_for_meeting(
        pool: &SqlitePool,
        meeting_id: &str,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query("DELETE FROM chat_messages WHERE meeting_id = ?")
            .bind(meeting_id)
            .execute(pool)
            .await?;

        Ok(result.rows_affected())
    }

    /// Fetch all meetings that have a completed summary. Returns (meeting_id, title, summary_markdown).
    pub async fn get_meetings_with_summaries(
        pool: &SqlitePool,
    ) -> Result<Vec<(String, String, String)>, sqlx::Error> {
        let rows = sqlx::query_as::<_, (String, String, String)>(
            r#"
            SELECT m.id, m.title, sp.result
            FROM meetings m
            JOIN summary_processes sp ON m.id = sp.meeting_id
            WHERE sp.status = 'completed' AND sp.result IS NOT NULL
            ORDER BY m.created_at DESC
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(rows)
    }
}

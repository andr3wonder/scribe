use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tracing::info;

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub struct PersonProfile {
    pub id: String,
    pub name: String,
    pub role: Option<String>,
    pub organization: Option<String>,
    pub total_meetings: i64,
    pub first_interaction: Option<String>,
    pub last_interaction: Option<String>,
    pub common_topics: Vec<String>,
    pub open_action_items: Vec<ActionItemBrief>,
    pub recent_meetings: Vec<MeetingBrief>,
    pub expertise_areas: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PersonSummary {
    pub id: String,
    pub name: String,
    pub role: Option<String>,
    pub meeting_count: i64,
    pub last_seen: Option<String>,
    pub open_items_count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommitmentDetail {
    pub id: String,
    pub description: String,
    pub meeting_title: String,
    pub meeting_date: String,
    pub status: String,
    pub due_date: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExpertInfo {
    pub person_name: String,
    pub meeting_count: i64,
    pub speaking_time: Option<f64>,
    pub recent_meeting: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MeetingPrep {
    pub person_name: String,
    pub relationship_summary: String,
    pub last_meeting: Option<MeetingBrief>,
    pub open_commitments_by_them: Vec<CommitmentDetail>,
    pub open_commitments_by_you: Vec<CommitmentDetail>,
    pub recent_decisions_together: Vec<DecisionBrief>,
    pub recent_topics: Vec<String>,
    pub suggested_agenda: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingBrief {
    pub id: String,
    pub title: String,
    pub date: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActionItemBrief {
    pub description: String,
    pub status: String,
    pub meeting_title: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DecisionBrief {
    pub description: String,
    pub meeting_title: String,
    pub date: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ParticipantDetail {
    pub person_name: String,
    pub role: Option<String>,
    pub speaking_time: Option<f64>,
}

// ---------------------------------------------------------------------------
// Internal row types for sqlx queries
// ---------------------------------------------------------------------------

#[derive(Debug, sqlx::FromRow)]
struct PersonStatsRow {
    id: String,
    name: String,
    role: Option<String>,
    organization: Option<String>,
    total_meetings: i64,
    first_interaction: Option<String>,
    last_interaction: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct TopicNameRow {
    name: String,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct ActionItemRow {
    id: String,
    description: String,
    status: String,
    meeting_title: String,
}

#[derive(Debug, sqlx::FromRow)]
struct MeetingBriefRow {
    id: String,
    title: String,
    created_at: String,
}

#[derive(Debug, sqlx::FromRow)]
struct PersonSummaryRow {
    id: String,
    name: String,
    role: Option<String>,
    meeting_count: i64,
    last_seen: Option<String>,
    open_items_count: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct CommitmentRow {
    id: String,
    description: String,
    meeting_title: String,
    meeting_date: String,
    status: String,
    due_date: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct ExpertRow {
    person_name: String,
    meeting_count: i64,
    total_speaking_time: Option<f64>,
    recent_meeting: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct DecisionRow {
    description: String,
    meeting_title: String,
    date: String,
}

#[derive(Debug, sqlx::FromRow)]
struct ParticipantRow {
    person_name: String,
    role: Option<String>,
    speaking_time_seconds: Option<f64>,
}

// ---------------------------------------------------------------------------
// Service functions
// ---------------------------------------------------------------------------

/// Full profile for a person: meeting history, topics, commitments, expertise
pub async fn get_person_profile(pool: &SqlitePool, name: &str) -> Result<PersonProfile, String> {
    info!("get_person_profile: name={}", name);

    // Basic info + stats
    let stats: PersonStatsRow = sqlx::query_as(
        r#"
        SELECT p.id, p.name, p.role, p.organization,
               COUNT(DISTINCT mp.meeting_id) as total_meetings,
               MIN(m.created_at) as first_interaction,
               MAX(m.created_at) as last_interaction
        FROM people p
        LEFT JOIN meeting_participants mp ON p.id = mp.person_id
        LEFT JOIN meetings m ON mp.meeting_id = m.id
        WHERE LOWER(p.name) = LOWER(?)
        GROUP BY p.id
        "#,
    )
    .bind(name)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("DB error fetching person stats: {}", e))?
    .ok_or_else(|| format!("Person '{}' not found", name))?;

    // Common topics
    let topic_rows: Vec<TopicNameRow> = sqlx::query_as(
        r#"
        SELECT DISTINCT t.name
        FROM topics t
        JOIN meeting_topics mt ON t.id = mt.topic_id
        JOIN meeting_participants mp ON mt.meeting_id = mp.meeting_id
        JOIN people p ON mp.person_id = p.id
        WHERE LOWER(p.name) = LOWER(?)
        ORDER BY t.mention_count DESC
        LIMIT 10
        "#,
    )
    .bind(name)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("DB error fetching topics: {}", e))?;

    let common_topics: Vec<String> = topic_rows.into_iter().map(|r| r.name).collect();

    // Open action items
    let action_rows: Vec<ActionItemRow> = sqlx::query_as(
        r#"
        SELECT ai.id, ai.description, ai.status,
               COALESCE(m.title, 'Unknown Meeting') as meeting_title
        FROM action_items ai
        LEFT JOIN meetings m ON ai.meeting_id = m.id
        WHERE LOWER(ai.owner) = LOWER(?) AND ai.status = 'open'
        ORDER BY ai.created_at DESC
        "#,
    )
    .bind(name)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("DB error fetching action items: {}", e))?;

    let open_action_items: Vec<ActionItemBrief> = action_rows
        .into_iter()
        .map(|r| ActionItemBrief {
            description: r.description,
            status: r.status,
            meeting_title: r.meeting_title,
        })
        .collect();

    // Recent meetings
    let meeting_rows: Vec<MeetingBriefRow> = sqlx::query_as(
        r#"
        SELECT DISTINCT m.id, m.title, m.created_at
        FROM meetings m
        JOIN meeting_participants mp ON m.id = mp.meeting_id
        JOIN people p ON mp.person_id = p.id
        WHERE LOWER(p.name) = LOWER(?)
        ORDER BY m.created_at DESC
        LIMIT 5
        "#,
    )
    .bind(name)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("DB error fetching recent meetings: {}", e))?;

    let recent_meetings: Vec<MeetingBrief> = meeting_rows
        .into_iter()
        .map(|r| MeetingBrief {
            id: r.id,
            title: r.title,
            date: r.created_at,
        })
        .collect();

    // Expertise areas: topics where this person has appeared in 2+ meetings
    let expertise_rows: Vec<TopicNameRow> = sqlx::query_as(
        r#"
        SELECT t.name
        FROM topics t
        JOIN meeting_topics mt ON t.id = mt.topic_id
        JOIN meeting_participants mp ON mt.meeting_id = mp.meeting_id
        JOIN people p ON mp.person_id = p.id
        WHERE LOWER(p.name) = LOWER(?)
        GROUP BY t.id
        HAVING COUNT(DISTINCT mt.meeting_id) >= 2
        ORDER BY COUNT(DISTINCT mt.meeting_id) DESC
        LIMIT 10
        "#,
    )
    .bind(name)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("DB error fetching expertise: {}", e))?;

    let expertise_areas: Vec<String> = expertise_rows.into_iter().map(|r| r.name).collect();

    Ok(PersonProfile {
        id: stats.id,
        name: stats.name,
        role: stats.role,
        organization: stats.organization,
        total_meetings: stats.total_meetings,
        first_interaction: stats.first_interaction,
        last_interaction: stats.last_interaction,
        common_topics,
        open_action_items,
        recent_meetings,
        expertise_areas,
    })
}

/// List all known people, sorted by meeting_count desc
pub async fn list_people(pool: &SqlitePool, limit: Option<i64>) -> Result<Vec<PersonSummary>, String> {
    let limit = limit.unwrap_or(50);
    info!("list_people: limit={}", limit);

    let rows: Vec<PersonSummaryRow> = sqlx::query_as(
        r#"
        SELECT p.id, p.name, p.role,
               COUNT(DISTINCT mp.meeting_id) as meeting_count,
               MAX(m.created_at) as last_seen,
               (SELECT COUNT(*) FROM action_items ai
                WHERE LOWER(ai.owner) = LOWER(p.name) AND ai.status = 'open') as open_items_count
        FROM people p
        LEFT JOIN meeting_participants mp ON p.id = mp.person_id
        LEFT JOIN meetings m ON mp.meeting_id = m.id
        GROUP BY p.id
        ORDER BY meeting_count DESC
        LIMIT ?
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("DB error listing people: {}", e))?;

    Ok(rows
        .into_iter()
        .map(|r| PersonSummary {
            id: r.id,
            name: r.name,
            role: r.role,
            meeting_count: r.meeting_count,
            last_seen: r.last_seen,
            open_items_count: r.open_items_count,
        })
        .collect())
}

/// Get commitments/action items for a person (open, completed, or all)
pub async fn get_commitments_for_person(
    pool: &SqlitePool,
    person_name: &str,
    status: Option<&str>,
) -> Result<Vec<CommitmentDetail>, String> {
    info!(
        "get_commitments_for_person: person={}, status={:?}",
        person_name, status
    );

    let rows: Vec<CommitmentRow> = match status {
        Some(s) => {
            sqlx::query_as(
                r#"
                SELECT ai.id, ai.description,
                       COALESCE(m.title, 'Unknown Meeting') as meeting_title,
                       COALESCE(m.created_at, ai.created_at) as meeting_date,
                       ai.status, ai.due_date
                FROM action_items ai
                LEFT JOIN meetings m ON ai.meeting_id = m.id
                WHERE LOWER(ai.owner) = LOWER(?) AND ai.status = ?
                ORDER BY ai.created_at DESC
                "#,
            )
            .bind(person_name)
            .bind(s)
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as(
                r#"
                SELECT ai.id, ai.description,
                       COALESCE(m.title, 'Unknown Meeting') as meeting_title,
                       COALESCE(m.created_at, ai.created_at) as meeting_date,
                       ai.status, ai.due_date
                FROM action_items ai
                LEFT JOIN meetings m ON ai.meeting_id = m.id
                WHERE LOWER(ai.owner) = LOWER(?)
                ORDER BY ai.created_at DESC
                "#,
            )
            .bind(person_name)
            .fetch_all(pool)
            .await
        }
    }
    .map_err(|e| format!("DB error fetching commitments: {}", e))?;

    Ok(rows
        .into_iter()
        .map(|r| CommitmentDetail {
            id: r.id,
            description: r.description,
            meeting_title: r.meeting_title,
            meeting_date: r.meeting_date,
            status: r.status,
            due_date: r.due_date,
        })
        .collect())
}

/// Get expertise map: who knows about what topic
pub async fn get_expertise_map(pool: &SqlitePool, topic: &str) -> Result<Vec<ExpertInfo>, String> {
    info!("get_expertise_map: topic={}", topic);

    let pattern = format!("%{}%", topic);

    let rows: Vec<ExpertRow> = sqlx::query_as(
        r#"
        SELECT p.name as person_name,
               COUNT(DISTINCT mt.meeting_id) as meeting_count,
               SUM(mp.speaking_time_seconds) as total_speaking_time,
               MAX(m.created_at) as recent_meeting
        FROM people p
        JOIN meeting_participants mp ON p.id = mp.person_id
        JOIN meeting_topics mt ON mp.meeting_id = mt.meeting_id
        JOIN topics t ON mt.topic_id = t.id
        JOIN meetings m ON mp.meeting_id = m.id
        WHERE LOWER(t.name) LIKE LOWER(?)
        GROUP BY p.id
        ORDER BY meeting_count DESC, total_speaking_time DESC
        "#,
    )
    .bind(&pattern)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("DB error fetching expertise map: {}", e))?;

    Ok(rows
        .into_iter()
        .map(|r| ExpertInfo {
            person_name: r.person_name,
            meeting_count: r.meeting_count,
            speaking_time: r.total_speaking_time,
            recent_meeting: r.recent_meeting,
        })
        .collect())
}

/// Prepare for a meeting with a person: relationship summary + open items + recent context
pub async fn prepare_for_meeting(
    pool: &SqlitePool,
    person_name: &str,
) -> Result<MeetingPrep, String> {
    info!("prepare_for_meeting: person={}", person_name);

    // Get the person profile for relationship summary data
    let profile = get_person_profile(pool, person_name).await?;

    // Last meeting
    let last_meeting = profile.recent_meetings.first().cloned();

    // Open commitments by them (action items where they are the owner)
    let open_commitments_by_them =
        get_commitments_for_person(pool, person_name, Some("open")).await?;

    // Open commitments by you (action items where owner is "You" or "you" in meetings shared with this person)
    let commitments_by_you_rows: Vec<CommitmentRow> = sqlx::query_as(
        r#"
        SELECT ai.id, ai.description,
               COALESCE(m.title, 'Unknown Meeting') as meeting_title,
               COALESCE(m.created_at, ai.created_at) as meeting_date,
               ai.status, ai.due_date
        FROM action_items ai
        JOIN meetings m ON ai.meeting_id = m.id
        JOIN meeting_participants mp ON m.id = mp.meeting_id
        JOIN people p ON mp.person_id = p.id
        WHERE LOWER(ai.owner) = 'you' AND ai.status = 'open'
          AND LOWER(p.name) = LOWER(?)
        ORDER BY ai.created_at DESC
        "#,
    )
    .bind(person_name)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("DB error fetching your commitments: {}", e))?;

    let open_commitments_by_you: Vec<CommitmentDetail> = commitments_by_you_rows
        .into_iter()
        .map(|r| CommitmentDetail {
            id: r.id,
            description: r.description,
            meeting_title: r.meeting_title,
            meeting_date: r.meeting_date,
            status: r.status,
            due_date: r.due_date,
        })
        .collect();

    // Recent decisions from shared meetings
    let decision_rows: Vec<DecisionRow> = sqlx::query_as(
        r#"
        SELECT d.description,
               COALESCE(m.title, 'Unknown Meeting') as meeting_title,
               COALESCE(m.created_at, d.created_at) as date
        FROM decisions d
        JOIN meetings m ON d.meeting_id = m.id
        JOIN meeting_participants mp ON m.id = mp.meeting_id
        JOIN people p ON mp.person_id = p.id
        WHERE LOWER(p.name) = LOWER(?)
        ORDER BY m.created_at DESC
        LIMIT 10
        "#,
    )
    .bind(person_name)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("DB error fetching decisions: {}", e))?;

    let recent_decisions_together: Vec<DecisionBrief> = decision_rows
        .into_iter()
        .map(|r| DecisionBrief {
            description: r.description,
            meeting_title: r.meeting_title,
            date: r.date,
        })
        .collect();

    // Recent topics from last 3 shared meetings
    let recent_topic_rows: Vec<TopicNameRow> = sqlx::query_as(
        r#"
        SELECT DISTINCT t.name
        FROM topics t
        JOIN meeting_topics mt ON t.id = mt.topic_id
        JOIN meeting_participants mp ON mt.meeting_id = mp.meeting_id
        JOIN people p ON mp.person_id = p.id
        WHERE LOWER(p.name) = LOWER(?)
          AND mt.meeting_id IN (
              SELECT DISTINCT m2.id
              FROM meetings m2
              JOIN meeting_participants mp2 ON m2.id = mp2.meeting_id
              JOIN people p2 ON mp2.person_id = p2.id
              WHERE LOWER(p2.name) = LOWER(?)
              ORDER BY m2.created_at DESC
              LIMIT 3
          )
        ORDER BY t.mention_count DESC
        "#,
    )
    .bind(person_name)
    .bind(person_name)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("DB error fetching recent topics: {}", e))?;

    let recent_topics: Vec<String> = recent_topic_rows.into_iter().map(|r| r.name).collect();

    // Build relationship summary
    let relationship_summary = build_relationship_summary(
        &profile.name,
        profile.total_meetings,
        profile.first_interaction.as_deref(),
        &profile.common_topics,
    );

    // Build suggested agenda from open items + stale topics
    let suggested_agenda = build_suggested_agenda(
        &open_commitments_by_them,
        &open_commitments_by_you,
        &recent_topics,
        &recent_decisions_together,
    );

    Ok(MeetingPrep {
        person_name: profile.name,
        relationship_summary,
        last_meeting,
        open_commitments_by_them,
        open_commitments_by_you,
        recent_decisions_together,
        recent_topics,
        suggested_agenda,
    })
}

/// Get all people in a specific meeting
pub async fn get_meeting_participants_detail(
    pool: &SqlitePool,
    meeting_id: &str,
) -> Result<Vec<ParticipantDetail>, String> {
    info!("get_meeting_participants_detail: meeting_id={}", meeting_id);

    let rows: Vec<ParticipantRow> = sqlx::query_as(
        r#"
        SELECT p.name as person_name, mp.role, mp.speaking_time_seconds
        FROM meeting_participants mp
        JOIN people p ON mp.person_id = p.id
        WHERE mp.meeting_id = ?
        ORDER BY mp.speaking_time_seconds DESC NULLS LAST
        "#,
    )
    .bind(meeting_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("DB error fetching participants: {}", e))?;

    Ok(rows
        .into_iter()
        .map(|r| ParticipantDetail {
            person_name: r.person_name,
            role: r.role,
            speaking_time: r.speaking_time_seconds,
        })
        .collect())
}

/// Merge two people records (dedup): moves all references from merge_id to keep_id,
/// then deletes the merged record.
pub async fn merge_people(pool: &SqlitePool, keep_id: &str, merge_id: &str) -> Result<(), String> {
    info!("merge_people: keep={}, merge={}", keep_id, merge_id);

    if keep_id == merge_id {
        return Err("Cannot merge a person with themselves".to_string());
    }

    // Verify both people exist
    let keep_exists: Option<(String,)> =
        sqlx::query_as("SELECT id FROM people WHERE id = ?")
            .bind(keep_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| format!("DB error checking keep_id: {}", e))?;

    if keep_exists.is_none() {
        return Err(format!("Person with id '{}' (keep) not found", keep_id));
    }

    let merge_row: Option<(String, String)> =
        sqlx::query_as("SELECT id, name FROM people WHERE id = ?")
            .bind(merge_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| format!("DB error checking merge_id: {}", e))?;

    let merge_name = match merge_row {
        Some((_, name)) => name,
        None => return Err(format!("Person with id '{}' (merge) not found", merge_id)),
    };

    // Update meeting_participants: reassign from merge_id to keep_id
    // Use INSERT OR IGNORE + DELETE pattern to handle duplicate (meeting_id, person_id) pairs
    sqlx::query(
        r#"
        UPDATE OR IGNORE meeting_participants
        SET person_id = ?
        WHERE person_id = ?
        "#,
    )
    .bind(keep_id)
    .bind(merge_id)
    .execute(pool)
    .await
    .map_err(|e| format!("DB error updating meeting_participants: {}", e))?;

    // Delete any remaining rows that couldn't be updated due to unique constraint
    sqlx::query("DELETE FROM meeting_participants WHERE person_id = ?")
        .bind(merge_id)
        .execute(pool)
        .await
        .map_err(|e| format!("DB error cleaning up meeting_participants: {}", e))?;

    // Get the keep person's name for action_items/decisions owner reassignment
    let keep_row: (String,) = sqlx::query_as("SELECT name FROM people WHERE id = ?")
        .bind(keep_id)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("DB error fetching keep person name: {}", e))?;
    let keep_name = keep_row.0;

    // Update action_items: reassign owner from merged person's name to kept person's name
    sqlx::query(
        r#"
        UPDATE action_items
        SET owner = ?
        WHERE LOWER(owner) = LOWER(?)
        "#,
    )
    .bind(&keep_name)
    .bind(&merge_name)
    .execute(pool)
    .await
    .map_err(|e| format!("DB error updating action_items owner: {}", e))?;

    // Update decisions: reassign decided_by from merged person's name to kept person's name
    sqlx::query(
        r#"
        UPDATE decisions
        SET decided_by = ?
        WHERE LOWER(decided_by) = LOWER(?)
        "#,
    )
    .bind(&keep_name)
    .bind(&merge_name)
    .execute(pool)
    .await
    .map_err(|e| format!("DB error updating decisions decided_by: {}", e))?;

    // Update the keep person's meeting count
    let new_meeting_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(DISTINCT meeting_id) FROM meeting_participants WHERE person_id = ?",
    )
    .bind(keep_id)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("DB error counting meetings: {}", e))?;

    sqlx::query("UPDATE people SET meeting_count = ? WHERE id = ?")
        .bind(new_meeting_count.0)
        .bind(keep_id)
        .execute(pool)
        .await
        .map_err(|e| format!("DB error updating meeting count: {}", e))?;

    // Delete the merged person record
    sqlx::query("DELETE FROM people WHERE id = ?")
        .bind(merge_id)
        .execute(pool)
        .await
        .map_err(|e| format!("DB error deleting merged person: {}", e))?;

    info!(
        "merge_people: successfully merged '{}' into keep_id={}",
        merge_name, keep_id
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn build_relationship_summary(
    name: &str,
    total_meetings: i64,
    first_interaction: Option<&str>,
    common_topics: &[String],
) -> String {
    let mut parts = Vec::new();

    match (total_meetings, first_interaction) {
        (0, _) => parts.push(format!("No recorded meetings with {}", name)),
        (count, Some(first)) => {
            // Extract just the date portion (YYYY-MM-DD) if the timestamp is longer
            let date_part = if first.len() >= 10 { &first[..10] } else { first };
            parts.push(format!("Met {} time{} since {}", count, if count == 1 { "" } else { "s" }, date_part));
        }
        (count, None) => {
            parts.push(format!("Met {} time{}", count, if count == 1 { "" } else { "s" }));
        }
    }

    if !common_topics.is_empty() {
        let topics_display: Vec<&str> = common_topics.iter().take(5).map(|s| s.as_str()).collect();
        parts.push(format!("Common topics: {}", topics_display.join(", ")));
    }

    parts.join(". ")
}

fn build_suggested_agenda(
    commitments_by_them: &[CommitmentDetail],
    commitments_by_you: &[CommitmentDetail],
    recent_topics: &[String],
    recent_decisions: &[DecisionBrief],
) -> Vec<String> {
    let mut agenda = Vec::new();

    // Open action items they owe
    for item in commitments_by_them.iter().take(3) {
        agenda.push(format!("Follow up: {}", item.description));
    }

    // Open action items you owe
    for item in commitments_by_you.iter().take(3) {
        agenda.push(format!("Update on: {}", item.description));
    }

    // Recent decisions that may need review
    if !recent_decisions.is_empty() {
        agenda.push(format!(
            "Review recent decision: {}",
            recent_decisions[0].description
        ));
    }

    // Topics from recent meetings that could use continued discussion
    for topic in recent_topics.iter().take(3) {
        let already_covered = agenda.iter().any(|a| a.to_lowercase().contains(&topic.to_lowercase()));
        if !already_covered {
            agenda.push(format!("Continue discussion: {}", topic));
        }
    }

    // If we have nothing, suggest a general catch-up
    if agenda.is_empty() {
        agenda.push("General catch-up and alignment".to_string());
    }

    agenda
}

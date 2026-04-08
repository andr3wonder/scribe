use crate::chat::service::LlmConfig;
use crate::database::repositories::{
    meeting::MeetingsRepository,
    setting::SettingsRepository,
    summary::SummaryProcessesRepository,
};
use crate::summary::llm_client::generate_summary;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tracing::{info, warn};
use uuid::Uuid;

use super::prompts::{build_extraction_user_prompt, EXTRACTION_SYSTEM_PROMPT};

// ─── Extraction result types ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionResult {
    pub decisions: Vec<Decision>,
    pub action_items: Vec<ActionItem>,
    pub open_questions: Vec<OpenQuestion>,
    pub topics: Vec<String>,
    pub people_mentioned: Vec<PersonMention>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub description: String,
    pub rationale: Option<String>,
    pub alternatives_considered: Option<Vec<String>>,
    pub decided_by: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionItem {
    pub description: String,
    pub owner: Option<String>,
    pub due_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenQuestion {
    pub question: String,
    pub asked_by: Option<String>,
    pub context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonMention {
    pub name: String,
    pub role: Option<String>,
}

// ─── Internal LLM response shape ────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct LlmExtractionResponse {
    #[serde(default)]
    decisions: Vec<LlmDecision>,
    #[serde(default)]
    action_items: Vec<LlmActionItem>,
    #[serde(default)]
    open_questions: Vec<LlmOpenQuestion>,
    #[serde(default)]
    topics: Vec<String>,
    #[serde(default)]
    people_mentioned: Vec<LlmPersonMention>,
}

#[derive(Debug, Deserialize)]
struct LlmDecision {
    description: String,
    rationale: Option<String>,
    #[serde(default)]
    alternatives_considered: Option<Vec<String>>,
    #[serde(default)]
    decided_by: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct LlmActionItem {
    description: String,
    owner: Option<String>,
    due_date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LlmOpenQuestion {
    question: String,
    asked_by: Option<String>,
    context: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LlmPersonMention {
    name: String,
    role: Option<String>,
}

// ─── Core extraction logic ──────────────────────────────────────────────────

/// Extract structured entities from a meeting using the specified LLM config.
pub async fn extract_from_meeting(
    pool: &SqlitePool,
    meeting_id: &str,
    config: &LlmConfig,
) -> Result<ExtractionResult, String> {
    info!("Starting extraction for meeting_id={}", meeting_id);

    // 1. Fetch transcript text
    let transcript = get_full_transcript(pool, meeting_id).await?;
    if transcript.trim().is_empty() {
        return Err(format!("Meeting {} has no transcript content", meeting_id));
    }

    // 2. Fetch summary markdown (optional)
    let summary = get_summary_markdown(pool, meeting_id).await;

    // 3. Build prompt
    let user_prompt = build_extraction_user_prompt(&transcript, summary.as_deref());

    // 4. Call LLM
    info!("Calling LLM for extraction, meeting_id={}", meeting_id);
    let client = reqwest::Client::new();
    let raw_response = generate_summary(
        &client,
        &config.provider,
        &config.model_name,
        &config.api_key,
        EXTRACTION_SYSTEM_PROMPT,
        &user_prompt,
        config.ollama_endpoint.as_deref(),
        config.custom_openai_endpoint.as_deref(),
        config.custom_openai_max_tokens,
        config.custom_openai_temperature,
        config.custom_openai_top_p,
        config.app_data_dir.as_ref(),
        None, // no cancellation token
    )
    .await?;

    // 5. Parse JSON response
    let parsed = parse_extraction_response(&raw_response)?;

    // 6. Convert to result
    let result = ExtractionResult {
        decisions: parsed.decisions.into_iter().map(|d| Decision {
            description: d.description,
            rationale: d.rationale,
            alternatives_considered: d.alternatives_considered,
            decided_by: d.decided_by,
        }).collect(),
        action_items: parsed.action_items.into_iter().map(|a| ActionItem {
            description: a.description,
            owner: a.owner,
            due_date: a.due_date,
        }).collect(),
        open_questions: parsed.open_questions.into_iter().map(|q| OpenQuestion {
            question: q.question,
            asked_by: q.asked_by,
            context: q.context,
        }).collect(),
        topics: parsed.topics,
        people_mentioned: parsed.people_mentioned.into_iter().map(|p| PersonMention {
            name: p.name,
            role: p.role,
        }).collect(),
    };

    // 7. Store results in DB
    store_extraction_results(pool, meeting_id, &result, &config.model_name).await?;

    info!(
        "Extraction complete for meeting_id={}: {} decisions, {} action items, {} questions, {} topics, {} people",
        meeting_id,
        result.decisions.len(),
        result.action_items.len(),
        result.open_questions.len(),
        result.topics.len(),
        result.people_mentioned.len(),
    );

    Ok(result)
}

/// Auto-configured extraction: reads LLM config from the settings table.
/// Designed to be called from background tasks without needing explicit provider/model args.
pub async fn extract_from_meeting_auto(
    pool: &SqlitePool,
    meeting_id: &str,
) -> Result<ExtractionResult, String> {
    let (provider_str, model_name) = get_llm_config_from_db(pool).await?;
    let config = LlmConfig::from_db(pool, &provider_str, &model_name, None).await?;
    extract_from_meeting(pool, meeting_id, &config).await
}

/// Extract all meetings that haven't been extracted yet.
/// Returns the number of meetings processed.
pub async fn extract_all_unextracted(pool: &SqlitePool) -> Result<usize, String> {
    let (provider_str, model_name) = get_llm_config_from_db(pool).await?;
    let config = LlmConfig::from_db(pool, &provider_str, &model_name, None).await?;

    // Find meetings with completed summaries that haven't been extracted
    let rows = sqlx::query_scalar::<_, String>(
        r#"
        SELECT sp.meeting_id
        FROM summary_processes sp
        LEFT JOIN extraction_status es ON sp.meeting_id = es.meeting_id
        WHERE sp.status = 'completed' AND es.meeting_id IS NULL
        ORDER BY sp.updated_at DESC
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to query unextracted meetings: {}", e))?;

    let total = rows.len();
    info!("Found {} unextracted meetings", total);

    let mut success_count = 0;
    for meeting_id in &rows {
        match extract_from_meeting(pool, meeting_id, &config).await {
            Ok(_) => {
                success_count += 1;
                info!("Extracted {}/{}: {}", success_count, total, meeting_id);
            }
            Err(e) => {
                warn!("Failed to extract meeting {}: {}", meeting_id, e);
            }
        }
    }

    Ok(success_count)
}

/// Get the current LLM provider and model from the settings table.
pub async fn get_llm_config_from_db(pool: &SqlitePool) -> Result<(String, String), String> {
    match SettingsRepository::get_model_config(pool).await {
        Ok(Some(config)) => Ok((config.provider, config.model)),
        Ok(None) => Err("No LLM model configured. Please configure a model in Settings.".to_string()),
        Err(e) => Err(format!("Failed to read LLM settings: {}", e)),
    }
}

// ─── Helpers ────────────────────────────────────────────────────────────────

/// Build the full transcript text from all transcript segments for a meeting.
async fn get_full_transcript(pool: &SqlitePool, meeting_id: &str) -> Result<String, String> {
    let meeting = MeetingsRepository::get_meeting(pool, meeting_id)
        .await
        .map_err(|e| format!("Failed to fetch meeting: {}", e))?
        .ok_or_else(|| format!("Meeting not found: {}", meeting_id))?;

    let transcript_text: String = meeting
        .transcripts
        .iter()
        .map(|t| t.text.as_str())
        .collect::<Vec<&str>>()
        .join("\n");

    Ok(transcript_text)
}

/// Get the summary markdown for a meeting, if one exists.
async fn get_summary_markdown(pool: &SqlitePool, meeting_id: &str) -> Option<String> {
    match SummaryProcessesRepository::get_summary_data(pool, meeting_id).await {
        Ok(Some(process)) if process.status.to_lowercase() == "completed" => {
            if let Some(result_str) = process.result {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&result_str) {
                    return parsed
                        .get("markdown")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                }
            }
            None
        }
        _ => None,
    }
}

/// Parse the LLM response into structured extraction data.
/// Handles common issues like markdown code fences and trailing commas.
fn parse_extraction_response(raw: &str) -> Result<LlmExtractionResponse, String> {
    let trimmed = raw.trim();

    // Try direct parse first
    if let Ok(parsed) = serde_json::from_str::<LlmExtractionResponse>(trimmed) {
        return Ok(parsed);
    }

    // Try to extract JSON from markdown code fences
    let json_str = extract_json_block(trimmed);

    // Try parsing the extracted block
    if let Ok(parsed) = serde_json::from_str::<LlmExtractionResponse>(&json_str) {
        return Ok(parsed);
    }

    // Try cleaning up common JSON issues
    let cleaned = clean_json(&json_str);
    if let Ok(parsed) = serde_json::from_str::<LlmExtractionResponse>(&cleaned) {
        return Ok(parsed);
    }

    // Last resort: try to parse as a generic Value to get a better error
    match serde_json::from_str::<serde_json::Value>(&cleaned) {
        Ok(val) => {
            // It parsed as JSON but doesn't match our struct — try converting
            serde_json::from_value::<LlmExtractionResponse>(val)
                .map_err(|e| format!("JSON structure doesn't match expected schema: {}", e))
        }
        Err(e) => {
            warn!("Failed to parse extraction response as JSON: {}", e);
            warn!("Raw response (first 500 chars): {}", &raw[..raw.len().min(500)]);
            Err(format!("Failed to parse LLM extraction response as JSON: {}", e))
        }
    }
}

/// Extract a JSON block from text that may be wrapped in markdown code fences.
fn extract_json_block(text: &str) -> String {
    // Look for ```json ... ``` or ``` ... ```
    if let Some(start) = text.find("```json") {
        let after_fence = &text[start + 7..];
        if let Some(end) = after_fence.find("```") {
            return after_fence[..end].trim().to_string();
        }
    }
    if let Some(start) = text.find("```") {
        let after_fence = &text[start + 3..];
        if let Some(end) = after_fence.find("```") {
            return after_fence[..end].trim().to_string();
        }
    }

    // Look for first { to last }
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            if end > start {
                return text[start..=end].to_string();
            }
        }
    }

    text.to_string()
}

/// Clean common JSON issues from LLM output.
fn clean_json(text: &str) -> String {
    let mut result = text.to_string();

    // Remove trailing commas before ] or }
    // This regex-free approach handles the common cases
    loop {
        let before = result.clone();
        result = result.replace(",]", "]");
        result = result.replace(",}", "}");
        result = result.replace(", ]", "]");
        result = result.replace(", }", "}");
        // Also handle whitespace/newline between comma and bracket
        while let Some(pos) = find_trailing_comma(&result) {
            result.remove(pos);
        }
        if result == before {
            break;
        }
    }

    result
}

/// Find a trailing comma followed by optional whitespace and a closing bracket/brace.
fn find_trailing_comma(text: &str) -> Option<usize> {
    let chars: Vec<char> = text.chars().collect();
    for i in 0..chars.len() {
        if chars[i] == ',' {
            // Look ahead past whitespace
            let mut j = i + 1;
            while j < chars.len() && (chars[j] == ' ' || chars[j] == '\n' || chars[j] == '\r' || chars[j] == '\t') {
                j += 1;
            }
            if j < chars.len() && (chars[j] == ']' || chars[j] == '}') {
                return Some(i);
            }
        }
    }
    None
}

// ─── Database storage ───────────────────────────────────────────────────────

async fn store_extraction_results(
    pool: &SqlitePool,
    meeting_id: &str,
    result: &ExtractionResult,
    model_name: &str,
) -> Result<(), String> {
    // Store decisions
    for decision in &result.decisions {
        let id = Uuid::new_v4().to_string();
        let alternatives = decision.alternatives_considered.as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default());
        let decided_by = decision.decided_by.as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default());

        sqlx::query(
            r#"INSERT INTO decisions (id, meeting_id, description, rationale, alternatives_considered, decided_by)
               VALUES ($1, $2, $3, $4, $5, $6)"#,
        )
        .bind(&id)
        .bind(meeting_id)
        .bind(&decision.description)
        .bind(&decision.rationale)
        .bind(&alternatives)
        .bind(&decided_by)
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to insert decision: {}", e))?;
    }

    // Store action items
    for item in &result.action_items {
        let id = Uuid::new_v4().to_string();

        sqlx::query(
            r#"INSERT INTO action_items (id, meeting_id, description, owner, due_date, status)
               VALUES ($1, $2, $3, $4, $5, 'open')"#,
        )
        .bind(&id)
        .bind(meeting_id)
        .bind(&item.description)
        .bind(&item.owner)
        .bind(&item.due_date)
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to insert action item: {}", e))?;
    }

    // Store open questions
    for question in &result.open_questions {
        let id = Uuid::new_v4().to_string();

        sqlx::query(
            r#"INSERT INTO open_questions (id, meeting_id, question, asked_by, context)
               VALUES ($1, $2, $3, $4, $5)"#,
        )
        .bind(&id)
        .bind(meeting_id)
        .bind(&question.question)
        .bind(&question.asked_by)
        .bind(&question.context)
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to insert open question: {}", e))?;
    }

    // Store topics (INSERT OR IGNORE on name, then update counts)
    for topic_name in &result.topics {
        let topic_id = Uuid::new_v4().to_string();

        // Insert or ignore the topic
        sqlx::query(
            r#"INSERT OR IGNORE INTO topics (id, name, first_seen_meeting, last_seen_meeting, mention_count)
               VALUES ($1, $2, $3, $3, 1)"#,
        )
        .bind(&topic_id)
        .bind(topic_name)
        .bind(meeting_id)
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to insert topic: {}", e))?;

        // Update existing topic's count and last_seen
        sqlx::query(
            r#"UPDATE topics SET mention_count = mention_count + 1, last_seen_meeting = $1
               WHERE name = $2 AND last_seen_meeting != $1"#,
        )
        .bind(meeting_id)
        .bind(topic_name)
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to update topic: {}", e))?;

        // Get the actual topic ID (may be the existing one, not the one we tried to insert)
        let actual_topic_id: String = sqlx::query_scalar(
            "SELECT id FROM topics WHERE name = $1 LIMIT 1",
        )
        .bind(topic_name)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("Failed to fetch topic id: {}", e))?;

        // Link topic to meeting
        sqlx::query(
            r#"INSERT OR IGNORE INTO meeting_topics (meeting_id, topic_id, relevance)
               VALUES ($1, $2, 'primary')"#,
        )
        .bind(meeting_id)
        .bind(&actual_topic_id)
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to link topic to meeting: {}", e))?;
    }

    // Store people (INSERT OR IGNORE on name, then update counts)
    for person in &result.people_mentioned {
        let person_id = Uuid::new_v4().to_string();

        // Insert or ignore the person
        sqlx::query(
            r#"INSERT INTO people (id, name, role, first_seen_at, last_seen_at, meeting_count)
               SELECT $1, $2, $3, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, 1
               WHERE NOT EXISTS (SELECT 1 FROM people WHERE name = $2)"#,
        )
        .bind(&person_id)
        .bind(&person.name)
        .bind(&person.role)
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to insert person: {}", e))?;

        // Update existing person's count, last_seen, and role if we have one now
        if let Some(role) = &person.role {
            sqlx::query(
                r#"UPDATE people SET meeting_count = meeting_count + 1, last_seen_at = CURRENT_TIMESTAMP, role = $1
                   WHERE name = $2"#,
            )
            .bind(role)
            .bind(&person.name)
            .execute(pool)
            .await
            .map_err(|e| format!("Failed to update person: {}", e))?;
        } else {
            sqlx::query(
                r#"UPDATE people SET meeting_count = meeting_count + 1, last_seen_at = CURRENT_TIMESTAMP
                   WHERE name = $1"#,
            )
            .bind(&person.name)
            .execute(pool)
            .await
            .map_err(|e| format!("Failed to update person: {}", e))?;
        }

        // Get the actual person ID
        let actual_person_id: String = sqlx::query_scalar(
            "SELECT id FROM people WHERE name = $1 LIMIT 1",
        )
        .bind(&person.name)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("Failed to fetch person id: {}", e))?;

        // Link person to meeting
        sqlx::query(
            r#"INSERT OR IGNORE INTO meeting_participants (meeting_id, person_id, role)
               VALUES ($1, $2, 'participant')"#,
        )
        .bind(meeting_id)
        .bind(&actual_person_id)
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to link person to meeting: {}", e))?;
    }

    // Record extraction status
    sqlx::query(
        r#"INSERT OR REPLACE INTO extraction_status
           (meeting_id, extracted_at, model_used, decision_count, action_item_count, question_count, topic_count, people_count)
           VALUES ($1, CURRENT_TIMESTAMP, $2, $3, $4, $5, $6, $7)"#,
    )
    .bind(meeting_id)
    .bind(model_name)
    .bind(result.decisions.len() as i64)
    .bind(result.action_items.len() as i64)
    .bind(result.open_questions.len() as i64)
    .bind(result.topics.len() as i64)
    .bind(result.people_mentioned.len() as i64)
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to insert extraction status: {}", e))?;

    Ok(())
}

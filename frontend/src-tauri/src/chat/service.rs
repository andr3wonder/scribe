use crate::database::repositories::{
    chat_message::{ChatMessageRow, ChatMessagesRepository},
    meeting::MeetingsRepository,
    setting::SettingsRepository,
    summary::SummaryProcessesRepository,
};
use crate::search::web_search;
use crate::summary::llm_client::{generate_summary, LLMProvider};
use sqlx::SqlitePool;
use std::path::PathBuf;
use tracing::{info, warn};

/// Configuration for the LLM call, resolved from provider + model strings.
pub struct LlmConfig {
    pub provider: LLMProvider,
    pub model_name: String,
    pub api_key: String,
    pub ollama_endpoint: Option<String>,
    pub custom_openai_endpoint: Option<String>,
    pub custom_openai_max_tokens: Option<u32>,
    pub custom_openai_temperature: Option<f32>,
    pub custom_openai_top_p: Option<f32>,
    pub app_data_dir: Option<PathBuf>,
}

impl LlmConfig {
    /// Build an LlmConfig from the database settings, matching the pattern in summary::service.
    pub async fn from_db(
        pool: &SqlitePool,
        model_provider: &str,
        model_name: &str,
        app_data_dir: Option<PathBuf>,
    ) -> Result<Self, String> {
        let provider = LLMProvider::from_str(model_provider)?;

        // Resolve API key
        let api_key = if provider == LLMProvider::Ollama
            || provider == LLMProvider::BuiltInAI
            || provider == LLMProvider::CustomOpenAI
            || provider == LLMProvider::ClaudeCLI
        {
            String::new()
        } else {
            match SettingsRepository::get_api_key(pool, model_provider).await {
                Ok(Some(key)) if !key.is_empty() => key,
                Ok(_) => return Err(format!("API key not found for {}", model_provider)),
                Err(e) => {
                    return Err(format!(
                        "Failed to retrieve API key for {}: {}",
                        model_provider, e
                    ))
                }
            }
        };

        // Resolve Ollama endpoint
        let ollama_endpoint = if provider == LLMProvider::Ollama {
            match SettingsRepository::get_model_config(pool).await {
                Ok(Some(config)) => config.ollama_endpoint,
                _ => None,
            }
        } else {
            None
        };

        // Resolve CustomOpenAI config
        let (custom_openai_endpoint, final_api_key, custom_openai_max_tokens, custom_openai_temperature, custom_openai_top_p) =
            if provider == LLMProvider::CustomOpenAI {
                match SettingsRepository::get_custom_openai_config(pool).await {
                    Ok(Some(config)) => {
                        info!("Using custom OpenAI endpoint: {}", config.endpoint);
                        (
                            Some(config.endpoint),
                            config.api_key.unwrap_or_default(),
                            config.max_tokens.map(|t| t as u32),
                            config.temperature,
                            config.top_p,
                        )
                    }
                    Ok(None) => {
                        return Err(
                            "Custom OpenAI provider selected but no configuration found".to_string(),
                        )
                    }
                    Err(e) => {
                        return Err(format!("Failed to retrieve custom OpenAI config: {}", e))
                    }
                }
            } else {
                (None, api_key, None, None, None)
            };

        Ok(LlmConfig {
            provider,
            model_name: model_name.to_string(),
            api_key: final_api_key,
            ollama_endpoint,
            custom_openai_endpoint,
            custom_openai_max_tokens,
            custom_openai_temperature,
            custom_openai_top_p,
            app_data_dir,
        })
    }
}

/// Call the LLM with a system prompt and user prompt, reusing the existing llm_client infrastructure.
async fn call_llm(config: &LlmConfig, system_prompt: &str, user_prompt: &str) -> Result<String, String> {
    let client = reqwest::Client::new();
    generate_summary(
        &client,
        &config.provider,
        &config.model_name,
        &config.api_key,
        system_prompt,
        user_prompt,
        config.ollama_endpoint.as_deref(),
        config.custom_openai_endpoint.as_deref(),
        config.custom_openai_max_tokens,
        config.custom_openai_temperature,
        config.custom_openai_top_p,
        config.app_data_dir.as_ref(),
        None, // no cancellation token for chat
    )
    .await
}

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

/// Ask a question about a specific meeting's transcript.
///
/// 1. Fetches the meeting's transcript from DB
/// 2. Fetches the meeting's summary from DB (if available)
/// 3. Builds a prompt with: system prompt + summary context + transcript context + last 10 chat messages + user question
/// 4. Calls the LLM using the existing llm_client infrastructure
/// 5. Saves both user message and assistant response to chat_messages table
/// 6. Returns the assistant's response
pub async fn ask_question(
    pool: &SqlitePool,
    meeting_id: &str,
    question: &str,
    config: &LlmConfig,
) -> Result<String, String> {
    info!(
        "ask_question called for meeting_id={} question={}",
        meeting_id,
        &question[..question.len().min(80)]
    );

    // 1. Fetch transcript
    let transcript = get_full_transcript(pool, meeting_id).await?;
    if transcript.trim().is_empty() {
        return Err("Meeting has no transcript content to answer questions about.".to_string());
    }

    // 2. Fetch summary (optional)
    let summary = get_summary_markdown(pool, meeting_id).await;

    // 3. Fetch recent chat history (last 10 messages)
    let history = ChatMessagesRepository::get_recent_messages(pool, meeting_id, 10)
        .await
        .map_err(|e| format!("Failed to fetch chat history: {}", e))?;

    // 4. Build prompts
    let system_prompt = build_meeting_system_prompt(&transcript, summary.as_deref());
    let user_prompt = build_user_prompt_with_history(&history, question);

    // 5. Enrich with web search if question needs external info
    let (enriched_prompt, search_used) = web_search::enrich_prompt_if_needed(&user_prompt, question).await;
    if search_used {
        info!("Web search used to enrich chat context for question: {}", &question[..question.len().min(60)]);
    }

    // 6. Call LLM
    let response = call_llm(config, &system_prompt, &enriched_prompt).await?;

    // 6. Persist both messages
    ChatMessagesRepository::insert_message(pool, meeting_id, "user", question)
        .await
        .map_err(|e| format!("Failed to save user message: {}", e))?;

    ChatMessagesRepository::insert_message(pool, meeting_id, "assistant", &response)
        .await
        .map_err(|e| format!("Failed to save assistant message: {}", e))?;

    info!(
        "Chat Q&A completed for meeting_id={}, response length={}",
        meeting_id,
        response.len()
    );

    Ok(response)
}

/// Ask a question with a directly provided transcript (for live recording mode).
/// Does NOT persist messages to DB.
pub async fn ask_with_transcript(
    transcript: &str,
    question: &str,
    config: &LlmConfig,
) -> Result<String, String> {
    info!(
        "ask_with_transcript: question={}, transcript_len={}",
        &question[..question.len().min(80)],
        transcript.len()
    );

    let system_prompt = build_meeting_system_prompt(transcript, None);
    let user_prompt = question.to_string();

    // Enrich with web search if needed
    let (enriched_prompt, _) = web_search::enrich_prompt_if_needed(&user_prompt, question).await;

    call_llm(config, &system_prompt, &enriched_prompt).await
}

/// Get full chat history for a meeting.
pub async fn get_chat_history(
    pool: &SqlitePool,
    meeting_id: &str,
) -> Result<Vec<ChatMessageRow>, String> {
    ChatMessagesRepository::get_messages_for_meeting(pool, meeting_id)
        .await
        .map_err(|e| format!("Failed to fetch chat history: {}", e))
}

/// Ask a question across all meetings (global chat).
///
/// 1. Fetches all meetings with summaries
/// 2. Asks LLM which meetings are relevant (by summary)
/// 3. Fetches transcripts for relevant meetings
/// 4. Asks LLM the actual question with that context
/// 5. Returns the response
pub async fn ask_global(
    pool: &SqlitePool,
    question: &str,
    config: &LlmConfig,
) -> Result<String, String> {
    info!(
        "ask_global called, question={}",
        &question[..question.len().min(80)]
    );

    // 1. Fetch all meetings with summaries
    let meetings_with_summaries = ChatMessagesRepository::get_meetings_with_summaries(pool)
        .await
        .map_err(|e| format!("Failed to fetch meetings with summaries: {}", e))?;

    if meetings_with_summaries.is_empty() {
        return Err(
            "No meetings with summaries found. Generate summaries first before asking global questions."
                .to_string(),
        );
    }

    // 2. Build a list of meetings for the LLM to pick from
    let mut meeting_list = String::new();
    for (i, (id, title, result_json)) in meetings_with_summaries.iter().enumerate() {
        // Extract markdown from the result JSON
        let summary_text = serde_json::from_str::<serde_json::Value>(result_json)
            .ok()
            .and_then(|v| v.get("markdown").and_then(|m| m.as_str()).map(|s| s.to_string()))
            .unwrap_or_else(|| "(summary not available)".to_string());

        // Truncate long summaries to keep the selection prompt reasonable
        let truncated = if summary_text.len() > 500 {
            format!("{}...", &summary_text[..500])
        } else {
            summary_text
        };

        meeting_list.push_str(&format!(
            "\n[{}] Meeting ID: {} | Title: {}\nSummary: {}\n",
            i + 1,
            id,
            title,
            truncated
        ));
    }

    let selection_system = "You are a meeting assistant. Given a user question and a list of meeting summaries, respond ONLY with a JSON array of the meeting IDs that are relevant to answering the question. If none are relevant, respond with an empty array []. Do not include any other text.";
    let selection_user = format!(
        "Question: {}\n\nAvailable meetings:\n{}\n\nRespond with a JSON array of relevant meeting IDs.",
        question, meeting_list
    );

    let selection_response = call_llm(config, selection_system, &selection_user).await?;

    // 3. Parse relevant meeting IDs
    let relevant_ids = parse_meeting_ids(&selection_response, &meetings_with_summaries);

    if relevant_ids.is_empty() {
        return Ok(
            "I could not find any meetings relevant to your question. Try rephrasing or asking about a specific meeting."
                .to_string(),
        );
    }

    info!(
        "Global chat: {} relevant meetings identified out of {}",
        relevant_ids.len(),
        meetings_with_summaries.len()
    );

    // 4. Fetch transcripts for relevant meetings and build context
    let mut context = String::new();
    for meeting_id in &relevant_ids {
        let title = meetings_with_summaries
            .iter()
            .find(|(id, _, _)| id == meeting_id)
            .map(|(_, t, _)| t.as_str())
            .unwrap_or("Unknown");

        match get_full_transcript(pool, meeting_id).await {
            Ok(transcript) => {
                // Cap each transcript to prevent exceeding context limits
                let capped = if transcript.len() > 15000 {
                    format!("{}...(truncated)", &transcript[..15000])
                } else {
                    transcript
                };
                context.push_str(&format!(
                    "\n--- Meeting: {} (ID: {}) ---\n{}\n",
                    title, meeting_id, capped
                ));
            }
            Err(e) => {
                warn!(
                    "Failed to fetch transcript for meeting {}: {}",
                    meeting_id, e
                );
            }
        }
    }

    if context.trim().is_empty() {
        return Err("Could not retrieve transcript content for relevant meetings.".to_string());
    }

    // 5. Ask the actual question with the gathered context
    let answer_system = format!(
        "You are a helpful meeting assistant. Answer the user's question based on the following meeting transcripts. \
         Be specific and reference which meeting the information comes from when relevant. \
         If the transcripts don't contain enough information, say so.\n\n{}",
        context
    );

    // Enrich with web search if question needs external info
    let (enriched_question, search_used) = web_search::enrich_prompt_if_needed(question, question).await;
    if search_used {
        info!("Web search used for global chat question");
    }

    let response = call_llm(config, &answer_system, &enriched_question).await?;

    info!(
        "Global chat completed, response length={}",
        response.len()
    );

    Ok(response)
}

// ─── Private helpers ───────────────────────────────────────────────────────────

fn build_meeting_system_prompt(transcript: &str, summary: Option<&str>) -> String {
    let mut prompt = String::from(
        "You are a helpful meeting assistant. Answer the user's question based on the meeting content provided below. \
         Be specific and cite relevant parts of the transcript when appropriate. \
         If the answer is not found in the meeting content, say so.\n\n",
    );

    if let Some(s) = summary {
        prompt.push_str("## Meeting Summary\n");
        prompt.push_str(s);
        prompt.push_str("\n\n");
    }

    prompt.push_str("## Meeting Transcript\n");

    // Cap the transcript to avoid blowing up the context window
    if transcript.len() > 80_000 {
        prompt.push_str(&transcript[..80_000]);
        prompt.push_str("\n...(transcript truncated)");
    } else {
        prompt.push_str(transcript);
    }

    prompt
}

fn build_user_prompt_with_history(history: &[ChatMessageRow], question: &str) -> String {
    if history.is_empty() {
        return question.to_string();
    }

    let mut prompt = String::from("Previous conversation:\n");
    for msg in history {
        let label = if msg.role == "user" { "User" } else { "Assistant" };
        prompt.push_str(&format!("{}: {}\n", label, msg.content));
    }
    prompt.push_str(&format!("\nUser: {}", question));
    prompt
}

/// Parse meeting IDs from the LLM selection response.
/// Tries to extract a JSON array; falls back to searching for known IDs in the text.
fn parse_meeting_ids(
    response: &str,
    all_meetings: &[(String, String, String)],
) -> Vec<String> {
    // Try to parse as JSON array
    let trimmed = response.trim();

    // Try to extract JSON array from the response (LLM might wrap it in markdown)
    let json_str = if let Some(start) = trimmed.find('[') {
        if let Some(end) = trimmed.rfind(']') {
            &trimmed[start..=end]
        } else {
            trimmed
        }
    } else {
        trimmed
    };

    if let Ok(ids) = serde_json::from_str::<Vec<String>>(json_str) {
        // Filter to only valid meeting IDs
        let valid: Vec<String> = ids
            .into_iter()
            .filter(|id| all_meetings.iter().any(|(mid, _, _)| mid == id))
            .collect();
        if !valid.is_empty() {
            return valid;
        }
    }

    // Fallback: look for any known meeting IDs mentioned in the response
    let mut found = Vec::new();
    for (id, _, _) in all_meetings {
        if response.contains(id.as_str()) {
            found.push(id.clone());
        }
    }
    found
}

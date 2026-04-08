/// Extraction module - extracts structured entities from meeting transcripts/summaries
///
/// This module contains:
/// - Prompts for instructing the LLM on what to extract
/// - Core extraction logic (LLM call, JSON parsing, DB storage)
/// - Tauri command handlers for frontend integration
///
/// Extracted entities: decisions, action items, open questions, topics, people

pub mod commands;
pub mod extractor;
pub mod prompts;

// Re-export Tauri commands (with their generated __cmd__ variants)
pub use commands::{
    __cmd__extract_all_meetings, __cmd__extract_meeting_entities,
    __cmd__get_extraction_status, __cmd__get_meeting_action_items,
    __cmd__get_meeting_decisions, __cmd__get_open_questions,
    __cmd__update_action_item_status, extract_all_meetings,
    extract_meeting_entities, get_extraction_status, get_meeting_action_items,
    get_meeting_decisions, get_open_questions, update_action_item_status,
};

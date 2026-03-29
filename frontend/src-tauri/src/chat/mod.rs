/// Chat module - handles per-meeting Q&A with LLM-powered answers
///
/// This module contains:
/// - Service layer for chat logic (building prompts, calling LLM, persisting messages)
/// - Tauri command handlers for frontend integration

pub mod commands;
pub mod service;

// Re-export Tauri commands (with their generated __cmd__ variants)
pub use commands::{
    __cmd__ask_global_question, __cmd__ask_meeting_question, __cmd__get_chat_history,
    ask_global_question, ask_meeting_question, get_chat_history,
};

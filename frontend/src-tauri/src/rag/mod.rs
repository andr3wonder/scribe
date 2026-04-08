pub mod chunker;
pub mod commands;
pub mod embeddings;
pub mod indexer;
pub mod model_manager;
pub mod search;
pub mod vector_index;

// Re-export Tauri commands (with their generated __cmd__ variants)
pub use commands::{
    __cmd__rag_download_model, __cmd__rag_get_index_status, __cmd__rag_index_all,
    __cmd__rag_index_meeting, __cmd__rag_is_model_ready, __cmd__rag_search,
    rag_download_model, rag_get_index_status, rag_index_all, rag_index_meeting, rag_is_model_ready,
    rag_search,
};

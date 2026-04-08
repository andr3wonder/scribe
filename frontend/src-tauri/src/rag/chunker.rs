use crate::summary::processor::{chunk_text, rough_token_count};

/// A single chunk produced from a meeting's transcript or summary.
#[derive(Debug, Clone)]
pub struct RagChunk {
    pub content: String,
    pub source_type: String,
    pub speaker: Option<String>,
    pub chunk_index: usize,
    pub token_count: usize,
}

/// Chunk a meeting transcript into overlapping pieces.
///
/// Uses the existing `chunk_text` utility with 300-token chunks and 50-token overlap.
pub fn chunk_transcript(transcript_text: &str) -> Vec<RagChunk> {
    if transcript_text.trim().is_empty() {
        return vec![];
    }

    let raw_chunks = chunk_text(transcript_text, 300, 50);

    raw_chunks
        .into_iter()
        .enumerate()
        .map(|(i, content)| RagChunk {
            token_count: rough_token_count(&content),
            content,
            source_type: "transcript".to_string(),
            speaker: None,
            chunk_index: i,
        })
        .collect()
}

/// Chunk a summary (Markdown) by splitting on `## ` headers.
///
/// Each section becomes its own chunk. If there are no `## ` headers
/// the entire summary is returned as a single chunk.
pub fn chunk_summary(summary_markdown: &str) -> Vec<RagChunk> {
    if summary_markdown.trim().is_empty() {
        return vec![];
    }

    let mut sections: Vec<String> = Vec::new();
    let mut current_section = String::new();

    for line in summary_markdown.lines() {
        if line.starts_with("## ") {
            // Push any accumulated section before starting a new one
            let trimmed = current_section.trim().to_string();
            if !trimmed.is_empty() {
                sections.push(trimmed);
            }
            current_section = String::new();
            current_section.push_str(line);
            current_section.push('\n');
        } else {
            current_section.push_str(line);
            current_section.push('\n');
        }
    }

    // Don't forget the last section
    let trimmed = current_section.trim().to_string();
    if !trimmed.is_empty() {
        sections.push(trimmed);
    }

    sections
        .into_iter()
        .enumerate()
        .map(|(i, content)| RagChunk {
            token_count: rough_token_count(&content),
            content,
            source_type: "summary".to_string(),
            speaker: None,
            chunk_index: i,
        })
        .collect()
}

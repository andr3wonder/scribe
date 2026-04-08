-- RAG chunks with embedded vectors
CREATE TABLE IF NOT EXISTS rag_chunks (
    id TEXT PRIMARY KEY,
    meeting_id TEXT NOT NULL,
    source_type TEXT NOT NULL CHECK(source_type IN ('transcript', 'summary')),
    content TEXT NOT NULL,
    speaker TEXT,
    chunk_index INTEGER NOT NULL,
    token_count INTEGER NOT NULL,
    embedding BLOB,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_rag_chunks_meeting ON rag_chunks(meeting_id);

-- FTS5 for keyword search (built into macOS system SQLite)
CREATE VIRTUAL TABLE IF NOT EXISTS rag_chunks_fts USING fts5(
    content,
    meeting_id UNINDEXED,
    chunk_id UNINDEXED,
    tokenize='porter unicode61'
);

-- Track indexing status per meeting
CREATE TABLE IF NOT EXISTS rag_index_status (
    meeting_id TEXT PRIMARY KEY,
    indexed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    chunk_count INTEGER NOT NULL DEFAULT 0,
    model_version TEXT NOT NULL,
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE
);

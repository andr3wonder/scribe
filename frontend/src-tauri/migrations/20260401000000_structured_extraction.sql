-- Structured extraction tables for meeting intelligence

-- Decisions made in meetings
CREATE TABLE IF NOT EXISTS decisions (
    id TEXT PRIMARY KEY,
    meeting_id TEXT NOT NULL,
    description TEXT NOT NULL,
    rationale TEXT,
    alternatives_considered TEXT,
    decided_by TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_decisions_meeting ON decisions(meeting_id);

-- Action items / commitments
CREATE TABLE IF NOT EXISTS action_items (
    id TEXT PRIMARY KEY,
    meeting_id TEXT NOT NULL,
    description TEXT NOT NULL,
    owner TEXT,
    due_date TEXT,
    status TEXT DEFAULT 'open' CHECK(status IN ('open', 'completed', 'overdue')),
    completed_at TIMESTAMP,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_action_items_meeting ON action_items(meeting_id);
CREATE INDEX IF NOT EXISTS idx_action_items_owner ON action_items(owner);
CREATE INDEX IF NOT EXISTS idx_action_items_status ON action_items(status);

-- Open questions that weren't resolved
CREATE TABLE IF NOT EXISTS open_questions (
    id TEXT PRIMARY KEY,
    meeting_id TEXT NOT NULL,
    question TEXT NOT NULL,
    asked_by TEXT,
    context TEXT,
    resolved INTEGER DEFAULT 0,
    resolved_in_meeting TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_open_questions_meeting ON open_questions(meeting_id);

-- Topics discussed (for cross-meeting tracking)
CREATE TABLE IF NOT EXISTS topics (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    first_seen_meeting TEXT,
    last_seen_meeting TEXT,
    mention_count INTEGER DEFAULT 1,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS meeting_topics (
    meeting_id TEXT NOT NULL,
    topic_id TEXT NOT NULL,
    relevance TEXT DEFAULT 'primary' CHECK(relevance IN ('primary', 'mentioned')),
    PRIMARY KEY (meeting_id, topic_id),
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE,
    FOREIGN KEY (topic_id) REFERENCES topics(id) ON DELETE CASCADE
);

-- People mentioned/speaking in meetings
CREATE TABLE IF NOT EXISTS people (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    aliases TEXT,
    role TEXT,
    organization TEXT,
    first_seen_at TIMESTAMP,
    last_seen_at TIMESTAMP,
    meeting_count INTEGER DEFAULT 0,
    voice_embedding BLOB,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_people_name ON people(name);

CREATE TABLE IF NOT EXISTS meeting_participants (
    meeting_id TEXT NOT NULL,
    person_id TEXT NOT NULL,
    role TEXT DEFAULT 'participant' CHECK(role IN ('participant', 'organizer', 'presenter')),
    speaking_time_seconds REAL,
    segment_count INTEGER,
    PRIMARY KEY (meeting_id, person_id),
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE,
    FOREIGN KEY (person_id) REFERENCES people(id) ON DELETE CASCADE
);

-- Track which meetings have been extracted
CREATE TABLE IF NOT EXISTS extraction_status (
    meeting_id TEXT PRIMARY KEY,
    extracted_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    model_used TEXT,
    decision_count INTEGER DEFAULT 0,
    action_item_count INTEGER DEFAULT 0,
    question_count INTEGER DEFAULT 0,
    topic_count INTEGER DEFAULT 0,
    people_count INTEGER DEFAULT 0,
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE
);

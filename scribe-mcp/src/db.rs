use anyhow::{Context, Result};
use rusqlite::{Connection, OpenFlags};
use std::path::PathBuf;
use std::sync::Mutex;

/// A read-only handle to the Scribe/Meetily SQLite database.
/// Wrapped in a Mutex because rusqlite::Connection is Send but not Sync,
/// and the MCP server framework requires Send + Sync.
pub struct ScribeDb {
    conn: Mutex<Connection>,
}

/// A meeting record.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Meeting {
    pub id: String,
    pub title: Option<String>,
    pub created_at: Option<String>,
    pub has_summary: bool,
}

/// A search result from FTS or LIKE fallback.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SearchResult {
    pub meeting_id: String,
    pub title: Option<String>,
    pub excerpt: String,
    pub score: f64,
}

// ── Structured extraction result types ─────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize)]
pub struct DecisionResult {
    pub id: String,
    pub meeting_id: String,
    pub meeting_title: Option<String>,
    pub meeting_date: Option<String>,
    pub description: String,
    pub rationale: Option<String>,
    pub alternatives_considered: Option<String>,
    pub decided_by: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ActionItemResult {
    pub id: String,
    pub meeting_id: String,
    pub meeting_title: Option<String>,
    pub meeting_date: Option<String>,
    pub description: String,
    pub owner: Option<String>,
    pub due_date: Option<String>,
    pub status: Option<String>,
    pub completed_at: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct OpenQuestionResult {
    pub id: String,
    pub meeting_id: String,
    pub meeting_title: Option<String>,
    pub meeting_date: Option<String>,
    pub question: String,
    pub asked_by: Option<String>,
    pub context: Option<String>,
    pub resolved: bool,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PersonProfile {
    pub id: String,
    pub name: String,
    pub aliases: Option<String>,
    pub role: Option<String>,
    pub organization: Option<String>,
    pub meeting_count: i64,
    pub first_seen_at: Option<String>,
    pub last_seen_at: Option<String>,
    pub recent_meetings: Vec<PersonMeetingInfo>,
    pub common_topics: Vec<String>,
    pub open_action_items: Vec<ActionItemResult>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PersonMeetingInfo {
    pub meeting_id: String,
    pub title: Option<String>,
    pub date: Option<String>,
    pub role: Option<String>,
    pub speaking_time_seconds: Option<f64>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SpeakerSegment {
    pub meeting_id: String,
    pub meeting_title: Option<String>,
    pub speaker: Option<String>,
    pub content: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MeetingByDate {
    pub id: String,
    pub title: Option<String>,
    pub created_at: Option<String>,
    pub has_summary: bool,
    pub has_extraction: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ExpertiseEntry {
    pub person_id: String,
    pub person_name: String,
    pub role: Option<String>,
    pub organization: Option<String>,
    pub meeting_count_with_topic: i64,
    pub total_speaking_time: Option<f64>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MeetingPrepResult {
    pub person: PersonProfile,
    pub open_items_by_them: Vec<ActionItemResult>,
    pub open_items_for_you: Vec<ActionItemResult>,
    pub recent_decisions: Vec<DecisionResult>,
    pub common_topics: Vec<String>,
    pub suggested_discussion_points: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TopicResult {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub first_seen_meeting: Option<String>,
    pub last_seen_meeting: Option<String>,
    pub mention_count: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MeetingParticipantResult {
    pub person_id: String,
    pub person_name: String,
    pub role: Option<String>,
    pub person_role: Option<String>,
    pub organization: Option<String>,
    pub speaking_time_seconds: Option<f64>,
    pub segment_count: Option<i64>,
}

impl ScribeDb {
    /// Open the database in read-only mode.
    pub fn open() -> Result<Self> {
        let db_path = Self::db_path()?;
        if !db_path.exists() {
            anyhow::bail!(
                "Scribe database not found at {}. Is the Meetily app installed?",
                db_path.display()
            );
        }

        let conn = Connection::open_with_flags(
            &db_path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .with_context(|| format!("Failed to open database at {}", db_path.display()))?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Returns the expected path to the Scribe SQLite database.
    fn db_path() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Could not determine home directory")?;
        Ok(home
            .join("Library")
            .join("Application Support")
            .join("com.meetily.ai")
            .join("meeting_minutes.sqlite"))
    }

    /// Acquire the database connection lock (panics if poisoned).
    fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().expect("database mutex poisoned")
    }

    /// Check if the FTS5 table exists.
    fn has_fts(&self) -> bool {
        self.conn()
            .prepare("SELECT 1 FROM rag_chunks_fts LIMIT 1")
            .is_ok()
    }

    /// Check if the rag_chunks table exists.
    fn has_rag_chunks(&self) -> bool {
        self.conn()
            .prepare("SELECT 1 FROM rag_chunks LIMIT 1")
            .is_ok()
    }

    /// List meetings with optional limit and offset.
    pub fn list_meetings(&self, limit: i64, offset: i64) -> Result<Vec<Meeting>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT m.id, m.title, m.created_at,
                    EXISTS(SELECT 1 FROM summary_processes sp WHERE sp.meeting_id = m.id AND sp.status = 'completed') as has_summary
             FROM meetings m
             ORDER BY m.created_at DESC
             LIMIT ?1 OFFSET ?2",
        )?;

        let rows = stmt.query_map([limit, offset], |row| {
            Ok(Meeting {
                id: row.get(0)?,
                title: row.get(1)?,
                created_at: row.get(2)?,
                has_summary: row.get::<_, i64>(3)? != 0,
            })
        })?;

        let mut meetings = Vec::new();
        for row in rows {
            meetings.push(row?);
        }
        Ok(meetings)
    }

    /// Get the full transcript for a meeting by concatenating all transcript segments.
    pub fn get_transcript(&self, meeting_id: &str) -> Result<String> {
        let conn = self.conn();

        // Verify meeting exists
        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM meetings WHERE id = ?1)",
            [meeting_id],
            |row| row.get(0),
        )?;

        if !exists {
            anyhow::bail!("Meeting not found: {}", meeting_id);
        }

        let mut stmt = conn.prepare(
            "SELECT COALESCE(speaker, ''), transcript
             FROM transcripts
             WHERE meeting_id = ?1
             ORDER BY timestamp ASC",
        )?;

        let rows = stmt.query_map([meeting_id], |row| {
            let speaker: String = row.get(0)?;
            let text: String = row.get(1)?;
            Ok((speaker, text))
        })?;

        let mut parts = Vec::new();
        for row in rows {
            let (speaker, text) = row?;
            if speaker.is_empty() {
                parts.push(text);
            } else {
                parts.push(format!("{}: {}", speaker, text));
            }
        }

        if parts.is_empty() {
            anyhow::bail!("No transcript found for meeting: {}", meeting_id);
        }

        Ok(parts.join("\n"))
    }

    /// Get the summary markdown for a meeting.
    pub fn get_summary(&self, meeting_id: &str) -> Result<String> {
        let conn = self.conn();

        // Verify meeting exists
        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM meetings WHERE id = ?1)",
            [meeting_id],
            |row| row.get(0),
        )?;

        if !exists {
            anyhow::bail!("Meeting not found: {}", meeting_id);
        }

        let result: Option<String> = conn
            .query_row(
                "SELECT result FROM summary_processes WHERE meeting_id = ?1 AND status = 'completed' LIMIT 1",
                [meeting_id],
                |row| row.get(0),
            )
            .ok();

        match result {
            Some(json_str) => {
                // The result column is JSON; extract the "markdown" field
                let parsed: serde_json::Value = serde_json::from_str(&json_str)
                    .with_context(|| "Failed to parse summary JSON")?;

                if let Some(markdown) = parsed.get("markdown").and_then(|v| v.as_str()) {
                    Ok(markdown.to_string())
                } else {
                    // Fall back to the full JSON string if no "markdown" key
                    Ok(json_str)
                }
            }
            None => {
                anyhow::bail!("No summary found for meeting: {}", meeting_id);
            }
        }
    }

    /// Search meetings using FTS5 if available, otherwise fall back to LIKE.
    pub fn search_meetings(&self, query: &str, limit: i64) -> Result<Vec<SearchResult>> {
        let sanitized = sanitize_fts_query(query);

        if self.has_fts() && !sanitized.is_empty() {
            self.search_fts(&sanitized, limit)
        } else if self.has_rag_chunks() {
            self.search_rag_chunks_like(query, limit)
        } else {
            self.search_transcripts_like(query, limit)
        }
    }

    /// Search using FTS5 on rag_chunks_fts.
    fn search_fts(&self, sanitized_query: &str, limit: i64) -> Result<Vec<SearchResult>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT rc.meeting_id, m.title, snippet(rag_chunks_fts, 0, '>>>', '<<<', '...', 64) as excerpt,
                    rank
             FROM rag_chunks_fts fts
             JOIN rag_chunks rc ON rc.id = fts.chunk_id
             JOIN meetings m ON m.id = rc.meeting_id
             WHERE rag_chunks_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )?;

        let rows = stmt.query_map(rusqlite::params![sanitized_query, limit], |row| {
            Ok(SearchResult {
                meeting_id: row.get(0)?,
                title: row.get(1)?,
                excerpt: row.get(2)?,
                score: row.get::<_, f64>(3)?.abs(),
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Fall back to LIKE search on rag_chunks content.
    fn search_rag_chunks_like(&self, query: &str, limit: i64) -> Result<Vec<SearchResult>> {
        let conn = self.conn();
        let pattern = format!("%{}%", query);
        let mut stmt = conn.prepare(
            "SELECT rc.meeting_id, m.title, substr(rc.content, 1, 200) as excerpt
             FROM rag_chunks rc
             JOIN meetings m ON m.id = rc.meeting_id
             WHERE rc.content LIKE ?1
             GROUP BY rc.meeting_id
             ORDER BY m.created_at DESC
             LIMIT ?2",
        )?;

        let rows = stmt.query_map(rusqlite::params![pattern, limit], |row| {
            Ok(SearchResult {
                meeting_id: row.get(0)?,
                title: row.get(1)?,
                excerpt: row.get(2)?,
                score: 1.0,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Check if a specific table exists in the database.
    fn table_exists(&self, table_name: &str) -> bool {
        let conn = self.conn();
        conn.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name=?1")
            .and_then(|mut s| s.exists(rusqlite::params![table_name]))
            .unwrap_or(false)
    }

    // ── Structured extraction queries ──────────────────────────────────────

    /// Get decisions, optionally filtered by topic, meeting_id, or person.
    pub fn get_decisions(
        &self,
        topic: Option<&str>,
        meeting_id: Option<&str>,
        person: Option<&str>,
        limit: i64,
    ) -> Result<Vec<DecisionResult>> {
        if !self.table_exists("decisions") {
            return Ok(vec![]);
        }
        let conn = self.conn();

        let mut sql = String::from(
            "SELECT d.id, d.meeting_id, m.title, m.created_at,
                    d.description, d.rationale, d.alternatives_considered,
                    d.decided_by, d.created_at
             FROM decisions d
             JOIN meetings m ON m.id = d.meeting_id
             WHERE 1=1"
        );
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(t) = topic {
            sql.push_str(" AND (d.description LIKE ?");
            let pattern = format!("%{}%", t);
            params.push(Box::new(pattern.clone()));
            sql.push_str(" OR d.rationale LIKE ?");
            params.push(Box::new(pattern));
            sql.push(')');
        }
        if let Some(mid) = meeting_id {
            sql.push_str(" AND d.meeting_id = ?");
            params.push(Box::new(mid.to_string()));
        }
        if let Some(p) = person {
            sql.push_str(" AND d.decided_by LIKE ?");
            params.push(Box::new(format!("%{}%", p)));
        }
        sql.push_str(" ORDER BY d.created_at DESC LIMIT ?");
        params.push(Box::new(limit));

        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(param_refs.as_slice(), |row| {
            Ok(DecisionResult {
                id: row.get(0)?,
                meeting_id: row.get(1)?,
                meeting_title: row.get(2)?,
                meeting_date: row.get(3)?,
                description: row.get(4)?,
                rationale: row.get(5)?,
                alternatives_considered: row.get(6)?,
                decided_by: row.get(7)?,
                created_at: row.get(8)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Get action items, optionally filtered by owner, status, meeting_id.
    pub fn get_action_items(
        &self,
        owner: Option<&str>,
        status: Option<&str>,
        meeting_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<ActionItemResult>> {
        if !self.table_exists("action_items") {
            return Ok(vec![]);
        }
        let conn = self.conn();

        let effective_status = status.unwrap_or("open");

        let mut sql = String::from(
            "SELECT a.id, a.meeting_id, m.title, m.created_at,
                    a.description, a.owner, a.due_date, a.status,
                    a.completed_at, a.created_at
             FROM action_items a
             JOIN meetings m ON m.id = a.meeting_id
             WHERE a.status = ?"
        );
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        params.push(Box::new(effective_status.to_string()));

        if let Some(o) = owner {
            sql.push_str(" AND a.owner LIKE ?");
            params.push(Box::new(format!("%{}%", o)));
        }
        if let Some(mid) = meeting_id {
            sql.push_str(" AND a.meeting_id = ?");
            params.push(Box::new(mid.to_string()));
        }
        sql.push_str(" ORDER BY a.created_at DESC LIMIT ?");
        params.push(Box::new(limit));

        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(param_refs.as_slice(), |row| {
            Ok(ActionItemResult {
                id: row.get(0)?,
                meeting_id: row.get(1)?,
                meeting_title: row.get(2)?,
                meeting_date: row.get(3)?,
                description: row.get(4)?,
                owner: row.get(5)?,
                due_date: row.get(6)?,
                status: row.get(7)?,
                completed_at: row.get(8)?,
                created_at: row.get(9)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Get action items without defaulting to open status (for internal use).
    fn get_action_items_all_status(
        &self,
        owner: Option<&str>,
        status: Option<&str>,
        meeting_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<ActionItemResult>> {
        if !self.table_exists("action_items") {
            return Ok(vec![]);
        }
        let conn = self.conn();

        let mut sql = String::from(
            "SELECT a.id, a.meeting_id, m.title, m.created_at,
                    a.description, a.owner, a.due_date, a.status,
                    a.completed_at, a.created_at
             FROM action_items a
             JOIN meetings m ON m.id = a.meeting_id
             WHERE 1=1"
        );
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(s) = status {
            sql.push_str(" AND a.status = ?");
            params.push(Box::new(s.to_string()));
        }
        if let Some(o) = owner {
            sql.push_str(" AND a.owner LIKE ?");
            params.push(Box::new(format!("%{}%", o)));
        }
        if let Some(mid) = meeting_id {
            sql.push_str(" AND a.meeting_id = ?");
            params.push(Box::new(mid.to_string()));
        }
        sql.push_str(" ORDER BY a.created_at DESC LIMIT ?");
        params.push(Box::new(limit));

        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(param_refs.as_slice(), |row| {
            Ok(ActionItemResult {
                id: row.get(0)?,
                meeting_id: row.get(1)?,
                meeting_title: row.get(2)?,
                meeting_date: row.get(3)?,
                description: row.get(4)?,
                owner: row.get(5)?,
                due_date: row.get(6)?,
                status: row.get(7)?,
                completed_at: row.get(8)?,
                created_at: row.get(9)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Get open (unresolved) questions, optionally filtered by topic or meeting.
    pub fn get_open_questions(
        &self,
        topic: Option<&str>,
        meeting_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<OpenQuestionResult>> {
        if !self.table_exists("open_questions") {
            return Ok(vec![]);
        }
        let conn = self.conn();

        let mut sql = String::from(
            "SELECT q.id, q.meeting_id, m.title, m.created_at,
                    q.question, q.asked_by, q.context, q.resolved, q.created_at
             FROM open_questions q
             JOIN meetings m ON m.id = q.meeting_id
             WHERE q.resolved = 0"
        );
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(t) = topic {
            sql.push_str(" AND (q.question LIKE ? OR q.context LIKE ?)");
            let pattern = format!("%{}%", t);
            params.push(Box::new(pattern.clone()));
            params.push(Box::new(pattern));
        }
        if let Some(mid) = meeting_id {
            sql.push_str(" AND q.meeting_id = ?");
            params.push(Box::new(mid.to_string()));
        }
        sql.push_str(" ORDER BY q.created_at DESC LIMIT ?");
        params.push(Box::new(limit));

        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(param_refs.as_slice(), |row| {
            Ok(OpenQuestionResult {
                id: row.get(0)?,
                meeting_id: row.get(1)?,
                meeting_title: row.get(2)?,
                meeting_date: row.get(3)?,
                question: row.get(4)?,
                asked_by: row.get(5)?,
                context: row.get(6)?,
                resolved: row.get::<_, i64>(7)? != 0,
                created_at: row.get(8)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Get a full profile for a person by name.
    pub fn get_person_profile(&self, name: &str) -> Result<Option<PersonProfile>> {
        if !self.table_exists("people") {
            return Ok(None);
        }
        let conn = self.conn();
        let pattern = format!("%{}%", name);

        // Find the person
        let person = conn.query_row(
            "SELECT id, name, aliases, role, organization, meeting_count, first_seen_at, last_seen_at
             FROM people WHERE name LIKE ?1 LIMIT 1",
            rusqlite::params![pattern],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                ))
            },
        );

        let (person_id, person_name, aliases, role, organization, meeting_count, first_seen_at, last_seen_at) =
            match person {
                Ok(p) => p,
                Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
                Err(e) => return Err(e.into()),
            };

        // Recent meetings for this person
        let recent_meetings = if self.table_exists("meeting_participants") {
            let mut stmt = conn.prepare(
                "SELECT mp.meeting_id, m.title, m.created_at, mp.role, mp.speaking_time_seconds
                 FROM meeting_participants mp
                 JOIN meetings m ON m.id = mp.meeting_id
                 WHERE mp.person_id = ?1
                 ORDER BY m.created_at DESC LIMIT 10"
            )?;
            let rows = stmt.query_map(rusqlite::params![person_id], |row| {
                Ok(PersonMeetingInfo {
                    meeting_id: row.get(0)?,
                    title: row.get(1)?,
                    date: row.get(2)?,
                    role: row.get(3)?,
                    speaking_time_seconds: row.get(4)?,
                })
            })?;
            let mut v = Vec::new();
            for r in rows { v.push(r?); }
            v
        } else {
            vec![]
        };

        // Common topics for this person
        let common_topics = if self.table_exists("meeting_topics") && self.table_exists("topics") && self.table_exists("meeting_participants") {
            let mut stmt = conn.prepare(
                "SELECT DISTINCT t.name
                 FROM topics t
                 JOIN meeting_topics mt ON mt.topic_id = t.id
                 JOIN meeting_participants mp ON mp.meeting_id = mt.meeting_id
                 WHERE mp.person_id = ?1
                 ORDER BY t.mention_count DESC LIMIT 10"
            )?;
            let rows = stmt.query_map(rusqlite::params![person_id], |row| {
                row.get::<_, String>(0)
            })?;
            let mut v = Vec::new();
            for r in rows { v.push(r?); }
            v
        } else {
            vec![]
        };

        // Open action items for this person
        let open_action_items = if self.table_exists("action_items") {
            let mut stmt = conn.prepare(
                "SELECT a.id, a.meeting_id, m.title, m.created_at,
                        a.description, a.owner, a.due_date, a.status,
                        a.completed_at, a.created_at
                 FROM action_items a
                 JOIN meetings m ON m.id = a.meeting_id
                 WHERE a.owner LIKE ?1 AND a.status = 'open'
                 ORDER BY a.created_at DESC LIMIT 20"
            )?;
            let rows = stmt.query_map(rusqlite::params![pattern], |row| {
                Ok(ActionItemResult {
                    id: row.get(0)?,
                    meeting_id: row.get(1)?,
                    meeting_title: row.get(2)?,
                    meeting_date: row.get(3)?,
                    description: row.get(4)?,
                    owner: row.get(5)?,
                    due_date: row.get(6)?,
                    status: row.get(7)?,
                    completed_at: row.get(8)?,
                    created_at: row.get(9)?,
                })
            })?;
            let mut v = Vec::new();
            for r in rows { v.push(r?); }
            v
        } else {
            vec![]
        };

        Ok(Some(PersonProfile {
            id: person_id,
            name: person_name,
            aliases,
            role,
            organization,
            meeting_count,
            first_seen_at,
            last_seen_at,
            recent_meetings,
            common_topics,
            open_action_items,
        }))
    }

    /// Get commitments (action items with meeting context), filtered by person/status.
    pub fn get_commitments(
        &self,
        person: Option<&str>,
        status: Option<&str>,
        limit: i64,
    ) -> Result<Vec<ActionItemResult>> {
        // Reuses action_items query but without the default open status
        self.get_action_items_all_status(person, status, None, limit)
    }

    /// Search rag_chunks by speaker, optionally filtered by query text.
    pub fn search_by_speaker(
        &self,
        person: &str,
        query: Option<&str>,
        limit: i64,
    ) -> Result<Vec<SpeakerSegment>> {
        if !self.has_rag_chunks() {
            return Ok(vec![]);
        }
        let conn = self.conn();
        let speaker_pattern = format!("%{}%", person);

        let mut sql = String::from(
            "SELECT rc.meeting_id, m.title, rc.speaker, rc.content
             FROM rag_chunks rc
             JOIN meetings m ON m.id = rc.meeting_id
             WHERE rc.speaker LIKE ?"
        );
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        params.push(Box::new(speaker_pattern));

        if let Some(q) = query {
            sql.push_str(" AND rc.content LIKE ?");
            params.push(Box::new(format!("%{}%", q)));
        }
        sql.push_str(" ORDER BY rc.meeting_id DESC LIMIT ?");
        params.push(Box::new(limit));

        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(param_refs.as_slice(), |row| {
            Ok(SpeakerSegment {
                meeting_id: row.get(0)?,
                meeting_title: row.get(1)?,
                speaker: row.get(2)?,
                content: row.get(3)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Get meetings by date or date range.
    pub fn get_meeting_by_date(
        &self,
        date: Option<&str>,
        date_from: Option<&str>,
        date_to: Option<&str>,
        limit: i64,
    ) -> Result<Vec<MeetingByDate>> {
        let conn = self.conn();
        let has_extraction = self.table_exists("extraction_status");

        let extraction_col = if has_extraction {
            "EXISTS(SELECT 1 FROM extraction_status es WHERE es.meeting_id = m.id)"
        } else {
            "0"
        };

        let mut sql = format!(
            "SELECT m.id, m.title, m.created_at,
                    EXISTS(SELECT 1 FROM summary_processes sp WHERE sp.meeting_id = m.id AND sp.status = 'completed') as has_summary,
                    {} as has_extraction
             FROM meetings m
             WHERE 1=1",
            extraction_col
        );
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(d) = date {
            sql.push_str(" AND date(m.created_at) = date(?)");
            params.push(Box::new(d.to_string()));
        }
        if let Some(df) = date_from {
            sql.push_str(" AND date(m.created_at) >= date(?)");
            params.push(Box::new(df.to_string()));
        }
        if let Some(dt) = date_to {
            sql.push_str(" AND date(m.created_at) <= date(?)");
            params.push(Box::new(dt.to_string()));
        }
        sql.push_str(" ORDER BY m.created_at DESC LIMIT ?");
        params.push(Box::new(limit));

        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(param_refs.as_slice(), |row| {
            Ok(MeetingByDate {
                id: row.get(0)?,
                title: row.get(1)?,
                created_at: row.get(2)?,
                has_summary: row.get::<_, i64>(3)? != 0,
                has_extraction: row.get::<_, i64>(4)? != 0,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Get people who discuss a topic, ordered by frequency/speaking time.
    pub fn get_expertise_map(&self, topic: &str) -> Result<Vec<ExpertiseEntry>> {
        if !self.table_exists("people") || !self.table_exists("meeting_participants") || !self.table_exists("meeting_topics") || !self.table_exists("topics") {
            return Ok(vec![]);
        }
        let conn = self.conn();
        let topic_pattern = format!("%{}%", topic);

        let mut stmt = conn.prepare(
            "SELECT p.id, p.name, p.role, p.organization,
                    COUNT(DISTINCT mt.meeting_id) as meeting_count_with_topic,
                    SUM(mp.speaking_time_seconds) as total_speaking_time
             FROM people p
             JOIN meeting_participants mp ON mp.person_id = p.id
             JOIN meeting_topics mt ON mt.meeting_id = mp.meeting_id
             JOIN topics t ON t.id = mt.topic_id
             WHERE t.name LIKE ?1
             GROUP BY p.id
             ORDER BY meeting_count_with_topic DESC, total_speaking_time DESC
             LIMIT 20"
        )?;

        let rows = stmt.query_map(rusqlite::params![topic_pattern], |row| {
            Ok(ExpertiseEntry {
                person_id: row.get(0)?,
                person_name: row.get(1)?,
                role: row.get(2)?,
                organization: row.get(3)?,
                meeting_count_with_topic: row.get(4)?,
                total_speaking_time: row.get(5)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Comprehensive meeting prep for a person.
    pub fn prepare_for_meeting(&self, person_name: &str) -> Result<MeetingPrepResult> {
        // Get person profile
        let person = self.get_person_profile(person_name)?
            .unwrap_or_else(|| PersonProfile {
                id: String::new(),
                name: person_name.to_string(),
                aliases: None,
                role: None,
                organization: None,
                meeting_count: 0,
                first_seen_at: None,
                last_seen_at: None,
                recent_meetings: vec![],
                common_topics: vec![],
                open_action_items: vec![],
            });

        // Open action items owned by them
        let open_items_by_them = if self.table_exists("action_items") {
            self.get_action_items_all_status(Some(person_name), Some("open"), None, 20)?
        } else {
            vec![]
        };

        // Open action items NOT owned by them (items that may be "yours" from meetings with them)
        let open_items_for_you = if self.table_exists("action_items") && self.table_exists("meeting_participants") && self.table_exists("people") {
            let conn = self.conn();
            let pattern = format!("%{}%", person_name);

            let mut stmt = conn.prepare(
                "SELECT a.id, a.meeting_id, m.title, m.created_at,
                        a.description, a.owner, a.due_date, a.status,
                        a.completed_at, a.created_at
                 FROM action_items a
                 JOIN meetings m ON m.id = a.meeting_id
                 JOIN meeting_participants mp ON mp.meeting_id = a.meeting_id
                 JOIN people p ON p.id = mp.person_id
                 WHERE p.name LIKE ?1
                   AND (a.owner NOT LIKE ?1 OR a.owner IS NULL)
                   AND a.status = 'open'
                 ORDER BY a.created_at DESC LIMIT 20"
            )?;
            let rows = stmt.query_map(rusqlite::params![pattern, pattern], |row| {
                Ok(ActionItemResult {
                    id: row.get(0)?,
                    meeting_id: row.get(1)?,
                    meeting_title: row.get(2)?,
                    meeting_date: row.get(3)?,
                    description: row.get(4)?,
                    owner: row.get(5)?,
                    due_date: row.get(6)?,
                    status: row.get(7)?,
                    completed_at: row.get(8)?,
                    created_at: row.get(9)?,
                })
            })?;
            let mut v = Vec::new();
            for r in rows { v.push(r?); }
            v
        } else {
            vec![]
        };

        // Recent decisions from shared meetings
        let recent_decisions = if self.table_exists("decisions") && self.table_exists("meeting_participants") && self.table_exists("people") {
            let conn = self.conn();
            let pattern = format!("%{}%", person_name);

            let mut stmt = conn.prepare(
                "SELECT d.id, d.meeting_id, m.title, m.created_at,
                        d.description, d.rationale, d.alternatives_considered,
                        d.decided_by, d.created_at
                 FROM decisions d
                 JOIN meetings m ON m.id = d.meeting_id
                 JOIN meeting_participants mp ON mp.meeting_id = d.meeting_id
                 JOIN people p ON p.id = mp.person_id
                 WHERE p.name LIKE ?1
                 ORDER BY d.created_at DESC LIMIT 10"
            )?;
            let rows = stmt.query_map(rusqlite::params![pattern], |row| {
                Ok(DecisionResult {
                    id: row.get(0)?,
                    meeting_id: row.get(1)?,
                    meeting_title: row.get(2)?,
                    meeting_date: row.get(3)?,
                    description: row.get(4)?,
                    rationale: row.get(5)?,
                    alternatives_considered: row.get(6)?,
                    decided_by: row.get(7)?,
                    created_at: row.get(8)?,
                })
            })?;
            let mut v = Vec::new();
            for r in rows { v.push(r?); }
            v
        } else {
            vec![]
        };

        let common_topics = person.common_topics.clone();

        // Generate suggested discussion points
        let mut suggestions = Vec::new();
        for item in &open_items_by_them {
            if item.due_date.is_some() {
                suggestions.push(format!("Follow up: {} (due {})", item.description, item.due_date.as_deref().unwrap_or("?")));
            } else {
                suggestions.push(format!("Follow up: {}", item.description));
            }
        }
        for item in &open_items_for_you {
            suggestions.push(format!("Update on: {}", item.description));
        }
        for decision in recent_decisions.iter().take(3) {
            suggestions.push(format!("Check on decision: {}", decision.description));
        }

        Ok(MeetingPrepResult {
            person,
            open_items_by_them,
            open_items_for_you: open_items_for_you,
            recent_decisions,
            common_topics,
            suggested_discussion_points: suggestions,
        })
    }

    /// Get topics, optionally sorted by trending (mention_count) or recent.
    pub fn get_topics(&self, trending: bool, limit: i64) -> Result<Vec<TopicResult>> {
        if !self.table_exists("topics") {
            return Ok(vec![]);
        }
        let conn = self.conn();

        let order = if trending {
            "t.mention_count DESC"
        } else {
            "t.last_seen_meeting DESC"
        };

        let sql = format!(
            "SELECT t.id, t.name, t.description, t.first_seen_meeting, t.last_seen_meeting, t.mention_count
             FROM topics t
             ORDER BY {}
             LIMIT ?1",
            order
        );

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params![limit], |row| {
            Ok(TopicResult {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                first_seen_meeting: row.get(3)?,
                last_seen_meeting: row.get(4)?,
                mention_count: row.get(5)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Get participants for a specific meeting.
    pub fn get_meeting_participants(&self, meeting_id: &str) -> Result<Vec<MeetingParticipantResult>> {
        if !self.table_exists("meeting_participants") || !self.table_exists("people") {
            return Ok(vec![]);
        }
        let conn = self.conn();

        let mut stmt = conn.prepare(
            "SELECT p.id, p.name, mp.role, p.role, p.organization,
                    mp.speaking_time_seconds, mp.segment_count
             FROM meeting_participants mp
             JOIN people p ON p.id = mp.person_id
             WHERE mp.meeting_id = ?1
             ORDER BY mp.speaking_time_seconds DESC"
        )?;

        let rows = stmt.query_map(rusqlite::params![meeting_id], |row| {
            Ok(MeetingParticipantResult {
                person_id: row.get(0)?,
                person_name: row.get(1)?,
                role: row.get(2)?,
                person_role: row.get(3)?,
                organization: row.get(4)?,
                speaking_time_seconds: row.get(5)?,
                segment_count: row.get(6)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Find decisions matching a query string (searches description + rationale).
    pub fn find_decision(&self, query: &str, limit: i64) -> Result<Vec<DecisionResult>> {
        if !self.table_exists("decisions") {
            return Ok(vec![]);
        }
        let conn = self.conn();
        let pattern = format!("%{}%", query);

        let mut stmt = conn.prepare(
            "SELECT d.id, d.meeting_id, m.title, m.created_at,
                    d.description, d.rationale, d.alternatives_considered,
                    d.decided_by, d.created_at
             FROM decisions d
             JOIN meetings m ON m.id = d.meeting_id
             WHERE d.description LIKE ?1 OR d.rationale LIKE ?1
             ORDER BY d.created_at DESC LIMIT ?2"
        )?;

        let rows = stmt.query_map(rusqlite::params![pattern, limit], |row| {
            Ok(DecisionResult {
                id: row.get(0)?,
                meeting_id: row.get(1)?,
                meeting_title: row.get(2)?,
                meeting_date: row.get(3)?,
                description: row.get(4)?,
                rationale: row.get(5)?,
                alternatives_considered: row.get(6)?,
                decided_by: row.get(7)?,
                created_at: row.get(8)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Fall back to LIKE search on transcripts.
    fn search_transcripts_like(&self, query: &str, limit: i64) -> Result<Vec<SearchResult>> {
        let conn = self.conn();
        let pattern = format!("%{}%", query);
        let mut stmt = conn.prepare(
            "SELECT t.meeting_id, m.title, substr(t.transcript, 1, 200) as excerpt
             FROM transcripts t
             JOIN meetings m ON m.id = t.meeting_id
             WHERE t.transcript LIKE ?1
             GROUP BY t.meeting_id
             ORDER BY m.created_at DESC
             LIMIT ?2",
        )?;

        let rows = stmt.query_map(rusqlite::params![pattern, limit], |row| {
            Ok(SearchResult {
                meeting_id: row.get(0)?,
                title: row.get(1)?,
                excerpt: row.get(2)?,
                score: 1.0,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }
}

/// Sanitize a query string for FTS5. Removes special characters that could
/// break FTS5 syntax (quotes, parentheses, operators) and wraps each word
/// as a prefix search term.
fn sanitize_fts_query(query: &str) -> String {
    let words: Vec<String> = query
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .filter(|w| !w.is_empty())
        .map(|w| format!("\"{}\"*", w))
        .collect();
    words.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_fts_query() {
        assert_eq!(sanitize_fts_query("hello world"), "\"hello\"* \"world\"*");
        assert_eq!(sanitize_fts_query("design (review)"), "\"design\"* \"review\"*");
        assert_eq!(sanitize_fts_query("it's a \"test\""), "\"its\"* \"a\"* \"test\"*");
        assert_eq!(sanitize_fts_query(""), "");
        assert_eq!(sanitize_fts_query("   "), "");
    }
}

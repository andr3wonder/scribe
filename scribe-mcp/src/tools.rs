use std::sync::Arc;

use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::router::tool::ToolRouter,
    model::*,
    handler::server::wrapper::Parameters,
    schemars, tool, tool_handler, tool_router,
};

use crate::db::ScribeDb;

// ── Tool parameter schemas ──────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SearchMeetingsParams {
    /// The search query to match against meeting transcripts and summaries
    pub query: String,
    /// Maximum number of results to return (default: 10)
    #[serde(default)]
    pub limit: Option<i64>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetTranscriptParams {
    /// The meeting ID to retrieve the transcript for
    pub meeting_id: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetSummaryParams {
    /// The meeting ID to retrieve the summary for
    pub meeting_id: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ListMeetingsParams {
    /// Maximum number of meetings to return (default: 20)
    #[serde(default)]
    pub limit: Option<i64>,
    /// Number of meetings to skip (default: 0)
    #[serde(default)]
    pub offset: Option<i64>,
}

// ── Structured extraction tool parameter schemas ────────────────────────────

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetDecisionsParams {
    /// Filter decisions by topic keyword (searches description and rationale)
    #[serde(default)]
    pub topic: Option<String>,
    /// Filter decisions by meeting ID
    #[serde(default)]
    pub meeting_id: Option<String>,
    /// Filter decisions by who decided (partial name match)
    #[serde(default)]
    pub person: Option<String>,
    /// Maximum number of results to return (default: 20)
    #[serde(default)]
    pub limit: Option<i64>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetActionItemsParams {
    /// Filter by action item owner (partial name match)
    #[serde(default)]
    pub owner: Option<String>,
    /// Filter by status: 'open', 'completed', or 'overdue'. Defaults to 'open' if not specified.
    #[serde(default)]
    pub status: Option<String>,
    /// Filter by meeting ID
    #[serde(default)]
    pub meeting_id: Option<String>,
    /// Maximum number of results to return (default: 20)
    #[serde(default)]
    pub limit: Option<i64>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetOpenQuestionsParams {
    /// Filter questions by topic keyword (searches question and context)
    #[serde(default)]
    pub topic: Option<String>,
    /// Filter questions by meeting ID
    #[serde(default)]
    pub meeting_id: Option<String>,
    /// Maximum number of results to return (default: 20)
    #[serde(default)]
    pub limit: Option<i64>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetPersonProfileParams {
    /// The name of the person to look up (partial match supported)
    pub name: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetCommitmentsParams {
    /// Filter by person name (partial match)
    #[serde(default)]
    pub person: Option<String>,
    /// Filter by status: 'open', 'completed', or 'overdue'
    #[serde(default)]
    pub status: Option<String>,
    /// Maximum number of results to return (default: 20)
    #[serde(default)]
    pub limit: Option<i64>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SearchBySpeakerParams {
    /// The speaker's name to search for (partial match)
    pub person: String,
    /// Optional text query to filter the speaker's segments
    #[serde(default)]
    pub query: Option<String>,
    /// Maximum number of results to return (default: 20)
    #[serde(default)]
    pub limit: Option<i64>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetMeetingByDateParams {
    /// Exact date to filter meetings (YYYY-MM-DD format)
    #[serde(default)]
    pub date: Option<String>,
    /// Start of date range (YYYY-MM-DD format)
    #[serde(default)]
    pub date_from: Option<String>,
    /// End of date range (YYYY-MM-DD format)
    #[serde(default)]
    pub date_to: Option<String>,
    /// Maximum number of results to return (default: 20)
    #[serde(default)]
    pub limit: Option<i64>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetExpertiseMapParams {
    /// The topic to find experts for (partial name match)
    pub topic: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PrepareForMeetingParams {
    /// The name of the person you are preparing to meet with
    pub person_name: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetTopicsParams {
    /// If true, order by mention count (trending). If false/omitted, order by most recently seen.
    #[serde(default)]
    pub trending: Option<bool>,
    /// Maximum number of results to return (default: 20)
    #[serde(default)]
    pub limit: Option<i64>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetMeetingParticipantsParams {
    /// The meeting ID to get participants for
    pub meeting_id: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct FindDecisionParams {
    /// Search query to match against decision descriptions and rationale
    pub query: String,
    /// Maximum number of results to return (default: 20)
    #[serde(default)]
    pub limit: Option<i64>,
}

// ── MCP Tool Server ─────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct ScribeMcpServer {
    db: Arc<ScribeDb>,
    tool_router: ToolRouter<ScribeMcpServer>,
}

#[tool_router]
impl ScribeMcpServer {
    pub fn new(db: ScribeDb) -> Self {
        Self {
            db: Arc::new(db),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        name = "search_meetings",
        description = "Search meeting transcripts and summaries by keyword. Uses full-text search if available, falls back to substring matching. Returns matching meetings with excerpts."
    )]
    fn search_meetings(
        &self,
        Parameters(params): Parameters<SearchMeetingsParams>,
    ) -> Result<CallToolResult, McpError> {
        let limit = params.limit.unwrap_or(10).min(50);
        match self.db.search_meetings(&params.query, limit) {
            Ok(results) => {
                let json = serde_json::to_string_pretty(&results).unwrap_or_default();
                if results.is_empty() {
                    Ok(CallToolResult::success(vec![Content::text(
                        format!("No meetings found matching '{}'", params.query),
                    )]))
                } else {
                    Ok(CallToolResult::success(vec![Content::text(json)]))
                }
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Search failed: {}",
                e
            ))])),
        }
    }

    #[tool(
        name = "get_transcript",
        description = "Get the full transcript for a specific meeting. Returns all transcript segments concatenated in chronological order with speaker labels."
    )]
    fn get_transcript(
        &self,
        Parameters(params): Parameters<GetTranscriptParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.db.get_transcript(&params.meeting_id) {
            Ok(transcript) => Ok(CallToolResult::success(vec![Content::text(transcript)])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to get transcript: {}",
                e
            ))])),
        }
    }

    #[tool(
        name = "get_summary",
        description = "Get the summary for a specific meeting. Returns the summary as markdown text."
    )]
    fn get_summary(
        &self,
        Parameters(params): Parameters<GetSummaryParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.db.get_summary(&params.meeting_id) {
            Ok(summary) => Ok(CallToolResult::success(vec![Content::text(summary)])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to get summary: {}",
                e
            ))])),
        }
    }

    #[tool(
        name = "list_meetings",
        description = "List all meetings in the Scribe database, ordered by most recent first. Returns meeting IDs, titles, creation dates, and whether a summary is available."
    )]
    fn list_meetings(
        &self,
        Parameters(params): Parameters<ListMeetingsParams>,
    ) -> Result<CallToolResult, McpError> {
        let limit = params.limit.unwrap_or(20).min(100);
        let offset = params.offset.unwrap_or(0);
        match self.db.list_meetings(limit, offset) {
            Ok(meetings) => {
                let json = serde_json::to_string_pretty(&meetings).unwrap_or_default();
                if meetings.is_empty() {
                    Ok(CallToolResult::success(vec![Content::text(
                        "No meetings found in the database.",
                    )]))
                } else {
                    Ok(CallToolResult::success(vec![Content::text(json)]))
                }
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to list meetings: {}",
                e
            ))])),
        }
    }

    // ── Structured extraction tools ────────────────────────────────────────

    #[tool(
        name = "get_decisions",
        description = "Get decisions made in meetings. Optionally filter by topic keyword, meeting ID, or person who decided. Returns decisions with rationale, alternatives considered, who decided, and the meeting context."
    )]
    fn get_decisions(
        &self,
        Parameters(params): Parameters<GetDecisionsParams>,
    ) -> Result<CallToolResult, McpError> {
        let limit = params.limit.unwrap_or(20).min(100);
        match self.db.get_decisions(
            params.topic.as_deref(),
            params.meeting_id.as_deref(),
            params.person.as_deref(),
            limit,
        ) {
            Ok(results) => {
                if results.is_empty() {
                    Ok(CallToolResult::success(vec![Content::text(
                        "No decisions found. The structured extraction tables may not exist yet or no decisions have been extracted.",
                    )]))
                } else {
                    let json = serde_json::to_string_pretty(&results).unwrap_or_default();
                    Ok(CallToolResult::success(vec![Content::text(json)]))
                }
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Error fetching decisions: {}",
                e
            ))])),
        }
    }

    #[tool(
        name = "get_action_items",
        description = "Get action items from meetings. Defaults to showing only 'open' items if no status is specified. Filter by owner name, status (open/completed/overdue), or meeting ID. Returns items with owner, due date, status, and meeting context."
    )]
    fn get_action_items(
        &self,
        Parameters(params): Parameters<GetActionItemsParams>,
    ) -> Result<CallToolResult, McpError> {
        let limit = params.limit.unwrap_or(20).min(100);
        match self.db.get_action_items(
            params.owner.as_deref(),
            params.status.as_deref(),
            params.meeting_id.as_deref(),
            limit,
        ) {
            Ok(results) => {
                if results.is_empty() {
                    Ok(CallToolResult::success(vec![Content::text(
                        "No action items found. The structured extraction tables may not exist yet or no matching action items exist.",
                    )]))
                } else {
                    let json = serde_json::to_string_pretty(&results).unwrap_or_default();
                    Ok(CallToolResult::success(vec![Content::text(json)]))
                }
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Error fetching action items: {}",
                e
            ))])),
        }
    }

    #[tool(
        name = "get_open_questions",
        description = "Get unresolved questions from meetings. Filter by topic keyword or meeting ID. Returns questions with who asked, context, and meeting info. Only shows questions that haven't been resolved yet."
    )]
    fn get_open_questions(
        &self,
        Parameters(params): Parameters<GetOpenQuestionsParams>,
    ) -> Result<CallToolResult, McpError> {
        let limit = params.limit.unwrap_or(20).min(100);
        match self.db.get_open_questions(
            params.topic.as_deref(),
            params.meeting_id.as_deref(),
            limit,
        ) {
            Ok(results) => {
                if results.is_empty() {
                    Ok(CallToolResult::success(vec![Content::text(
                        "No open questions found. The structured extraction tables may not exist yet or all questions have been resolved.",
                    )]))
                } else {
                    let json = serde_json::to_string_pretty(&results).unwrap_or_default();
                    Ok(CallToolResult::success(vec![Content::text(json)]))
                }
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Error fetching open questions: {}",
                e
            ))])),
        }
    }

    #[tool(
        name = "get_person_profile",
        description = "Get a comprehensive profile for a person. Returns their role, organization, meeting count, recent meetings, common topics discussed, and open action items assigned to them. Useful for understanding your relationship with someone."
    )]
    fn get_person_profile(
        &self,
        Parameters(params): Parameters<GetPersonProfileParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.db.get_person_profile(&params.name) {
            Ok(Some(profile)) => {
                let json = serde_json::to_string_pretty(&profile).unwrap_or_default();
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Ok(None) => Ok(CallToolResult::success(vec![Content::text(format!(
                "No person found matching '{}'. The people table may not exist yet or no matching person has been extracted.",
                params.name
            ))])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Error fetching person profile: {}",
                e
            ))])),
        }
    }

    #[tool(
        name = "get_commitments",
        description = "Get commitments (action items) with meeting context. Filter by person name and/or status. Unlike get_action_items, this does not default to 'open' status — it returns all statuses unless you specify one. Good for reviewing all commitments someone has made."
    )]
    fn get_commitments(
        &self,
        Parameters(params): Parameters<GetCommitmentsParams>,
    ) -> Result<CallToolResult, McpError> {
        let limit = params.limit.unwrap_or(20).min(100);
        match self.db.get_commitments(
            params.person.as_deref(),
            params.status.as_deref(),
            limit,
        ) {
            Ok(results) => {
                if results.is_empty() {
                    Ok(CallToolResult::success(vec![Content::text(
                        "No commitments found. The structured extraction tables may not exist yet or no matching commitments exist.",
                    )]))
                } else {
                    let json = serde_json::to_string_pretty(&results).unwrap_or_default();
                    Ok(CallToolResult::success(vec![Content::text(json)]))
                }
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Error fetching commitments: {}",
                e
            ))])),
        }
    }

    #[tool(
        name = "search_by_speaker",
        description = "Search transcript chunks attributed to a specific speaker. Optionally filter by a text query to find what a specific person said about a topic. Returns matching segments with meeting context."
    )]
    fn search_by_speaker(
        &self,
        Parameters(params): Parameters<SearchBySpeakerParams>,
    ) -> Result<CallToolResult, McpError> {
        let limit = params.limit.unwrap_or(20).min(100);
        match self.db.search_by_speaker(
            &params.person,
            params.query.as_deref(),
            limit,
        ) {
            Ok(results) => {
                if results.is_empty() {
                    Ok(CallToolResult::success(vec![Content::text(format!(
                        "No transcript segments found for speaker '{}'. The rag_chunks table may not have speaker attribution or no matching segments exist.",
                        params.person
                    ))]))
                } else {
                    let json = serde_json::to_string_pretty(&results).unwrap_or_default();
                    Ok(CallToolResult::success(vec![Content::text(json)]))
                }
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Error searching by speaker: {}",
                e
            ))])),
        }
    }

    #[tool(
        name = "get_meeting_by_date",
        description = "Get meetings by date or date range. Provide 'date' for a specific day (YYYY-MM-DD), or 'date_from'/'date_to' for a range. Returns meetings with summary and extraction availability. If no date filters provided, returns most recent meetings."
    )]
    fn get_meeting_by_date(
        &self,
        Parameters(params): Parameters<GetMeetingByDateParams>,
    ) -> Result<CallToolResult, McpError> {
        let limit = params.limit.unwrap_or(20).min(100);
        match self.db.get_meeting_by_date(
            params.date.as_deref(),
            params.date_from.as_deref(),
            params.date_to.as_deref(),
            limit,
        ) {
            Ok(results) => {
                if results.is_empty() {
                    Ok(CallToolResult::success(vec![Content::text(
                        "No meetings found for the specified date range.",
                    )]))
                } else {
                    let json = serde_json::to_string_pretty(&results).unwrap_or_default();
                    Ok(CallToolResult::success(vec![Content::text(json)]))
                }
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Error fetching meetings by date: {}",
                e
            ))])),
        }
    }

    #[tool(
        name = "get_expertise_map",
        description = "Find people who discuss a specific topic, ordered by how frequently they engage with it and their total speaking time. Useful for identifying subject matter experts across your meetings."
    )]
    fn get_expertise_map(
        &self,
        Parameters(params): Parameters<GetExpertiseMapParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.db.get_expertise_map(&params.topic) {
            Ok(results) => {
                if results.is_empty() {
                    Ok(CallToolResult::success(vec![Content::text(format!(
                        "No expertise data found for topic '{}'. The structured extraction tables may not exist yet or no people have been linked to this topic.",
                        params.topic
                    ))]))
                } else {
                    let json = serde_json::to_string_pretty(&results).unwrap_or_default();
                    Ok(CallToolResult::success(vec![Content::text(json)]))
                }
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Error fetching expertise map: {}",
                e
            ))])),
        }
    }

    #[tool(
        name = "prepare_for_meeting",
        description = "Comprehensive meeting preparation for a person. Returns their profile, relationship history, open action items they own, open items from shared meetings that others own, recent shared decisions, common topics, and suggested discussion points. This is the most powerful tool for preparing before a 1:1 or group meeting."
    )]
    fn prepare_for_meeting(
        &self,
        Parameters(params): Parameters<PrepareForMeetingParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.db.prepare_for_meeting(&params.person_name) {
            Ok(result) => {
                let json = serde_json::to_string_pretty(&result).unwrap_or_default();
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Error preparing for meeting: {}",
                e
            ))])),
        }
    }

    #[tool(
        name = "get_topics",
        description = "Get topics discussed across meetings. Set 'trending' to true to order by mention count (most discussed first), or leave false/omitted to order by most recently seen. Returns topic names, descriptions, mention counts, and first/last seen dates."
    )]
    fn get_topics(
        &self,
        Parameters(params): Parameters<GetTopicsParams>,
    ) -> Result<CallToolResult, McpError> {
        let limit = params.limit.unwrap_or(20).min(100);
        let trending = params.trending.unwrap_or(false);
        match self.db.get_topics(trending, limit) {
            Ok(results) => {
                if results.is_empty() {
                    Ok(CallToolResult::success(vec![Content::text(
                        "No topics found. The topics table may not exist yet or no topics have been extracted.",
                    )]))
                } else {
                    let json = serde_json::to_string_pretty(&results).unwrap_or_default();
                    Ok(CallToolResult::success(vec![Content::text(json)]))
                }
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Error fetching topics: {}",
                e
            ))])),
        }
    }

    #[tool(
        name = "get_meeting_participants",
        description = "Get all participants in a specific meeting with their roles (participant/organizer/presenter), speaking time in seconds, and segment count. Also includes the person's job role and organization."
    )]
    fn get_meeting_participants(
        &self,
        Parameters(params): Parameters<GetMeetingParticipantsParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.db.get_meeting_participants(&params.meeting_id) {
            Ok(results) => {
                if results.is_empty() {
                    Ok(CallToolResult::success(vec![Content::text(format!(
                        "No participants found for meeting '{}'. The meeting_participants/people tables may not exist yet or no participants have been extracted.",
                        params.meeting_id
                    ))]))
                } else {
                    let json = serde_json::to_string_pretty(&results).unwrap_or_default();
                    Ok(CallToolResult::success(vec![Content::text(json)]))
                }
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Error fetching meeting participants: {}",
                e
            ))])),
        }
    }

    #[tool(
        name = "find_decision",
        description = "Search for decisions by keyword query. Searches both the decision description and rationale fields. Use this when you want to find a specific decision but don't know which meeting it was in."
    )]
    fn find_decision(
        &self,
        Parameters(params): Parameters<FindDecisionParams>,
    ) -> Result<CallToolResult, McpError> {
        let limit = params.limit.unwrap_or(20).min(100);
        match self.db.find_decision(&params.query, limit) {
            Ok(results) => {
                if results.is_empty() {
                    Ok(CallToolResult::success(vec![Content::text(format!(
                        "No decisions found matching '{}'. The decisions table may not exist yet or no matching decisions have been extracted.",
                        params.query
                    ))]))
                } else {
                    let json = serde_json::to_string_pretty(&results).unwrap_or_default();
                    Ok(CallToolResult::success(vec![Content::text(json)]))
                }
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Error finding decision: {}",
                e
            ))])),
        }
    }
}

#[tool_handler]
impl ServerHandler for ScribeMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .build(),
        )
        .with_server_info(Implementation::new("scribe-mcp", env!("CARGO_PKG_VERSION")))
        .with_protocol_version(ProtocolVersion::V_2024_11_05)
        .with_instructions(
            "Scribe MCP Server — structured meeting intelligence from the Meetily/Scribe database. \
             Core tools: 'list_meetings' to browse, 'search_meetings' to find by keyword, \
             'get_transcript' for full text, 'get_summary' for AI-generated summary. \
             Structured intelligence: 'get_decisions' and 'find_decision' for decisions, \
             'get_action_items' and 'get_commitments' for action items, \
             'get_open_questions' for unresolved questions, \
             'get_person_profile' for people profiles, 'search_by_speaker' for speaker-attributed content, \
             'get_meeting_by_date' for date-based lookup, 'get_expertise_map' for topic experts, \
             'get_topics' for topic browsing, 'get_meeting_participants' for meeting attendees, \
             and 'prepare_for_meeting' for comprehensive meeting prep with a person."
                .to_string(),
        )
    }
}

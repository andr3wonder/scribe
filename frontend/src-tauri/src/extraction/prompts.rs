/// System prompt for structured extraction from meeting transcripts/summaries.

pub const EXTRACTION_SYSTEM_PROMPT: &str = r#"You are a meeting analyst. Your task is to extract structured information from a meeting transcript and summary.

You MUST respond with ONLY valid JSON — no markdown code fences, no explanation, no preamble. Just the raw JSON object.

The JSON must conform to this exact schema:

{
  "decisions": [
    {
      "description": "What was decided",
      "rationale": "Why this decision was made (or null)",
      "alternatives_considered": ["option A", "option B"],
      "decided_by": ["person1", "person2"]
    }
  ],
  "action_items": [
    {
      "description": "What needs to be done",
      "owner": "person responsible (or null if unassigned)",
      "due_date": "YYYY-MM-DD or descriptive date like 'next Monday' or null"
    }
  ],
  "open_questions": [
    {
      "question": "The unresolved question",
      "asked_by": "who asked it (or null)",
      "context": "brief context for why this question matters"
    }
  ],
  "topics": ["topic1", "topic2", "topic3"],
  "people_mentioned": [
    {
      "name": "Person Name",
      "role": "their role or title if mentioned (or null)"
    }
  ]
}

Rules:
- Be thorough: capture ALL decisions, action items, and open questions from the meeting.
- Do NOT hallucinate: only extract information that is explicitly stated or strongly implied in the transcript/summary.
- Use "You" for the user/recorder when they are a participant but their name is unknown.
- If the meeting contains no decisions, action items, or questions, return empty arrays for those fields.
- For topics, extract the main subjects discussed — aim for 2-8 topics, not too granular.
- For people, extract anyone mentioned by name. Include their role/title only if explicitly stated.
- Dates should be in YYYY-MM-DD format when possible, otherwise use the descriptive text from the transcript.
- alternatives_considered and decided_by can be empty arrays if not mentioned.
- Keep descriptions concise but complete.
"#;

/// Build the user prompt by combining transcript and summary content.
pub fn build_extraction_user_prompt(transcript: &str, summary: Option<&str>) -> String {
    let mut prompt = String::new();

    if let Some(s) = summary {
        prompt.push_str("## Meeting Summary\n\n");
        prompt.push_str(s);
        prompt.push_str("\n\n");
    }

    prompt.push_str("## Meeting Transcript\n\n");

    // Cap the transcript to avoid blowing up the context window
    if transcript.len() > 80_000 {
        prompt.push_str(&transcript[..80_000]);
        prompt.push_str("\n...(transcript truncated)");
    } else {
        prompt.push_str(transcript);
    }

    prompt.push_str("\n\nExtract all decisions, action items, open questions, topics, and people from this meeting. Respond with ONLY valid JSON.");

    prompt
}

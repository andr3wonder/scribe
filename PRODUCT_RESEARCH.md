# Scribe Product Research: Path to Best-on-Market

## Research Status
- [x] Scribe codebase audit (completed)
- [x] Competitor feature analysis (completed — 7 tools deep-dived)
- [x] Meeting MCP landscape (completed — 6 MCP servers analyzed)
- [x] JTBD analysis (completed)
- [x] Gap analysis (completed)
- [x] Roadmap recommendation (completed)

---

## Current Scribe Assessment

### What We Have (Strengths)

| Area | Grade | Notes |
|------|-------|-------|
| Audio capture | A | Dual-stream (mic+system), Bluetooth-aware, VAD, device monitoring, crash recovery |
| Transcription | A | Dual engine (Whisper Metal GPU + Parakeet ONNX), 50+ languages, retranscription |
| LLM intelligence | A- | 8 providers, 3 chat modes, edit-via-chat, web search, template system |
| RAG search | B+ | Hybrid vector+FTS5, snowflake embeddings, just built, clean architecture |
| MCP server | B | 4 tools (search, transcript, summary, list), stdio transport, works with Claude Code |
| Recording UX | B+ | Pause/resume, live transcript, live chat during recording |
| Data layer | B | SQLite, IndexedDB recovery, incremental audio checkpoints |

### What's Missing (Weaknesses)

| Area | Grade | Gap |
|------|-------|-----|
| Speaker diarization | F | Cannot distinguish who said what. Single stream. |
| Calendar integration | F | No Google Calendar, Outlook, or Apple Calendar |
| People/relationship intelligence | F | No concept of "people" in the data model |
| Sharing/collaboration | F | Single-user, no sharing, no export |
| Export | D | Copy-to-clipboard only. No PDF, DOCX, SRT |
| Integrations | D | MCP only. No Notion, Slack, email |
| Meeting detection intelligence | C | Audio-based only (macOS). No calendar-aware detection |
| Notes feature | F | Placeholder page with hardcoded demo data |
| Streaming responses | D | All LLM responses arrive at once |
| Custom templates | D | Only 2 built-in templates |

---

## Competitor Analysis

### Market Landscape (7 tools researched)

| Tool | Approach | Rating | Key Differentiator | MCP? |
|------|----------|--------|-------------------|------|
| **Granola** | Notepad + AI (no bot) | 4.9/5 | Human-AI note blending, no meeting bot | Yes |
| **Fathom** | Bot transcription | 4.96/5 | Best free tier, 15+ templates, Deal View | No |
| **Grain** | Bot + clips | ~4.5/5 | Best clip/highlight creation | Yes |
| **Otter.ai** | Bot transcription | ~4.2/5 | Real-time live captions, slide capture | No |
| **Fireflies** | Bot transcription | ~4.0/5 | 200+ integrations, AI Skills marketplace | No |
| **tl;dv** | Bot transcription | 4.12/5 | Unlimited free recordings, sales coaching | No |
| **Read.ai** | Copilot + analytics | ~4.3/5 | Engagement analytics, cross-platform (email+chat+meetings) | No |

### What The Best Do That We Don't

1. **No-bot architecture** (Granola) — #1 user complaint across ALL competitors is the bot joining calls. Granola captures system audio directly. **We already do this.** This is an advantage.
2. **Human-AI note blending** (Granola) — Type rough notes during meeting, AI enhances with transcript context. Our notes feature is a dead placeholder.
3. **Speaker diarization** — Every competitor has it. We don't. Blocks people intelligence.
4. **Templates** (Fathom has 15+, Granola 5+, tl;dv has MEDDIC) — We have 2.
5. **CRM auto-sync** — Salesforce/HubSpot field-level population. Not relevant for our positioning.
6. **Calendar integration** — All competitors have it. Auto-names meetings, identifies participants.
7. **Export** — PDF, DOCX, video clips. We only have copy-to-clipboard.
8. **Generous free tier** — Fathom/tl;dv offer unlimited free recordings. We're local-only (free by nature).

### Where ALL Competitors Fall Short (Our Opportunity)

- **None provide structured data to AI agents** (only Granola and Grain have MCP, but limited)
- **People/relationship intelligence is shallow** — speaker labels at best, no relationship tracking
- **No cross-tool context** — meeting notes live in a silo, can't feed other agents
- **Action items aren't tracked across meetings** — extracted per-meeting but not connected
- **No "prepare me for my next meeting with X"** — the killer unmet need
- **All are cloud-dependent** — we're local-first (privacy advantage)

### What makes these tools "good enough" (commodity features):
- Auto-transcription
- AI summaries
- Action item extraction
- Meeting recording
- Search across meetings

### Where they all fall short:
- None provide structured data to AI agents
- People/relationship intelligence is shallow (speaker labels at best)
- No cross-tool context (meeting notes live in a silo)
- Action items aren't tracked across meetings
- No "prepare me for my next meeting with X" capability

---

## The Unmet JTBD

### Thesis

The market has solved: **"Help me capture what happened in my meeting"**

Nobody has solved: **"Be the memory layer that makes my AI agents actually useful for professional interactions"**

### Why This Matters Now

1. **Agentic AI is here** — Claude Code, Cursor, Devin, custom agents are becoming daily tools
2. **Agents are context-starved** — They can read code, browse the web, but have ZERO knowledge of your meetings, relationships, commitments
3. **Meeting data is the richest professional context** — Who you talk to, what you decide, what you commit to, how relationships evolve
4. **MCP is the protocol** — Standardized way for agents to access this context

### The JTBD, Reframed for Agentic Era

> "Give my AI agents rich, structured context about my professional interactions so they can actually help me with work"

Sub-jobs:
1. "My agent should know what my team decided in yesterday's standup" (decision memory)
2. "My agent should know what I owe Sarah and what she owes me" (commitment tracking)
3. "My agent should prepare me for my 1:1 with context from our last 5 meetings" (relationship intelligence)
4. "My agent should know who's responsible for what across my projects" (accountability graph)
5. "My agent should flag when commitments are overdue or contradicted" (proactive intelligence)

### Competitive Positioning

| Tool | Value Prop | Limitation |
|------|-----------|-----------|
| Granola | Best note-taking UX | Notes are an endpoint, not a source. No API for agents. |
| Otter.ai | Best real-time transcription | Transcript dump, no structured intelligence |
| Fireflies.ai | Best team collaboration | Cloud-only, no local, no agent access |
| **Scribe** | **Meeting memory layer for agentic workflows** | **Building this** |

The key differentiation: **Scribe is infrastructure, not just an app.** Other tools are destinations you visit. Scribe is a source your agents query.

---

## People Intelligence Layer (New Concept)

### The Missing Data Model

Currently Scribe has: `meetings` → `transcripts` → `summaries`. Flat. No concept of people.

What we need:

```
People
  └── Relationships (meeting history, frequency, topics)
       └── Commitments (who owes what to whom)
            └── Decisions (what was decided, by whom, when)
                 └── Topics (recurring themes across meetings with this person)
```

### New Database Entities

**`people`** — Individuals identified across meetings
- id, name, email?, role?, organization?
- first_seen_at, last_seen_at
- meeting_count, total_interaction_minutes

**`meeting_participants`** — Who was in each meeting
- meeting_id, person_id, role (speaker/listener)
- speaking_time, segment_count

**`commitments`** — Action items tied to people
- id, meeting_id, person_id (owner), description
- status (open/completed/overdue), due_date?
- created_at, completed_at

**`decisions`** — Key decisions tied to meetings and people
- id, meeting_id, description, decided_by (person_ids)
- created_at

**`topics`** — Recurring themes
- id, name, description
- Auto-extracted from meeting content via LLM

**`meeting_topics`** — Many-to-many: meetings ↔ topics

### How People Get Identified

1. **Speaker diarization** — Detect distinct speakers in audio (speaker_0, speaker_1, etc.)
2. **Name resolution** — LLM reads transcript and maps speaker_0 → "Sarah", speaker_1 → "Mike" based on context clues (introductions, name mentions, "Sarah, what do you think?")
3. **Calendar correlation** — If calendar integration exists, match meeting attendees to speakers
4. **User confirmation** — For first few meetings, ask user to confirm speaker → person mapping. Learn voice embeddings for future auto-identification.

### MCP Tools This Enables

```
search_meetings          — existing (find meetings by content)
get_transcript           — existing (full transcript)
get_summary              — existing (summary markdown)
list_meetings            — existing (browse meetings)

# NEW: People intelligence
get_person_profile       — { name } → meeting history, topics, commitments, last interaction
get_commitments          — { person?, status? } → open/overdue items with context
get_decisions            — { topic?, person?, date_range? } → decisions made
get_meeting_prep         — { person_name } → everything needed for next meeting with them
get_relationship_summary — { person } → overall relationship context, frequency, key topics
search_by_speaker        — { person } → everything this person has said across all meetings
get_topics               — trending/recurring topics across meetings
```

**The killer MCP tool: `get_meeting_prep`**

When you tell Claude "prepare me for my 1:1 with Sarah tomorrow", it calls:
1. `get_person_profile("Sarah")` — who is Sarah, how often do you meet
2. `get_commitments(person="Sarah", status="open")` — what's open between you
3. `search_meetings(query="Sarah", limit=5)` — recent meeting context
4. Returns structured prep: open items, recent decisions, topics to follow up on

No meeting tool exposes this. This is the white space.

---

## Speaker Diarization: Technical Approaches

### Option A: pyannote.audio (Python, best quality)
- State-of-the-art speaker diarization
- Requires Python sidecar process
- Models: ~100MB
- Quality: Excellent (DER ~5-10%)
- Latency: Post-processing (not real-time)

### Option B: whisper-diarize / WhisperX
- Whisper transcription + speaker diarization in one pipeline
- Requires pyannote as dependency
- Aligns word-level timestamps with speaker segments

### Option C: Speakrs (Rust, from Meetily open-source)
- Native Rust speaker diarization
- Already explored (mentioned in conversation history)
- Lighter weight but likely lower quality

### Option D: Speaker embeddings via ONNX
- Similar to our RAG embedding approach
- Extract speaker voice embeddings from audio segments
- Cluster embeddings to identify distinct speakers
- Can reuse existing `ort` dependency
- Would need a speaker embedding model (ECAPA-TDNN, ~20MB ONNX)

### Recommendation
Start with **Option D** (ONNX speaker embeddings) because:
- Reuses existing `ort` infrastructure
- No Python dependency
- Can build incrementally (cluster first, then map to names)
- Speaker embeddings become "voice fingerprints" for cross-meeting identification

---

## MCP Server Enhancement Plan

### Current State
4 basic tools: search, transcript, summary, list. Reads from SQLite directly.

### Where Current MCP Falls Short
- No people awareness
- No commitment/action item tracking
- No cross-meeting intelligence
- Returns raw text, not structured data
- No temporal queries ("what happened last week?")
- No relationship queries ("meetings with Sarah")

### Enhanced MCP Tool Set

**Tier 1: Data access (have today)**
- `search_meetings` — keyword + semantic search
- `get_transcript` — full transcript
- `get_summary` — summary markdown
- `list_meetings` — browse all meetings

**Tier 2: Structured intelligence (build next)**
- `get_action_items` — { meeting_id? person? status? } → structured action items
- `get_decisions` — { meeting_id? topic? } → decisions with context
- `get_meeting_by_date` — { date_range } → meetings in that period
- `search_by_topic` — { topic } → all meetings about this topic

**Tier 3: People/relationship (the differentiator)**
- `get_person_profile` — full relationship context
- `get_commitments` — open items between you and someone
- `get_meeting_prep` — prepare for next meeting with someone
- `get_relationship_summary` — overall relationship health
- `search_by_speaker` — everything a person has said
- `list_people` — all people you've met with, sorted by frequency/recency

**Tier 4: Proactive intelligence (the vision)**
- `get_overdue_items` — commitments past due date
- `get_stale_relationships` — people you haven't talked to in a while
- `get_contradictions` — decisions that conflict across meetings
- `get_weekly_summary` — auto-generated week-in-review

---

## Meeting MCP Landscape (Confirmed)

### What Exists Today

| MCP Server | Platform | Tools | People Data? | Cross-Meeting? | Action Items? |
|-----------|----------|-------|-------------|---------------|--------------|
| **acai** (Granola) | Go, 1 star | 15 tools (best) | Speaker utterances only | Full-text search | Yes (CRUD) |
| **mcp-limitless** | TS, 24 stars | 8 tools | Speaker names in content | Natural language temporal | Extraction only |
| **Fathom-Simple-MCP** | Python, 2 stars | 6 tools | Participants (internal/external) | Partial search | Optional |
| **granola-hosted-mcp** | TS, 0 stars | 6 tools | No | Keyword search | No |
| **ChatterBoxIO** | TS, 8 stars | 3 tools | No | No | No |
| **Our scribe-mcp** | Rust | 4 tools | No | FTS5 + semantic | No |

### The Gap Matrix — What Nobody Has

| Capability | Best Existing | **Us (Proposed)** |
|-----------|--------------|-------------------|
| People profiles across meetings | None | **Yes** — persistent profiles, meeting history, topics per person |
| Cross-meeting relationship intelligence | None | **Yes** — "who do I meet most?", relationship strength |
| Per-person action item tracking | acai has per-meeting only | **Yes** — "what does Sarah owe me across all meetings?" |
| Semantic vector search | Our current RAG only | **Yes** — hybrid vector + FTS5 already built |
| Speaker analytics over time | None | **Yes** — speaking time, participation patterns |
| Meeting-to-meeting continuity | None | **Yes** — follow-up detection, recurring topics |
| Meeting prep | None | **Yes** — `get_meeting_prep(person)` combining everything |
| Relationship graph | None (nex-crm is general, not meeting-specific) | **Yes** — derived from actual meeting co-occurrence |

### Key Insight
The ecosystem is **fragmented and shallow**. Existing MCP servers are API wrappers — they access data but don't build intelligence on top of it. Nobody builds a knowledge graph from meeting data. The "acai" Granola server is the most feature-rich with 15 tools, but even it treats meetings as isolated documents with no people model.

**The white space is confirmed: meeting intelligence as a knowledge graph, exposed via MCP.**

---

## Feature Priority Matrix

### Must-Have for "Best on Market" (Impact × Feasibility)

| # | Feature | Impact | Effort | Why |
|---|---------|--------|--------|-----|
| 1 | Speaker diarization | Critical | High (2-3 weeks) | Without this, can't build people intelligence. Every competitor has it. |
| 2 | People data model + extraction | Critical | Medium (1 week) | Foundation for the entire differentiating thesis |
| 3 | Enhanced MCP (people tools) | Critical | Medium (1 week) | The actual product differentiator |
| 4 | Calendar integration | High | Medium (1 week) | Auto-names meetings, identifies participants, enables meeting prep |
| 5 | Action item extraction + tracking | High | Low (2-3 days) | LLM extracts from transcripts, stored in DB, queryable via MCP |
| 6 | Commitment tracking across meetings | High | Medium (1 week) | "What do I owe Sarah?" — the killer use case |
| 7 | Export (PDF, markdown, SRT) | Medium | Low (2-3 days) | Table stakes, every competitor has it |
| 8 | Streaming LLM responses | Medium | Low (1-2 days) | UX polish, feels much more responsive |
| 9 | Meeting prep tool | High | Low (days) | Combines people profile + commitments + recent context |
| 10 | Auto-index on startup | Low | Low (hours) | Quality-of-life for RAG |

### Implementation Order (Critical Path)

```
Speaker Diarization (blocks everything people-related)
  → People Data Model
    → People Extraction (LLM identifies speakers → creates/updates people records)
      → Enhanced MCP Tools (people profile, commitments, meeting prep)
        → Calendar Integration (enriches people data with attendee info)

In parallel:
  Action Item Extraction → Commitment Tracking
  Export → Streaming → UI Polish
```

---

## Open Questions

1. **Speaker diarization approach**: ONNX voice embeddings (keep it Rust-native) vs pyannote (Python sidecar, better quality)?
2. **Calendar integration**: Which provider first? Apple Calendar (native on Mac) vs Google Calendar (most common)?
3. **People resolution**: How to handle when the same person appears in different meetings? Voice fingerprint matching vs name matching vs calendar matching?
4. **Privacy**: People profiles contain sensitive relationship data. How to handle? Local-only (already our model) but what about MCP access?
5. **Scope**: Build all this in Scribe (Tauri app) or focus the effort on the MCP server (where the differentiation lives)?

---

## The Vision

**Scribe isn't a meeting notes app. It's a professional relationship intelligence platform that happens to start with meetings.**

The app captures meetings. The intelligence layer extracts structured data (people, decisions, commitments, topics). The MCP server makes this available to any AI agent. The result: your agents actually know your professional world.

```
You: "Claude, prepare me for my 1:1 with Sarah tomorrow"

Claude (via Scribe MCP):
- Last met Sarah 3 days ago (Design Review)
- Open items: You owe her the Figma prototype review. She owes you the user research summary.
- Recurring topics: persona targeting, Q3 roadmap
- Recent decision: Team agreed to ship V1 by end of month
- Suggested agenda: Follow up on Figma review, discuss V1 timeline risk, review user research findings
```

No tool does this today. This is the gap.

---

## Divergent Thinking: Contrarian Perspective

### Why the "People Intelligence" Path Might Be Wrong

1. **Cold-start problem**: People intelligence requires ~20+ meetings before it's useful. Granola is useful from meeting #1. Nobody waits weeks for value.
2. **Speaker diarization is unsolved locally**: pyannote needs Python. ONNX clustering will produce "Speaker 2" not "Sarah." Trust erodes every time it's wrong.
3. **"Stale relationships" is creepy**: Quantifying relationships feels like LinkedIn notification energy. The people who'd want this (sales reps) already have Gong/Fathom.
4. **MCP users are developers, not meeting-tool buyers**: The Venn diagram of "daily MCP users" and "people who buy meeting tools" is tiny right now.

### The Instagram-to-Flickr Insight

> Flickr had better features. Instagram had the filter. You take a photo, it looks good, you share. Done.

**Scribe's "filter moment"**: Meeting ends (auto-detected) → macOS notification appears in 5 seconds with 3-line summary + top action item → swipe to dismiss or tap for more. That's the entire product for 80% of users. Everything else (RAG, MCP, people graph, 8 providers) is Flickr's 1TB storage — impressive, irrelevant to adoption.

### The Real Competition Is Doing Nothing

Not Granola. Most people leave meetings and just... remember. Or type 3 bullets in Apple Notes. To beat "doing nothing," the product must require LESS effort than doing nothing.

### Apple Is Coming

macOS will ship native meeting transcription. The entire local-transcription advantage evaporates overnight. What survives? Not the Whisper engine. Not summaries. What survives: **the open agent data layer** (MCP) and the **multi-LLM architecture** (not locked to Apple Intelligence).

### Wild Product Ideas (Same Tech Stack)

| Idea | User | Pain Point | Revenue Potential |
|------|------|-----------|-------------------|
| **Court Reporter** | Law firms, clinics | HIPAA-compliant local transcription | $200/seat/month |
| **Podcast Producer** | Creators | Record conversation → production-ready episode, zero editing | Freemium + $20/mo |
| **Language Coach** | Professionals | Real-time speaking feedback (filler words, pace, hedging) | $15/mo |
| **Anti-Meeting App** | Managers | "Could this have been an email?" + meeting waste index | Enterprise |
| **Living Lecture Notes** | Students | YouTube/Zoom lecture → study guide with practice questions | $10/mo, massive market |
| **Rewind for Conversations** | Everyone | Search your entire spoken life ("what restaurant did Jake mention?") | $20/mo |

---

## Divergent Thinking: Behavioral Psychology

### Why People ACTUALLY Take Meeting Notes (Not What They Say)
- **Performance of competence** — signals "I'm engaged" to the room
- **Anxiety management** — gives hands something to do, creates illusion of control
- **Evidence collection (CYA)** — defense against revisionist history ("you said you'd do X")
- **Processing, not recording** — they think by writing, then never re-read the notes
- **Implication**: If you automate notes, you remove a coping mechanism. You need to replace the EMOTIONAL functions, not just the informational one.

### The Real Emotional Need (Layered)
1. **Surface**: Fear of forgetting (weak — reason people sign up, not why they stay)
2. **Functional**: Accountability anxiety / CYA (powerful in large orgs)
3. **Identity**: "I am the person who has their act together" (this is where relationship intelligence becomes emotionally compelling)
4. **Deepest**: Cognitive overload — "I cannot keep up. I'm going to miss something and it will be my fault." **The tool that wins makes this feeling go away.**

> The winning emotional pitch is not "never miss a detail." It is **"you are on top of everything."**

### Why People Try Meeting Tools and Stop (Day 7 Drop-Off)
1. **Social cost recurs every meeting** — "Do people think I'm recording them?" Setup cost is one-time, discomfort is ongoing.
2. **Output isn't actionable** — You get a summary, read it, think "okay, that's what happened." Then what? No habit loop closes.
3. **Replaces low-friction with higher-friction** — Before: jot 3 bullets. After: review transcript, check AI summary, maybe edit, maybe share. Even 5 min of friction kills adoption.
4. **No social reinforcement** — Nobody else uses it. Nobody says "great notes from yesterday."
5. **"Good enough" competitor is doing nothing** — Most people muddle through fine.

### Habit Loops That Actually Work

| Loop | Trigger | Action | Reward |
|------|---------|--------|--------|
| **Morning Briefing** | Open laptop | See "here's what matters today" | Feel oriented, in control |
| **Pre-Meeting Confidence** | 5 min before meeting notification | Glance at brief (who, what last time, open items) | Walk in feeling prepared, impress people |
| **Follow-Through** | Meeting ends | See commitments crystallized | Fire off follow-ups while fresh |
| **Search-and-Find** | "Didn't we discuss this?" | Search meeting history | Find exact quote — feel vindicated |

> Key insight: the daily loop should require ZERO effort. Present information; don't ask for input.

### The "Aha Moment" That Converts Users

**Not** the first transcription. **Not** the first summary.

> **The aha moment is the first time you FIND something you would have lost.** You search, you find the exact quote from 3 weeks ago, and you think: "I would have NEVER remembered this. This would have been gone."

Second candidate: the first time a pre-meeting brief makes you look good in front of someone who matters.

### Helpful vs Creepy (People Intelligence Design Rules)
- **"Your relationship with Sarah" = empowering. "Sarah's profile" = surveillance.**
- Never surface patterns about OTHER people's behavior unless explicitly asked
- Never show sentiment scores or behavioral trends (feels clinical/dehumanizing)
- Surface specific moments instead: "Last time, Maria seemed excited about the new direction"
- Never make it easy to build a case against someone
- **People don't want a profile of others. They want to FEEL like they know others.** Profile is data. Knowing is a feeling.

### The Ideal First User
**The newly promoted engineering manager** who went from 2-3 meetings/day to 6-8. Drowning in conversations, terrified of dropping balls, technically savvy enough to adopt new tools. Their pain: "Sarah told me something important in our 1:1 last week and I can't remember what."

### The One-Line Strategy
> **Stop building a note-taking tool. Build a professional memory** — one that makes people feel like the most attentive, reliable, prepared person in every room they walk into.

Design principles:
1. Zero-effort capture (no bots, no buttons)
2. Automatic surfacing (tool brings info to you)
3. Social reward, not productivity metrics
4. Relationship-centric, not data-centric
5. Invisible until powerful (feel like a good memory, not software)

---

## Divergent Thinking: Search/IR Rethink

### Vector Search May Be Wrong for Meetings
Embeddings destroy temporal structure, social dynamics, emotional valence, and speech acts. "Who said what AFTER the disagreement about the budget" is a sequence query, not a similarity query.

### The Unit of Retrieval Shouldn't Be a Chunk
People don't think in 300-token blocks. Better retrieval units:

| Unit | Example Query |
|------|--------------|
| **Decision** | "What did we decide about the launch date?" |
| **Commitment** | "What did I promise Sarah?" |
| **Open question** | "What was left unresolved?" |
| **Disagreement** | "What did we disagree about?" |
| **Topic arc** | "What was the full discussion about pricing?" |

**Implementation**: After summary, make a second LLM pass extracting `{decisions, commitments, open_questions, disagreements}`. Each gets its own embedding and FTS5 entry. Search across both chunks AND semantic units.

### Best Search Is No Search: Proactive Intelligence
- **Pre-meeting briefing** (30 min before): "Meeting with Sarah in 30m. Open items: Figma review (you owe her)."
- **Commitment decay alerts**: "You committed to the API spec review 5 days ago. Not mentioned since."
- **Decision drift detection**: "Team decided Postgres on Mar 5. Someone proposed DynamoDB on Mar 20."
- **Meeting quality signals**: "3 unresolved questions, 2 commitments with no due date, 80% speaking time from one person."

### Temporal Search (Nobody Has This)
Add `meeting_date` to search queries. Support: "what happened this week?", "how has our position on X changed over time?", "what keeps coming up but never gets resolved?"

### When RAG Beats Full-Context (and Vice Versa)
At 665KB total text, Claude can fit everything. RAG wins for: specific lookups, source citation, cost/latency. Full-context wins for: temporal evolution, negative queries ("what wasn't discussed?"), contradiction detection.

**Recommendation**: Always send full list of meeting titles+dates+topics as "map." Use RAG to "zoom in" on specific excerpts.

---

## Divergent Thinking: Developer/Power User Perspective

### The "Holy Shit" Moment
> Typing `"what was decided about caching?"` in Claude Code and getting exact quotes, attributions, dates, and rationale in 2 seconds. That ends 30-minute circular Slack threads.

### 10 Real Claude Code Prompts (What Power Users Would Actually Type)
1. "What context from meetings is relevant to this PR?"
2. "Write my weekly status update based on this week's meetings"
3. "What did the team decide about the API migration? Give me the exact quote"
4. "Prepare me for my 1:1 with Sarah — open items, recent topics, decisions pending"
5. "Find every meeting where someone mentioned technical debt in the auth service"
6. "What zombie topics keep coming up but never get resolved?"
7. "Create a design doc context section from the last 3 architecture meetings"
8. "What action items are assigned to me across all recent meetings?"
9. "Who should I talk to about the caching strategy? Who's discussed it?"
10. "Diff what was planned in Monday's meeting vs what actually happened by Friday"

### 8 New MCP Tools Needed
`get_meeting_context`, `who_said_what`, `my_action_items`, `meeting_diff`, `find_decision`, `prepare_for_meeting`, `get_meeting_by_date`, `search_by_speaker`

### Structured Extraction Pipeline (Key Architecture Addition)
After each meeting, run a second LLM pass extracting:
```json
{
  "decisions": [{"what": "Use Postgres", "who_decided": ["Sarah", "Mike"], "rationale": "..."}],
  "action_items": [{"owner": "You", "task": "Review API spec", "due": "Friday"}],
  "risks": [{"description": "Timeline slippage", "raised_by": "Sarah"}],
  "open_questions": [{"question": "Who owns the migration?", "asked_by": "Mike"}]
}
```
This is what makes MCP tools actually useful — structured data, not just text search.

### Write Tools (Not Just Read)
MCP server should also create artifacts: Jira tickets from action items, correct transcription errors, link meeting series, update action item status, synthesize multiple meetings into ADRs/design docs.

---

## Divergent Thinking: Business Strategy

### The Actual Buyer (Ranked)
1. **Solo consultants / fractional execs** — 15-25 client meetings/week, $150-500/hr, pay $30-50/mo instantly. Pain: "I'm mixing up what I promised Client X vs Client Y."
2. **Developer/AI power users** — Want Claude to have meeting memory. MCP is the draw. Distribution channel, not revenue.
3. **Regulated professionals (legal, therapy, finance)** — CANNOT use cloud. Will pay $50-100/mo. Currently underserved. Hard to reach.

### The Wedge (Not Transcription)
> The demo that sells this product: "Claude, what did Sarah say about the Q3 timeline?" → Claude returns the exact quote, commitments, and follow-up context via Scribe MCP. That's the Product Hunt video.

### Distribution: MCP Server IS the GTM
1. Developer searches "meeting MCP server"
2. Finds scribe-mcp, installs it
3. It reads the Scribe SQLite DB (but no meetings yet)
4. "No meetings found. Install Scribe to start recording: [link]"
5. Records one meeting, queries Claude about it → aha moment

**This is the Dropbox playbook.** The MCP server is the synced folder.

### Pricing Model
| Tier | Price | What |
|------|-------|------|
| Free | $0 | Unlimited recording, transcription, basic MCP (4 tools), local LLM |
| Pro | $15/mo | Cloud LLM, people intelligence, meeting prep, action tracking, calendar, export |

### The Moat = Data Flywheel
After 6 months of meetings, your Scribe DB contains your entire professional relationship graph. Switching costs become massive. The people intelligence layer IS the moat.

### $100M vs $1M
- **$1M**: Better recorder for privacy-conscious devs. Caps at $50-100K/yr.
- **$100M**: Professional relationship knowledge graph. Every AI agent queries it. Enterprise licensing ($50-100/seat). Industry-specific (legal, healthcare). The difference is ambition, not features.

### High-Margin Niche: Legal
Lawyers bill $200-400/hr. Cannot use cloud (attorney-client privilege). Currently pay court reporters or write notes from memory. Would pay $100-200/mo/seat. "Talk to 10 lawyers about their meeting note workflow. If 7 say 'I'd pay $100/mo,' you've found your niche."

---

## Divergent Thinking: 20 Non-Meeting Use Cases (Same Tech Stack)

**Key insight**: Local-only architecture is the REAL differentiator. Most of these use cases involve data too sensitive for cloud — legal privilege, therapy, medical records, children's speech, creative IP. The tech stack enables products that CANNOT exist as cloud services.

### Top 10 (by market size x feasibility)

| # | Idea | User | Pain | Revenue |
|---|------|------|------|---------|
| 1 | **Ambient Clinical Documentation** | Primary care doctors | 2hrs charting per 1hr patient care. Existing cloud tools $1K+/mo, HIPAA risk | $200/mo (vs $1,500 for Nuance DAX) |
| 2 | **Therapy Session Companion** | Therapy patients | Forget 80% of session insights by the time they're home. Can't take notes during therapy | $15/mo. Therapists would recommend it |
| 3 | **ADHD Focus Guardian** | Adults with ADHD (WFH) | Losing track of what they were doing, forgetting verbal instructions, context-switch recovery | $15/mo. 48M Americans with ADHD |
| 4 | **Language Shadowing Coach** | Intermediate language learners | Plateau between "can read" and "can speak." Tutors cost $25-60/hr | $10/mo (replaces $25/hr tutor) |
| 5 | **Podcast Post-Production** | Independent podcasters (4M+ active) | 3-5hrs editing per episode. Can't afford $50-200/episode editors | $20/mo |
| 6 | **Courtroom Scribe** | Solo attorneys, small firms | Court reporters $300-800/day. Attorney-client privilege blocks cloud | $200/mo/seat |
| 7 | **Accessibility Live Captioner** | 48M Americans with hearing loss | No good solution for real-world captioning (restaurants, appointments) | $20/mo |
| 8 | **Voice Journal + Emotional Intel** | Anyone who wants to journal but finds writing tedious | Strong mental health evidence for journaling, most quit in weeks | $10-15/mo |
| 9 | **Lecture Time Machine** | University STEM students | Professor says key insight, you miss it while writing. Can't rewind live lectures | $10/mo during academic year |
| 10 | **Rubber Duck Debugger** | Solo/remote developers | "Explain it out loud" debugging but with AI that asks Socratic questions and has code context via MCP | $15/mo |

### The Weird-but-Compelling Ones
- **Dream Catcher** — Capture dreams by mumbling half-asleep. LLM reconstructs coherent narrative. Tracks recurring symbols over months. Lucid dreaming community would pay.
- **Parenting Black Box** — Track child's language development. "What was their first sentence?" Generates SLP-ready speech samples.
- **Argument Referee** — With mutual consent, search conversation history to resolve "you never said that" disputes. Relationship tool, not surveillance.
- **Verbal Agreement Tracker** — For freelancers: "On Jan 15 at 14:32, you said the logo redesign is included in the $5,000 scope." Prevents scope creep disputes.

### Platform Insight
> These aren't 20 separate products. They're 20 use cases for a **local audio intelligence platform**. The meeting app is just the first instantiation. The ONNX embeddings, LLM pipeline, RAG search, and MCP server are reusable across ALL of these.

---

## Divergent Thinking: Agentic Context & Orchestration

### What Context Different Agents Need From Meetings

| Agent Type | Context Needed | MCP Tool | Example |
|-----------|---------------|----------|---------|
| **Code agent** | Architecture decisions, trade-offs, who built what | `get_architecture_decisions(topic)` | Claude Code knows to use Postgres, not Redis, because Tuesday's meeting decided that |
| **Writing agent** | Stakeholder positions, effective framings, exact quotes | `get_stakeholder_positions(topic)`, `get_effective_framings()` | Follow-up email leads with "security audit readiness" angle because that resonated with the VP |
| **Planning agent** | Timeline commitments with confidence, blockers, dependencies | `get_timeline_commitments()`, `get_blockers_and_risks()` | Sprint plan notes Mike's commitment was "tentative" and flags his competing priority |
| **Research agent** | Unanswered questions, knowledge gaps, unverified assumptions | `get_unanswered_questions()`, `get_unverified_assumptions()` | Researches Q4 traffic projections because someone said "at current scale, sure" with uncertainty |
| **Communication agent** | Settled decisions, sensitive topics, political landscape | `get_settled_decisions(topic)`, `check_message_safety(draft)` | Warns you: "This decision was made by CTO-level authority. Sarah and Mike were strong advocates. Don't reopen casually." |
| **Personal agent** | Follow-ups owed, meeting prep, relationship maintenance | `get_daily_briefing()`, `get_my_follow_ups(urgency)` | "Before your 10AM with Sarah — you owe her the Figma review from last Thursday. She'll ask." |

### How Scribe Could BE an Agent (Not Just a Data Source)

**During the meeting (real-time sidebar):**
- Fact-check claims against past meetings: "Confirmed — March 3 design review agreed on GraphQL for public API"
- Surface relevant prior context as topics come up
- Running structured summary (decisions, action items, open questions) — live

**After the meeting (automated pipeline):**
```
Meeting Ends (auto-detected, 5 sec)
  → Quick summary notification
  → Full structured extraction (action items, decisions, commitments)
  → Agent orchestration:
      ├── Draft follow-up email (writing agent)
      ├── Create Jira tickets for action items (planning agent)
      ├── Update project docs (docs agent)
      ├── Send action item DMs via Slack (comms agent)
      └── Schedule follow-up if agreed (calendar agent)
  → Human review queue: "Approve, edit, or discard each"
```

**Between meetings (proactive intelligence):**
- "You committed to the API spec review 5 days ago. Not mentioned since."
- "Sarah was frustrated in yesterday's discussion (detected: interruptions, hedging, silence). Consider following up privately."
- "4 meetings about auth migration, no decision made. Same objection every time (compliance risk). Schedule a decision-forcing meeting?"

### Novel Context Types

| Context Type | What It Captures | How Agents Use It |
|-------------|-----------------|-------------------|
| **Emotional context** | Tension moments, enthusiasm, energy trajectory, engagement by person | Communication agent adjusts tone; planning agent flags "low conviction" decisions |
| **Power dynamics** | Who defers to whom, who decides, interruption patterns, influence map | "Get Sarah on board first — VP follows her recommendation 80% of the time" |
| **Knowledge graph** | What each person knows (demonstrated expertise, not LinkedIn) | Code agent suggests "Ask Jake about K8s — he's the expert based on 7 meetings" |
| **Decision quality** | Time spent, perspectives heard, evidence cited, dissent handled? | "Microservices decision scored 35/100 — only 2 of 6 people spoke. Revisit?" |
| **Implicit context** | Hedging, strategic omissions, proxy language, subtext | "PM said 'tracking to schedule' but hedged 3 times and spoke 30% faster. Likely unacknowledged risk." |

### The Architectural Vision: 6 Layers

```
Layer 0: Audio Capture (HAVE TODAY)
Layer 1: Document Intelligence — transcripts, summaries, FTS5 (HAVE TODAY)
Layer 2: Structured Extraction — action items, decisions, commitments (BUILD NEXT)
Layer 3: People Intelligence — profiles, relationship graph, expertise map (THE DIFFERENTIATOR)
Layer 4: Meta-Intelligence — emotional context, power dynamics, decision quality (THE MOAT)
Layer 5: Agentic Behaviors — real-time assist, post-meeting automation, proactive nudges (THE EXPERIENCE)
Layer 6: Ambient Intelligence — always-on, multiple capture modes, running professional context model (THE VISION)
```

Each layer builds on the one below. You can't have people intelligence without speaker diarization. You can't have decision quality without emotional context.

### The Key Insight

> Every meeting tool treats meetings as **documents to be summarized**. Scribe's opportunity is to treat meetings as **events that update a living model of your professional world** — and make that model available to any agent.

### MCP Dual-Role (Unique Architecture)

Scribe is both:
- **MCP Server** — exposes meeting intelligence to Claude Code, other agents
- **MCP Client** — after meetings, calls Jira MCP, Slack MCP, Calendar MCP to execute follow-up actions

No existing tool does both. This is the meta-agent architecture.

---

## Research Synthesis: What's Converging

Across 7 divergent perspectives, several themes emerged repeatedly:

### Points of Agreement (High Confidence)
1. **The 5-second notification moment is the product** — meeting ends, summary appears, zero effort. This is the "filter" that drives adoption.
2. **Structured extraction (decisions, commitments, action items) is the highest-leverage next step** — not people profiles (too much cold-start risk), not more search (already good enough). Extract structured entities from every meeting.
3. **The MCP server is distribution, not revenue** — keep it free, make it the best. It's how developers discover Scribe.
4. **Local-only is the real moat** — not as "privacy feature" but as the enabler of use cases that CANNOT exist in the cloud (legal, therapy, medical, sensitive relationship analysis).
5. **Speaker diarization is the critical unlock** — blocks people intelligence, which blocks the entire differentiation thesis. But it's high-risk engineering.

### Points of Disagreement (Needs Decision)
1. **People intelligence: build now vs later?** Contrarian says cold-start kills it. Business strategist says it's the moat. Both are right — need a version that provides value from meeting #1.
2. **Target user: developers vs consultants vs legal?** Each has different distribution and revenue profiles. Can't serve all three well.
3. **Meeting tool vs platform?** Is this a meeting app that exposes MCP, or an audio intelligence platform where meetings are one mode?
4. **Build depth or breadth?** 20 MCP tools for meetings, or 4 great tools across meetings + voice memos + learning capture?

---

## Deep Dive: Context-to-Agents Research

### The Ecosystem Gap (Hard Data)
- **20,558 total MCP servers** exist (glama.ai directory)
- **Only ~14 for meetings** — all with single-digit GitHub stars
- **Workplace & Productivity**: 599 servers (2.9% of ecosystem). Developer Tools: 7,312 (35.6%)
- **Nobody owns "professional context layer for agents"** as a category yet

### What Works Today vs What's Missing

| Layer | Status | Who |
|-------|--------|-----|
| Meeting capture | Mature | Granola, Fathom, Fireflies, Otter |
| Meeting → structured data | Moderate | Each tool's own summaries |
| Meeting → MCP | Very early | ~14 projects, all fragile |
| Cross-source synthesis | Experimental | DevsContext (1 star) |
| Enterprise context | Growing | Dust.tt ($125M funded) |
| Developer workflow memory | Growing | Pieces.dev (OS-level capture) |

### Where Granola Specifically Fails as Agent Context Source

**Real user pain (from GitHub issues, HN threads):**

1. **Cache breaks constantly**: Granola has gone through cache v3→v4→v5→v6 in under a year. Every community MCP server breaks silently on app updates. "My plugin was working fine until today" (Obsidian plugin user). Deprecation notice on granola-mcp-daily-script: "Granola removed transcript data from local cache."

2. **Official API is crippled**: Only 2 endpoints (list notes, get single note). **No search.** No write-back. No webhooks. Rate limited. Free users: last 30 days only. This is why 23 community repos exist — people building the same "search Granola with embeddings" over and over.

3. **Transcripts disappear**: Moved server-side, local cache no longer has them. `get_meeting_transcript` returns "No transcript available" even when visible in Granola's UI. Community tools extract auth tokens from unencrypted `supabase.json` as workaround.

4. **Speaker diarization is binary**: Official API returns "microphone" vs "speaker" — no names. HN user: "The inability to tell who said what is a show stopper." Another tested 20-person meeting: "entire conversation under a single speaker, in a single paragraph."

5. **No cross-meeting intelligence**: No search, no semantic queries, no "what did Sarah say about X across all meetings." Community built LanceDB vector indexes and BM25 search engines to fill this gap.

6. **Privacy incidents**: User "extremely disturbed" that Granola made 1:1 notes folders public to entire organization. Auth tokens stored unencrypted locally.

7. **Stale data in MCP**: Cache loaded once at startup. Meetings added after MCP server starts are invisible. "If you have a meeting at 10am and ask Claude at 2pm, the MCP server may not see it."

**What this means for us**: Every Granola pain point is a Scribe advantage:
- We own the database (SQLite, stable schema, never breaks)
- We have full search (FTS5 + vector, already built)
- Transcripts always available locally (never moved server-side)
- Local-first by architecture (no privacy incidents possible)
- Real-time data access (app and MCP read same live database)

### Critical Insight: Less Context Is Better
Cursor's research on "Dynamic Context Discovery": providing **fewer details up front** improves agent performance. Lazy MCP loading reduced tokens by **46.9%**. Raw meeting transcripts would be counterproductive. Agents need **selective, relevant** context retrieved on demand.

### The DevsContext Model (Most Instructive Architecture)
DevsContext (github.com/Pro0f/devscontext) synthesizes: Jira + Fireflies + Slack + Gmail + local docs → structured context blocks with **attribution and rationale**:
- "In the March 15 standup, Sarah decided to use event sourcing for billing because of audit requirements (BILL-234)"
- Not just "use event sourcing" — the WHO, WHEN, WHY, and TICKET REFERENCE

### Agent With Context vs Without

| Without Meeting Context | With Meeting Context |
|------------------------|---------------------|
| Knows code but not WHY decisions were made | "Team chose Postgres over DynamoDB due to cost at scale (March 12 arch review)" |
| Treats ambiguous requirements as greenfield | Resolves by referencing the decision discussion |
| Has to ask developer to re-explain | Already knows priorities from standup/all-hands |
| Generic solutions | Solutions that match team conventions discussed in meetings |

### Key User Pain Points (Real Voices)
- **Claude Code issue #41283**: "Memory identity is derived from filesystem path, causing orphaned memories" — people want context that follows THEM, not project directories
- **Claude Code issue #15222**: "Decision History Tracking with DECISIONS.md" — users explicitly want AI to know about decisions from meetings
- **Limitless acquired by Meta (Dec 2025)**: "Capture everything ambient" failed as standalone business. Meeting-specific, consent-based approaches more viable.
- **Pieces.dev captures what you DO but not what you SAY** — meeting/conversation context is the gap

### Why Nobody Has Built the "Professional Context Layer" Yet
1. **Capture is hard** — meetings, screen, email, Slack each need different tech and different privacy models
2. **Signal extraction is harder** — raw transcripts are useless. Value is in extracted decisions, action items, attribution
3. **Privacy is the third rail** — enterprises won't let one tool see Slack + email + meetings + code simultaneously
4. **Business model tension** — nobody wants to be "just infrastructure." Meeting tools want to own meetings, knowledge tools want to own the wiki
5. **MCP is too new** — ecosystem is still "one server per source." The aggregation layer hasn't been built

### The Viable Architecture (Emerging Consensus)
Not a single monolithic context layer. Instead:
1. **Specialized MCP servers per domain** (meetings, code, tasks, docs) — each does structured extraction well
2. **The AI agent as the orchestrator** — queries multiple MCP servers, synthesizes
3. **Meeting data as first-class structured data** — not raw transcripts, but decisions, action items, attribution, linked across meetings

### The Context Layer Landscape

| Product | What It Provides | Meeting Data? | MCP? | For Individuals? |
|---------|-----------------|--------------|------|-----------------|
| **Dust.tt** | Enterprise multi-source (17+ connectors) | Via Gong/Slack only (no capture) | Yes (bidirectional) | No ($$$) |
| **Pieces.dev** | Developer activity (IDE, browser, 9mo memory) | No | Yes (`ask_pieces_ltm`) | Yes |
| **Limitless/Rewind** | Screen + audio capture | Yes (but locked in, now dead) | No | N/A (acquired by Meta) |
| **Notion MCP** | Workspace pages/databases | Manual notes only | Yes (22 tools) | Yes |
| **Obsidian MCP** | Local vault notes | Manual notes only | Yes (community) | Yes |
| **Granola** | Meeting notes | Yes (primary) | Announced, thin | Yes |
| **Scribe (us)** | Meeting transcripts + structured data | Yes (primary, local-first) | Yes (4 tools, building 10+) | Yes |

### Completely Novel Idea Found: Auto-Generate CLAUDE.md from Meetings
Zero implementations exist. After meetings, automatically update project context files with new decisions, ownership changes, and technical constraints discussed. E.g., meeting decides "use Postgres" → CLAUDE.md gets `## Architecture Decisions\n- Database: PostgreSQL (decided 2026-03-25, rationale: cost at scale)`. Every coding session from then on, Claude knows.

---

---

## Deep Dive: GTM/Sales Agent Opportunity

### The Sales MCP Stack Today

| Layer | MCP Status | Key Players |
|-------|-----------|-------------|
| **CRM** | Mature | Salesforce (official MCP), HubSpot (118 stars community), Pipedrive |
| **Sales engagement** | Emerging | Outreach.io (76 tools, 1 star) |
| **Prospecting** | Emerging | LinkedIn (198 stars), Apollo.io |
| **Conversation intelligence** | **Almost nothing** | Gong MCP (29 stars, transcript only), Fireflies MCP (0 stars) |
| **Revenue intelligence** | **Nothing** | No Clari, no 6sense, no Bombora |
| **Data enrichment** | **Nothing** | No ZoomInfo, no Clay, no Clearbit |

### The Critical Gap
**Nobody connects structured meeting intelligence to the sales agent stack via MCP.**

Existing Gong MCP servers dump raw transcripts. They do NOT provide:
- "What objections did the prospect raise?"
- "What competitors were mentioned?"
- "What was the agreed next step?"
- "Who are the decision-makers identified in calls?"
- "How has buyer sentiment changed over 3 calls?"

### Why This Matters for GTM Agents
Autonomous sales agents (11x Alice, Artisan Ava) need conversation context but build proprietary integrations. None use MCP today. An MCP standard for structured meeting intelligence would let ANY agent tap in.

The meeting intelligence MCP sits between:
- **CRM** (deal state) — Salesforce MCP already exists
- **Sales engagement** (outreach state) — Outreach MCP exists
- **Conversation state** — **NOBODY HAS THIS** via MCP

### Our Position
We don't build the GTM agent. We build the **conversation intelligence layer** that any GTM agent queries via MCP. Same structured data (decisions, commitments, people, topics) serves engineering agents AND sales agents. The data model is horizontal; the use case is determined by the calling agent.

### The GTM Stack Map (From Research)
```
[AI Agents: 11x, Artisan, Regie] → use → [Enrichment: ZoomInfo, Clay, Apollo]
                                                    ↓
[Outreach: Outreach, Salesloft] ← sequences         ↓
                                                    ↓
[Meetings happen] → captured by → [Conv Intelligence: Gong, Chorus, Fathom]
                                                    ↓ (some sync, mostly trapped)
[CRM: Salesforce, HubSpot] ← manual + partial sync  ↓
                                                    ↓
[Revenue Intelligence: Gong, Clari] → forecasting
```

**The broken link**: AI agents have enrichment data (who the prospect IS) but are blind to conversation data (what was DISCUSSED). Gong captures it but keeps it walled. The richest signal about a customer relationship is the most poorly distributed data in the stack.

### Key Stat
- Gong has 250+ integrations but its Zapier connector only triggers on "New Call" with limited data
- **No conversation intelligence tool has an official MCP server**
- Clay mentions connecting Claygent to "any MCP server — from Salesforce, to Gong" but the Gong MCP doesn't exist yet
- 11x, Artisan, Regie: **none use MCP today** — all proprietary integrations

### Sales-Specific MCP Tools That Would Be Unique
- `get_buyer_signals(prospect)` — Extract interest/objection/sentiment from calls
- `get_competitive_mentions(competitor_name)` — What prospects say about competitors
- `get_deal_context(account_name)` — All meetings with an account, structured
- `get_next_steps(meeting_id)` — Committed follow-ups from a sales call
- `get_stakeholder_map(account)` — Who's the champion, who's the blocker, from conversation evidence

---

## Final Synthesis: What To Build

### The Thesis (Validated)
Scribe is the **voice layer** in a multi-source context graph. It captures what was SAID — the most candid, nuanced, contextual form of professional knowledge. Written docs are formal. Slack is fragmented. What someone tells you face-to-face is where the real wisdom lives. Scribe captures that and makes it available to any agent via MCP.

### What Makes This Different From Granola
| Dimension | Granola | Scribe |
|-----------|---------|--------|
| Data stability | Cache breaks v3→v4→v5→v6 | SQLite, stable schema, we own it |
| API/MCP | 2 endpoints, no search | Hybrid RAG search + structured tools |
| Transcripts | Disappear to cloud, inaccessible | Always local, always available |
| Speaker ID | Binary ("microphone" vs "speaker") | Building named diarization |
| Search | None in official API | FTS5 + vector hybrid, already built |
| Privacy | Made 1:1s public to org (user report) | Local-first by architecture |
| Structured data | Raw notes only | Extracting decisions, commitments, action items |
| Agent context | Thin pass-through | Selective, relevant context on demand |

### Recommended Next Actions (Prioritized)

**Tier 1: Ship This Week**
1. **Structured extraction pipeline** — After each meeting, LLM extracts `{decisions, action_items, commitments, open_questions}` as queryable JSON in new DB tables
2. **Enhance MCP to 8 tools** — `get_action_items`, `get_decisions`, `get_meeting_by_date`, `search_by_topic`, `find_decision`, `prepare_for_meeting`, `get_meeting_context`, `who_said_what`
3. **5-second notification** — Meeting ends → auto-summary notification. The "Instagram filter" moment.

**Tier 2: Ship This Month**
4. **Speaker diarization prototype** — ONNX voice embeddings to distinguish speakers (even as "Speaker 1/2/3")
5. **Auto-generate CLAUDE.md** — After meetings with architecture decisions, automatically update project context. Completely novel, zero implementations exist.
6. **Ship MCP as standalone installable** — Homebrew formula or npm package. Get listed on MCP directories.

**Tier 3: Build the Moat**
7. **People data model + extraction** — Persistent profiles, relationship tracking, cross-meeting linking
8. **Calendar integration** — Enriches people data, enables meeting prep
9. **Proactive intelligence** — Pre-meeting briefings, commitment decay alerts, decision drift detection

### The One-Liner
> **Scribe: The voice layer for AI agents.** Your meetings become structured, searchable context that every AI tool can query.

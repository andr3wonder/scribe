#!/usr/bin/env python3
"""Import all Granola meetings into Meetily's database.

Reads Granola's local cache file and imports each meeting with:
- Title
- Date (created_at)
- Transcript segments (with timestamps and speaker info)
- Auto-triggers summary generation for each imported meeting

Usage:
    python3 scripts/import_granola.py [--dry-run] [--limit N]
"""

import json
import os
import sqlite3
import sys
import uuid
from datetime import datetime

GRANOLA_CACHE = os.path.expanduser(
    "~/Library/Application Support/Granola/cache-v6.json"
)
MEETILY_DB = os.path.expanduser(
    "~/Library/Application Support/com.meetily.ai/meeting_minutes.sqlite"
)


def load_granola_data():
    """Load and parse Granola's cache file."""
    with open(GRANOLA_CACHE) as f:
        data = json.load(f)
    state = data["cache"]["state"]
    documents = state.get("documents", {})
    transcripts = state.get("transcripts", {})
    return documents, transcripts


def get_existing_meeting_titles(conn):
    """Get set of existing meeting titles to avoid duplicates."""
    cursor = conn.execute("SELECT title FROM meetings")
    return {row[0] for row in cursor.fetchall()}


def import_meeting(conn, doc, transcript_segments):
    """Import a single Granola meeting into Meetily."""
    meeting_id = f"meeting-{uuid.uuid4()}"
    title = doc.get("title", "Imported from Granola")
    created_at = doc.get("created_at", datetime.now().isoformat())
    updated_at = doc.get("updated_at", created_at)

    # Insert meeting (schema: id, title, created_at, updated_at, folder_path)
    conn.execute(
        """INSERT INTO meetings (id, title, created_at, updated_at)
           VALUES (?, ?, ?, ?)""",
        (meeting_id, title, created_at, updated_at),
    )

    # Insert transcript segments
    # Schema: id, meeting_id, transcript, timestamp, summary, action_items, key_points,
    #         audio_start_time, audio_end_time, duration, speaker
    for i, seg in enumerate(transcript_segments):
        text = seg.get("text", "").strip()
        if not text:
            continue

        start_ts = seg.get("start_timestamp", "")
        end_ts = seg.get("end_timestamp", "")
        source = seg.get("source", "unknown")

        # Calculate audio timestamps (seconds from meeting start)
        audio_start = 0.0
        audio_end = 0.0
        duration = 0.0
        try:
            if start_ts and created_at:
                start_dt = datetime.fromisoformat(start_ts.replace("Z", "+00:00"))
                created_dt = datetime.fromisoformat(created_at.replace("Z", "+00:00"))
                audio_start = max(0, (start_dt - created_dt).total_seconds())
                if end_ts:
                    end_dt = datetime.fromisoformat(end_ts.replace("Z", "+00:00"))
                    audio_end = max(0, (end_dt - created_dt).total_seconds())
                    duration = audio_end - audio_start
        except (ValueError, TypeError):
            pass

        conn.execute(
            """INSERT INTO transcripts
               (id, meeting_id, transcript, timestamp, audio_start_time, audio_end_time, duration, speaker)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?)""",
            (
                str(uuid.uuid4()),
                meeting_id,
                text,
                start_ts or created_at,
                audio_start,
                audio_end,
                duration,
                "mic" if source == "microphone" else "system",
            ),
        )

    # If Granola already has a summary/notes, import that too
    notes_md = doc.get("notes_markdown", "")
    summary_text = doc.get("summary", "")
    if notes_md or summary_text:
        summary_content = notes_md or summary_text
        process_id = str(uuid.uuid4())
        result_json = json.dumps({
            "markdown": summary_content,
            "meeting_name": title,
        })
        conn.execute(
            """INSERT OR IGNORE INTO summary_processes
               (meeting_id, status, result, created_at, updated_at)
               VALUES (?, 'completed', ?, ?, ?)""",
            (meeting_id, result_json, created_at, updated_at),
        )

    return meeting_id, title, len(transcript_segments)


def main():
    dry_run = "--dry-run" in sys.argv
    limit = None
    for i, arg in enumerate(sys.argv):
        if arg == "--limit" and i + 1 < len(sys.argv):
            limit = int(sys.argv[i + 1])

    if not os.path.exists(GRANOLA_CACHE):
        print(f"Granola cache not found at: {GRANOLA_CACHE}")
        sys.exit(1)

    if not os.path.exists(MEETILY_DB):
        print(f"Meetily database not found at: {MEETILY_DB}")
        print("Please launch Meetily at least once first.")
        sys.exit(1)

    print("Loading Granola data...")
    documents, transcripts = load_granola_data()
    print(f"Found {len(documents)} meetings, {len(transcripts)} transcript sets")

    conn = sqlite3.connect(MEETILY_DB)
    existing_titles = get_existing_meeting_titles(conn)

    imported = 0
    skipped = 0

    # Sort by created_at
    sorted_docs = sorted(
        documents.values(),
        key=lambda d: d.get("created_at", ""),
    )

    for doc in sorted_docs:
        if limit and imported >= limit:
            break

        title = doc.get("title", "")
        doc_id = doc.get("id", "")
        created = doc.get("created_at", "unknown")
        deleted = doc.get("deleted_at")

        # Skip deleted meetings
        if deleted:
            continue

        # Skip duplicates
        if title in existing_titles:
            print(f"  SKIP (duplicate): {title}")
            skipped += 1
            continue

        # Get transcript for this document
        segments = transcripts.get(doc_id, [])

        if dry_run:
            print(f"  DRY RUN: {title} ({created[:10]}) - {len(segments)} segments")
            imported += 1
            continue

        try:
            meeting_id, title, seg_count = import_meeting(conn, doc, segments)
            conn.commit()
            imported += 1
            print(f"  IMPORTED: {title} ({created[:10]}) - {seg_count} segments → {meeting_id}")
        except Exception as e:
            print(f"  ERROR: {title}: {e}")
            conn.rollback()

    conn.close()

    print(f"\nDone! Imported: {imported}, Skipped: {skipped}")
    if dry_run:
        print("(dry run - no changes made)")
    else:
        print("Restart Meetily to see imported meetings.")


if __name__ == "__main__":
    main()

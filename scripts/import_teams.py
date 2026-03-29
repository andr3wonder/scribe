#!/usr/bin/env python3
"""Import Microsoft Teams call transcripts into Meetily.

Reads .txt files exported from Teams (call-transcript--*.txt format).
Format:
  Title
  Speaker Name
  Recorded on <date> via Microsoft Teams, <duration>
  Participants section
  Transcript section with: timestamp | Speaker\ntext

Usage:
    python3 scripts/import_teams.py <file1.txt> [file2.txt ...]
    python3 scripts/import_teams.py ~/Downloads/call-transcript--*.txt
"""

import json
import os
import re
import sqlite3
import sys
import uuid
from datetime import datetime, timedelta

MEETILY_DB = os.path.expanduser(
    "~/Library/Application Support/com.meetily.ai/meeting_minutes.sqlite"
)


def parse_teams_transcript(filepath):
    """Parse a Teams transcript text file."""
    with open(filepath, encoding="utf-8") as f:
        content = f.read()

    lines = content.strip().split("\n")

    # Title is first non-empty line
    title = ""
    for line in lines:
        if line.strip():
            title = line.strip()
            break

    # Find recording date
    created_at = datetime.now().isoformat() + "Z"
    duration_minutes = 0
    for line in lines:
        match = re.search(r"Recorded on (.+?) via Microsoft Teams(?:, (\d+)m)?", line)
        if match:
            date_str = match.group(1).strip()
            try:
                dt = datetime.strptime(date_str, "%b %d, %Y")
                created_at = dt.isoformat() + "Z"
            except ValueError:
                try:
                    dt = datetime.strptime(date_str, "%B %d, %Y")
                    created_at = dt.isoformat() + "Z"
                except ValueError:
                    pass
            if match.group(2):
                duration_minutes = int(match.group(2))
            break

    # Parse participants
    participants = []
    in_participants = False
    for line in lines:
        if line.strip() == "Participants":
            in_participants = True
            continue
        if line.strip() == "Transcript":
            in_participants = False
            continue
        if in_participants and line.strip() and not line.strip().endswith(":"):
            # Lines like "Vanessa Lopes, LMS Strategic Account Director"
            name = line.strip().split(",")[0].strip()
            if name and len(name) > 1:
                participants.append(name)

    # Parse transcript segments
    # Format: "0:00 | Speaker\ntext" or "0:00:00 | Speaker\ntext"
    segments = []
    in_transcript = False
    current_speaker = ""
    current_text = ""
    current_timestamp = ""

    for line in lines:
        if line.strip() == "Transcript":
            in_transcript = True
            continue
        if not in_transcript:
            continue

        # Check for timestamp line: "0:00 | Speaker" or "0:00:00 | Speaker"
        ts_match = re.match(r"^(\d+:\d+(?::\d+)?)\s*\|\s*(.+)$", line.strip())
        if ts_match:
            # Save previous segment
            if current_text.strip():
                segments.append({
                    "timestamp": current_timestamp,
                    "speaker": current_speaker,
                    "text": current_text.strip(),
                })
            current_timestamp = ts_match.group(1)
            current_speaker = ts_match.group(2).strip()
            current_text = ""
        elif line.strip():
            current_text += " " + line.strip() if current_text else line.strip()

    # Don't forget the last segment
    if current_text.strip():
        segments.append({
            "timestamp": current_timestamp,
            "speaker": current_speaker,
            "text": current_text.strip(),
        })

    return {
        "title": title,
        "created_at": created_at,
        "duration_minutes": duration_minutes,
        "participants": participants,
        "segments": segments,
    }


def timestamp_to_seconds(ts):
    """Convert '0:00' or '0:00:00' to seconds."""
    parts = ts.split(":")
    if len(parts) == 3:
        return int(parts[0]) * 3600 + int(parts[1]) * 60 + int(parts[2])
    elif len(parts) == 2:
        return int(parts[0]) * 60 + int(parts[1])
    return 0


def import_to_meetily(conn, parsed):
    """Import a parsed Teams transcript into Meetily's database."""
    meeting_id = f"meeting-{uuid.uuid4()}"
    title = parsed["title"]
    created_at = parsed["created_at"]
    updated_at = created_at

    # Check for duplicates
    cursor = conn.execute("SELECT id FROM meetings WHERE title = ?", (title,))
    if cursor.fetchone():
        return None, f"SKIP (duplicate): {title}"

    # Insert meeting
    conn.execute(
        "INSERT INTO meetings (id, title, created_at, updated_at) VALUES (?, ?, ?, ?)",
        (meeting_id, title, created_at, updated_at),
    )

    # Insert transcript segments
    for i, seg in enumerate(parsed["segments"]):
        audio_start = timestamp_to_seconds(seg["timestamp"])
        # Estimate end time from next segment or add 10s
        audio_end = audio_start + 10
        if i + 1 < len(parsed["segments"]):
            audio_end = timestamp_to_seconds(parsed["segments"][i + 1]["timestamp"])

        speaker_text = f"[{seg['speaker']}] {seg['text']}"

        conn.execute(
            """INSERT INTO transcripts
               (id, meeting_id, transcript, timestamp, audio_start_time, audio_end_time, duration, speaker)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?)""",
            (
                str(uuid.uuid4()),
                meeting_id,
                speaker_text,
                created_at,
                audio_start,
                audio_end,
                audio_end - audio_start,
                "mic" if (parsed["participants"] and seg["speaker"] == parsed["participants"][0]) else "system",
            ),
        )

    conn.commit()
    return meeting_id, f"IMPORTED: {title} ({created_at[:10]}) - {len(parsed['segments'])} segments"


def main():
    if len(sys.argv) < 2:
        print("Usage: python3 scripts/import_teams.py <transcript1.txt> [transcript2.txt ...]")
        print("       python3 scripts/import_teams.py ~/Downloads/call-transcript--*.txt")
        sys.exit(1)

    files = sys.argv[1:]

    if not os.path.exists(MEETILY_DB):
        print(f"Meetily database not found at: {MEETILY_DB}")
        sys.exit(1)

    conn = sqlite3.connect(MEETILY_DB)
    imported = 0

    for filepath in files:
        if not os.path.exists(filepath):
            print(f"  File not found: {filepath}")
            continue

        try:
            parsed = parse_teams_transcript(filepath)
            meeting_id, msg = import_to_meetily(conn, parsed)
            print(f"  {msg}")
            if meeting_id:
                imported += 1
        except Exception as e:
            print(f"  ERROR ({os.path.basename(filepath)}): {e}")

    conn.close()
    print(f"\nDone! Imported {imported} Teams transcript(s).")
    if imported > 0:
        print("Restart Meetily to see imported meetings.")


if __name__ == "__main__":
    main()

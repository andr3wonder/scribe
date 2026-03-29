#!/usr/bin/env python3
"""
Fetch Microsoft Teams meeting transcripts via Graph API.
Uses device code flow for authentication (no app registration needed for personal use).

Usage:
    python3 scripts/fetch_teams_transcripts.py [--days 30] [--import]

First run will prompt you to authenticate via browser.
Subsequent runs use cached tokens.
"""

import json
import os
import sys
import sqlite3
import uuid
import webbrowser
from datetime import datetime, timedelta, timezone

try:
    import requests
except ImportError:
    print("Installing requests...")
    os.system(f"{sys.executable} -m pip install requests --quiet")
    import requests

# Microsoft Graph API endpoints
GRAPH_BASE = "https://graph.microsoft.com/v1.0"
# Use the well-known "Microsoft Teams" client ID for device code flow
# This is a public client that can access Teams data
CLIENT_ID = "1fec8e78-bce4-4aaf-ab1b-5451cc387264"  # Teams desktop client
TENANT_ID = "organizations"  # Works for any org account
SCOPES = "OnlineMeetings.Read OnlineMeetingTranscript.Read.All"

TOKEN_CACHE = os.path.expanduser("~/.cache/meetily-teams-token.json")
MEETILY_DB = os.path.expanduser("~/Library/Application Support/com.meetily.ai/meeting_minutes.sqlite")


def get_token():
    """Get an access token via device code flow or cached token."""
    # Check cache
    if os.path.exists(TOKEN_CACHE):
        with open(TOKEN_CACHE) as f:
            cached = json.load(f)
        # Check if token is still valid (with 5 min buffer)
        expires = datetime.fromisoformat(cached.get("expires_at", "2000-01-01"))
        if expires > datetime.now(timezone.utc) + timedelta(minutes=5):
            return cached["access_token"]
        # Try refresh
        if "refresh_token" in cached:
            token = refresh_token(cached["refresh_token"])
            if token:
                return token

    # Device code flow
    return device_code_auth()


def device_code_auth():
    """Authenticate using device code flow (user approves in browser)."""
    print("\n🔐 Microsoft Teams Authentication Required")
    print("=" * 50)

    # Request device code
    resp = requests.post(
        f"https://login.microsoftonline.com/{TENANT_ID}/oauth2/v2.0/devicecode",
        data={
            "client_id": CLIENT_ID,
            "scope": SCOPES,
        },
    )
    if resp.status_code != 200:
        print(f"Failed to get device code: {resp.text}")
        sys.exit(1)

    data = resp.json()
    user_code = data["user_code"]
    verification_uri = data["verification_uri"]
    device_code = data["device_code"]
    interval = data.get("interval", 5)

    print(f"\n1. Go to: {verification_uri}")
    print(f"2. Enter code: {user_code}")
    print(f"\nOpening browser...")
    webbrowser.open(verification_uri)

    # Poll for token
    print("Waiting for you to approve...")
    import time
    while True:
        time.sleep(interval)
        resp = requests.post(
            f"https://login.microsoftonline.com/{TENANT_ID}/oauth2/v2.0/token",
            data={
                "client_id": CLIENT_ID,
                "device_code": device_code,
                "grant_type": "urn:ietf:params:oauth:grant-type:device_code",
            },
        )
        result = resp.json()
        if "access_token" in result:
            save_token(result)
            print("✅ Authenticated successfully!")
            return result["access_token"]
        elif result.get("error") == "authorization_pending":
            continue
        elif result.get("error") == "expired_token":
            print("❌ Device code expired. Please try again.")
            sys.exit(1)
        else:
            print(f"Auth error: {result.get('error_description', result)}")
            sys.exit(1)


def refresh_token(refresh_tok):
    """Refresh an expired access token."""
    resp = requests.post(
        f"https://login.microsoftonline.com/{TENANT_ID}/oauth2/v2.0/token",
        data={
            "client_id": CLIENT_ID,
            "refresh_token": refresh_tok,
            "grant_type": "refresh_token",
            "scope": SCOPES,
        },
    )
    if resp.status_code == 200:
        result = resp.json()
        save_token(result)
        return result["access_token"]
    return None


def save_token(token_data):
    """Cache token to disk."""
    os.makedirs(os.path.dirname(TOKEN_CACHE), exist_ok=True)
    expires_in = token_data.get("expires_in", 3600)
    token_data["expires_at"] = (datetime.now(timezone.utc) + timedelta(seconds=expires_in)).isoformat()
    with open(TOKEN_CACHE, "w") as f:
        json.dump(token_data, f, indent=2)


def get_meetings(token, days=30):
    """Get recent online meetings."""
    headers = {"Authorization": f"Bearer {token}"}

    # Get meetings from calendar events that have online meeting info
    start = (datetime.now(timezone.utc) - timedelta(days=days)).isoformat()
    end = datetime.now(timezone.utc).isoformat()

    resp = requests.get(
        f"{GRAPH_BASE}/me/calendarView",
        headers=headers,
        params={
            "startDateTime": start,
            "endDateTime": end,
            "$filter": "isOnlineMeeting eq true",
            "$select": "subject,start,end,onlineMeeting,organizer,attendees",
            "$orderby": "start/dateTime desc",
            "$top": 50,
        },
    )

    if resp.status_code != 200:
        print(f"Failed to get meetings: {resp.status_code} {resp.text[:200]}")
        return []

    return resp.json().get("value", [])


def get_transcript(token, meeting_id):
    """Get transcript for a specific online meeting."""
    headers = {"Authorization": f"Bearer {token}"}

    # List transcripts for this meeting
    resp = requests.get(
        f"{GRAPH_BASE}/me/onlineMeetings/{meeting_id}/transcripts",
        headers=headers,
    )

    if resp.status_code != 200:
        return None

    transcripts = resp.json().get("value", [])
    if not transcripts:
        return None

    # Get the first transcript's content
    transcript_id = transcripts[0]["id"]
    resp = requests.get(
        f"{GRAPH_BASE}/me/onlineMeetings/{meeting_id}/transcripts/{transcript_id}/content",
        headers=headers,
        params={"$format": "text/vtt"},
    )

    if resp.status_code == 200:
        return resp.text
    return None


def parse_vtt(vtt_content):
    """Parse VTT transcript into segments with speaker info."""
    import re
    segments = []
    lines = vtt_content.strip().split("\n")
    i = 0
    while i < len(lines):
        line = lines[i].strip()
        # Look for timestamp lines: 00:00:00.000 --> 00:00:05.000
        ts_match = re.match(r"(\d+:\d+:\d+\.\d+)\s*-->\s*(\d+:\d+:\d+\.\d+)", line)
        if ts_match:
            start_ts = ts_match.group(1)
            end_ts = ts_match.group(2)
            # Next lines are the text (possibly with speaker tag)
            i += 1
            text_lines = []
            while i < len(lines) and lines[i].strip():
                text_lines.append(lines[i].strip())
                i += 1
            text = " ".join(text_lines)
            # Extract speaker if present: <v Speaker Name>text</v>
            speaker_match = re.match(r"<v\s+(.+?)>(.+?)(?:</v>)?$", text)
            if speaker_match:
                speaker = speaker_match.group(1).strip()
                text = speaker_match.group(2).strip()
            else:
                speaker = ""

            if text:
                segments.append({
                    "start": start_ts,
                    "end": end_ts,
                    "speaker": speaker,
                    "text": text,
                })
        i += 1
    return segments


def vtt_ts_to_seconds(ts):
    """Convert VTT timestamp to seconds."""
    parts = ts.split(":")
    h, m = int(parts[0]), int(parts[1])
    s = float(parts[2])
    return h * 3600 + m * 60 + s


def import_to_meetily(conn, title, date_str, segments):
    """Import a meeting with transcript into Meetily's database."""
    # Check for duplicates
    cursor = conn.execute("SELECT id FROM meetings WHERE title = ?", (title,))
    if cursor.fetchone():
        return None, f"SKIP (duplicate): {title}"

    meeting_id = f"meeting-{uuid.uuid4()}"
    created_at = date_str

    conn.execute(
        "INSERT INTO meetings (id, title, created_at, updated_at) VALUES (?, ?, ?, ?)",
        (meeting_id, title, created_at, created_at),
    )

    for i, seg in enumerate(segments):
        audio_start = vtt_ts_to_seconds(seg["start"])
        audio_end = vtt_ts_to_seconds(seg["end"])
        text = f"[{seg['speaker']}] {seg['text']}" if seg["speaker"] else seg["text"]

        conn.execute(
            """INSERT INTO transcripts (id, meeting_id, transcript, timestamp, audio_start_time, audio_end_time, duration, speaker)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?)""",
            (
                str(uuid.uuid4()),
                meeting_id,
                text,
                created_at,
                audio_start,
                audio_end,
                audio_end - audio_start,
                "mic" if i == 0 else "system",  # Will be overridden by speaker name in text
            ),
        )

    conn.commit()
    return meeting_id, f"IMPORTED: {title} ({date_str[:10]}) - {len(segments)} segments"


def main():
    days = 30
    do_import = False

    for i, arg in enumerate(sys.argv[1:]):
        if arg == "--days" and i + 1 < len(sys.argv) - 1:
            days = int(sys.argv[i + 2])
        if arg == "--import":
            do_import = True

    print(f"Fetching Teams meetings from the last {days} days...")

    token = get_token()
    meetings = get_meetings(token, days)

    print(f"Found {len(meetings)} online meetings")

    if not meetings:
        print("No meetings found. Try --days 60 for a wider range.")
        return

    conn = sqlite3.connect(MEETILY_DB) if do_import else None
    imported = 0
    with_transcript = 0

    for meeting in meetings:
        title = meeting.get("subject", "Untitled Meeting")
        start = meeting.get("start", {}).get("dateTime", "")
        online_meeting = meeting.get("onlineMeeting", {})
        join_url = online_meeting.get("joinUrl", "")

        # Extract online meeting ID from join URL
        online_meeting_id = online_meeting.get("conferenceId", "")

        print(f"\n  {title} ({start[:10]})")

        # Try to get transcript
        if online_meeting_id:
            transcript_vtt = get_transcript(token, online_meeting_id)
            if transcript_vtt:
                segments = parse_vtt(transcript_vtt)
                with_transcript += 1
                print(f"    ✅ Transcript: {len(segments)} segments")

                if do_import and conn:
                    mid, msg = import_to_meetily(conn, title, start, segments)
                    print(f"    {msg}")
                    if mid:
                        imported += 1
            else:
                print(f"    ⚠️  No transcript available")
        else:
            print(f"    ⚠️  No online meeting ID")

    if conn:
        conn.close()

    print(f"\n{'=' * 50}")
    print(f"Meetings found: {len(meetings)}")
    print(f"With transcripts: {with_transcript}")
    if do_import:
        print(f"Imported: {imported}")
        if imported > 0:
            print("Restart Meetily to see imported meetings.")
    else:
        print("\nRun with --import to import into Meetily:")
        print(f"  python3 scripts/fetch_teams_transcripts.py --days {days} --import")


if __name__ == "__main__":
    main()

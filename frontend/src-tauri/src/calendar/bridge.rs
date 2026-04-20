//! Shells out to `osascript -l JavaScript` to query the macOS Calendar app.
//!
//! We embed the JXA script at compile time so the .app bundle has no external
//! script dependency. Linux/Windows: returns empty vec — caller decides what to do.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

#[cfg(target_os = "macos")]
const JXA_SCRIPT: &str = include_str!("query_calendar.jxa");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub uid: String,
    pub title: String,
    /// Epoch millis
    pub start_ms: i64,
    pub end_ms: i64,
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub notes_snippet: String,
}

impl CalendarEvent {
    /// Best-effort extraction of a conferencing URL from the event.
    /// Checks, in order: dedicated url field, location, notes.
    pub fn conference_url(&self) -> Option<String> {
        let candidates = [self.url.as_str(), self.location.as_str(), self.notes_snippet.as_str()];
        for c in candidates {
            if let Some(found) = extract_meeting_url(c) {
                return Some(found);
            }
        }
        None
    }
}

/// Extract the first known meeting URL from a string. Recognizes Teams, Zoom,
/// Google Meet, Webex, and generic https links as a fallback.
fn extract_meeting_url(s: &str) -> Option<String> {
    if s.is_empty() {
        return None;
    }
    let patterns = [
        "https://teams.microsoft.com/l/meetup-join",
        "https://teams.live.com/meet",
        "https://teams.microsoft.com/meet",
        "https://zoom.us/j/",
        "https://us02web.zoom.us/j/",
        "https://us06web.zoom.us/j/",
        "zoommtg://",
        "https://meet.google.com/",
        "https://webex.com/meet/",
        "https://linkedin.zoom.us/j/",
    ];
    for pat in patterns {
        if let Some(idx) = s.find(pat) {
            let rest = &s[idx..];
            // Take until whitespace, newline, or closing paren/angle bracket
            let end = rest
                .find(|c: char| c.is_whitespace() || c == '>' || c == ')' || c == '"')
                .unwrap_or(rest.len());
            return Some(rest[..end].to_string());
        }
    }
    None
}

/// Query macOS Calendar for events starting in the next `lookahead_seconds`.
#[cfg(target_os = "macos")]
pub async fn query_upcoming_events(lookahead_seconds: u64) -> Result<Vec<CalendarEvent>> {
    let lookahead_str = lookahead_seconds.to_string();
    let output = tokio::process::Command::new("osascript")
        .args(["-l", "JavaScript", "-e", JXA_SCRIPT, "--", &lookahead_str])
        .output()
        .await
        .map_err(|e| anyhow!("failed to spawn osascript: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(anyhow!(
            "osascript failed (status {:?}): {}",
            output.status.code(),
            stderr.trim()
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stdout = stdout.trim();
    if stdout.is_empty() || stdout == "null" {
        return Ok(vec![]);
    }

    // JXA returns `{ "__error": "..." }` on internal failure
    if stdout.starts_with("{\"__error\"") {
        #[derive(Deserialize)]
        struct Err {
            __error: String,
        }
        let e: Err = serde_json::from_str(stdout).unwrap_or(Err {
            __error: "unknown".into(),
        });
        return Err(anyhow!("calendar query error: {}", e.__error));
    }

    let events: Vec<CalendarEvent> = serde_json::from_str(stdout)
        .map_err(|e| anyhow!("failed to parse calendar JSON: {e}; raw: {stdout}"))?;
    Ok(events)
}

#[cfg(not(target_os = "macos"))]
pub async fn query_upcoming_events(_lookahead_seconds: u64) -> Result<Vec<CalendarEvent>> {
    // Calendar integration is macOS-only for now.
    Ok(vec![])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_teams_url() {
        let s = "Microsoft Teams Meeting\nJoin: https://teams.microsoft.com/l/meetup-join/abc?x=1 (Team A)";
        assert_eq!(
            extract_meeting_url(s).unwrap(),
            "https://teams.microsoft.com/l/meetup-join/abc?x=1"
        );
    }

    #[test]
    fn extracts_zoom_url() {
        let s = "Join Zoom Meeting https://us02web.zoom.us/j/1234567890?pwd=abc\nMeeting ID: 123";
        assert_eq!(
            extract_meeting_url(s).unwrap(),
            "https://us02web.zoom.us/j/1234567890?pwd=abc"
        );
    }

    #[test]
    fn returns_none_when_no_url() {
        assert_eq!(extract_meeting_url("Team sync — Conf Room B"), None);
    }
}

//! Recognizes when one of the running apps using system audio is a
//! conferencing / meeting app — so we only fire a "meeting started"
//! notification for real meetings, not for Spotify or a YouTube tab.
//!
//! App names come from `ca::System::processes()` via
//! `RunningApp::localized_name()` (see system_detector.rs:358), which
//! returns the name macOS shows in the menu bar. Those names vary by
//! build / locale, so we match on lowercase substrings.

/// Lowercase substrings that indicate a conferencing app. Match is
/// case-insensitive and `contains()`-based so "Microsoft Teams",
/// "Microsoft Teams (work or school)", "Teams", and "teams helper"
/// all match the `"teams"` entry.
///
/// To add a new meeting app: drop its localized-name substring here.
const MEETING_APP_SUBSTRINGS: &[&str] = &[
    "teams",          // Microsoft Teams (all variants)
    "zoom",           // Zoom, zoom.us
    "webex",          // Cisco Webex
    "google meet",    // Google Meet (rarely its own process, but just in case)
    "chrome",         // Meet / Teams web in Chrome — noisy; see is_meeting_app() note
    "bluejeans",      // BlueJeans
    "goto",           // GoTo Meeting
    "discord",        // Discord calls
    "slack",          // Slack huddles
    "whereby",        // Whereby
    "around",         // Around
    "figma",          // FigJam audio rooms — optional
];

/// Noisy apps that we treat as meeting-like only if no better signal exists.
/// (Future use — for now we keep them in the main list.)
#[allow(dead_code)]
const AMBIGUOUS_APPS: &[&str] = &["chrome", "safari", "firefox", "arc"];

/// Returns the first app in the list that looks like a meeting app,
/// or `None` if nothing matches.
pub fn find_meeting_app(apps: &[String]) -> Option<String> {
    apps.iter()
        .find(|a| is_meeting_app(a))
        .cloned()
}

/// Is this app name a known conferencing app?
///
/// Note: "chrome" is included because Teams/Meet running in a browser
/// tab produces system audio attributed to Chrome. This is the tradeoff —
/// miss browser-based meetings OR occasionally false-positive on a YouTube
/// tab. The user can mute the notification if it misfires; they can't
/// summon it if we miss a Teams-in-Chrome call. We err toward recall.
pub fn is_meeting_app(app_name: &str) -> bool {
    let lower = app_name.to_lowercase();
    MEETING_APP_SUBSTRINGS.iter().any(|s| lower.contains(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_teams_variants() {
        assert!(is_meeting_app("Microsoft Teams"));
        assert!(is_meeting_app("Microsoft Teams (work or school)"));
        assert!(is_meeting_app("Teams"));
    }

    #[test]
    fn matches_zoom() {
        assert!(is_meeting_app("zoom.us"));
        assert!(is_meeting_app("Zoom"));
    }

    #[test]
    fn rejects_music_apps() {
        assert!(!is_meeting_app("Spotify"));
        assert!(!is_meeting_app("Music"));
        assert!(!is_meeting_app("QuickTime Player"));
    }

    #[test]
    fn find_meeting_app_picks_first_match() {
        let apps = vec!["Spotify".into(), "Microsoft Teams".into(), "Slack".into()];
        assert_eq!(find_meeting_app(&apps).as_deref(), Some("Microsoft Teams"));
    }

    #[test]
    fn find_meeting_app_returns_none_when_only_noise() {
        let apps = vec!["Spotify".into(), "Music".into()];
        assert_eq!(find_meeting_app(&apps), None);
    }
}

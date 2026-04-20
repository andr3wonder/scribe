//! Background task that polls the macOS Calendar for upcoming meetings and
//! fires a notification ~1 minute before a recordable meeting starts.
//!
//! Responsibilities:
//!   1. Poll `bridge::query_upcoming_events` on an interval
//!   2. Filter to "recordable" events via `predicate::is_recordable_meeting`
//!   3. Deduplicate (one notification per event UID)
//!   4. Call into the existing `NotificationManager::show_meeting_reminder`
//!   5. Emit a Tauri event so an open frontend can show a rich prompt
//!
//! Kept intentionally simple: no persistence, in-memory dedup only. On app
//! restart, events you've already been notified about will re-notify, which
//! is fine because the time window is tight (events < 1 min away).

use crate::calendar::bridge::{query_upcoming_events, CalendarEvent};
use crate::calendar::predicate::is_recordable_meeting;
use crate::notifications::commands::NotificationManagerState;
use log::{debug as log_debug, error as log_error, info as log_info, warn as log_warn};
use serde::Serialize;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tokio::sync::Mutex;
use tokio::time::{interval, MissedTickBehavior};

/// How often to poll Calendar. Calendar access through `osascript` is cheap
/// but not free, so we don't hammer it.
const POLL_INTERVAL: Duration = Duration::from_secs(30);

/// How far ahead to look. Must exceed `NOTIFY_LEAD_SECS` so we see events
/// before they cross the notify threshold.
const LOOKAHEAD_SECS: u64 = 180;

/// Notify when an event starts within this many seconds from now.
const NOTIFY_LEAD_SECS: i64 = 75;

/// Payload emitted to the frontend when a meeting is about to start.
/// The UI can listen for this and surface a "Start recording?" banner.
#[derive(Debug, Clone, Serialize)]
struct MeetingStartingPayload {
    uid: String,
    title: String,
    start_ms: i64,
    seconds_until: i64,
    conference_url: Option<String>,
}

/// Spawn the monitor. Safe to call once at app startup.
pub fn spawn<R: Runtime>(app: AppHandle<R>) {
    tauri::async_runtime::spawn(async move {
        run(app).await;
    });
}

async fn run<R: Runtime>(app: AppHandle<R>) {
    log_info!("calendar monitor: started (poll={}s, lead={}s)", POLL_INTERVAL.as_secs(), NOTIFY_LEAD_SECS);

    let already_notified: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
    let mut ticker = interval(POLL_INTERVAL);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

    loop {
        ticker.tick().await;

        let events = match query_upcoming_events(LOOKAHEAD_SECS).await {
            Ok(e) => e,
            Err(e) => {
                // Don't spam logs — common cause is missing Calendar permission on first run.
                log_warn!("calendar monitor: query failed: {e}");
                continue;
            }
        };

        if events.is_empty() {
            log_debug!("calendar monitor: no events in window");
            continue;
        }

        let now_ms = chrono::Utc::now().timestamp_millis();

        for ev in events {
            let seconds_until = (ev.start_ms - now_ms) / 1000;
            if seconds_until > NOTIFY_LEAD_SECS || seconds_until < -30 {
                continue;
            }

            if !is_recordable_meeting(&ev) {
                continue;
            }

            let mut notified = already_notified.lock().await;
            if !notified.insert(ev.uid.clone()) {
                continue; // already notified this UID
            }
            drop(notified);

            notify(&app, &ev, seconds_until).await;
        }
    }
}

async fn notify<R: Runtime>(app: &AppHandle<R>, ev: &CalendarEvent, seconds_until: i64) {
    let minutes_until = ((seconds_until.max(0)) as u64 + 59) / 60; // ceil
    let title = if ev.title.is_empty() {
        None
    } else {
        Some(ev.title.clone())
    };

    log_info!(
        "calendar monitor: notifying for '{}' (starts in {}s)",
        ev.title,
        seconds_until
    );

    // 1) Fire the system notification via the existing NotificationManager
    let state = app.state::<NotificationManagerState<R>>();
    let guard = state.read().await;
    if let Some(mgr) = guard.as_ref() {
        if let Err(e) = mgr
            .show_meeting_reminder(minutes_until.max(1), title.clone())
            .await
        {
            log_error!("calendar monitor: show_meeting_reminder failed: {e}");
        }
    } else {
        log_warn!("calendar monitor: NotificationManager not yet initialized; skipping");
    }
    drop(guard);

    // 2) Emit a Tauri event so the frontend (if open) can show a richer prompt
    let payload = MeetingStartingPayload {
        uid: ev.uid.clone(),
        title: ev.title.clone(),
        start_ms: ev.start_ms,
        seconds_until,
        conference_url: ev.conference_url(),
    };
    if let Err(e) = app.emit("meeting-starting-prompt", &payload) {
        log_error!("calendar monitor: failed to emit event: {e}");
    }
}

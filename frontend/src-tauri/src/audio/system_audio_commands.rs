use tauri::{command, AppHandle, Emitter, Manager, State};
use crate::audio::{
    start_system_audio_capture, list_system_audio_devices, check_system_audio_permissions,
    SystemAudioDetector, SystemAudioEvent, new_system_audio_callback,
    meeting_apps,
};
use crate::notifications::commands::NotificationManagerState;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use serde::Serialize;
use anyhow::Result;

// Global state for system audio detector
type SystemAudioDetectorState = Arc<Mutex<Option<SystemAudioDetector>>>;

/// Minimum gap between meeting-start notifications for the same app. Prevents
/// noise when audio briefly stops/starts (e.g., user mutes, screen share swap).
const NOTIFY_COOLDOWN: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, Serialize)]
struct MeetingStartingPayload {
    app_name: String,
    all_apps: Vec<String>,
}

/// Start system audio monitoring without requiring Tauri State (for startup use)
pub async fn start_system_audio_monitoring_internal(app_handle: AppHandle) -> Result<(), String> {
    let mut detector = SystemAudioDetector::new();

    // Shared cooldown state across callback invocations. Key: last-notified app name.
    let last_notify: Arc<Mutex<Option<(String, Instant)>>> = Arc::new(Mutex::new(None));

    let callback = new_system_audio_callback(move |event| {
        match event {
            SystemAudioEvent::SystemAudioStarted(apps) => {
                tracing::info!("System audio started by apps: {:?}", apps);
                // Always emit the raw event — existing frontend listeners rely on it.
                let _ = app_handle.emit("system-audio-started", apps.clone());

                // Only surface a user-facing notification if a real meeting app
                // is producing audio. This filters out Spotify, YouTube, etc.
                let meeting_app = match meeting_apps::find_meeting_app(&apps) {
                    Some(a) => a,
                    None => {
                        tracing::debug!("No meeting app in audio list; skipping notification");
                        return;
                    }
                };

                // Cooldown: don't re-notify the same app within NOTIFY_COOLDOWN.
                if let Ok(mut guard) = last_notify.lock() {
                    if let Some((prev_app, when)) = guard.as_ref() {
                        if prev_app == &meeting_app && when.elapsed() < NOTIFY_COOLDOWN {
                            tracing::debug!(
                                "Suppressing duplicate meeting-start notification for {} (cooldown)",
                                meeting_app
                            );
                            return;
                        }
                    }
                    *guard = Some((meeting_app.clone(), Instant::now()));
                }

                let payload = MeetingStartingPayload {
                    app_name: meeting_app.clone(),
                    all_apps: apps.clone(),
                };
                let _ = app_handle.emit("meeting-starting-prompt", &payload);

                // Fire the notification via NotificationManager so user settings /
                // DND are respected. Reuses the existing `show_meeting_reminder`
                // plumbing (settings.show_meeting_reminders, reminder_minutes list).
                //
                // Callback runs on a CoreAudio native thread — we hop onto the tokio
                // runtime to reach the async NotificationManager.
                let app_for_notify = app_handle.clone();
                let app_name_for_notify = meeting_app.clone();
                tauri::async_runtime::spawn(async move {
                    let state = app_for_notify.state::<NotificationManagerState<tauri::Wry>>();
                    let guard = state.read().await;
                    if let Some(mgr) = guard.as_ref() {
                        // Use "0 minutes until" to mean "happening now". 0 must be
                        // in meeting_reminder_minutes for the notification to fire;
                        // see defaults in notifications/settings.rs.
                        let title = format!("{} meeting started", app_name_for_notify);
                        if let Err(e) = mgr
                            .show_meeting_reminder(0, Some(title))
                            .await
                        {
                            tracing::error!("show_meeting_reminder failed: {}", e);
                        }
                    } else {
                        tracing::warn!(
                            "NotificationManager not ready; falling back to raw notification"
                        );
                        use tauri_plugin_notification::NotificationExt;
                        let _ = app_for_notify
                            .notification()
                            .builder()
                            .title("Meeting started")
                            .body(&format!(
                                "{} is running — open Scribe to record",
                                app_name_for_notify
                            ))
                            .show();
                    }
                });
            }
            SystemAudioEvent::SystemAudioStopped => {
                let _ = app_handle.emit("system-audio-stopped", ());
                tracing::info!("System audio stopped");
            }
        }
    });

    detector.start(callback);
    std::mem::forget(detector);
    Ok(())
}

/// Start system audio capture (for capturing system output audio)
#[command]
pub async fn start_system_audio_capture_command() -> Result<String, String> {
    match start_system_audio_capture().await {
        Ok(_stream) => {
            // TODO: Store the stream in global state if needed for management
            Ok("System audio capture started successfully".to_string())
        }
        Err(e) => Err(format!("Failed to start system audio capture: {}", e))
    }
}

/// List available system audio devices
#[command]
pub async fn list_system_audio_devices_command() -> Result<Vec<String>, String> {
    list_system_audio_devices()
        .map_err(|e| format!("Failed to list system audio devices: {}", e))
}

/// Check if the app has permission to access system audio
#[command]
pub async fn check_system_audio_permissions_command() -> bool {
    check_system_audio_permissions()
}

/// Start monitoring system audio usage by other applications
#[command]
pub async fn start_system_audio_monitoring(
    app_handle: AppHandle,
    detector_state: State<'_, SystemAudioDetectorState>
) -> Result<(), String> {
    let mut detector_guard = detector_state.lock()
        .map_err(|e| format!("Failed to acquire detector lock: {}", e))?;

    if detector_guard.is_some() {
        return Err("System audio monitoring is already active".to_string());
    }

    let mut detector = SystemAudioDetector::new();

    // Create callback that emits events to the frontend
    let callback = new_system_audio_callback(move |event| {
        match event {
            SystemAudioEvent::SystemAudioStarted(apps) => {
                tracing::info!("System audio started by apps: {:?}", apps);
                let _ = app_handle.emit("system-audio-started", apps);
            }
            SystemAudioEvent::SystemAudioStopped => {
                let _ = app_handle.emit("system-audio-stopped", ());
                tracing::info!("System audio stopped");
            }
        }
    });

    detector.start(callback);
    *detector_guard = Some(detector);

    Ok(())
}

/// Stop monitoring system audio usage
#[command]
pub async fn stop_system_audio_monitoring(
    detector_state: State<'_, SystemAudioDetectorState>
) -> Result<(), String> {
    let mut detector_guard = detector_state.lock()
        .map_err(|e| format!("Failed to acquire detector lock: {}", e))?;

    if let Some(mut detector) = detector_guard.take() {
        detector.stop();
        Ok(())
    } else {
        Err("System audio monitoring is not active".to_string())
    }
}

/// Get the current status of system audio monitoring
#[command]
pub async fn get_system_audio_monitoring_status(
    detector_state: State<'_, SystemAudioDetectorState>
) -> Result<bool, String> {
    let detector_guard = detector_state.lock()
        .map_err(|e| format!("Failed to acquire detector lock: {}", e))?;

    Ok(detector_guard.is_some())
}

/// Initialize the system audio detector state in Tauri app
pub fn init_system_audio_state() -> SystemAudioDetectorState {
    Arc::new(Mutex::new(None))
}

// Event payload types for frontend
#[derive(serde::Serialize, Clone)]
pub struct SystemAudioStartedPayload {
    pub apps: Vec<String>,
}

#[derive(serde::Serialize, Clone)]
pub struct SystemAudioStoppedPayload;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_list_system_audio_devices() {
        let devices = list_system_audio_devices_command().await;
        match devices {
            Ok(device_list) => {
                println!("System audio devices: {:?}", device_list);
                assert!(device_list.len() >= 0); // Should at least not crash
            }
            Err(e) => {
                println!("Error listing devices: {}", e);
                // This might fail on CI or systems without audio
            }
        }
    }

    #[tokio::test]
    async fn test_check_permissions() {
        let has_permission = check_system_audio_permissions_command().await;
        println!("Has system audio permissions: {}", has_permission);
        // This is mainly a smoke test to ensure it doesn't crash
    }
}
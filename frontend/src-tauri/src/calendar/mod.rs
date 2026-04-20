//! macOS Calendar integration — polls for upcoming meetings and prompts the
//! user to start recording.
//!
//! Entry point: `monitor::spawn(app_handle)` from `lib.rs::run()` at startup.

pub mod bridge;
pub mod monitor;
pub mod predicate;

pub use bridge::{query_upcoming_events, CalendarEvent};
pub use predicate::is_recordable_meeting;

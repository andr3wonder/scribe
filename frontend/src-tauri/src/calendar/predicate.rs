//! Defines what counts as a "recordable" meeting worth prompting for.
//!
//! This is deliberately called out as a user-authored decision — different
//! people have very different intuitions about which events should trigger
//! a recording prompt. See `is_recordable_meeting` below.

use crate::calendar::bridge::CalendarEvent;

/// Decide whether to prompt the user to record this event.
///
/// Called for every upcoming event within the lookahead window. Return
/// `true` to fire a notification, `false` to silently ignore.
///
/// ## Signals available on `ev`
/// - `ev.title`            — event title ("Team Standup", "1:1 w/ Alice", ...)
/// - `ev.location`         — "Conference Room B", "Microsoft Teams Meeting", ...
/// - `ev.url`              — dedicated URL field (often empty; not all apps set it)
/// - `ev.notes_snippet`    — first 2KB of the event notes/description
/// - `ev.conference_url()` — best-effort extracted Teams/Zoom/Meet/Webex URL
///                           (searches url → location → notes)
/// - `ev.start_ms`, `ev.end_ms` — epoch millis
///
/// ## Design questions worth thinking about
///
/// 1. **URL-only, or title/keyword too?**
///    Strictest: only events with a Teams/Zoom/Meet URL. Safe but misses
///    in-person meetings or custom conferencing.
///
/// 2. **What about 1:1s?** Often personal, maybe shouldn't auto-record.
///    Could skip events with "1:1", "/", "&", or that have only 2 attendees.
///
/// 3. **What about focus blocks?** "Focus time", "Heads down", "OOO" —
///    should be filtered out even if they somehow got a URL attached.
///
/// 4. **Duration sanity?** An "all-day" event is probably not a meeting.
///    Events < 10 min are often reminders, not meetings.
///
/// 5. **Declined events?** JXA can pull status, but for v1 we don't —
///    worth revisiting if this feels noisy.
pub fn is_recordable_meeting(ev: &CalendarEvent) -> bool {
    // TODO(you): Implement the predicate. Reasonable starting point below —
    // replace, extend, or tighten.
    let _ = ev;
    false
}

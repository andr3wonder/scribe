/// People intelligence module - profiles, relationship tracking, expertise mapping, commitments
///
/// This module contains:
/// - Service layer for people queries (profiles, commitments, expertise, meeting prep)
/// - Tauri command handlers for frontend integration

pub mod commands;
pub mod service;

// Re-export Tauri commands (with their generated __cmd__ variants)
pub use commands::{
    __cmd__get_commitments_for_person, __cmd__get_expertise_map,
    __cmd__get_meeting_participants_cmd, __cmd__get_person_profile, __cmd__list_people,
    __cmd__merge_people_cmd, __cmd__prepare_for_meeting, get_commitments_for_person,
    get_expertise_map, get_meeting_participants_cmd, get_person_profile, list_people,
    merge_people_cmd, prepare_for_meeting,
};

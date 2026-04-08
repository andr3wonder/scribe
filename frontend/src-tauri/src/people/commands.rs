use crate::state::AppState;
use tauri::{AppHandle, Manager, Runtime};
use tracing::info;

use super::service::{
    self, CommitmentDetail, ExpertInfo, MeetingPrep, ParticipantDetail, PersonProfile,
    PersonSummary,
};

/// Get a full profile for a person by name.
#[tauri::command]
pub async fn get_person_profile<R: Runtime>(
    app: AppHandle<R>,
    name: String,
) -> Result<PersonProfile, String> {
    info!("Command: get_person_profile name={}", name);

    let state = app.state::<AppState>();
    let pool = state.db_manager.pool();

    service::get_person_profile(pool, &name).await
}

/// List all known people, sorted by meeting count descending.
#[tauri::command]
pub async fn list_people<R: Runtime>(
    app: AppHandle<R>,
    limit: Option<i64>,
) -> Result<Vec<PersonSummary>, String> {
    info!("Command: list_people limit={:?}", limit);

    let state = app.state::<AppState>();
    let pool = state.db_manager.pool();

    service::list_people(pool, limit).await
}

/// Get commitments/action items for a person. Optionally filter by status (open, completed, overdue).
#[tauri::command]
pub async fn get_commitments_for_person<R: Runtime>(
    app: AppHandle<R>,
    person_name: String,
    status: Option<String>,
) -> Result<Vec<CommitmentDetail>, String> {
    info!(
        "Command: get_commitments_for_person person={}, status={:?}",
        person_name, status
    );

    let state = app.state::<AppState>();
    let pool = state.db_manager.pool();

    service::get_commitments_for_person(pool, &person_name, status.as_deref()).await
}

/// Get expertise map: who has discussed a given topic and how much.
#[tauri::command]
pub async fn get_expertise_map<R: Runtime>(
    app: AppHandle<R>,
    topic: String,
) -> Result<Vec<ExpertInfo>, String> {
    info!("Command: get_expertise_map topic={}", topic);

    let state = app.state::<AppState>();
    let pool = state.db_manager.pool();

    service::get_expertise_map(pool, &topic).await
}

/// Prepare for a meeting with a person: relationship summary, open items, recent context, suggested agenda.
#[tauri::command]
pub async fn prepare_for_meeting<R: Runtime>(
    app: AppHandle<R>,
    person_name: String,
) -> Result<MeetingPrep, String> {
    info!("Command: prepare_for_meeting person={}", person_name);

    let state = app.state::<AppState>();
    let pool = state.db_manager.pool();

    service::prepare_for_meeting(pool, &person_name).await
}

/// Get all participants in a specific meeting with detail.
#[tauri::command]
pub async fn get_meeting_participants_cmd<R: Runtime>(
    app: AppHandle<R>,
    meeting_id: String,
) -> Result<Vec<ParticipantDetail>, String> {
    info!(
        "Command: get_meeting_participants_cmd meeting_id={}",
        meeting_id
    );

    let state = app.state::<AppState>();
    let pool = state.db_manager.pool();

    service::get_meeting_participants_detail(pool, &meeting_id).await
}

/// Merge two people records (dedup). Moves all references from merge_id to keep_id.
#[tauri::command]
pub async fn merge_people_cmd<R: Runtime>(
    app: AppHandle<R>,
    keep_id: String,
    merge_id: String,
) -> Result<(), String> {
    info!(
        "Command: merge_people_cmd keep={}, merge={}",
        keep_id, merge_id
    );

    let state = app.state::<AppState>();
    let pool = state.db_manager.pool();

    service::merge_people(pool, &keep_id, &merge_id).await
}

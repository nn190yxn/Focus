#[cfg(feature = "desktop-app")]
use crate::{
    domain::planning::{DailyNote, DailyNoteInput, WeeklyGoal, WeeklyGoalInput},
    repositories::database::Database,
    services::planning_service::PlanningService,
    CommandResult, DomainError,
};

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn note_get(
    database: tauri::State<'_, Database>,
    note_date: String,
) -> CommandResult<Option<DailyNote>> {
    result(PlanningService::new(&database).get_note(note_date))
}

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn note_save(
    database: tauri::State<'_, Database>,
    input: DailyNoteInput,
) -> CommandResult<DailyNote> {
    result(PlanningService::new(&database).save_note(input))
}

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn weekly_goal_list(
    database: tauri::State<'_, Database>,
    week_starts_on: String,
    timezone: String,
) -> CommandResult<Vec<WeeklyGoal>> {
    result(PlanningService::new(&database).list_weekly_goals(week_starts_on, timezone))
}

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn weekly_goal_save(
    database: tauri::State<'_, Database>,
    input: WeeklyGoalInput,
    timezone: String,
) -> CommandResult<WeeklyGoal> {
    result(PlanningService::new(&database).save_weekly_goal(input, timezone))
}

#[cfg(feature = "desktop-app")]
fn result<T>(value: Result<T, DomainError>) -> CommandResult<T> {
    CommandResult::from_result(module_path!(), value, 1)
}

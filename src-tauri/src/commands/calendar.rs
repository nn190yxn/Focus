#[cfg(feature = "desktop-app")]
use crate::{
    domain::calendar::{CalendarPeriodResult, CalendarQuery},
    repositories::database::Database,
    services::calendar_service::CalendarService,
    CommandResult, DomainError,
};

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn calendar_get_period(
    database: tauri::State<'_, Database>,
    query: CalendarQuery,
) -> CommandResult<CalendarPeriodResult> {
    result(CalendarService::new(&database).get_period(query))
}

#[cfg(feature = "desktop-app")]
fn result<T>(value: Result<T, DomainError>) -> CommandResult<T> {
    CommandResult::from_result(module_path!(), value, 1)
}

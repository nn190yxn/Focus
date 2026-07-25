#[cfg(feature = "desktop-app")]
use crate::{
    domain::{calendar::CalendarQuery, statistics::StatisticsSummary},
    repositories::database::Database,
    services::statistics_service::StatisticsService,
    CommandResult, DomainError,
};

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn statistics_get_summary(
    database: tauri::State<'_, Database>,
    query: CalendarQuery,
) -> CommandResult<StatisticsSummary> {
    result(StatisticsService::new(&database).get_summary(query))
}

#[cfg(feature = "desktop-app")]
fn result<T>(value: Result<T, DomainError>) -> CommandResult<T> {
    CommandResult::from_result(module_path!(), value, 1)
}

#[cfg(feature = "desktop-app")]
use crate::{
    domain::today::TodayDigest, repositories::database::Database,
    services::today_service::TodayService, CommandResult, DomainError,
};

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn today_get_digest(
    database: tauri::State<'_, Database>,
    date: String,
) -> CommandResult<TodayDigest> {
    result(TodayService::new(&database).get_digest(&date))
}

#[cfg(feature = "desktop-app")]
fn result<T>(value: Result<T, DomainError>) -> CommandResult<T> {
    CommandResult::from_result(module_path!(), value, 1)
}

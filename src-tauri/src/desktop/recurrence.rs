use chrono::Utc;
use tauri::{Emitter, Manager};

use crate::{
    repositories::database::Database,
    services::recurrence_service::{GenerationTrigger, RecurrenceScheduler},
    DomainError,
};

pub fn reconcile_and_emit(
    app: &tauri::AppHandle,
    trigger: GenerationTrigger,
) -> Result<usize, DomainError> {
    let database = app.state::<Database>();
    let affected_count = RecurrenceScheduler::new(&database)
        .reconcile_active_to_utc_now(trigger, Utc::now())?
        .into_iter()
        .map(|summary| summary.affected_count)
        .sum();
    if affected_count > 0 {
        app.emit("today://changed", ())
            .map_err(|error| DomainError {
                code: "TODAY_EVENT_FAILED".into(),
                message: error.to_string(),
                field: None,
            })?;
    }
    Ok(affected_count)
}

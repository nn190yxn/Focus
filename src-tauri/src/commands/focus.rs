#[cfg(feature = "desktop-app")]
use crate::{
    domain::focus::{
        FocusCompletionKind, FocusReconcileResult, FocusSession, FocusState, FocusTarget,
    },
    repositories::database::Database,
    services::focus_service::FocusService,
    CommandResult, DomainError,
};

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn focus_get_state(
    database: tauri::State<'_, Database>,
    app: tauri::AppHandle,
) -> CommandResult<FocusState> {
    result(reconcile_and_emit(&database, &app).map(|outcome| outcome.state))
}

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn focus_reconcile(
    database: tauri::State<'_, Database>,
    app: tauri::AppHandle,
) -> CommandResult<FocusReconcileResult> {
    result(reconcile_and_emit(&database, &app))
}

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn focus_start(
    database: tauri::State<'_, Database>,
    app: tauri::AppHandle,
    target: FocusTarget,
    planned_minutes: u16,
) -> CommandResult<FocusState> {
    result(after_focus_change(
        FocusService::new(&database).start(target, planned_minutes),
        &app,
    ))
}

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn focus_pause(
    database: tauri::State<'_, Database>,
    app: tauri::AppHandle,
) -> CommandResult<FocusState> {
    result(after_focus_change(
        FocusService::new(&database).pause(),
        &app,
    ))
}

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn focus_resume(
    database: tauri::State<'_, Database>,
    app: tauri::AppHandle,
) -> CommandResult<FocusState> {
    result(after_focus_change(
        FocusService::new(&database).resume(),
        &app,
    ))
}

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn focus_reset(
    database: tauri::State<'_, Database>,
    app: tauri::AppHandle,
) -> CommandResult<FocusState> {
    result(after_focus_change(
        FocusService::new(&database).reset(),
        &app,
    ))
}

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn focus_finish(
    database: tauri::State<'_, Database>,
    app: tauri::AppHandle,
    completion_kind: FocusCompletionKind,
) -> CommandResult<FocusSession> {
    let service = FocusService::new(&database);
    result(service.finish(completion_kind).inspect(|session| {
        handle_completed(&database, &app, session);
        if let Ok(state) = service.get_state() {
            crate::desktop::focus_events::emit_focus_changed(&app, &state);
        }
    }))
}

#[cfg(feature = "desktop-app")]
pub(crate) fn reconcile_and_emit(
    database: &Database,
    app: &tauri::AppHandle,
) -> Result<FocusReconcileResult, DomainError> {
    let outcome = FocusService::new(database).reconcile()?;
    if let Some(session) = outcome.completed_session.as_ref() {
        handle_completed(database, app, session);
        crate::desktop::focus_events::emit_focus_changed(app, &outcome.state);
    }
    Ok(outcome)
}

#[cfg(feature = "desktop-app")]
fn after_focus_change(
    outcome: Result<FocusState, DomainError>,
    app: &tauri::AppHandle,
) -> Result<FocusState, DomainError> {
    crate::desktop::focus_events::after_focus_change(outcome, |state| {
        crate::desktop::focus_events::emit_focus_changed(app, state);
    })
}

#[cfg(feature = "desktop-app")]
fn handle_completed(database: &Database, app: &tauri::AppHandle, session: &FocusSession) {
    use tauri::Emitter;

    let _ = crate::services::notification_service::NotificationService::new(database)
        .notify_focus_completed(
            session,
            &crate::desktop::notifications::TauriNotificationPublisher::new(app),
        );
    let _ = app.emit("focus://completed", session);
}

#[cfg(feature = "desktop-app")]
fn result<T>(value: Result<T, DomainError>) -> CommandResult<T> {
    CommandResult::from_result(module_path!(), value, 1)
}

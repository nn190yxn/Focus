use crate::CommandResult;

#[cfg_attr(feature = "desktop-app", tauri::command)]
pub fn health() -> CommandResult<&'static str> {
    CommandResult::success("ready", 0)
}

#[cfg_attr(feature = "desktop-app", tauri::command)]
pub fn diagnostic_command_failure(command: String, error: String) {
    log::warn!(
        "{}",
        crate::diagnostic_invocation_failure_event(&command, &error)
    );
}

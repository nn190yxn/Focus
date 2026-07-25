use crate::DomainError;

pub fn after_today_change<T>(
    outcome: Result<T, DomainError>,
    on_changed: impl FnOnce(),
) -> Result<T, DomainError> {
    let value = outcome?;
    on_changed();
    Ok(value)
}

#[cfg(feature = "desktop-app")]
pub fn emit_today_changed(app: &tauri::AppHandle) {
    use tauri::Emitter;

    let _ = app.emit("today://changed", ());
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    #[test]
    fn successful_changes_emit_once() {
        let emissions = Cell::new(0);

        let result = after_today_change(Ok("saved"), || emissions.set(emissions.get() + 1));

        assert_eq!(result.unwrap(), "saved");
        assert_eq!(emissions.get(), 1);
    }

    #[test]
    fn failed_changes_do_not_emit() {
        let emissions = Cell::new(0);
        let error = DomainError {
            code: "TASK_NOT_FOUND".into(),
            message: "missing".into(),
            field: None,
        };

        let result: Result<(), DomainError> =
            after_today_change(Err(error), || emissions.set(emissions.get() + 1));

        assert_eq!(result.unwrap_err().code, "TASK_NOT_FOUND");
        assert_eq!(emissions.get(), 0);
    }
}

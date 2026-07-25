use crate::DomainError;

pub fn after_memo_change<T>(
    outcome: Result<T, DomainError>,
    on_changed: impl FnOnce(),
) -> Result<T, DomainError> {
    let value = outcome?;
    on_changed();
    Ok(value)
}

#[cfg(feature = "desktop-app")]
pub fn emit_memo_changed(app: &tauri::AppHandle) {
    use tauri::Emitter;

    let _ = app.emit("memo://changed", ());
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    #[test]
    fn successful_changes_emit_once() {
        let emissions = Cell::new(0);

        let result = after_memo_change(Ok("saved"), || emissions.set(emissions.get() + 1));

        assert_eq!(result.unwrap(), "saved");
        assert_eq!(emissions.get(), 1);
    }

    #[test]
    fn failed_changes_preserve_the_error_without_emitting() {
        let emissions = Cell::new(0);
        let error = DomainError {
            code: "MEMO_SAVE_FAILED".into(),
            message: "memo could not be saved".into(),
            field: None,
        };

        let result: Result<(), DomainError> =
            after_memo_change(Err(error), || emissions.set(emissions.get() + 1));

        assert_eq!(result.unwrap_err().code, "MEMO_SAVE_FAILED");
        assert_eq!(emissions.get(), 0);
    }
}

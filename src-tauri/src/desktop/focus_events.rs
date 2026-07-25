use crate::{domain::focus::FocusState, DomainError};

pub fn after_focus_change(
    outcome: Result<FocusState, DomainError>,
    on_changed: impl FnOnce(&FocusState),
) -> Result<FocusState, DomainError> {
    let state = outcome?;
    on_changed(&state);
    Ok(state)
}

#[cfg(feature = "desktop-app")]
pub fn emit_focus_changed(app: &tauri::AppHandle, state: &FocusState) {
    use tauri::Emitter;

    let _ = app.emit("focus://state-changed", state);
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use chrono::{TimeZone, Utc};

    use super::*;

    #[test]
    fn successful_changes_emit_once() {
        let emissions = Cell::new(0);
        let state = FocusState::Ready {
            server_time: Utc.with_ymd_and_hms(2026, 7, 22, 10, 0, 0).unwrap(),
        };

        let result = after_focus_change(Ok(state.clone()), |emitted| {
            assert_eq!(emitted, &state);
            emissions.set(emissions.get() + 1);
        });

        assert_eq!(result.unwrap(), state);
        assert_eq!(emissions.get(), 1);
    }

    #[test]
    fn failed_changes_do_not_emit() {
        let emissions = Cell::new(0);
        let error = DomainError {
            code: "FOCUS_NOT_ACTIVE".into(),
            message: "missing".into(),
            field: None,
        };

        let result = after_focus_change(Err(error), |_| emissions.set(emissions.get() + 1));

        assert_eq!(result.unwrap_err().code, "FOCUS_NOT_ACTIVE");
        assert_eq!(emissions.get(), 0);
    }
}

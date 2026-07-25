pub const MEMO_OPEN_REQUESTED_EVENT: &str = "memo://open-requested";

pub trait MemoNotificationActivationTarget {
    fn show_main_window(&self) -> Result<(), crate::DomainError>;
    fn emit_open_requested(&self, memo_id: &str) -> Result<(), crate::DomainError>;
}

pub fn activate(
    target: &impl MemoNotificationActivationTarget,
    memo_id: &str,
) -> Result<(), crate::DomainError> {
    let memo_id = validate_memo_id(memo_id)?;
    target.show_main_window()?;
    target.emit_open_requested(&memo_id)
}

fn validate_memo_id(memo_id: &str) -> Result<String, crate::DomainError> {
    let parsed = uuid::Uuid::parse_str(memo_id).map_err(|_| invalid_activation_error())?;
    let canonical = parsed.to_string();
    if canonical != memo_id {
        return Err(invalid_activation_error());
    }
    Ok(canonical)
}

fn invalid_activation_error() -> crate::DomainError {
    crate::DomainError {
        code: "MEMO_NOTIFICATION_ACTIVATION_INVALID".into(),
        message: "memo notification activation identifier is invalid".into(),
        field: Some("memoId".into()),
    }
}

#[cfg(feature = "desktop-app")]
impl MemoNotificationActivationTarget for tauri::AppHandle {
    fn show_main_window(&self) -> Result<(), crate::DomainError> {
        crate::desktop::tray::show_main_window(self)
    }

    fn emit_open_requested(&self, memo_id: &str) -> Result<(), crate::DomainError> {
        use tauri::Emitter;

        self.emit(MEMO_OPEN_REQUESTED_EVENT, memo_id)
            .map_err(|error| crate::DomainError {
                code: "MEMO_NOTIFICATION_ACTIVATION_FAILED".into(),
                message: error.to_string(),
                field: None,
            })
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    #[derive(Default)]
    struct FakeTarget {
        calls: RefCell<Vec<String>>,
        show_error: Option<crate::DomainError>,
        emit_error: Option<crate::DomainError>,
    }

    impl MemoNotificationActivationTarget for FakeTarget {
        fn show_main_window(&self) -> Result<(), crate::DomainError> {
            self.calls.borrow_mut().push("show".into());
            self.show_error.clone().map_or(Ok(()), Err)
        }

        fn emit_open_requested(&self, memo_id: &str) -> Result<(), crate::DomainError> {
            self.calls.borrow_mut().push(format!("emit:{memo_id}"));
            self.emit_error.clone().map_or(Ok(()), Err)
        }
    }

    #[test]
    fn valid_activation_shows_main_window_before_emitting_memo_id() {
        let target = FakeTarget::default();
        let memo_id = "ebbc2524-ae61-4ae7-b62e-5cc8eb6ed112";

        activate(&target, memo_id).unwrap();

        assert_eq!(
            *target.calls.borrow(),
            vec!["show".to_string(), format!("emit:{memo_id}")]
        );
    }

    #[test]
    fn invalid_activation_is_rejected_without_touching_the_window() {
        let target = FakeTarget::default();

        let error = activate(&target, "../memo-secret").unwrap_err();

        assert_eq!(error.code, "MEMO_NOTIFICATION_ACTIVATION_INVALID");
        assert_eq!(error.field.as_deref(), Some("memoId"));
        assert!(target.calls.borrow().is_empty());
    }

    #[test]
    fn window_activation_failure_prevents_event_emission() {
        let target = FakeTarget {
            show_error: Some(crate::DomainError {
                code: "MAIN_WINDOW_MISSING".into(),
                message: "missing".into(),
                field: None,
            }),
            ..Default::default()
        };

        let error = activate(&target, "ebbc2524-ae61-4ae7-b62e-5cc8eb6ed112").unwrap_err();

        assert_eq!(error.code, "MAIN_WINDOW_MISSING");
        assert_eq!(*target.calls.borrow(), ["show"]);
    }
}

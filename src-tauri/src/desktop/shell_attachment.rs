use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ShellFallbackReason {
    UnsupportedPlatform,
    HostNotFound,
    AttachmentFailed,
    DetachmentFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellAttachError {
    pub reason: ShellFallbackReason,
    pub message: String,
}

impl ShellAttachError {
    pub fn new(reason: ShellFallbackReason, message: impl Into<String>) -> Self {
        Self {
            reason,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ShellAttachError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ShellAttachError {}

pub trait ShellHostAdapter {
    type Host: Copy + Eq;

    fn discover_host(&self) -> Result<Self::Host, ShellAttachError>;
    fn is_host_valid(&self, host: Self::Host) -> bool;
    fn attach(&self, window: usize, host: Self::Host) -> Result<(), ShellAttachError>;
    fn detach(&self, window: usize) -> Result<(), ShellAttachError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellAttachmentOutcome {
    DesktopAttached {
        recovered: bool,
    },
    Floating,
    FloatingFallback {
        reason: ShellFallbackReason,
        should_notify: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppliedShellMode {
    Desktop,
    Floating,
}

impl ShellAttachmentOutcome {
    pub fn applied_mode(&self) -> AppliedShellMode {
        match self {
            Self::DesktopAttached { .. } => AppliedShellMode::Desktop,
            Self::Floating | Self::FloatingFallback { .. } => AppliedShellMode::Floating,
        }
    }

    pub fn recovered(&self) -> bool {
        matches!(self, Self::DesktopAttached { recovered: true })
    }
}

pub struct ShellAttachmentManager<A: ShellHostAdapter> {
    adapter: A,
    attached_host: Option<A::Host>,
    desktop_requested: bool,
    fallback_reported: bool,
}

impl<A: ShellHostAdapter> ShellAttachmentManager<A> {
    pub fn new(adapter: A) -> Self {
        Self {
            adapter,
            attached_host: None,
            desktop_requested: false,
            fallback_reported: false,
        }
    }

    pub fn set_desktop_mode(&mut self, window: usize) -> ShellAttachmentOutcome {
        self.desktop_requested = true;
        self.attach(window, false)
    }

    pub fn set_floating_mode(
        &mut self,
        window: usize,
    ) -> Result<ShellAttachmentOutcome, ShellAttachError> {
        self.adapter.detach(window)?;
        self.attached_host = None;
        self.desktop_requested = false;
        self.fallback_reported = false;
        Ok(ShellAttachmentOutcome::Floating)
    }

    pub fn recover_if_needed(&mut self, window: usize) -> ShellAttachmentOutcome {
        if !self.desktop_requested {
            return ShellAttachmentOutcome::Floating;
        }
        if self
            .attached_host
            .is_some_and(|host| self.adapter.is_host_valid(host))
        {
            return ShellAttachmentOutcome::DesktopAttached { recovered: false };
        }
        self.attach(window, true)
    }

    fn attach(&mut self, window: usize, recovered: bool) -> ShellAttachmentOutcome {
        let result = self
            .adapter
            .discover_host()
            .and_then(|host| self.adapter.attach(window, host).map(|()| host));
        match result {
            Ok(host) => {
                self.attached_host = Some(host);
                self.fallback_reported = false;
                ShellAttachmentOutcome::DesktopAttached { recovered }
            }
            Err(error) => {
                let _ = self.adapter.detach(window);
                self.attached_host = None;
                let should_notify = !self.fallback_reported;
                self.fallback_reported = true;
                ShellAttachmentOutcome::FloatingFallback {
                    reason: error.reason,
                    should_notify,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    #[derive(Default)]
    struct FakeState {
        host: Option<u64>,
        valid: bool,
        fail_attach: bool,
        attach_calls: Vec<(usize, u64)>,
        detach_calls: Vec<usize>,
    }

    #[derive(Clone)]
    struct FakeAdapter(Arc<Mutex<FakeState>>);

    impl ShellHostAdapter for FakeAdapter {
        type Host = u64;

        fn discover_host(&self) -> Result<Self::Host, ShellAttachError> {
            self.0.lock().unwrap().host.ok_or_else(|| {
                ShellAttachError::new(ShellFallbackReason::HostNotFound, "host missing")
            })
        }

        fn is_host_valid(&self, host: Self::Host) -> bool {
            let state = self.0.lock().unwrap();
            state.valid && state.host == Some(host)
        }

        fn attach(&self, window: usize, host: Self::Host) -> Result<(), ShellAttachError> {
            let mut state = self.0.lock().unwrap();
            state.attach_calls.push((window, host));
            if state.fail_attach {
                Err(ShellAttachError::new(
                    ShellFallbackReason::AttachmentFailed,
                    "attach failed",
                ))
            } else {
                Ok(())
            }
        }

        fn detach(&self, window: usize) -> Result<(), ShellAttachError> {
            self.0.lock().unwrap().detach_calls.push(window);
            Ok(())
        }
    }

    fn fixture(host: Option<u64>) -> (ShellAttachmentManager<FakeAdapter>, Arc<Mutex<FakeState>>) {
        let state = Arc::new(Mutex::new(FakeState {
            host,
            valid: host.is_some(),
            ..Default::default()
        }));
        (
            ShellAttachmentManager::new(FakeAdapter(state.clone())),
            state,
        )
    }

    #[test]
    fn desktop_mode_attaches_and_floating_mode_detaches() {
        let (mut manager, state) = fixture(Some(41));
        assert_eq!(
            manager.set_desktop_mode(7),
            ShellAttachmentOutcome::DesktopAttached { recovered: false }
        );
        assert_eq!(
            manager.set_floating_mode(7).unwrap(),
            ShellAttachmentOutcome::Floating
        );
        let state = state.lock().unwrap();
        assert_eq!(state.attach_calls, vec![(7, 41)]);
        assert_eq!(state.detach_calls, vec![7]);
    }

    #[test]
    fn invalid_host_is_discovered_and_reattached() {
        let (mut manager, state) = fixture(Some(41));
        manager.set_desktop_mode(7);
        {
            let mut state = state.lock().unwrap();
            state.host = Some(42);
            state.valid = true;
        }

        assert_eq!(
            manager.recover_if_needed(7),
            ShellAttachmentOutcome::DesktopAttached { recovered: true }
        );
        assert_eq!(state.lock().unwrap().attach_calls, vec![(7, 41), (7, 42)]);
    }

    #[test]
    fn repeated_failures_only_request_one_fallback_notification() {
        let (mut manager, _state) = fixture(None);
        assert_eq!(
            manager.set_desktop_mode(7),
            ShellAttachmentOutcome::FloatingFallback {
                reason: ShellFallbackReason::HostNotFound,
                should_notify: true,
            }
        );
        assert_eq!(
            manager.recover_if_needed(7),
            ShellAttachmentOutcome::FloatingFallback {
                reason: ShellFallbackReason::HostNotFound,
                should_notify: false,
            }
        );
    }

    #[test]
    fn outcomes_define_window_layer_and_recovery_state() {
        assert_eq!(
            ShellAttachmentOutcome::DesktopAttached { recovered: false }.applied_mode(),
            AppliedShellMode::Desktop
        );
        assert!(!ShellAttachmentOutcome::DesktopAttached { recovered: false }.recovered());
        assert!(ShellAttachmentOutcome::DesktopAttached { recovered: true }.recovered());
        assert_eq!(
            ShellAttachmentOutcome::FloatingFallback {
                reason: ShellFallbackReason::HostNotFound,
                should_notify: true,
            }
            .applied_mode(),
            AppliedShellMode::Floating
        );
    }
}

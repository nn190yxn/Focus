use std::{cell::RefCell, rc::Rc};

use arrive_focus_core::{
    desktop::{
        shell_attachment::{
            ShellAttachError, ShellAttachmentManager, ShellAttachmentOutcome, ShellFallbackReason,
            ShellHostAdapter,
        },
        widget_window::WidgetWindowBehavior,
    },
    domain::widget::WidgetMode,
    repositories::database::Database,
    services::widget_service::WidgetService,
};

#[derive(Default)]
struct FakeShellState {
    host: Option<u64>,
    fail_attach: bool,
    attach_calls: Vec<(usize, u64)>,
    detach_calls: Vec<usize>,
}

#[derive(Clone)]
struct FakeShellAdapter(Rc<RefCell<FakeShellState>>);

impl ShellHostAdapter for FakeShellAdapter {
    type Host = u64;

    fn discover_host(&self) -> Result<Self::Host, ShellAttachError> {
        self.0.borrow().host.ok_or_else(|| {
            ShellAttachError::new(ShellFallbackReason::HostNotFound, "desktop host missing")
        })
    }

    fn is_host_valid(&self, host: Self::Host) -> bool {
        self.0.borrow().host == Some(host)
    }

    fn attach(&self, window: usize, host: Self::Host) -> Result<(), ShellAttachError> {
        let mut state = self.0.borrow_mut();
        state.attach_calls.push((window, host));
        if state.fail_attach {
            Err(ShellAttachError::new(
                ShellFallbackReason::AttachmentFailed,
                "attachment failed",
            ))
        } else {
            Ok(())
        }
    }

    fn detach(&self, window: usize) -> Result<(), ShellAttachError> {
        self.0.borrow_mut().detach_calls.push(window);
        Ok(())
    }
}

fn shell_fixture(
    host: Option<u64>,
) -> (
    ShellAttachmentManager<FakeShellAdapter>,
    Rc<RefCell<FakeShellState>>,
) {
    let state = Rc::new(RefCell::new(FakeShellState {
        host,
        ..Default::default()
    }));
    (
        ShellAttachmentManager::new(FakeShellAdapter(state.clone())),
        state,
    )
}

#[test]
fn desktop_attachment_recovers_after_explorer_replaces_its_host() {
    let (mut manager, state) = shell_fixture(Some(101));

    assert_eq!(
        manager.set_desktop_mode(7),
        ShellAttachmentOutcome::DesktopAttached { recovered: false }
    );
    state.borrow_mut().host = Some(202);

    assert_eq!(
        manager.recover_if_needed(7),
        ShellAttachmentOutcome::DesktopAttached { recovered: true }
    );
    assert_eq!(state.borrow().attach_calls, vec![(7, 101), (7, 202)]);
}

#[test]
fn fallback_notifies_once_per_failure_episode_and_recovers() {
    let (mut manager, state) = shell_fixture(None);

    assert_eq!(
        manager.set_desktop_mode(9),
        ShellAttachmentOutcome::FloatingFallback {
            reason: ShellFallbackReason::HostNotFound,
            should_notify: true,
        }
    );
    assert_eq!(
        manager.recover_if_needed(9),
        ShellAttachmentOutcome::FloatingFallback {
            reason: ShellFallbackReason::HostNotFound,
            should_notify: false,
        }
    );

    state.borrow_mut().host = Some(303);
    assert_eq!(
        manager.recover_if_needed(9),
        ShellAttachmentOutcome::DesktopAttached { recovered: true }
    );

    state.borrow_mut().host = None;
    assert_eq!(
        manager.recover_if_needed(9),
        ShellAttachmentOutcome::FloatingFallback {
            reason: ShellFallbackReason::HostNotFound,
            should_notify: true,
        }
    );
}

#[test]
fn unlock_persists_interaction_after_a_floating_fallback() {
    let (mut manager, _state) = shell_fixture(None);
    assert!(matches!(
        manager.set_desktop_mode(11),
        ShellAttachmentOutcome::FloatingFallback { .. }
    ));

    let database = Database::open_in_memory().unwrap();
    let service = WidgetService::new(&database);
    let mut input = service.get().unwrap().input;
    input.mode = WidgetMode::Desktop;
    input.locked = true;
    service.update(input).unwrap();

    let locked_behavior = WidgetWindowBehavior::new(WidgetMode::Floating, true);
    assert!(locked_behavior.always_on_top);
    assert!(locked_behavior.ignore_cursor_events);
    assert!(!locked_behavior.resizable);

    let unlocked = service.unlock().unwrap();
    let unlocked_behavior = WidgetWindowBehavior::new(WidgetMode::Floating, unlocked.input.locked);
    assert!(!unlocked.input.locked);
    assert!(unlocked_behavior.always_on_top);
    assert!(!unlocked_behavior.ignore_cursor_events);
    assert!(unlocked_behavior.resizable);
    assert!(!service.get().unwrap().input.locked);
}

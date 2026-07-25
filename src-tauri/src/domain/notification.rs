use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NotificationKind {
    FocusCompleted,
    TaskDue,
    RecurringTaskDue,
    MemoReminder,
}

impl NotificationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FocusCompleted => "focusCompleted",
            Self::TaskDue => "taskDue",
            Self::RecurringTaskDue => "recurringTaskDue",
            Self::MemoReminder => "memoReminder",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationCandidate {
    pub kind: NotificationKind,
    pub source_id: String,
    pub title: String,
    pub scheduled_for: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemNotification {
    pub title: String,
    pub body: String,
    pub sound_enabled: bool,
    pub activation: Option<SystemNotificationActivation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemNotificationActivation {
    OpenMemo { memo_id: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReminderWindow {
    pub starts_at: DateTime<FixedOffset>,
    pub ends_at: DateTime<FixedOffset>,
}

pub fn next_reminder_scan_cursor(
    previous_scan: DateTime<FixedOffset>,
    now: DateTime<FixedOffset>,
    reconciliation_succeeded: bool,
) -> DateTime<FixedOffset> {
    if reconciliation_succeeded {
        now
    } else {
        previous_scan
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn at(hour: u32, minute: u32) -> DateTime<FixedOffset> {
        FixedOffset::east_opt(8 * 3600)
            .unwrap()
            .with_ymd_and_hms(2026, 7, 20, hour, minute, 0)
            .unwrap()
    }

    #[test]
    fn reminder_scan_cursor_advances_only_after_success() {
        assert_eq!(
            next_reminder_scan_cursor(at(9, 0), at(9, 15), true),
            at(9, 15)
        );
        assert_eq!(
            next_reminder_scan_cursor(at(9, 0), at(9, 15), false),
            at(9, 0)
        );
    }
}

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::DomainError;

pub const MIN_FOCUS_MINUTES: u16 = 1;
pub const MAX_FOCUS_MINUTES: u16 = 180;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ActiveFocusStatus {
    Running,
    Paused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FocusCompletionKind {
    Deadline,
    Early,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusTarget {
    pub task_id: Option<String>,
    pub task_instance_id: Option<String>,
}

impl FocusTarget {
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.task_id.is_some() == self.task_instance_id.is_some() {
            return Err(focus_error(
                "FOCUS_TARGET_INVALID",
                "select exactly one task or task instance",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveFocus {
    pub task_id: Option<String>,
    pub task_instance_id: Option<String>,
    pub status: ActiveFocusStatus,
    pub planned_seconds: i64,
    pub remaining_seconds: i64,
    pub started_at: DateTime<Utc>,
    pub target_ends_at: Option<DateTime<Utc>>,
    pub paused_at: Option<DateTime<Utc>>,
    pub interruption_count: i64,
    pub updated_at: DateTime<Utc>,
}

impl ActiveFocus {
    pub fn start(
        target: FocusTarget,
        planned_minutes: u16,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        target.validate()?;
        if !(MIN_FOCUS_MINUTES..=MAX_FOCUS_MINUTES).contains(&planned_minutes) {
            return Err(DomainError {
                code: "FOCUS_DURATION_INVALID".into(),
                message: "focus duration must be between 1 and 180 minutes".into(),
                field: Some("plannedMinutes".into()),
            });
        }
        let planned_seconds = i64::from(planned_minutes) * 60;
        Ok(Self {
            task_id: target.task_id,
            task_instance_id: target.task_instance_id,
            status: ActiveFocusStatus::Running,
            planned_seconds,
            remaining_seconds: planned_seconds,
            started_at: now,
            target_ends_at: Some(now + Duration::seconds(planned_seconds)),
            paused_at: None,
            interruption_count: 0,
            updated_at: now,
        })
    }

    pub fn remaining_at(&self, now: DateTime<Utc>) -> i64 {
        match self.status {
            ActiveFocusStatus::Running => self
                .target_ends_at
                .map(|target| (target - now).num_seconds().clamp(0, self.planned_seconds))
                .unwrap_or(0),
            ActiveFocusStatus::Paused => self.remaining_seconds.clamp(0, self.planned_seconds),
        }
    }

    pub fn pause(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        if self.status != ActiveFocusStatus::Running {
            return Err(focus_error("FOCUS_NOT_RUNNING", "focus is not running"));
        }
        self.remaining_seconds = self.remaining_at(now);
        self.status = ActiveFocusStatus::Paused;
        self.target_ends_at = None;
        self.paused_at = Some(now);
        self.interruption_count += 1;
        self.updated_at = now;
        Ok(())
    }

    pub fn resume(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        if self.status != ActiveFocusStatus::Paused {
            return Err(focus_error("FOCUS_NOT_PAUSED", "focus is not paused"));
        }
        self.status = ActiveFocusStatus::Running;
        self.target_ends_at = Some(now + Duration::seconds(self.remaining_seconds));
        self.paused_at = None;
        self.updated_at = now;
        Ok(())
    }

    pub fn actual_seconds_at(&self, now: DateTime<Utc>) -> i64 {
        self.planned_seconds - self.remaining_at(now)
    }

    pub fn to_state(&self, now: DateTime<Utc>) -> Result<FocusState, DomainError> {
        Ok(match self.status {
            ActiveFocusStatus::Running => FocusState::Running {
                task_id: self.task_id.clone(),
                task_instance_id: self.task_instance_id.clone(),
                planned_seconds: self.planned_seconds,
                remaining_seconds: self.remaining_at(now),
                started_at: self.started_at,
                target_ends_at: self.target_ends_at.ok_or_else(|| {
                    focus_error(
                        "FOCUS_STATE_CORRUPTED",
                        "running focus is missing its deadline",
                    )
                })?,
                interruption_count: self.interruption_count,
                server_time: now,
            },
            ActiveFocusStatus::Paused => FocusState::Paused {
                task_id: self.task_id.clone(),
                task_instance_id: self.task_instance_id.clone(),
                planned_seconds: self.planned_seconds,
                remaining_seconds: self.remaining_at(now),
                started_at: self.started_at,
                paused_at: self.paused_at.ok_or_else(|| {
                    focus_error(
                        "FOCUS_STATE_CORRUPTED",
                        "paused focus is missing its pause time",
                    )
                })?,
                interruption_count: self.interruption_count,
                server_time: now,
            },
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum FocusState {
    Ready {
        server_time: DateTime<Utc>,
    },
    Running {
        task_id: Option<String>,
        task_instance_id: Option<String>,
        planned_seconds: i64,
        remaining_seconds: i64,
        started_at: DateTime<Utc>,
        target_ends_at: DateTime<Utc>,
        interruption_count: i64,
        server_time: DateTime<Utc>,
    },
    Paused {
        task_id: Option<String>,
        task_instance_id: Option<String>,
        planned_seconds: i64,
        remaining_seconds: i64,
        started_at: DateTime<Utc>,
        paused_at: DateTime<Utc>,
        interruption_count: i64,
        server_time: DateTime<Utc>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusSession {
    pub id: String,
    pub task_id: Option<String>,
    pub task_instance_id: Option<String>,
    pub project_id: Option<String>,
    pub planned_seconds: i64,
    pub actual_seconds: i64,
    pub interruption_count: i64,
    pub completion_kind: FocusCompletionKind,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusReconcileResult {
    pub state: FocusState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_session: Option<FocusSession>,
}

pub fn focus_error(code: &str, message: &str) -> DomainError {
    DomainError {
        code: code.into(),
        message: message.into(),
        field: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(second: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 7, 19, 10, 0, second).unwrap()
    }

    fn target() -> FocusTarget {
        FocusTarget {
            task_id: Some("task-1".into()),
            task_instance_id: None,
        }
    }

    #[test]
    fn pause_and_resume_preserve_remaining_time() {
        let mut focus = ActiveFocus::start(target(), 25, at(0)).unwrap();
        focus.pause(at(10)).unwrap();
        assert_eq!(focus.remaining_seconds, 1_490);
        assert_eq!(focus.interruption_count, 1);

        focus.resume(at(20)).unwrap();
        assert_eq!(
            focus.target_ends_at,
            Some(at(20) + Duration::seconds(1_490))
        );
    }

    #[test]
    fn duration_and_target_are_validated() {
        assert_eq!(
            ActiveFocus::start(target(), 0, at(0)).unwrap_err().code,
            "FOCUS_DURATION_INVALID"
        );
        assert_eq!(
            ActiveFocus::start(
                FocusTarget {
                    task_id: None,
                    task_instance_id: None,
                },
                25,
                at(0),
            )
            .unwrap_err()
            .code,
            "FOCUS_TARGET_INVALID"
        );
    }

    #[test]
    fn state_serializes_with_a_direct_status_tag() {
        let running = ActiveFocus::start(target(), 15, at(0))
            .unwrap()
            .to_state(at(0))
            .unwrap();
        let value = serde_json::to_value(running).unwrap();
        assert_eq!(value["state"], "running");
        assert_eq!(value["plannedSeconds"], 900);
        assert_eq!(value["remainingSeconds"], 900);
    }
}

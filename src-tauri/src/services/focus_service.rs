use chrono::{DateTime, Utc};

use crate::{
    domain::{
        focus::{
            focus_error, ActiveFocus, FocusCompletionKind, FocusReconcileResult, FocusSession,
            FocusState, FocusTarget,
        },
        recurrence::TaskInstanceStatus,
    },
    repositories::{
        database::Database, focus_repository::FocusRepository,
        project_repository::ProjectRepository, recurrence_repository::RecurrenceRepository,
        task_repository::TaskRepository,
    },
    DomainError,
};

pub struct FocusService<'a> {
    database: &'a Database,
}

impl<'a> FocusService<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn get_state(&self) -> Result<FocusState, DomainError> {
        self.get_state_at(Utc::now())
    }

    pub fn start(
        &self,
        target: FocusTarget,
        planned_minutes: u16,
    ) -> Result<FocusState, DomainError> {
        self.start_at(target, planned_minutes, Utc::now())
    }

    pub fn pause(&self) -> Result<FocusState, DomainError> {
        self.pause_at(Utc::now())
    }

    pub fn resume(&self) -> Result<FocusState, DomainError> {
        self.resume_at(Utc::now())
    }

    pub fn reset(&self) -> Result<FocusState, DomainError> {
        self.reset_at(Utc::now())
    }

    pub fn finish(
        &self,
        completion_kind: FocusCompletionKind,
    ) -> Result<FocusSession, DomainError> {
        self.finish_at(completion_kind, Utc::now())
    }

    pub fn reconcile(&self) -> Result<FocusReconcileResult, DomainError> {
        self.reconcile_at(Utc::now())
    }

    pub(crate) fn get_state_at(&self, now: DateTime<Utc>) -> Result<FocusState, DomainError> {
        match FocusRepository::new(self.database).get_active()? {
            Some(active) => active.to_state(now),
            None => Ok(FocusState::Ready { server_time: now }),
        }
    }

    pub(crate) fn start_at(
        &self,
        target: FocusTarget,
        planned_minutes: u16,
        now: DateTime<Utc>,
    ) -> Result<FocusState, DomainError> {
        target.validate()?;
        let repository = FocusRepository::new(self.database);
        if repository.get_active()?.is_some() {
            return Err(focus_error(
                "FOCUS_ALREADY_ACTIVE",
                "an active focus already exists",
            ));
        }
        let focus = ActiveFocus::start(target, planned_minutes, now)?;
        self.validate_target(&FocusTarget {
            task_id: focus.task_id.clone(),
            task_instance_id: focus.task_instance_id.clone(),
        })?;
        repository.insert_active(&focus)?;
        focus.to_state(now)
    }

    pub(crate) fn pause_at(&self, now: DateTime<Utc>) -> Result<FocusState, DomainError> {
        let repository = FocusRepository::new(self.database);
        let mut focus = repository.get_active()?.ok_or_else(focus_not_active)?;
        focus.pause(now)?;
        ensure_updated(repository.update_active(&focus)?)?;
        focus.to_state(now)
    }

    pub(crate) fn resume_at(&self, now: DateTime<Utc>) -> Result<FocusState, DomainError> {
        let repository = FocusRepository::new(self.database);
        let mut focus = repository.get_active()?.ok_or_else(focus_not_active)?;
        focus.resume(now)?;
        ensure_updated(repository.update_active(&focus)?)?;
        focus.to_state(now)
    }

    pub(crate) fn reset_at(&self, now: DateTime<Utc>) -> Result<FocusState, DomainError> {
        let repository = FocusRepository::new(self.database);
        let focus = repository.get_active()?.ok_or_else(focus_not_active)?;
        let project_id = self.project_id_for(&focus)?;
        let session = make_session(&focus, project_id, FocusCompletionKind::Cancelled, now);
        repository.finalize(&session)?;
        Ok(FocusState::Ready { server_time: now })
    }

    pub(crate) fn reconcile_at(
        &self,
        now: DateTime<Utc>,
    ) -> Result<FocusReconcileResult, DomainError> {
        let repository = FocusRepository::new(self.database);
        let completed_session = repository.complete_due(now, &uuid::Uuid::new_v4().to_string())?;
        let state = match repository.get_active()? {
            Some(active) => active.to_state(now)?,
            None => FocusState::Ready { server_time: now },
        };
        Ok(FocusReconcileResult {
            state,
            completed_session,
        })
    }

    pub(crate) fn finish_at(
        &self,
        completion_kind: FocusCompletionKind,
        now: DateTime<Utc>,
    ) -> Result<FocusSession, DomainError> {
        if completion_kind == FocusCompletionKind::Cancelled {
            return Err(focus_error(
                "FOCUS_COMPLETION_KIND_INVALID",
                "use reset to cancel an active focus",
            ));
        }
        let repository = FocusRepository::new(self.database);
        if completion_kind == FocusCompletionKind::Deadline {
            if let Some(session) =
                repository.complete_due(now, &uuid::Uuid::new_v4().to_string())?
            {
                return Ok(session);
            }
            return match repository.get_active()? {
                Some(_) => Err(focus_error(
                    "FOCUS_DEADLINE_NOT_REACHED",
                    "focus deadline has not been reached",
                )),
                None => Err(focus_not_active()),
            };
        }
        let focus = repository.get_active()?.ok_or_else(focus_not_active)?;
        let project_id = self.project_id_for(&focus)?;
        let session = make_session(&focus, project_id, completion_kind, now);
        repository.finalize(&session)?;
        Ok(session)
    }

    fn validate_target(&self, target: &FocusTarget) -> Result<(), DomainError> {
        if let Some(task_id) = target.task_id.as_deref() {
            let task = TaskRepository::new(self.database)
                .get(task_id)?
                .ok_or_else(|| focus_error("FOCUS_TASK_NOT_FOUND", "task was not found"))?;
            if task.status != "pending" {
                return Err(focus_error("FOCUS_TASK_UNAVAILABLE", "task is not pending"));
            }
            self.validate_project(task.project_id.as_deref())?;
        }
        if let Some(instance_id) = target.task_instance_id.as_deref() {
            let instance = RecurrenceRepository::new(self.database)
                .get_instance(instance_id)?
                .ok_or_else(|| {
                    focus_error(
                        "FOCUS_TASK_INSTANCE_NOT_FOUND",
                        "task instance was not found",
                    )
                })?;
            if instance.status != TaskInstanceStatus::Pending {
                return Err(focus_error(
                    "FOCUS_TASK_INSTANCE_UNAVAILABLE",
                    "task instance is not pending",
                ));
            }
            self.validate_project(instance.snapshot_project_id.as_deref())?;
        }
        Ok(())
    }

    fn validate_project(&self, project_id: Option<&str>) -> Result<(), DomainError> {
        let Some(project_id) = project_id else {
            return Ok(());
        };
        if ProjectRepository::new(self.database)
            .get(project_id)?
            .is_some_and(|project| project.status == "paused")
        {
            return Err(focus_error(
                "FOCUS_PROJECT_PAUSED",
                "the target project is paused",
            ));
        }
        Ok(())
    }

    fn project_id_for(&self, focus: &ActiveFocus) -> Result<Option<String>, DomainError> {
        if let Some(task_id) = focus.task_id.as_deref() {
            return Ok(TaskRepository::new(self.database)
                .get(task_id)?
                .and_then(|task| task.project_id));
        }
        if let Some(instance_id) = focus.task_instance_id.as_deref() {
            return Ok(RecurrenceRepository::new(self.database)
                .get_instance(instance_id)?
                .and_then(|instance| instance.snapshot_project_id));
        }
        Ok(None)
    }
}

fn make_session(
    focus: &ActiveFocus,
    project_id: Option<String>,
    completion_kind: FocusCompletionKind,
    now: DateTime<Utc>,
) -> FocusSession {
    FocusSession {
        id: uuid::Uuid::new_v4().to_string(),
        task_id: focus.task_id.clone(),
        task_instance_id: focus.task_instance_id.clone(),
        project_id,
        planned_seconds: focus.planned_seconds,
        actual_seconds: focus.actual_seconds_at(now),
        interruption_count: focus.interruption_count,
        completion_kind,
        started_at: focus.started_at,
        ended_at: now,
        created_at: now,
    }
}

fn focus_not_active() -> DomainError {
    focus_error("FOCUS_NOT_ACTIVE", "there is no active focus")
}

fn ensure_updated(updated: bool) -> Result<(), DomainError> {
    if updated {
        Ok(())
    } else {
        Err(focus_not_active())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::recurrence::{RecurrencePattern, RecurrenceRule, RecurrenceStatus},
        repositories::{
            focus_repository::FocusRepository, project_repository::ProjectRecord,
            recurrence_repository::TaskInstanceRecord, task_repository::TaskRecord,
        },
    };
    use chrono::{Duration, TimeZone};
    use proptest::prelude::*;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 7, 19, 10, 0, 0).unwrap()
    }

    fn setup() -> (Database, FocusTarget) {
        let database = Database::open_in_memory().unwrap();
        let timestamp = now().to_rfc3339();
        let task = TaskRecord {
            id: "task-1".into(),
            project_id: None,
            title: "Write tests".into(),
            category: "work".into(),
            priority: 2,
            scheduled_date: Some("2026-07-19".into()),
            scheduled_time: None,
            status: "pending".into(),
            completed_at: None,
            created_at: timestamp.clone(),
            updated_at: timestamp,
        };
        TaskRepository::new(&database).insert(&task).unwrap();
        (
            database,
            FocusTarget {
                task_id: Some(task.id),
                task_instance_id: None,
            },
        )
    }

    fn pause_target_project(database: &Database, target: &FocusTarget) {
        let timestamp = now().to_rfc3339();
        ProjectRepository::new(database)
            .insert(&ProjectRecord {
                id: "project-paused".into(),
                name: "Paused project".into(),
                description: String::new(),
                color: "mint".into(),
                icon: "folder".into(),
                status: "paused".into(),
                started_on: "2026-07-01".into(),
                target_on: None,
                created_at: timestamp.clone(),
                updated_at: timestamp,
            })
            .unwrap();
        let task_id = target.task_id.as_deref().unwrap();
        let mut task = TaskRepository::new(database).get(task_id).unwrap().unwrap();
        task.project_id = Some("project-paused".into());
        TaskRepository::new(database).update(&task).unwrap();
    }

    fn recurring_target(database: &Database) -> FocusTarget {
        let timestamp = now().to_rfc3339();
        RecurrenceRepository::new(database)
            .insert_rule(&RecurrenceRule {
                id: "rule-paused".into(),
                task_template_id: "task-1".into(),
                pattern: RecurrencePattern::Daily { interval: 1 },
                local_time: None,
                timezone: "UTC".into(),
                starts_on: "2026-07-19".into(),
                ends_on: None,
                status: RecurrenceStatus::Active,
                version: 1,
            })
            .unwrap();
        RecurrenceRepository::new(database)
            .upsert_instances(
                &[TaskInstanceRecord {
                    id: "instance-paused".into(),
                    recurrence_rule_id: "rule-paused".into(),
                    rule_version: 1,
                    scheduled_date: "2026-07-19".into(),
                    scheduled_at: None,
                    snapshot_title: "Paused instance".into(),
                    snapshot_project_id: Some("project-paused".into()),
                    status: TaskInstanceStatus::Pending,
                    completed_at: None,
                    source_instance_id: None,
                    created_at: timestamp.clone(),
                    updated_at: timestamp,
                }],
                false,
            )
            .unwrap();
        FocusTarget {
            task_id: None,
            task_instance_id: Some("instance-paused".into()),
        }
    }

    #[test]
    fn full_state_machine_persists_an_early_session() {
        let (database, target) = setup();
        let service = FocusService::new(&database);
        service.start_at(target.clone(), 25, now()).unwrap();
        assert_eq!(
            service.start_at(target, 25, now()).unwrap_err().code,
            "FOCUS_ALREADY_ACTIVE"
        );

        service.pause_at(now() + Duration::seconds(60)).unwrap();
        let paused = service.get_state_at(now() + Duration::seconds(90)).unwrap();
        assert!(matches!(
            paused,
            FocusState::Paused {
                remaining_seconds: 1_440,
                interruption_count: 1,
                ..
            }
        ));

        service.resume_at(now() + Duration::seconds(120)).unwrap();
        let session = service
            .finish_at(FocusCompletionKind::Early, now() + Duration::seconds(180))
            .unwrap();
        assert_eq!(session.actual_seconds, 120);
        assert_eq!(session.interruption_count, 1);
        assert_eq!(session.completion_kind, FocusCompletionKind::Early);
        assert!(FocusRepository::new(&database)
            .get_session(&session.id)
            .unwrap()
            .is_some());
        assert!(matches!(
            service.get_state_at(now()).unwrap(),
            FocusState::Ready { .. }
        ));
    }

    #[test]
    fn paused_project_blocks_task_and_recurring_instance_focus() {
        let (database, target) = setup();
        pause_target_project(&database, &target);
        let service = FocusService::new(&database);

        assert_eq!(
            service.start_at(target, 25, now()).unwrap_err().code,
            "FOCUS_PROJECT_PAUSED"
        );
        assert_eq!(
            service
                .start_at(recurring_target(&database), 25, now())
                .unwrap_err()
                .code,
            "FOCUS_PROJECT_PAUSED"
        );
    }

    #[test]
    fn deadline_completion_requires_elapsed_timer() {
        let (database, target) = setup();
        let service = FocusService::new(&database);
        service.start_at(target, 1, now()).unwrap();
        assert_eq!(
            service
                .finish_at(FocusCompletionKind::Deadline, now())
                .unwrap_err()
                .code,
            "FOCUS_DEADLINE_NOT_REACHED"
        );
        let session = service
            .finish_at(FocusCompletionKind::Deadline, now() + Duration::seconds(75))
            .unwrap();
        assert_eq!(session.actual_seconds, 60);
        assert_eq!(session.ended_at, now() + Duration::seconds(60));
    }

    #[test]
    fn reconcile_calibrates_running_and_preserves_paused_time() {
        let (database, target) = setup();
        let service = FocusService::new(&database);
        service.start_at(target, 1, now()).unwrap();

        let backwards = service.reconcile_at(now() - Duration::seconds(30)).unwrap();
        assert!(matches!(
            backwards.state,
            FocusState::Running {
                remaining_seconds: 60,
                ..
            }
        ));
        let forward = service.reconcile_at(now() + Duration::seconds(25)).unwrap();
        assert!(matches!(
            forward.state,
            FocusState::Running {
                remaining_seconds: 35,
                ..
            }
        ));

        service.pause_at(now() + Duration::seconds(30)).unwrap();
        let paused = service.reconcile_at(now() + Duration::hours(8)).unwrap();
        assert!(matches!(
            paused.state,
            FocusState::Paused {
                remaining_seconds: 30,
                ..
            }
        ));
        assert!(paused.completed_session.is_none());
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(128))]

        // Validates P6 and requirements R3.2, R3.6, and R4.5.
        #[test]
        fn p6_reconciliation_tracks_the_persisted_deadline_within_two_seconds(
            planned_minutes in 1u16..=180,
            elapsed_milliseconds in 0i64..=86_400_000,
        ) {
            let (database, target) = setup();
            let service = FocusService::new(&database);
            service.start_at(target, planned_minutes, now()).unwrap();
            let target_ends_at = now() + Duration::minutes(i64::from(planned_minutes));
            let reconcile_time = now() + Duration::milliseconds(elapsed_milliseconds);
            let expected_milliseconds = (target_ends_at - reconcile_time)
                .num_milliseconds()
                .max(0);

            let outcome = service.reconcile_at(reconcile_time).unwrap();
            let actual_seconds = match outcome.state {
                FocusState::Running { remaining_seconds, target_ends_at: persisted_target, .. } => {
                    prop_assert_eq!(persisted_target, target_ends_at);
                    prop_assert!(outcome.completed_session.is_none());
                    remaining_seconds
                }
                FocusState::Ready { .. } => {
                    prop_assert_eq!(expected_milliseconds, 0);
                    prop_assert!(outcome.completed_session.is_some());
                    0
                }
                FocusState::Paused { .. } => {
                    prop_assert!(false, "reconciliation changed a running focus to paused");
                    0
                }
            };
            let error_milliseconds = (actual_seconds * 1_000 - expected_milliseconds).abs();

            prop_assert!(error_milliseconds < 2_000);
        }
    }

    #[test]
    fn repeated_due_reconciliation_completes_once() {
        let (database, target) = setup();
        let service = FocusService::new(&database);
        service.start_at(target, 1, now()).unwrap();

        let first = service.reconcile_at(now() + Duration::minutes(10)).unwrap();
        let second = service.reconcile_at(now() + Duration::minutes(10)).unwrap();
        let session = first.completed_session.unwrap();
        assert_eq!(session.completion_kind, FocusCompletionKind::Deadline);
        assert_eq!(session.actual_seconds, 60);
        assert_eq!(session.ended_at, now() + Duration::minutes(1));
        assert!(second.completed_session.is_none());
        assert_eq!(
            FocusRepository::new(&database)
                .count_sessions(FocusCompletionKind::Deadline)
                .unwrap(),
            1
        );
    }

    #[test]
    fn concurrent_due_reconciliation_completes_once() {
        use std::sync::{Arc, Barrier};

        let (database, target) = setup();
        let database = Arc::new(database);
        FocusService::new(&database)
            .start_at(target, 1, now())
            .unwrap();
        let barrier = Arc::new(Barrier::new(8));
        let handles = (0..8)
            .map(|_| {
                let database = Arc::clone(&database);
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    FocusService::new(&database)
                        .reconcile_at(now() + Duration::minutes(2))
                        .unwrap()
                })
            })
            .collect::<Vec<_>>();
        let completed = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .filter(|outcome| outcome.completed_session.is_some())
            .count();

        assert_eq!(completed, 1);
        assert_eq!(
            FocusRepository::new(&database)
                .count_sessions(FocusCompletionKind::Deadline)
                .unwrap(),
            1
        );
    }

    #[test]
    fn reset_records_a_cancelled_session() {
        let (database, target) = setup();
        let service = FocusService::new(&database);
        service.start_at(target, 15, now()).unwrap();
        service.reset_at(now() + Duration::seconds(30)).unwrap();
        assert_eq!(
            FocusRepository::new(&database)
                .count_sessions(FocusCompletionKind::Cancelled)
                .unwrap(),
            1
        );
        assert!(matches!(
            service.get_state_at(now()).unwrap(),
            FocusState::Ready { .. }
        ));
    }
}

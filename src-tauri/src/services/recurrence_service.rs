use chrono::{
    DateTime, Duration, NaiveDate, NaiveDateTime, NaiveTime, SecondsFormat, TimeZone, Utc,
};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

use crate::{
    domain::recurrence::{
        scheduled_dates, RecurrenceChangeScope, RecurrenceRule, RecurrenceStatus,
        TaskInstanceStatus,
    },
    repositories::{
        database::Database,
        recurrence_repository::{RecurrenceRepository, TaskInstanceRecord},
        task_repository::TaskRepository,
    },
    DomainError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenerationTrigger {
    Startup,
    DayBoundary,
    RuleChanged { rule_id: String },
    TimezoneChanged { rule_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationSummary {
    pub rule_id: String,
    pub scheduled_count: usize,
    pub affected_count: usize,
}

pub struct RecurrenceScheduler<'a> {
    database: &'a Database,
}

impl<'a> RecurrenceScheduler<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn run(
        &self,
        trigger: GenerationTrigger,
        range_start: NaiveDate,
        range_end: NaiveDate,
    ) -> Result<Vec<GenerationSummary>, DomainError> {
        if range_start > range_end {
            return Err(scheduler_error(
                "RECURRENCE_RANGE_INVALID",
                "generation end date must be on or after start date",
                Some("rangeEnd"),
            ));
        }
        let repository = RecurrenceRepository::new(self.database);
        match trigger {
            GenerationTrigger::Startup | GenerationTrigger::DayBoundary => repository
                .list_active_rules()?
                .into_iter()
                .map(|rule| self.generate_rule(&rule, range_start, range_end, false))
                .collect(),
            GenerationTrigger::RuleChanged { rule_id }
            | GenerationTrigger::TimezoneChanged { rule_id } => {
                let rule = repository
                    .get_rule(&rule_id)?
                    .ok_or_else(|| recurrence_not_found(&rule_id))?;
                Ok(vec![self.generate_rule(
                    &rule,
                    range_start,
                    range_end,
                    true,
                )?])
            }
        }
    }

    pub fn reconcile_active_to_utc_now(
        &self,
        trigger: GenerationTrigger,
        utc_now: DateTime<Utc>,
    ) -> Result<Vec<GenerationSummary>, DomainError> {
        let backfill = match trigger {
            GenerationTrigger::Startup => true,
            GenerationTrigger::DayBoundary => false,
            _ => {
                return Err(scheduler_error(
                    "RECURRENCE_TRIGGER_INVALID",
                    "automatic reconciliation requires a startup or day-boundary trigger",
                    None,
                ));
            }
        };
        RecurrenceRepository::new(self.database)
            .list_active_rules()?
            .into_iter()
            .map(|rule| {
                let timezone = parse_timezone(&rule.timezone)?;
                let local_today = utc_now.with_timezone(&timezone).date_naive();
                let starts_on = parse_local_date(&rule.starts_on, "startsOn")?;
                if starts_on > local_today {
                    return Ok(GenerationSummary {
                        rule_id: rule.id,
                        scheduled_count: 0,
                        affected_count: 0,
                    });
                }
                self.generate_rule(
                    &rule,
                    if backfill { starts_on } else { local_today },
                    local_today,
                    false,
                )
            })
            .collect()
    }

    fn generate_rule(
        &self,
        rule: &RecurrenceRule,
        range_start: NaiveDate,
        range_end: NaiveDate,
        refresh_pending: bool,
    ) -> Result<GenerationSummary, DomainError> {
        let dates = scheduled_dates(rule, range_start, range_end)?;
        if dates.is_empty() {
            return Ok(GenerationSummary {
                rule_id: rule.id.clone(),
                scheduled_count: 0,
                affected_count: 0,
            });
        }
        let template = TaskRepository::new(self.database)
            .get(&rule.task_template_id)?
            .ok_or_else(|| task_template_not_found(&rule.task_template_id))?;
        let instances = build_instances(rule, &dates, &template.title, &template.project_id)?;
        let affected_count = RecurrenceRepository::new(self.database)
            .upsert_instances(&instances, refresh_pending)?;
        Ok(GenerationSummary {
            rule_id: rule.id.clone(),
            scheduled_count: dates.len(),
            affected_count,
        })
    }
}

pub struct RecurrenceService<'a> {
    database: &'a Database,
}

impl<'a> RecurrenceService<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn get_rule(&self, rule_id: &str) -> Result<RecurrenceRule, DomainError> {
        self.require_rule(rule_id)
    }

    pub fn create_rule(
        &self,
        rule: RecurrenceRule,
        range_start: NaiveDate,
        range_end: NaiveDate,
    ) -> Result<GenerationSummary, DomainError> {
        if range_start > range_end {
            return Err(scheduler_error(
                "RECURRENCE_RANGE_INVALID",
                "generation end date must be on or after start date",
                Some("rangeEnd"),
            ));
        }
        rule.validate()?;
        if rule.version != 1 {
            return Err(scheduler_error(
                "RECURRENCE_VERSION_INVALID",
                "a new rule must start at version one",
                Some("version"),
            ));
        }
        let template = TaskRepository::new(self.database)
            .get(&rule.task_template_id)?
            .ok_or_else(|| task_template_not_found(&rule.task_template_id))?;
        let dates = scheduled_dates(&rule, range_start, range_end)?;
        let instances = build_instances(&rule, &dates, &template.title, &template.project_id)?;
        let affected_count = RecurrenceRepository::new(self.database)
            .insert_rule_and_instances(&rule, &instances)?;
        Ok(GenerationSummary {
            rule_id: rule.id,
            scheduled_count: dates.len(),
            affected_count,
        })
    }

    pub fn complete_instance(&self, instance_id: &str) -> Result<TaskInstanceRecord, DomainError> {
        let instance = self.require_instance(instance_id)?;
        if instance.status == TaskInstanceStatus::Completed {
            return Ok(instance);
        }
        self.require_pending(&instance)?;
        let now = now();
        if !RecurrenceRepository::new(self.database).set_instance_status(
            instance_id,
            TaskInstanceStatus::Completed,
            Some(&now),
            &now,
        )? {
            return Err(instance_not_actionable(instance_id));
        }
        self.require_instance(instance_id)
    }

    pub fn skip_instance(&self, instance_id: &str) -> Result<TaskInstanceRecord, DomainError> {
        let instance = self.require_instance(instance_id)?;
        if instance.status == TaskInstanceStatus::Skipped {
            return Ok(instance);
        }
        self.require_pending(&instance)?;
        let now = now();
        if !RecurrenceRepository::new(self.database).set_instance_status(
            instance_id,
            TaskInstanceStatus::Skipped,
            None,
            &now,
        )? {
            return Err(instance_not_actionable(instance_id));
        }
        self.require_instance(instance_id)
    }

    pub fn delay_instance_today(
        &self,
        instance_id: &str,
        local_time: &str,
    ) -> Result<TaskInstanceRecord, DomainError> {
        let instance = self.require_instance(instance_id)?;
        self.require_pending(&instance)?;
        let rule = self.require_rule(&instance.recurrence_rule_id)?;
        let mut changed_rule = rule;
        changed_rule.local_time = Some(local_time.to_string());
        changed_rule.validate()?;
        let date = parse_local_date(&instance.scheduled_date, "scheduledDate")?;
        let target = scheduled_at(&changed_rule, date)?.ok_or_else(|| {
            scheduler_error(
                "INSTANCE_TIME_INVALID",
                "a delayed instance requires a local time",
                Some("localTime"),
            )
        })?;
        if let Some(current) = &instance.scheduled_at {
            let current = chrono::DateTime::parse_from_rfc3339(current).map_err(|_| {
                scheduler_error(
                    "INSTANCE_DATA_INVALID",
                    "stored instance time is invalid",
                    None,
                )
            })?;
            let target_time = chrono::DateTime::parse_from_rfc3339(&target).map_err(|_| {
                scheduler_error("INSTANCE_TIME_INVALID", "delayed time is invalid", None)
            })?;
            if target_time <= current {
                return Err(scheduler_error(
                    "INSTANCE_DELAY_INVALID",
                    "delayed time must be later than the current time",
                    Some("localTime"),
                ));
            }
        }
        let now = now();
        if !RecurrenceRepository::new(self.database).delay_instance(
            instance_id,
            Some(&target),
            &now,
        )? {
            return Err(instance_not_actionable(instance_id));
        }
        self.require_instance(instance_id)
    }

    pub fn reschedule_instance_tomorrow(
        &self,
        instance_id: &str,
    ) -> Result<TaskInstanceRecord, DomainError> {
        let source = self.require_instance(instance_id)?;
        if source.status == TaskInstanceStatus::Rescheduled {
            return Ok(source);
        }
        self.require_pending(&source)?;
        let rule = self.require_rule(&source.recurrence_rule_id)?;
        let source_date = parse_local_date(&source.scheduled_date, "scheduledDate")?;
        let target_date = source_date.succ_opt().ok_or_else(|| {
            scheduler_error(
                "INSTANCE_DATE_INVALID",
                "tomorrow exceeds the supported date range",
                Some("scheduledDate"),
            )
        })?;
        let now = now();
        let target = TaskInstanceRecord {
            id: uuid::Uuid::new_v4().to_string(),
            recurrence_rule_id: source.recurrence_rule_id.clone(),
            rule_version: source.rule_version,
            scheduled_date: target_date.format("%Y-%m-%d").to_string(),
            scheduled_at: scheduled_at(&rule, target_date)?,
            snapshot_title: source.snapshot_title.clone(),
            snapshot_project_id: source.snapshot_project_id.clone(),
            status: TaskInstanceStatus::Pending,
            completed_at: None,
            source_instance_id: Some(source.id.clone()),
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        if !RecurrenceRepository::new(self.database).reschedule_instance(
            instance_id,
            &target,
            &now,
        )? {
            return Err(instance_not_actionable(instance_id));
        }
        self.require_instance(instance_id)
    }

    pub fn set_rule_status(
        &self,
        rule_id: &str,
        target: RecurrenceStatus,
    ) -> Result<RecurrenceRule, DomainError> {
        if target == RecurrenceStatus::Active {
            return Err(scheduler_error(
                "RECURRENCE_STATUS_INVALID",
                "this operation only supports pausing or ending a rule",
                Some("status"),
            ));
        }
        let mut rule = self.require_rule(rule_id)?;
        if rule.status == target {
            return Ok(rule);
        }
        if rule.status == RecurrenceStatus::Ended {
            return Err(scheduler_error(
                "RECURRENCE_STATUS_INVALID",
                "an ended rule cannot change status",
                Some("status"),
            ));
        }
        rule.status = target;
        rule.version = rule.version.checked_add(1).ok_or_else(|| {
            scheduler_error(
                "RECURRENCE_VERSION_INVALID",
                "rule version exceeds the supported range",
                Some("version"),
            )
        })?;
        if !RecurrenceRepository::new(self.database).update_rule(&rule)? {
            return Err(recurrence_not_found(rule_id));
        }
        Ok(rule)
    }

    pub fn apply_schedule_change(
        &self,
        proposed: RecurrenceRule,
        scope: RecurrenceChangeScope,
        range_end: NaiveDate,
    ) -> Result<GenerationSummary, DomainError> {
        proposed.validate()?;
        let current = self.require_rule(&proposed.id)?;
        if proposed.task_template_id != current.task_template_id {
            return Err(scheduler_error(
                "RECURRENCE_CHANGE_INVALID",
                "schedule changes must keep the task template",
                Some("taskTemplateId"),
            ));
        }
        match scope {
            RecurrenceChangeScope::ThisInstance { instance_id } => {
                let instance = self.require_instance(&instance_id)?;
                self.require_pending(&instance)?;
                if instance.recurrence_rule_id != proposed.id {
                    return Err(scheduler_error(
                        "RECURRENCE_CHANGE_INVALID",
                        "instance does not belong to the recurrence rule",
                        Some("instanceId"),
                    ));
                }
                let date = parse_local_date(&instance.scheduled_date, "scheduledDate")?;
                let target = scheduled_at(&proposed, date)?;
                let now = now();
                let affected = RecurrenceRepository::new(self.database).delay_instance(
                    &instance_id,
                    target.as_deref(),
                    &now,
                )?;
                if !affected {
                    return Err(instance_not_actionable(&instance_id));
                }
                Ok(GenerationSummary {
                    rule_id: proposed.id,
                    scheduled_count: 1,
                    affected_count: 1,
                })
            }
            RecurrenceChangeScope::Future { effective_on } => {
                let effective_on = parse_local_date(&effective_on, "effectiveOn")?;
                if effective_on > range_end {
                    return Err(scheduler_error(
                        "RECURRENCE_RANGE_INVALID",
                        "generation end date must be on or after the effective date",
                        Some("rangeEnd"),
                    ));
                }
                if proposed.version != current.version.saturating_add(1) {
                    return Err(scheduler_error(
                        "RECURRENCE_VERSION_INVALID",
                        "future changes must increment the rule version by one",
                        Some("version"),
                    ));
                }
                if proposed.status != current.status {
                    return Err(scheduler_error(
                        "RECURRENCE_CHANGE_INVALID",
                        "schedule changes must preserve rule status",
                        Some("status"),
                    ));
                }
                let dates = scheduled_dates(&proposed, effective_on, range_end)?;
                let template = TaskRepository::new(self.database)
                    .get(&proposed.task_template_id)?
                    .ok_or_else(|| task_template_not_found(&proposed.task_template_id))?;
                let instances =
                    build_instances(&proposed, &dates, &template.title, &template.project_id)?;
                let total_affected = RecurrenceRepository::new(self.database)
                    .update_rule_and_instances(&proposed, &instances)?;
                if total_affected == 0 {
                    return Err(recurrence_not_found(&proposed.id));
                }
                Ok(GenerationSummary {
                    rule_id: proposed.id,
                    scheduled_count: dates.len(),
                    affected_count: total_affected - 1,
                })
            }
        }
    }

    fn require_instance(&self, instance_id: &str) -> Result<TaskInstanceRecord, DomainError> {
        RecurrenceRepository::new(self.database)
            .get_instance(instance_id)?
            .ok_or_else(|| instance_not_found(instance_id))
    }

    fn require_rule(&self, rule_id: &str) -> Result<RecurrenceRule, DomainError> {
        RecurrenceRepository::new(self.database)
            .get_rule(rule_id)?
            .ok_or_else(|| recurrence_not_found(rule_id))
    }

    fn require_pending(&self, instance: &TaskInstanceRecord) -> Result<(), DomainError> {
        if instance.status != TaskInstanceStatus::Pending {
            return Err(instance_not_actionable(&instance.id));
        }
        Ok(())
    }
}

fn build_instances(
    rule: &RecurrenceRule,
    dates: &[NaiveDate],
    title: &str,
    project_id: &Option<String>,
) -> Result<Vec<TaskInstanceRecord>, DomainError> {
    let now = now();
    dates
        .iter()
        .map(|date| {
            Ok(TaskInstanceRecord {
                id: uuid::Uuid::new_v4().to_string(),
                recurrence_rule_id: rule.id.clone(),
                rule_version: rule.version,
                scheduled_date: date.format("%Y-%m-%d").to_string(),
                scheduled_at: scheduled_at(rule, *date)?,
                snapshot_title: title.to_string(),
                snapshot_project_id: project_id.clone(),
                status: TaskInstanceStatus::Pending,
                completed_at: None,
                source_instance_id: None,
                created_at: now.clone(),
                updated_at: now.clone(),
            })
        })
        .collect()
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn parse_local_date(value: &str, field: &str) -> Result<NaiveDate, DomainError> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| {
        scheduler_error(
            "RECURRENCE_DATE_INVALID",
            "date must use YYYY-MM-DD",
            Some(field),
        )
    })
}

fn scheduled_at(rule: &RecurrenceRule, date: NaiveDate) -> Result<Option<String>, DomainError> {
    let Some(local_time) = &rule.local_time else {
        return Ok(None);
    };
    let time = NaiveTime::parse_from_str(local_time, "%H:%M").map_err(|_| {
        scheduler_error(
            "RECURRENCE_LOCAL_TIME_INVALID",
            "local time must use HH:MM",
            Some("localTime"),
        )
    })?;
    let timezone = parse_timezone(&rule.timezone)?;
    let local = date.and_time(time);
    let resolved = resolve_local_datetime(timezone, local).ok_or_else(|| {
        scheduler_error(
            "RECURRENCE_LOCAL_TIME_INVALID",
            "local time could not be resolved in the configured timezone",
            Some("localTime"),
        )
    })?;
    Ok(Some(
        resolved
            .with_timezone(&Utc)
            .to_rfc3339_opts(SecondsFormat::Secs, true),
    ))
}

fn parse_timezone(value: &str) -> Result<Tz, DomainError> {
    value.parse::<Tz>().map_err(|_| {
        scheduler_error(
            "RECURRENCE_TIMEZONE_INVALID",
            "timezone must be a valid IANA identifier",
            Some("timezone"),
        )
    })
}

fn resolve_local_datetime(timezone: Tz, local: NaiveDateTime) -> Option<chrono::DateTime<Tz>> {
    for offset in 0..=180 {
        if let Some(value) = timezone
            .from_local_datetime(&(local + Duration::minutes(offset)))
            .earliest()
        {
            return Some(value);
        }
    }
    None
}

fn recurrence_not_found(rule_id: &str) -> DomainError {
    scheduler_error(
        "RECURRENCE_NOT_FOUND",
        &format!("recurrence rule {rule_id} was not found"),
        None,
    )
}

fn instance_not_found(instance_id: &str) -> DomainError {
    scheduler_error(
        "INSTANCE_NOT_FOUND",
        &format!("task instance {instance_id} was not found"),
        None,
    )
}

fn instance_not_actionable(instance_id: &str) -> DomainError {
    scheduler_error(
        "INSTANCE_NOT_ACTIONABLE",
        &format!("task instance {instance_id} has already been processed or started"),
        None,
    )
}

fn task_template_not_found(task_id: &str) -> DomainError {
    scheduler_error(
        "TASK_TEMPLATE_NOT_FOUND",
        &format!("task template {task_id} was not found"),
        None,
    )
}

fn scheduler_error(code: &str, message: &str, field: Option<&str>) -> DomainError {
    DomainError {
        code: code.into(),
        message: message.into(),
        field: field.map(str::to_string),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{
            recurrence::{RecurrencePattern, RecurrenceStatus},
            task::TaskInput,
        },
        repositories::recurrence_repository::RecurrenceRepository,
        services::task_service::TaskService,
    };
    use proptest::prelude::*;

    fn date(value: &str) -> NaiveDate {
        NaiveDate::parse_from_str(value, "%Y-%m-%d").unwrap()
    }

    fn setup(pattern: RecurrencePattern, starts_on: &str) -> (Database, RecurrenceRule) {
        let database = Database::open_in_memory().unwrap();
        let task = TaskService::new(&database)
            .create(
                TaskInput {
                    project_id: None,
                    title: "Review the roadmap".into(),
                    category: "work".into(),
                    priority: 2,
                    scheduled_date: None,
                    scheduled_time: None,
                    check_items: vec![],
                },
                date(starts_on),
            )
            .unwrap();
        let rule = RecurrenceRule {
            id: "rule-1".into(),
            task_template_id: task.task.id,
            pattern,
            local_time: Some("09:30".into()),
            timezone: "Asia/Shanghai".into(),
            starts_on: starts_on.into(),
            ends_on: None,
            status: RecurrenceStatus::Active,
            version: 1,
        };
        RecurrenceRepository::new(&database)
            .insert_rule(&rule)
            .unwrap();
        (database, rule)
    }

    fn recurrence_pattern_strategy() -> impl Strategy<Value = RecurrencePattern> {
        prop_oneof![
            (1u32..8).prop_map(|interval| RecurrencePattern::Daily { interval }),
            Just(RecurrencePattern::Weekdays),
            (1u32..5, prop::collection::btree_set(1u8..8, 1..8)).prop_map(
                |(interval, weekdays)| RecurrencePattern::Weekly {
                    interval,
                    weekdays: weekdays.into_iter().collect(),
                }
            ),
            (1u32..5, 1u8..32).prop_map(|(interval, day_of_month)| {
                RecurrencePattern::Monthly {
                    interval,
                    day_of_month,
                }
            }),
        ]
    }

    fn start_date_strategy() -> impl Strategy<Value = NaiveDate> {
        (2024i32..2032, 1u32..13, 1u32..29)
            .prop_map(|(year, month, day)| NaiveDate::from_ymd_opt(year, month, day).unwrap())
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        // Validates P1 and acceptance criteria R13.3 and R13.9.
        #[test]
        fn p1_repeated_generation_keeps_one_instance_per_rule_and_date(
            pattern in recurrence_pattern_strategy(),
            starts_on in start_date_strategy(),
            range_offset in 0i64..15,
            range_span in 0i64..120,
            repeated_runs in 2usize..7,
            local_time in prop_oneof![Just(None), (0u8..24, 0u8..60).prop_map(|(hour, minute)| Some(format!("{hour:02}:{minute:02}")))],
            timezone in prop::sample::select(vec!["UTC", "Asia/Shanghai", "America/New_York", "Europe/London"]),
        ) {
            let database = Database::open_in_memory().unwrap();
            let task = TaskService::new(&database)
                .create(
                    TaskInput {
                        project_id: None,
                        title: "Property task".into(),
                        category: "work".into(),
                        priority: 1,
                        scheduled_date: None,
                        scheduled_time: None,
                        check_items: vec![],
                    },
                    starts_on,
                )
                .unwrap();
            let rule = RecurrenceRule {
                id: "p1-rule".into(),
                task_template_id: task.task.id,
                pattern,
                local_time,
                timezone: timezone.into(),
                starts_on: starts_on.format("%Y-%m-%d").to_string(),
                ends_on: None,
                status: RecurrenceStatus::Active,
                version: 1,
            };
            RecurrenceRepository::new(&database).insert_rule(&rule).unwrap();
            let range_start = starts_on + Duration::days(range_offset);
            let range_end = range_start + Duration::days(range_span);
            let expected_dates = scheduled_dates(&rule, range_start, range_end).unwrap();
            let scheduler = RecurrenceScheduler::new(&database);

            for _ in 0..repeated_runs {
                scheduler
                    .run(GenerationTrigger::Startup, range_start, range_end)
                    .unwrap();
            }

            let instances = RecurrenceRepository::new(&database)
                .list_instances_for_rule(&rule.id)
                .unwrap();
            let unique_dates = instances
                .iter()
                .map(|instance| instance.scheduled_date.as_str())
                .collect::<std::collections::BTreeSet<_>>();

            prop_assert_eq!(instances.len(), expected_dates.len());
            prop_assert_eq!(unique_dates.len(), instances.len());
            prop_assert_eq!(
                unique_dates.into_iter().collect::<Vec<_>>(),
                expected_dates
                    .iter()
                    .map(|date| date.format("%Y-%m-%d").to_string())
                    .collect::<Vec<_>>()
            );
        }

        // Validates P3 and acceptance criteria R13.4, R13.6, and R13.12.
        #[test]
        fn p3_future_changes_preserve_processed_and_started_instances(
            initial_pattern in recurrence_pattern_strategy(),
            proposed_pattern in recurrence_pattern_strategy(),
            starts_on in start_date_strategy(),
            proposed_time in prop_oneof![Just(None), (0u8..24, 0u8..60).prop_map(|(hour, minute)| Some(format!("{hour:02}:{minute:02}")))],
            proposed_timezone in prop::sample::select(vec!["UTC", "Asia/Shanghai", "America/New_York", "Europe/London", "Australia/Sydney"]),
        ) {
            let starts_on_text = starts_on.format("%Y-%m-%d").to_string();
            let range_end = starts_on + Duration::days(400);
            let (database, rule) = setup(initial_pattern, &starts_on_text);
            RecurrenceScheduler::new(&database)
                .run(GenerationTrigger::Startup, starts_on, range_end)
                .unwrap();
            let repository = RecurrenceRepository::new(&database);
            let initial = repository.list_instances_for_rule(&rule.id).unwrap();
            prop_assert!(initial.len() >= 3);

            let service = RecurrenceService::new(&database);
            service.complete_instance(&initial[0].id).unwrap();
            service.skip_instance(&initial[1].id).unwrap();
            database
                .write(|tx| {
                    tx.execute(
                        "INSERT INTO focus_sessions(id, task_instance_id, planned_seconds, actual_seconds, interruption_count, completion_kind, started_at, ended_at, created_at) VALUES ('p3-session', ?1, 1500, 300, 0, 'cancelled', '2026-01-01T01:00:00Z', '2026-01-01T01:05:00Z', '2026-01-01T01:05:00Z')",
                        [&initial[2].id],
                    )?;
                    Ok(())
                })
                .unwrap();

            let protected_before = initial[..3]
                .iter()
                .map(|instance| repository.get_instance(&instance.id).unwrap().unwrap())
                .collect::<Vec<_>>();
            TaskService::new(&database)
                .update(
                    &rule.task_template_id,
                    TaskInput {
                        project_id: None,
                        title: "Changed template title".into(),
                        category: "work".into(),
                        priority: 2,
                        scheduled_date: None,
                        scheduled_time: None,
                        check_items: vec![],
                    },
                    starts_on,
                )
                .unwrap();

            let mut proposed = rule;
            proposed.pattern = proposed_pattern;
            proposed.local_time = proposed_time;
            proposed.timezone = proposed_timezone.into();
            proposed.version = 2;
            service
                .apply_schedule_change(
                    proposed,
                    RecurrenceChangeScope::Future {
                        effective_on: starts_on_text,
                    },
                    range_end,
                )
                .unwrap();

            let protected_after = protected_before
                .iter()
                .map(|instance| repository.get_instance(&instance.id).unwrap().unwrap())
                .collect::<Vec<_>>();
            prop_assert_eq!(protected_after, protected_before);
        }
    }

    #[test]
    fn creates_a_rule_and_its_initial_instances_atomically() {
        let database = Database::open_in_memory().unwrap();
        let task = TaskService::new(&database)
            .create(
                TaskInput {
                    project_id: None,
                    title: "Daily review".into(),
                    category: "work".into(),
                    priority: 2,
                    scheduled_date: None,
                    scheduled_time: None,
                    check_items: vec![],
                },
                date("2026-07-20"),
            )
            .unwrap();
        let rule = RecurrenceRule {
            id: "created-rule".into(),
            task_template_id: task.task.id,
            pattern: RecurrencePattern::Daily { interval: 1 },
            local_time: Some("09:00".into()),
            timezone: "Asia/Shanghai".into(),
            starts_on: "2026-07-20".into(),
            ends_on: None,
            status: RecurrenceStatus::Active,
            version: 1,
        };

        let summary = RecurrenceService::new(&database)
            .create_rule(rule.clone(), date("2026-07-20"), date("2026-07-22"))
            .unwrap();

        assert_eq!(summary.scheduled_count, 3);
        assert_eq!(summary.affected_count, 3);
        assert_eq!(
            RecurrenceService::new(&database)
                .get_rule("created-rule")
                .unwrap(),
            rule
        );
        assert_eq!(
            RecurrenceRepository::new(&database)
                .list_instances_for_rule("created-rule")
                .unwrap()
                .len(),
            3
        );
    }

    #[test]
    fn generates_month_end_instances_idempotently() {
        let (database, _) = setup(
            RecurrencePattern::Monthly {
                interval: 1,
                day_of_month: 31,
            },
            "2026-01-31",
        );
        let scheduler = RecurrenceScheduler::new(&database);

        let first = scheduler
            .run(
                GenerationTrigger::Startup,
                date("2026-01-01"),
                date("2026-04-30"),
            )
            .unwrap();
        let second = scheduler
            .run(
                GenerationTrigger::Startup,
                date("2026-01-01"),
                date("2026-04-30"),
            )
            .unwrap();
        let instances = RecurrenceRepository::new(&database)
            .list_instances_for_rule("rule-1")
            .unwrap();

        assert_eq!(first[0].affected_count, 4);
        assert_eq!(second[0].affected_count, 0);
        assert_eq!(
            instances
                .iter()
                .map(|instance| instance.scheduled_date.as_str())
                .collect::<Vec<_>>(),
            ["2026-01-31", "2026-02-28", "2026-03-31", "2026-04-30"]
        );
        assert!(instances
            .iter()
            .all(|instance| instance.snapshot_title == "Review the roadmap"));
    }

    #[test]
    fn backfills_missing_days_and_refreshes_only_pending_instances() {
        let (database, mut rule) = setup(RecurrencePattern::Daily { interval: 1 }, "2026-07-20");
        let scheduler = RecurrenceScheduler::new(&database);
        scheduler
            .run(
                GenerationTrigger::Startup,
                date("2026-07-20"),
                date("2026-07-20"),
            )
            .unwrap();
        let backfill = scheduler
            .run(
                GenerationTrigger::DayBoundary,
                date("2026-07-20"),
                date("2026-07-22"),
            )
            .unwrap();
        assert_eq!(backfill[0].affected_count, 2);

        database
            .write(|tx| {
                tx.execute(
                    "UPDATE task_instances SET status = 'completed', completed_at = '2026-07-20T02:00:00Z' WHERE recurrence_rule_id = 'rule-1' AND scheduled_date = '2026-07-20'",
                    [],
                )?;
                Ok(())
            })
            .unwrap();
        rule.version = 2;
        rule.timezone = "UTC".into();
        RecurrenceRepository::new(&database)
            .update_rule(&rule)
            .unwrap();
        scheduler
            .run(
                GenerationTrigger::TimezoneChanged {
                    rule_id: rule.id.clone(),
                },
                date("2026-07-20"),
                date("2026-07-22"),
            )
            .unwrap();

        let instances = RecurrenceRepository::new(&database)
            .list_instances_for_rule("rule-1")
            .unwrap();
        assert_eq!(instances.len(), 3);
        assert_eq!(instances[0].status, TaskInstanceStatus::Completed);
        assert_eq!(instances[0].rule_version, 1);
        assert!(instances[1..].iter().all(|instance| {
            instance.rule_version == 2
                && instance
                    .scheduled_at
                    .as_deref()
                    .unwrap()
                    .ends_with("09:30:00Z")
        }));
    }

    #[test]
    fn automatic_reconciliation_backfills_open_rules_and_uses_each_rule_timezone() {
        let (database, _) = setup(RecurrencePattern::Daily { interval: 1 }, "2026-07-20");
        let scheduler = RecurrenceScheduler::new(&database);
        let utc_now = Utc.with_ymd_and_hms(2026, 7, 21, 16, 30, 0).unwrap();

        let startup = scheduler
            .reconcile_active_to_utc_now(GenerationTrigger::Startup, utc_now)
            .unwrap();
        let repeated = scheduler
            .reconcile_active_to_utc_now(GenerationTrigger::Startup, utc_now)
            .unwrap();

        assert_eq!(startup[0].affected_count, 3);
        assert_eq!(repeated[0].affected_count, 0);
        assert_eq!(
            RecurrenceRepository::new(&database)
                .list_instances_for_rule("rule-1")
                .unwrap()
                .into_iter()
                .map(|instance| instance.scheduled_date)
                .collect::<Vec<_>>(),
            ["2026-07-20", "2026-07-21", "2026-07-22"]
        );
    }

    #[test]
    fn day_boundary_reconciliation_only_generates_the_current_local_day() {
        let (database, _) = setup(RecurrencePattern::Daily { interval: 1 }, "2026-07-20");
        let scheduler = RecurrenceScheduler::new(&database);
        let utc_now = Utc.with_ymd_and_hms(2026, 7, 21, 16, 30, 0).unwrap();

        let summary = scheduler
            .reconcile_active_to_utc_now(GenerationTrigger::DayBoundary, utc_now)
            .unwrap();

        assert_eq!(summary[0].affected_count, 1);
        assert_eq!(
            RecurrenceRepository::new(&database)
                .list_instances_for_rule("rule-1")
                .unwrap()[0]
                .scheduled_date,
            "2026-07-22"
        );
    }

    #[test]
    fn normalizes_nonexistent_dst_times_to_the_first_valid_minute() {
        let (_, mut rule) = setup(RecurrencePattern::Daily { interval: 1 }, "2026-03-08");
        rule.local_time = Some("02:30".into());
        rule.timezone = "America/New_York".into();

        assert_eq!(
            scheduled_at(&rule, date("2026-03-08")).unwrap().as_deref(),
            Some("2026-03-08T07:00:00Z")
        );
    }

    #[test]
    fn reports_invalid_ranges_and_unknown_rules() {
        let (database, _) = setup(RecurrencePattern::Daily { interval: 1 }, "2026-07-20");
        let scheduler = RecurrenceScheduler::new(&database);
        assert_eq!(
            scheduler
                .run(
                    GenerationTrigger::Startup,
                    date("2026-07-21"),
                    date("2026-07-20"),
                )
                .unwrap_err()
                .code,
            "RECURRENCE_RANGE_INVALID"
        );
        assert_eq!(
            scheduler
                .run(
                    GenerationTrigger::RuleChanged {
                        rule_id: "missing".into(),
                    },
                    date("2026-07-20"),
                    date("2026-07-20"),
                )
                .unwrap_err()
                .code,
            "RECURRENCE_NOT_FOUND"
        );
    }

    #[test]
    fn completes_and_skips_instances_idempotently() {
        let (database, _) = setup(RecurrencePattern::Daily { interval: 1 }, "2026-07-20");
        RecurrenceScheduler::new(&database)
            .run(
                GenerationTrigger::Startup,
                date("2026-07-20"),
                date("2026-07-21"),
            )
            .unwrap();
        let instances = RecurrenceRepository::new(&database)
            .list_instances_for_rule("rule-1")
            .unwrap();
        let service = RecurrenceService::new(&database);

        let completed = service.complete_instance(&instances[0].id).unwrap();
        assert_eq!(completed.status, TaskInstanceStatus::Completed);
        assert!(completed.completed_at.is_some());
        assert_eq!(
            service.complete_instance(&instances[0].id).unwrap().status,
            TaskInstanceStatus::Completed
        );

        let skipped = service.skip_instance(&instances[1].id).unwrap();
        assert_eq!(skipped.status, TaskInstanceStatus::Skipped);
        assert!(skipped.completed_at.is_none());
        assert_eq!(
            service.skip_instance(&instances[1].id).unwrap().status,
            TaskInstanceStatus::Skipped
        );
        assert_eq!(
            service
                .complete_instance(&instances[1].id)
                .unwrap_err()
                .code,
            "INSTANCE_NOT_ACTIONABLE"
        );
    }

    #[test]
    fn delays_today_and_reschedules_to_the_unique_tomorrow_instance() {
        let (database, _) = setup(RecurrencePattern::Daily { interval: 1 }, "2026-07-20");
        RecurrenceScheduler::new(&database)
            .run(
                GenerationTrigger::Startup,
                date("2026-07-20"),
                date("2026-07-21"),
            )
            .unwrap();
        let repository = RecurrenceRepository::new(&database);
        let instances = repository.list_instances_for_rule("rule-1").unwrap();
        let service = RecurrenceService::new(&database);

        let delayed = service
            .delay_instance_today(&instances[0].id, "10:30")
            .unwrap();
        assert_eq!(delayed.scheduled_date, "2026-07-20");
        assert_eq!(
            delayed.scheduled_at.as_deref(),
            Some("2026-07-20T02:30:00Z")
        );
        assert_eq!(
            service
                .delay_instance_today(&instances[0].id, "09:30")
                .unwrap_err()
                .code,
            "INSTANCE_DELAY_INVALID"
        );

        let source = service
            .reschedule_instance_tomorrow(&instances[0].id)
            .unwrap();
        assert_eq!(source.status, TaskInstanceStatus::Rescheduled);
        let after = repository.list_instances_for_rule("rule-1").unwrap();
        assert_eq!(after.len(), 2);
        assert_eq!(after[1].status, TaskInstanceStatus::Pending);
        assert_eq!(
            after[1].source_instance_id.as_deref(),
            Some(source.id.as_str())
        );
    }

    #[test]
    fn pauses_and_ends_rules_without_changing_existing_instances() {
        let (database, _) = setup(RecurrencePattern::Daily { interval: 1 }, "2026-07-20");
        RecurrenceScheduler::new(&database)
            .run(
                GenerationTrigger::Startup,
                date("2026-07-20"),
                date("2026-07-21"),
            )
            .unwrap();
        let service = RecurrenceService::new(&database);

        let paused = service
            .set_rule_status("rule-1", RecurrenceStatus::Paused)
            .unwrap();
        assert_eq!(paused.status, RecurrenceStatus::Paused);
        assert_eq!(paused.version, 2);
        let ended = service
            .set_rule_status("rule-1", RecurrenceStatus::Ended)
            .unwrap();
        assert_eq!(ended.status, RecurrenceStatus::Ended);
        assert_eq!(ended.version, 3);
        assert_eq!(
            service
                .set_rule_status("rule-1", RecurrenceStatus::Paused)
                .unwrap_err()
                .code,
            "RECURRENCE_STATUS_INVALID"
        );
        assert_eq!(
            RecurrenceRepository::new(&database)
                .list_instances_for_rule("rule-1")
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn applies_one_instance_or_future_schedule_changes_without_touching_history() {
        let (database, rule) = setup(RecurrencePattern::Daily { interval: 1 }, "2026-07-20");
        RecurrenceScheduler::new(&database)
            .run(
                GenerationTrigger::Startup,
                date("2026-07-20"),
                date("2026-07-22"),
            )
            .unwrap();
        let repository = RecurrenceRepository::new(&database);
        let initial = repository.list_instances_for_rule("rule-1").unwrap();
        let service = RecurrenceService::new(&database);
        service.complete_instance(&initial[0].id).unwrap();
        database
            .write(|tx| {
                tx.execute(
                    "INSERT INTO focus_sessions(id, task_instance_id, planned_seconds, actual_seconds, interruption_count, completion_kind, started_at, ended_at, created_at) VALUES ('session-1', ?1, 1500, 300, 0, 'cancelled', '2026-07-21T01:00:00Z', '2026-07-21T01:05:00Z', '2026-07-21T01:05:00Z')",
                    [&initial[1].id],
                )?;
                Ok(())
            })
            .unwrap();

        let mut changed = rule.clone();
        changed.local_time = Some("11:00".into());
        changed.timezone = "UTC".into();
        changed.version = 2;
        let summary = service
            .apply_schedule_change(
                changed.clone(),
                RecurrenceChangeScope::Future {
                    effective_on: "2026-07-20".into(),
                },
                date("2026-07-22"),
            )
            .unwrap();
        assert_eq!(summary.affected_count, 1);
        let after_future = repository.list_instances_for_rule("rule-1").unwrap();
        assert_eq!(after_future[0].rule_version, 1);
        assert_eq!(after_future[0].status, TaskInstanceStatus::Completed);
        assert_eq!(after_future[1].rule_version, 1);
        assert_eq!(after_future[2].rule_version, 2);
        assert_eq!(
            after_future[2].scheduled_at.as_deref(),
            Some("2026-07-22T11:00:00Z")
        );

        changed.local_time = Some("12:00".into());
        service
            .apply_schedule_change(
                changed,
                RecurrenceChangeScope::ThisInstance {
                    instance_id: after_future[2].id.clone(),
                },
                date("2026-07-22"),
            )
            .unwrap();
        let only_instance = repository
            .get_instance(&after_future[2].id)
            .unwrap()
            .unwrap();
        assert_eq!(
            only_instance.scheduled_at.as_deref(),
            Some("2026-07-22T12:00:00Z")
        );
        assert_eq!(
            repository
                .get_rule("rule-1")
                .unwrap()
                .unwrap()
                .local_time
                .as_deref(),
            Some("11:00")
        );
    }
}

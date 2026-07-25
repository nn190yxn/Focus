use std::{cmp::Ordering, collections::HashSet};

use chrono::{DateTime, NaiveDate};
use chrono_tz::Tz;

use crate::{
    domain::today::{TodayDigest, TodayDigestItem, TodaySourceKind},
    repositories::{
        database::Database,
        today_repository::{TodayCandidate, TodayRepository},
    },
    DomainError,
};

pub struct TodayService<'a> {
    database: &'a Database,
}

impl<'a> TodayService<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn get_digest(&self, date: &str) -> Result<TodayDigest, DomainError> {
        let digest_date = NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|_| {
            today_error(
                "TODAY_DATE_INVALID",
                "digest date must use YYYY-MM-DD",
                Some("date"),
            )
        })?;
        let candidates = TodayRepository::new(self.database).list_candidates(date)?;
        let mut identities = HashSet::new();
        let mut items = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            let identity = (candidate.source_kind, candidate.source_id.clone());
            if identities.insert(identity) {
                items.push(to_digest_item(candidate, digest_date)?);
            }
        }
        items.sort_by(compare_items);
        Ok(TodayDigest {
            date: date.to_string(),
            items,
        })
    }
}

fn to_digest_item(
    candidate: TodayCandidate,
    digest_date: NaiveDate,
) -> Result<TodayDigestItem, DomainError> {
    let scheduled_date =
        NaiveDate::parse_from_str(&candidate.scheduled_date, "%Y-%m-%d").map_err(|_| {
            today_error(
                "TODAY_DATA_INVALID",
                "stored scheduled date is invalid",
                None,
            )
        })?;
    let scheduled_time = match (candidate.scheduled_time, candidate.scheduled_at) {
        (Some(time), _) => Some(time),
        (None, Some(timestamp)) => Some(instance_local_time(
            &timestamp,
            candidate.timezone.as_deref(),
        )?),
        (None, None) => None,
    };
    Ok(TodayDigestItem {
        source_kind: candidate.source_kind,
        source_id: candidate.source_id,
        item_kind: candidate.item_kind,
        recurrence_rule_id: candidate.recurrence_rule_id,
        title: candidate.title,
        category: candidate.category,
        priority: candidate.priority,
        scheduled_date: candidate.scheduled_date,
        scheduled_time,
        status: candidate.status,
        completed_at: candidate.completed_at,
        project: candidate.project,
        is_overdue: scheduled_date < digest_date,
        created_at: candidate.created_at,
    })
}

fn instance_local_time(timestamp: &str, timezone: Option<&str>) -> Result<String, DomainError> {
    let timestamp = DateTime::parse_from_rfc3339(timestamp).map_err(|_| {
        today_error(
            "TODAY_DATA_INVALID",
            "stored instance timestamp is invalid",
            None,
        )
    })?;
    let timezone = timezone
        .ok_or_else(|| today_error("TODAY_DATA_INVALID", "instance timezone is missing", None))?
        .parse::<Tz>()
        .map_err(|_| {
            today_error(
                "TODAY_DATA_INVALID",
                "stored instance timezone is invalid",
                None,
            )
        })?;
    Ok(timestamp
        .with_timezone(&timezone)
        .format("%H:%M")
        .to_string())
}

fn compare_items(left: &TodayDigestItem, right: &TodayDigestItem) -> Ordering {
    right
        .is_overdue
        .cmp(&left.is_overdue)
        .then_with(|| match (&left.scheduled_time, &right.scheduled_time) {
            (Some(left), Some(right)) => left.cmp(right),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        })
        .then_with(|| right.priority.cmp(&left.priority))
        .then_with(|| left.created_at.cmp(&right.created_at))
        .then_with(|| source_rank(left.source_kind).cmp(&source_rank(right.source_kind)))
        .then_with(|| left.source_id.cmp(&right.source_id))
}

fn source_rank(source: TodaySourceKind) -> u8 {
    match source {
        TodaySourceKind::Task => 0,
        TodaySourceKind::RecurringInstance => 1,
    }
}

fn today_error(code: &str, message: &str, field: Option<&str>) -> DomainError {
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
            recurrence::{RecurrencePattern, RecurrenceRule, RecurrenceStatus, TaskInstanceStatus},
            today::{TodayItemKind, TodayItemStatus},
        },
        repositories::{
            recurrence_repository::{RecurrenceRepository, TaskInstanceRecord},
            task_repository::{TaskRecord, TaskRepository},
        },
    };
    use proptest::prelude::*;

    const NOW: &str = "2026-07-20T00:00:00Z";

    #[derive(Debug, Clone)]
    struct DigestSourceSpec {
        kind: u8,
        date_offset: i64,
        status: u8,
    }

    fn digest_date_strategy() -> impl Strategy<Value = NaiveDate> {
        (2024i32..2032, 1u32..13, 1u32..29)
            .prop_map(|(year, month, day)| NaiveDate::from_ymd_opt(year, month, day).unwrap())
    }

    fn digest_source_strategy() -> impl Strategy<Value = Vec<DigestSourceSpec>> {
        prop::collection::vec(
            (0u8..3, -3i64..4, 0u8..4).prop_map(|(kind, date_offset, status)| DigestSourceSpec {
                kind,
                date_offset,
                status,
            }),
            0..25,
        )
    }

    fn task(
        id: &str,
        project_id: Option<&str>,
        priority: i64,
        scheduled_date: &str,
        scheduled_time: Option<&str>,
        status: &str,
    ) -> TaskRecord {
        TaskRecord {
            id: id.into(),
            project_id: project_id.map(str::to_string),
            title: format!("Task {id}"),
            category: "work".into(),
            priority,
            scheduled_date: Some(scheduled_date.into()),
            scheduled_time: scheduled_time.map(str::to_string),
            status: status.into(),
            completed_at: (status == "completed").then(|| "2026-07-20T10:00:00Z".into()),
            created_at: NOW.into(),
            updated_at: NOW.into(),
        }
    }

    fn insert_rule_and_instance(
        database: &Database,
        rule_id: &str,
        task_id: &str,
        instance_id: &str,
        scheduled_date: &str,
        scheduled_at: Option<&str>,
        status: TaskInstanceStatus,
    ) {
        RecurrenceRepository::new(database)
            .insert_rule(&RecurrenceRule {
                id: rule_id.into(),
                task_template_id: task_id.into(),
                pattern: RecurrencePattern::Daily { interval: 1 },
                local_time: Some("09:00".into()),
                timezone: "Asia/Shanghai".into(),
                starts_on: "2026-07-01".into(),
                ends_on: None,
                status: RecurrenceStatus::Active,
                version: 1,
            })
            .unwrap();
        RecurrenceRepository::new(database)
            .upsert_instances(
                &[TaskInstanceRecord {
                    id: instance_id.into(),
                    recurrence_rule_id: rule_id.into(),
                    rule_version: 1,
                    scheduled_date: scheduled_date.into(),
                    scheduled_at: scheduled_at.map(str::to_string),
                    snapshot_title: format!("Instance {instance_id}"),
                    snapshot_project_id: None,
                    status,
                    completed_at: (status == TaskInstanceStatus::Completed)
                        .then(|| "2026-07-20T11:00:00Z".into()),
                    source_instance_id: None,
                    created_at: NOW.into(),
                    updated_at: NOW.into(),
                }],
                false,
            )
            .unwrap();
    }

    fn insert_digest_source(
        database: &Database,
        digest_date: NaiveDate,
        index: usize,
        spec: &DigestSourceSpec,
    ) -> Option<(TodaySourceKind, String, TodayItemKind, bool)> {
        let source_id = format!("property-source-{index}");
        let scheduled_date = digest_date + chrono::Duration::days(spec.date_offset);
        let scheduled_date_text = scheduled_date.format("%Y-%m-%d").to_string();
        let eligible_status = spec.status <= 1;
        let eligible_date = spec.date_offset == 0 || (spec.date_offset < 0 && spec.status == 0);

        if spec.kind < 2 {
            let status = match spec.status {
                0 => "pending",
                1 => "completed",
                _ => "removed",
            };
            TaskRepository::new(database)
                .insert(&TaskRecord {
                    id: source_id.clone(),
                    project_id: (spec.kind == 1).then(|| "property-project".into()),
                    title: format!("Property task {index}"),
                    category: "work".into(),
                    priority: (index % 4) as i64,
                    scheduled_date: Some(scheduled_date_text),
                    scheduled_time: None,
                    status: status.into(),
                    completed_at: (status == "completed").then(|| NOW.into()),
                    created_at: NOW.into(),
                    updated_at: NOW.into(),
                })
                .unwrap();

            return (eligible_status && eligible_date).then_some({
                (
                    TodaySourceKind::Task,
                    source_id,
                    if spec.kind == 0 {
                        TodayItemKind::OrdinaryTask
                    } else {
                        TodayItemKind::ProjectTask
                    },
                    spec.date_offset < 0,
                )
            });
        }

        let template_id = format!("property-template-{index}");
        TaskRepository::new(database)
            .insert(&TaskRecord {
                id: template_id.clone(),
                project_id: None,
                title: format!("Property template {index}"),
                category: "study".into(),
                priority: (index % 4) as i64,
                scheduled_date: Some(digest_date.format("%Y-%m-%d").to_string()),
                scheduled_time: None,
                status: "pending".into(),
                completed_at: None,
                created_at: NOW.into(),
                updated_at: NOW.into(),
            })
            .unwrap();
        let instance_status = match spec.status {
            0 => TaskInstanceStatus::Pending,
            1 => TaskInstanceStatus::Completed,
            2 => TaskInstanceStatus::Skipped,
            _ => TaskInstanceStatus::Rescheduled,
        };
        let rule_id = format!("property-rule-{index}");
        RecurrenceRepository::new(database)
            .insert_rule(&RecurrenceRule {
                id: rule_id.clone(),
                task_template_id: template_id,
                pattern: RecurrencePattern::Daily { interval: 1 },
                local_time: None,
                timezone: "UTC".into(),
                starts_on: scheduled_date.format("%Y-%m-%d").to_string(),
                ends_on: None,
                status: RecurrenceStatus::Active,
                version: 1,
            })
            .unwrap();
        RecurrenceRepository::new(database)
            .upsert_instances(
                &[TaskInstanceRecord {
                    id: source_id.clone(),
                    recurrence_rule_id: rule_id,
                    rule_version: 1,
                    scheduled_date: scheduled_date.format("%Y-%m-%d").to_string(),
                    scheduled_at: None,
                    snapshot_title: format!("Property instance {index}"),
                    snapshot_project_id: None,
                    status: instance_status,
                    completed_at: (instance_status == TaskInstanceStatus::Completed)
                        .then(|| NOW.into()),
                    source_instance_id: None,
                    created_at: NOW.into(),
                    updated_at: NOW.into(),
                }],
                false,
            )
            .unwrap();

        (eligible_status && eligible_date).then_some({
            (
                TodaySourceKind::RecurringInstance,
                source_id,
                TodayItemKind::RecurringInstance,
                spec.date_offset < 0,
            )
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        // Validates P4 and acceptance criteria R13.7 and R13.9.
        #[test]
        fn p4_digest_contains_every_eligible_source_once(
            digest_date in digest_date_strategy(),
            generated_sources in digest_source_strategy(),
        ) {
            let database = Database::open_in_memory().unwrap();
            database
                .write(|tx| {
                    tx.execute(
                        "INSERT INTO projects(id, name, color, icon, status, started_on, created_at, updated_at) VALUES ('property-project', 'Property project', 'blue', 'folder', 'active', '2024-01-01', ?1, ?1)",
                        [NOW],
                    )?;
                    Ok(())
                })
                .unwrap();
            let mut sources = vec![
                DigestSourceSpec { kind: 0, date_offset: 0, status: 0 },
                DigestSourceSpec { kind: 1, date_offset: 0, status: 0 },
                DigestSourceSpec { kind: 2, date_offset: 0, status: 0 },
                DigestSourceSpec { kind: 0, date_offset: -1, status: 0 },
            ];
            sources.extend(generated_sources);
            let expected = sources
                .iter()
                .enumerate()
                .filter_map(|(index, spec)| insert_digest_source(&database, digest_date, index, spec))
                .collect::<Vec<_>>();
            let digest = TodayService::new(&database)
                .get_digest(&digest_date.format("%Y-%m-%d").to_string())
                .unwrap();
            let identities = digest
                .items
                .iter()
                .map(|item| (item.source_kind, item.source_id.clone()))
                .collect::<HashSet<_>>();

            prop_assert_eq!(digest.items.len(), identities.len());
            prop_assert_eq!(digest.items.len(), expected.len());
            for (source_kind, source_id, item_kind, is_overdue) in expected {
                let item = digest
                    .items
                    .iter()
                    .find(|item| item.source_kind == source_kind && item.source_id == source_id)
                    .unwrap();
                prop_assert_eq!(item.item_kind, item_kind);
                prop_assert_eq!(item.is_overdue, is_overdue);
            }
        }
    }

    #[test]
    fn aggregates_sources_excludes_templates_and_sorts_stably() {
        let database = Database::open_in_memory().unwrap();
        database
            .write(|tx| {
                tx.execute(
                    "INSERT INTO projects(id, name, color, icon, status, started_on, created_at, updated_at) VALUES ('project-1', 'Launch', 'blue', 'rocket', 'paused', '2026-07-01', ?1, ?1)",
                    [NOW],
                )?;
                Ok(())
            })
            .unwrap();
        let repository = TaskRepository::new(&database);
        for record in [
            task("overdue-task", None, 0, "2026-07-19", None, "pending"),
            task("task-b", None, 1, "2026-07-20", Some("09:00"), "pending"),
            task(
                "task-a",
                Some("project-1"),
                3,
                "2026-07-20",
                Some("09:00"),
                "pending",
            ),
            task("completed-task", None, 0, "2026-07-20", None, "completed"),
            task(
                "future-task",
                None,
                3,
                "2026-07-21",
                Some("08:00"),
                "pending",
            ),
            task(
                "template-1",
                None,
                2,
                "2026-07-20",
                Some("07:00"),
                "pending",
            ),
            task(
                "template-2",
                None,
                2,
                "2026-07-20",
                Some("07:00"),
                "pending",
            ),
        ] {
            repository.insert(&record).unwrap();
        }
        insert_rule_and_instance(
            &database,
            "rule-1",
            "template-1",
            "overdue-instance",
            "2026-07-19",
            Some("2026-07-19T00:00:00Z"),
            TaskInstanceStatus::Pending,
        );
        insert_rule_and_instance(
            &database,
            "rule-2",
            "template-2",
            "today-instance",
            "2026-07-20",
            Some("2026-07-20T10:00:00Z"),
            TaskInstanceStatus::Completed,
        );

        let first = TodayService::new(&database)
            .get_digest("2026-07-20")
            .unwrap();
        let second = TodayService::new(&database)
            .get_digest("2026-07-20")
            .unwrap();
        assert_eq!(first, second);
        assert_eq!(
            first
                .items
                .iter()
                .map(|item| item.source_id.as_str())
                .collect::<Vec<_>>(),
            [
                "overdue-instance",
                "overdue-task",
                "task-a",
                "task-b",
                "today-instance",
                "completed-task"
            ]
        );
        assert_eq!(first.items[0].scheduled_time.as_deref(), Some("08:00"));
        assert_eq!(first.items[2].item_kind, TodayItemKind::ProjectTask);
        assert_eq!(first.items[2].project.as_ref().unwrap().status, "paused");
        assert_eq!(first.items[4].status, TodayItemStatus::Completed);
        assert!(first
            .items
            .iter()
            .all(|item| item.source_id != "template-1" && item.source_id != "template-2"));
    }

    #[test]
    fn rejects_invalid_dates_and_ignores_processed_instance_sources() {
        let database = Database::open_in_memory().unwrap();
        assert_eq!(
            TodayService::new(&database)
                .get_digest("2026/07/20")
                .unwrap_err()
                .code,
            "TODAY_DATE_INVALID"
        );
        let repository = TaskRepository::new(&database);
        repository
            .insert(&task(
                "template-skipped",
                None,
                1,
                "2026-07-20",
                None,
                "pending",
            ))
            .unwrap();
        insert_rule_and_instance(
            &database,
            "rule-skipped",
            "template-skipped",
            "skipped-instance",
            "2026-07-20",
            None,
            TaskInstanceStatus::Skipped,
        );

        assert!(TodayService::new(&database)
            .get_digest("2026-07-20")
            .unwrap()
            .items
            .is_empty());
    }
}

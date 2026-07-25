use std::{cmp::Ordering, collections::BTreeMap};

use chrono::{DateTime, Datelike, Duration, LocalResult, NaiveDate, TimeZone, Utc, Weekday};
use chrono_tz::Tz;

use crate::{
    domain::{
        calendar::{
            CalendarDay, CalendarFocusSession, CalendarPeriod, CalendarPeriodResult,
            CalendarProject, CalendarQuery, CalendarTaskItem,
        },
        task::TaskListFilter,
    },
    repositories::{
        calendar_repository::{CalendarRepository, CalendarTaskCandidate},
        database::Database,
        project_repository::ProjectRepository,
    },
    DomainError,
};

pub struct CalendarService<'a> {
    database: &'a Database,
}

impl<'a> CalendarService<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn get_period(&self, query: CalendarQuery) -> Result<CalendarPeriodResult, DomainError> {
        validate_filters(&query)?;
        let anchor = NaiveDate::parse_from_str(&query.anchor_date, "%Y-%m-%d").map_err(|_| {
            calendar_error(
                "CALENDAR_DATE_INVALID",
                "anchor date must use YYYY-MM-DD",
                Some("anchorDate"),
            )
        })?;
        let timezone = query.timezone.parse::<Tz>().map_err(|_| {
            calendar_error(
                "CALENDAR_TIMEZONE_INVALID",
                "timezone must be a valid IANA timezone",
                Some("timezone"),
            )
        })?;
        let (starts_on, ends_on) = period_bounds(query.period, anchor)?;
        let after_end = ends_on
            .checked_add_signed(Duration::days(1))
            .ok_or_else(|| {
                calendar_error(
                    "CALENDAR_DATE_INVALID",
                    "calendar period exceeds supported dates",
                    None,
                )
            })?;
        let utc_start = local_day_start(starts_on, timezone)?;
        let utc_end = local_day_start(after_end, timezone)?;
        let repository = CalendarRepository::new(self.database);
        let planned = repository.list_planned_tasks(
            &format_date(starts_on),
            &format_date(ends_on),
            query.category.as_deref(),
            query.project_id.as_deref(),
        )?;
        let completed = repository.list_completed_tasks(
            &utc_start.to_rfc3339(),
            &utc_end.to_rfc3339(),
            query.category.as_deref(),
            query.project_id.as_deref(),
        )?;
        let mut sessions = repository.list_focus_sessions(
            &utc_start.to_rfc3339(),
            &utc_end.to_rfc3339(),
            query.category.as_deref(),
            query.project_id.as_deref(),
        )?;
        let mut planned = planned
            .into_iter()
            .map(to_task_item)
            .collect::<Result<Vec<_>, _>>()?;
        let mut completed = completed
            .into_iter()
            .map(to_task_item)
            .collect::<Result<Vec<_>, _>>()?;
        planned.sort_by(compare_planned);
        completed.sort_by(compare_completed);
        sessions.sort_by(compare_sessions);

        let mut days = day_map(starts_on, ends_on)?;
        for item in planned {
            if let Some(day) = item
                .scheduled_date
                .as_deref()
                .and_then(|date| days.get_mut(date))
            {
                day.planned_tasks.push(item);
            }
        }
        for item in completed {
            let date = local_date(item.completed_at.as_deref(), timezone, "completed_at")?;
            if let Some(day) = days.get_mut(&date) {
                day.completed_tasks.push(item);
            }
        }
        for session in sessions {
            let date = local_date(Some(&session.ended_at), timezone, "ended_at")?;
            if let Some(day) = days.get_mut(&date) {
                day.focus_sessions.push(session);
            }
        }
        let projects = ProjectRepository::new(self.database)
            .list()?
            .into_iter()
            .map(|project| CalendarProject {
                id: project.id,
                name: project.name,
                color: project.color,
                icon: project.icon,
                status: project.status,
            })
            .collect();

        Ok(CalendarPeriodResult {
            period: query.period,
            starts_on: format_date(starts_on),
            ends_on: format_date(ends_on),
            days: days.into_values().collect(),
            projects,
        })
    }
}

fn validate_filters(query: &CalendarQuery) -> Result<(), DomainError> {
    TaskListFilter {
        category: query.category.clone(),
        ..TaskListFilter::default()
    }
    .validate()?;
    if query.project_id.as_deref().is_some_and(str::is_empty) {
        return Err(calendar_error(
            "CALENDAR_PROJECT_INVALID",
            "project id cannot be empty",
            Some("projectId"),
        ));
    }
    Ok(())
}

fn period_bounds(
    period: CalendarPeriod,
    anchor: NaiveDate,
) -> Result<(NaiveDate, NaiveDate), DomainError> {
    match period {
        CalendarPeriod::Week => {
            let starts_on = anchor - Duration::days(days_from_monday(anchor.weekday()));
            Ok((starts_on, starts_on + Duration::days(6)))
        }
        CalendarPeriod::Month => {
            let starts_on = date(anchor.year(), anchor.month(), 1)?;
            let next_month = if anchor.month() == 12 {
                date(anchor.year() + 1, 1, 1)?
            } else {
                date(anchor.year(), anchor.month() + 1, 1)?
            };
            Ok((starts_on, next_month - Duration::days(1)))
        }
        CalendarPeriod::Year => Ok((
            date(anchor.year(), 1, 1)?,
            date(anchor.year() + 1, 1, 1)? - Duration::days(1),
        )),
    }
}

fn days_from_monday(weekday: Weekday) -> i64 {
    i64::from(weekday.num_days_from_monday())
}

fn date(year: i32, month: u32, day: u32) -> Result<NaiveDate, DomainError> {
    NaiveDate::from_ymd_opt(year, month, day).ok_or_else(|| {
        calendar_error(
            "CALENDAR_DATE_INVALID",
            "calendar period exceeds supported dates",
            None,
        )
    })
}

fn day_map(
    starts_on: NaiveDate,
    ends_on: NaiveDate,
) -> Result<BTreeMap<String, CalendarDay>, DomainError> {
    let mut days = BTreeMap::new();
    let mut current = starts_on;
    loop {
        let value = format_date(current);
        days.insert(
            value.clone(),
            CalendarDay {
                date: value,
                planned_tasks: Vec::new(),
                completed_tasks: Vec::new(),
                focus_sessions: Vec::new(),
            },
        );
        if current == ends_on {
            break;
        }
        current = current
            .checked_add_signed(Duration::days(1))
            .ok_or_else(|| {
                calendar_error(
                    "CALENDAR_DATE_INVALID",
                    "calendar period exceeds supported dates",
                    None,
                )
            })?;
    }
    Ok(days)
}

fn local_day_start(date: NaiveDate, timezone: Tz) -> Result<DateTime<Utc>, DomainError> {
    let midnight = date.and_hms_opt(0, 0, 0).expect("midnight is valid");
    for minute in 0..=1_440 {
        let local = midnight + Duration::minutes(minute);
        match timezone.from_local_datetime(&local) {
            LocalResult::Single(value) => return Ok(value.with_timezone(&Utc)),
            LocalResult::Ambiguous(first, second) => {
                return Ok(first.min(second).with_timezone(&Utc));
            }
            LocalResult::None => {}
        }
    }
    Err(calendar_error(
        "CALENDAR_TIMEZONE_INVALID",
        "timezone has no valid instant for a calendar boundary",
        Some("timezone"),
    ))
}

fn local_date(timestamp: Option<&str>, timezone: Tz, field: &str) -> Result<String, DomainError> {
    let timestamp = timestamp.ok_or_else(|| {
        calendar_error(
            "CALENDAR_DATA_INVALID",
            "completed item has no timestamp",
            None,
        )
    })?;
    let timestamp = DateTime::parse_from_rfc3339(timestamp).map_err(|_| {
        calendar_error(
            "CALENDAR_DATA_INVALID",
            &format!("stored {field} timestamp is invalid"),
            None,
        )
    })?;
    Ok(timestamp
        .with_timezone(&timezone)
        .format("%Y-%m-%d")
        .to_string())
}

fn to_task_item(mut candidate: CalendarTaskCandidate) -> Result<CalendarTaskItem, DomainError> {
    if let Some(timestamp) = candidate.scheduled_at {
        let timestamp = DateTime::parse_from_rfc3339(&timestamp).map_err(|_| {
            calendar_error(
                "CALENDAR_DATA_INVALID",
                "stored instance scheduled timestamp is invalid",
                None,
            )
        })?;
        let timezone = candidate
            .timezone
            .ok_or_else(|| {
                calendar_error(
                    "CALENDAR_DATA_INVALID",
                    "stored instance timezone is missing",
                    None,
                )
            })?
            .parse::<Tz>()
            .map_err(|_| {
                calendar_error(
                    "CALENDAR_DATA_INVALID",
                    "stored instance timezone is invalid",
                    None,
                )
            })?;
        candidate.item.scheduled_time = Some(
            timestamp
                .with_timezone(&timezone)
                .format("%H:%M")
                .to_string(),
        );
    }
    Ok(candidate.item)
}

fn compare_planned(left: &CalendarTaskItem, right: &CalendarTaskItem) -> Ordering {
    left.scheduled_date
        .cmp(&right.scheduled_date)
        .then_with(|| compare_optional_time(&left.scheduled_time, &right.scheduled_time))
        .then_with(|| left.title.cmp(&right.title))
        .then_with(|| left.source_id.cmp(&right.source_id))
}

fn compare_completed(left: &CalendarTaskItem, right: &CalendarTaskItem) -> Ordering {
    compare_timestamps(left.completed_at.as_deref(), right.completed_at.as_deref())
        .then_with(|| left.title.cmp(&right.title))
        .then_with(|| left.source_id.cmp(&right.source_id))
}

fn compare_sessions(left: &CalendarFocusSession, right: &CalendarFocusSession) -> Ordering {
    compare_timestamps(Some(&left.ended_at), Some(&right.ended_at))
        .then_with(|| left.id.cmp(&right.id))
}

fn compare_timestamps(left: Option<&str>, right: Option<&str>) -> Ordering {
    let left = left.and_then(|value| DateTime::parse_from_rfc3339(value).ok());
    let right = right.and_then(|value| DateTime::parse_from_rfc3339(value).ok());
    left.cmp(&right)
}

fn compare_optional_time(left: &Option<String>, right: &Option<String>) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => left.cmp(right),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn format_date(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

fn calendar_error(code: &str, message: &str, field: Option<&str>) -> DomainError {
    DomainError {
        code: code.into(),
        message: message.into(),
        field: field.map(str::to_string),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::calendar::{CalendarSourceKind, CalendarTaskStatus};
    use rusqlite::params;

    const STAMP: &str = "2026-01-01T00:00:00Z";

    fn query(period: CalendarPeriod, anchor_date: &str, timezone: &str) -> CalendarQuery {
        CalendarQuery {
            period,
            anchor_date: anchor_date.into(),
            timezone: timezone.into(),
            category: None,
            project_id: None,
        }
    }

    fn insert_project(database: &Database, id: &str) {
        database
            .write(|tx| {
                tx.execute(
                    "INSERT INTO projects(id, name, description, color, icon, status, started_on, created_at, updated_at)
                     VALUES (?1, ?2, '', 'mint', 'folder', 'active', '2026-01-01', ?3, ?3)",
                    params![id, format!("Project {id}"), STAMP],
                )?;
                Ok(())
            })
            .unwrap();
    }

    #[allow(clippy::too_many_arguments)]
    fn insert_task(
        database: &Database,
        id: &str,
        title: &str,
        category: &str,
        project_id: Option<&str>,
        scheduled_date: Option<&str>,
        scheduled_time: Option<&str>,
        status: &str,
        completed_at: Option<&str>,
    ) {
        database
            .write(|tx| {
                tx.execute(
                    "INSERT INTO tasks(id, project_id, title, category, priority, scheduled_date, scheduled_time, status, completed_at, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6, ?7, ?8, ?9, ?9)",
                    params![
                        id,
                        project_id,
                        title,
                        category,
                        scheduled_date,
                        scheduled_time,
                        status,
                        completed_at,
                        STAMP
                    ],
                )?;
                Ok(())
            })
            .unwrap();
    }

    struct InstanceFixture<'a> {
        template_id: &'a str,
        rule_id: &'a str,
        instance_id: &'a str,
        scheduled_date: &'a str,
        project_id: Option<&'a str>,
        status: &'a str,
        completed_at: Option<&'a str>,
    }

    fn insert_rule_and_instance(database: &Database, fixture: InstanceFixture<'_>) {
        database
            .write(|tx| {
                tx.execute(
                    "INSERT INTO recurrence_rules(id, task_template_id, pattern_json, local_time, timezone, starts_on, status, version, created_at, updated_at)
                     VALUES (?1, ?2, '{}', '09:00', 'Asia/Shanghai', '2026-01-01', 'active', 1, ?3, ?3)",
                    params![fixture.rule_id, fixture.template_id, STAMP],
                )?;
                tx.execute(
                    "INSERT INTO task_instances(id, recurrence_rule_id, rule_version, scheduled_date, scheduled_at, snapshot_title, snapshot_project_id, status, completed_at, created_at, updated_at)
                     VALUES (?1, ?2, 1, ?3, '2026-07-20T01:00:00Z', ?4, ?5, ?6, ?7, ?8, ?8)",
                    params![
                        fixture.instance_id,
                        fixture.rule_id,
                        fixture.scheduled_date,
                        format!("Instance {}", fixture.instance_id),
                        fixture.project_id,
                        fixture.status,
                        fixture.completed_at,
                        STAMP
                    ],
                )?;
                Ok(())
            })
            .unwrap();
    }

    #[allow(clippy::too_many_arguments)]
    fn insert_focus(
        database: &Database,
        id: &str,
        task_id: Option<&str>,
        instance_id: Option<&str>,
        project_id: Option<&str>,
        completion_kind: &str,
        started_at: &str,
        ended_at: &str,
    ) {
        database
            .write(|tx| {
                tx.execute(
                    "INSERT INTO focus_sessions(id, task_id, task_instance_id, project_id, planned_seconds, actual_seconds, interruption_count, completion_kind, started_at, ended_at, created_at)
                     VALUES (?1, ?2, ?3, ?4, 1500, 1200, 0, ?5, ?6, ?7, ?8)",
                    params![
                        id,
                        task_id,
                        instance_id,
                        project_id,
                        completion_kind,
                        started_at,
                        ended_at,
                        STAMP
                    ],
                )?;
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn week_month_and_leap_year_use_natural_boundaries() {
        let database = Database::open_in_memory().unwrap();
        let service = CalendarService::new(&database);

        let week = service
            .get_period(query(CalendarPeriod::Week, "2026-07-22", "UTC"))
            .unwrap();
        assert_eq!(
            (week.starts_on.as_str(), week.ends_on.as_str()),
            ("2026-07-20", "2026-07-26")
        );
        assert_eq!(week.days.len(), 7);

        let month = service
            .get_period(query(CalendarPeriod::Month, "2026-02-10", "UTC"))
            .unwrap();
        assert_eq!(
            (month.starts_on.as_str(), month.ends_on.as_str()),
            ("2026-02-01", "2026-02-28")
        );
        assert_eq!(month.days.len(), 28);

        let year = service
            .get_period(query(CalendarPeriod::Year, "2024-08-01", "UTC"))
            .unwrap();
        assert_eq!(
            (year.starts_on.as_str(), year.ends_on.as_str()),
            ("2024-01-01", "2024-12-31")
        );
        assert_eq!(year.days.len(), 366);
        assert_eq!(year.days[59].date, "2024-02-29");
    }

    #[test]
    fn empty_period_contains_every_zero_data_day() {
        let database = Database::open_in_memory().unwrap();
        let result = CalendarService::new(&database)
            .get_period(query(CalendarPeriod::Month, "2026-04-15", "Asia/Shanghai"))
            .unwrap();

        assert_eq!(result.days.len(), 30);
        assert!(result.days.iter().all(|day| day.planned_tasks.is_empty()
            && day.completed_tasks.is_empty()
            && day.focus_sessions.is_empty()));
    }

    #[test]
    fn planned_tasks_exclude_templates_and_include_recurring_instances() {
        let database = Database::open_in_memory().unwrap();
        insert_task(
            &database,
            "ordinary",
            "Ordinary",
            "life",
            None,
            Some("2026-07-20"),
            Some("10:00"),
            "pending",
            None,
        );
        insert_task(
            &database,
            "template",
            "Template",
            "study",
            None,
            Some("2026-07-20"),
            Some("09:00"),
            "pending",
            None,
        );
        insert_rule_and_instance(
            &database,
            InstanceFixture {
                template_id: "template",
                rule_id: "rule",
                instance_id: "instance",
                scheduled_date: "2026-07-20",
                project_id: None,
                status: "pending",
                completed_at: None,
            },
        );

        let result = CalendarService::new(&database)
            .get_period(query(CalendarPeriod::Week, "2026-07-20", "UTC"))
            .unwrap();
        let day = &result.days[0];
        assert_eq!(day.planned_tasks.len(), 2);
        assert_eq!(day.planned_tasks[0].source_id, "instance");
        assert_eq!(
            day.planned_tasks[0].source_kind,
            CalendarSourceKind::RecurringInstance
        );
        assert_eq!(day.planned_tasks[0].category, "study");
        assert_eq!(day.planned_tasks[1].source_id, "ordinary");
    }

    #[test]
    fn task_completion_uses_query_timezone_local_date() {
        let database = Database::open_in_memory().unwrap();
        insert_task(
            &database,
            "completed",
            "Completed",
            "work",
            None,
            Some("2026-07-19"),
            None,
            "completed",
            Some("2026-07-19T16:30:00Z"),
        );

        let result = CalendarService::new(&database)
            .get_period(query(CalendarPeriod::Week, "2026-07-20", "Asia/Shanghai"))
            .unwrap();
        assert_eq!(result.days[0].completed_tasks.len(), 1);
        assert_eq!(result.days[0].completed_tasks[0].source_id, "completed");
        assert_eq!(
            result.days[0].completed_tasks[0].status,
            CalendarTaskStatus::Completed
        );
    }

    #[test]
    fn focus_uses_local_end_date_and_excludes_cancelled_sessions() {
        let database = Database::open_in_memory().unwrap();
        insert_focus(
            &database,
            "deadline",
            None,
            None,
            None,
            "deadline",
            "2026-07-19T16:00:00Z",
            "2026-07-19T16:30:00Z",
        );
        insert_focus(
            &database,
            "cancelled",
            None,
            None,
            None,
            "cancelled",
            "2026-07-19T17:00:00Z",
            "2026-07-19T17:10:00Z",
        );

        let result = CalendarService::new(&database)
            .get_period(query(CalendarPeriod::Week, "2026-07-20", "Asia/Shanghai"))
            .unwrap();
        assert_eq!(result.days[0].focus_sessions.len(), 1);
        assert_eq!(result.days[0].focus_sessions[0].id, "deadline");
    }

    #[test]
    fn dst_week_uses_utc_half_open_bounds() {
        let database = Database::open_in_memory().unwrap();
        insert_focus(
            &database,
            "inside",
            None,
            None,
            None,
            "early",
            "2026-03-08T06:45:00Z",
            "2026-03-08T07:15:00Z",
        );
        insert_focus(
            &database,
            "next-week",
            None,
            None,
            None,
            "early",
            "2026-03-09T04:30:00Z",
            "2026-03-09T05:00:00Z",
        );

        let result = CalendarService::new(&database)
            .get_period(query(
                CalendarPeriod::Week,
                "2026-03-08",
                "America/New_York",
            ))
            .unwrap();
        assert_eq!(
            (result.starts_on.as_str(), result.ends_on.as_str()),
            ("2026-03-02", "2026-03-08")
        );
        assert_eq!(result.days[6].focus_sessions.len(), 1);
        assert_eq!(result.days[6].focus_sessions[0].id, "inside");
    }

    #[test]
    fn category_and_project_filters_cover_tasks_instances_and_focus() {
        let database = Database::open_in_memory().unwrap();
        insert_project(&database, "p1");
        insert_project(&database, "p2");
        insert_task(
            &database,
            "work-p1",
            "Work P1",
            "work",
            Some("p1"),
            Some("2026-07-20"),
            None,
            "completed",
            Some("2026-07-20T10:00:00Z"),
        );
        insert_task(
            &database,
            "life-p2",
            "Life P2",
            "life",
            Some("p2"),
            Some("2026-07-20"),
            None,
            "pending",
            None,
        );
        insert_task(
            &database,
            "template-p1",
            "Template P1",
            "work",
            Some("p1"),
            None,
            None,
            "pending",
            None,
        );
        insert_rule_and_instance(
            &database,
            InstanceFixture {
                template_id: "template-p1",
                rule_id: "rule-p1",
                instance_id: "instance-p1",
                scheduled_date: "2026-07-20",
                project_id: Some("p1"),
                status: "pending",
                completed_at: None,
            },
        );
        insert_focus(
            &database,
            "focus-p1",
            Some("work-p1"),
            None,
            Some("p1"),
            "deadline",
            "2026-07-20T11:00:00Z",
            "2026-07-20T11:25:00Z",
        );
        insert_focus(
            &database,
            "independent",
            None,
            None,
            None,
            "early",
            "2026-07-20T12:00:00Z",
            "2026-07-20T12:10:00Z",
        );

        let mut filtered = query(CalendarPeriod::Week, "2026-07-20", "UTC");
        filtered.category = Some("work".into());
        filtered.project_id = Some("p1".into());
        let result = CalendarService::new(&database)
            .get_period(filtered)
            .unwrap();
        let day = &result.days[0];
        assert_eq!(
            day.planned_tasks
                .iter()
                .map(|item| item.source_id.as_str())
                .collect::<Vec<_>>(),
            vec!["instance-p1", "work-p1"]
        );
        assert_eq!(
            day.completed_tasks
                .iter()
                .map(|item| item.source_id.as_str())
                .collect::<Vec<_>>(),
            vec!["work-p1"]
        );
        assert_eq!(
            day.focus_sessions
                .iter()
                .map(|session| session.id.as_str())
                .collect::<Vec<_>>(),
            vec!["focus-p1"]
        );
        assert_eq!(result.projects.len(), 2);
    }
}

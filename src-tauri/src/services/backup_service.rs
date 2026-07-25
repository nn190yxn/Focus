use std::{
    fs,
    io::{Read, Write},
    path::Path,
};

use chrono::{DateTime, SecondsFormat, Utc};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    domain::backup::{
        backup_error, BackupEnvelope, BackupExportResult, BackupRestoreResult, ValidatedBackup,
        CURRENT_BACKUP_FORMAT_VERSION, LEGACY_BACKUP_FORMAT_VERSION, MAX_BACKUP_BYTES,
    },
    repositories::{
        backup_repository::{BackupHistoryEntry, BackupRepository},
        database::Database,
    },
    DomainError,
};

pub struct BackupService<'a> {
    repository: BackupRepository<'a>,
}

impl<'a> BackupService<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self {
            repository: BackupRepository::new(database),
        }
    }

    pub fn export_json(&self) -> Result<String, DomainError> {
        self.export_json_at(Utc::now())
    }

    pub fn export_json_at(&self, exported_at: DateTime<Utc>) -> Result<String, DomainError> {
        serialize_envelope(BackupEnvelope {
            format_version: CURRENT_BACKUP_FORMAT_VERSION,
            exported_at: format_timestamp(exported_at),
            data: self.repository.snapshot()?,
        })
        .map(|(json, _)| json)
    }

    pub fn export_to_path(
        &self,
        path: &Path,
        exported_at: DateTime<Utc>,
    ) -> Result<BackupExportResult, DomainError> {
        let (json, summary) = serialize_envelope(BackupEnvelope {
            format_version: CURRENT_BACKUP_FORMAT_VERSION,
            exported_at: format_timestamp(exported_at),
            data: self.repository.snapshot()?,
        })?;
        write_backup_file(path, json.as_bytes())?;
        let _ = self.repository.record_history(&BackupHistoryEntry {
            id: Uuid::new_v4().to_string(),
            kind: "manual".into(),
            path: display_path(path),
            format_version: CURRENT_BACKUP_FORMAT_VERSION,
            checksum: checksum(json.as_bytes()),
            created_at: format_timestamp(exported_at),
        });
        Ok(BackupExportResult {
            path: display_path(path),
            summary,
        })
    }

    pub fn inspect_path(path: &Path) -> Result<ValidatedBackup, DomainError> {
        let input = read_backup_file(path)?;
        Self::parse_json(&input)
    }

    pub fn restore(
        &self,
        backup: ValidatedBackup,
        source_path: &Path,
        snapshot_directory: &Path,
        restored_at: DateTime<Utc>,
    ) -> Result<BackupRestoreResult, DomainError> {
        fs::create_dir_all(snapshot_directory).map_err(|error| {
            file_error("BACKUP_SNAPSHOT_FAILED", error, Some("snapshotDirectory"))
        })?;
        let snapshot_path = snapshot_directory.join(format!(
            "pre-restore-{}-{}.json",
            restored_at.format("%Y%m%dT%H%M%SZ"),
            Uuid::new_v4()
        ));
        let created_at = format_timestamp(restored_at);
        let mut preserved_history = None;
        let restore_result =
            self.repository
                .restore_with_snapshot(&backup.envelope.data, |current_data| {
                    let (json, _) = serialize_envelope(BackupEnvelope {
                        format_version: CURRENT_BACKUP_FORMAT_VERSION,
                        exported_at: created_at.clone(),
                        data: current_data.clone(),
                    })?;
                    write_backup_file(&snapshot_path, json.as_bytes()).map_err(|error| {
                        DomainError {
                            code: "BACKUP_SNAPSHOT_FAILED".into(),
                            message: error.message,
                            field: error.field,
                        }
                    })?;
                    let history = BackupHistoryEntry {
                        id: Uuid::new_v4().to_string(),
                        kind: "pre_restore".into(),
                        path: display_path(&snapshot_path),
                        format_version: CURRENT_BACKUP_FORMAT_VERSION,
                        checksum: checksum(json.as_bytes()),
                        created_at: created_at.clone(),
                    };
                    preserved_history = Some(history.clone());
                    Ok(history)
                });
        let history = match restore_result {
            Ok(history) => history,
            Err(error) => {
                if let Some(history) = &preserved_history {
                    let _ = self.repository.record_history(history);
                }
                return Err(error);
            }
        };
        Ok(BackupRestoreResult {
            source_path: display_path(source_path),
            snapshot_path: history.path,
            summary: backup.summary,
        })
    }

    pub fn parse_json(input: &str) -> Result<ValidatedBackup, DomainError> {
        if input.len() > MAX_BACKUP_BYTES {
            return Err(backup_error(
                "BACKUP_FILE_TOO_LARGE",
                "backup file exceeds the supported size",
                None,
            ));
        }
        let mut value: serde_json::Value = serde_json::from_str(input).map_err(format_error)?;
        let format_version = value
            .get("formatVersion")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| u32::try_from(value).ok())
            .ok_or_else(|| {
                backup_error(
                    "BACKUP_FORMAT_INVALID",
                    "formatVersion must be a positive integer",
                    Some("formatVersion"),
                )
            })?;
        if ![LEGACY_BACKUP_FORMAT_VERSION, CURRENT_BACKUP_FORMAT_VERSION].contains(&format_version)
        {
            return Err(backup_error(
                "BACKUP_VERSION_UNSUPPORTED",
                format!("unsupported backup format version: {format_version}"),
                Some("formatVersion"),
            ));
        }
        if format_version == LEGACY_BACKUP_FORMAT_VERSION {
            add_empty_memo_collections(&mut value)?;
        }
        let envelope: BackupEnvelope = serde_json::from_value(value).map_err(format_error)?;
        let summary = envelope.validate()?;
        Ok(ValidatedBackup { envelope, summary })
    }
}

fn add_empty_memo_collections(value: &mut serde_json::Value) -> Result<(), DomainError> {
    let data = value
        .get_mut("data")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| {
            backup_error(
                "BACKUP_FORMAT_INVALID",
                "data must be an object",
                Some("data"),
            )
        })?;
    for field in ["memos", "memoTags", "memoTagLinks", "memoReminders"] {
        data.entry(field).or_insert_with(|| serde_json::json!([]));
    }
    Ok(())
}

fn serialize_envelope(
    envelope: BackupEnvelope,
) -> Result<(String, crate::domain::backup::BackupImportSummary), DomainError> {
    let summary = envelope.validate()?;
    let json = serde_json::to_string_pretty(&envelope)
        .map_err(|error| backup_error("BACKUP_SERIALIZATION_FAILED", error.to_string(), None))?;
    Ok((json, summary))
}

fn read_backup_file(path: &Path) -> Result<String, DomainError> {
    let file = fs::File::open(path)
        .map_err(|error| file_error("BACKUP_FILE_READ_FAILED", error, Some("path")))?;
    let mut bytes = Vec::new();
    file.take((MAX_BACKUP_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| file_error("BACKUP_FILE_READ_FAILED", error, Some("path")))?;
    if bytes.len() > MAX_BACKUP_BYTES {
        return Err(backup_error(
            "BACKUP_FILE_TOO_LARGE",
            "backup file exceeds the supported size",
            Some("path"),
        ));
    }
    String::from_utf8(bytes)
        .map_err(|error| backup_error("BACKUP_FORMAT_INVALID", error.to_string(), Some("path")))
}

fn write_backup_file(path: &Path, bytes: &[u8]) -> Result<(), DomainError> {
    let mut file = fs::File::create(path)
        .map_err(|error| file_error("BACKUP_FILE_WRITE_FAILED", error, Some("path")))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| file_error("BACKUP_FILE_WRITE_FAILED", error, Some("path")))
}

fn checksum(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn format_timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn file_error(code: &str, error: std::io::Error, field: Option<&str>) -> DomainError {
    backup_error(code, error.to_string(), field)
}

fn format_error(error: serde_json::Error) -> DomainError {
    backup_error("BACKUP_FORMAT_INVALID", error.to_string(), None)
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use proptest::prelude::*;
    use rusqlite::params;
    use serde_json::json;

    use crate::domain::{
        backup::{
            BackupActiveFocus, BackupCheckItem, BackupData, BackupFocusSession, BackupMemo,
            BackupMemoReminder, BackupMemoTag, BackupMemoTagLink, BackupNote, BackupPreference,
            BackupProject, BackupRecurrenceRule, BackupTask, BackupTaskInstance, BackupWeeklyGoal,
        },
        recurrence::RecurrencePattern,
    };

    use super::*;

    const NOW: &str = "2026-07-21T08:00:00Z";

    #[derive(Debug, Clone)]
    struct BackupBranchSeed {
        title: String,
        flags: u8,
        planned_seconds: i64,
        actual_seconds_seed: u16,
    }

    #[derive(Debug, Clone)]
    struct MemoGraphSeed {
        title: String,
        body: String,
        flags: u8,
        tag_mask: u16,
    }

    #[derive(Debug, PartialEq, Eq)]
    struct NormalizedMemoBackup {
        memos: Vec<BackupMemo>,
        tags: Vec<BackupMemoTag>,
        links: Vec<BackupMemoTagLink>,
        reminders: Vec<BackupMemoReminder>,
    }

    fn backup_branch_seed() -> impl Strategy<Value = BackupBranchSeed> {
        (
            "[A-Za-z0-9][A-Za-z0-9 _-]{0,31}",
            any::<u8>(),
            1_i64..=7_200,
            any::<u16>(),
        )
            .prop_map(|(title, flags, planned_seconds, actual_seconds_seed)| {
                BackupBranchSeed {
                    title,
                    flags,
                    planned_seconds,
                    actual_seconds_seed,
                }
            })
    }

    fn memo_graph_seed() -> impl Strategy<Value = MemoGraphSeed> {
        (
            "[A-Za-z0-9][A-Za-z0-9 _-]{0,31}",
            "[A-Za-z0-9 .,_-]{0,80}",
            any::<u8>(),
            any::<u16>(),
        )
            .prop_map(|(title, body, flags, tag_mask)| MemoGraphSeed {
                title,
                body,
                flags,
                tag_mask,
            })
    }

    fn valid_memo_backup_data() -> impl Strategy<Value = BackupData> {
        prop::collection::vec(memo_graph_seed(), 0..7).prop_map(|seeds| {
            let mut data = BackupData::default();
            let tag_count = seeds.len().min(6);

            for index in 0..tag_count {
                data.memo_tags.push(BackupMemoTag {
                    id: format!("memo-tag-{index:02}"),
                    name: format!("Tag {index:02}"),
                    normalized_name: format!("tag {index:02}"),
                    created_at: NOW.into(),
                });
            }

            for (index, seed) in seeds.iter().enumerate() {
                let memo_id = format!("memo-{index:02}");
                data.memos.push(BackupMemo {
                    id: memo_id.clone(),
                    title: seed.title.clone(),
                    body: seed.body.clone(),
                    pinned_at: (seed.flags & 1 != 0).then(|| NOW.into()),
                    created_at: NOW.into(),
                    updated_at: NOW.into(),
                });

                for tag_index in 0..tag_count {
                    if seed.tag_mask & (1 << tag_index) != 0 {
                        data.memo_tag_links.push(BackupMemoTagLink {
                            memo_id: memo_id.clone(),
                            tag_id: format!("memo-tag-{tag_index:02}"),
                        });
                    }
                }

                if seed.flags & 2 != 0 {
                    data.memo_reminders
                        .push(generated_memo_reminder(index, seed.flags, memo_id));
                }
            }

            data
        })
    }

    fn generated_memo_reminder(index: usize, flags: u8, memo_id: String) -> BackupMemoReminder {
        let weekday_offset = index % 5;
        let day = 20 + weekday_offset;
        let starts_on = format!("2026-07-{day:02}");
        let timezone = ["UTC", "Asia/Shanghai", "America/New_York", "Europe/London"]
            [usize::from((flags / 4) % 4)];
        let status = ["active", "completed", "cancelled"][usize::from((flags / 16) % 3)];
        let recurring = flags & 4 != 0;
        let frequency_index = usize::from((flags / 8) % 4);
        let (frequency, interval, weekdays, monthly_day) = if recurring {
            match frequency_index {
                0 => (Some("daily".into()), Some(1), vec![], None),
                1 => (Some("weekdays".into()), Some(1), vec![], None),
                2 => (
                    Some("weekly".into()),
                    Some(i64::from(flags % 3) + 1),
                    vec![u8::try_from(weekday_offset + 1).unwrap()],
                    None,
                ),
                _ => (
                    Some("monthly".into()),
                    Some(i64::from(flags % 3) + 1),
                    vec![],
                    Some(day as i64),
                ),
            }
        } else {
            (None, None, vec![], None)
        };

        BackupMemoReminder {
            id: format!("memo-reminder-{index:02}"),
            memo_id,
            schedule_kind: if recurring { "recurring" } else { "once" }.into(),
            frequency,
            interval,
            weekdays,
            monthly_day,
            local_time: "09:30".into(),
            starts_on,
            ends_on: None,
            timezone: timezone.into(),
            next_scheduled_for: (status == "active").then(|| generated_occurrence(day, timezone)),
            status: status.into(),
            created_at: NOW.into(),
            updated_at: NOW.into(),
        }
    }

    fn generated_occurrence(day: usize, timezone: &str) -> String {
        let hour = match timezone {
            "UTC" => 9,
            "Asia/Shanghai" => 1,
            "America/New_York" => 13,
            "Europe/London" => 8,
            _ => unreachable!(),
        };
        format!("2026-07-{day:02}T{hour:02}:30:00Z")
    }

    fn normalized_memo_backup(mut data: BackupData) -> NormalizedMemoBackup {
        data.memos.sort_by(|left, right| left.id.cmp(&right.id));
        data.memo_tags.sort_by(|left, right| left.id.cmp(&right.id));
        data.memo_tag_links.sort_by(|left, right| {
            (&left.memo_id, &left.tag_id).cmp(&(&right.memo_id, &right.tag_id))
        });
        data.memo_reminders
            .sort_by(|left, right| left.id.cmp(&right.id));
        NormalizedMemoBackup {
            memos: data.memos,
            tags: data.memo_tags,
            links: data.memo_tag_links,
            reminders: data.memo_reminders,
        }
    }

    fn valid_backup_data() -> impl Strategy<Value = BackupData> {
        (
            prop::collection::vec(backup_branch_seed(), 0..5),
            any::<bool>(),
            any::<bool>(),
        )
            .prop_map(|(branches, include_active_focus, active_uses_instance)| {
                let mut data = BackupData::default();

                for (index, seed) in branches.iter().enumerate() {
                    let suffix = format!("{index:02}");
                    let project_id = format!("project-{suffix}");
                    let task_id = format!("task-{suffix}");
                    let rule_id = format!("rule-{suffix}");
                    let instance_id = format!("instance-{suffix}");
                    let day = 21 + index;
                    let date = format!("2026-07-{day:02}");
                    let completed_at = || Some(NOW.to_string());
                    let task_status = match seed.flags % 3 {
                        0 => "pending",
                        1 => "completed",
                        _ => "removed",
                    };
                    let instance_status = match (seed.flags / 3) % 4 {
                        0 => "pending",
                        1 => "completed",
                        2 => "skipped",
                        _ => "rescheduled",
                    };
                    let pattern = match (seed.flags / 5) % 4 {
                        0 => RecurrencePattern::Daily {
                            interval: u32::from(seed.flags % 7) + 1,
                        },
                        1 => RecurrencePattern::Weekdays,
                        2 => RecurrencePattern::Weekly {
                            interval: u32::from(seed.flags % 4) + 1,
                            weekdays: vec![1, 3, 5],
                        },
                        _ => RecurrencePattern::Monthly {
                            interval: u32::from(seed.flags % 6) + 1,
                            day_of_month: (day as u8).min(31),
                        },
                    };

                    data.projects.push(BackupProject {
                        id: project_id.clone(),
                        name: seed.title.clone(),
                        description: format!("Generated project {suffix}"),
                        color: ["mint", "blue", "amber", "rose"][usize::from(seed.flags % 4)]
                            .into(),
                        icon: "folder".into(),
                        status: ["active", "paused", "completed", "archived"]
                            [usize::from((seed.flags / 2) % 4)]
                        .into(),
                        started_on: date.clone(),
                        target_on: (seed.flags & 1 != 0).then(|| "2026-08-31".into()),
                        created_at: NOW.into(),
                        updated_at: NOW.into(),
                    });
                    data.tasks.push(BackupTask {
                        id: task_id.clone(),
                        project_id: (seed.flags & 2 != 0).then(|| project_id.clone()),
                        title: seed.title.clone(),
                        category: ["work", "study", "health", "life"]
                            [usize::from((seed.flags / 4) % 4)]
                        .into(),
                        priority: i64::from(seed.flags % 4),
                        scheduled_date: (seed.flags & 4 != 0).then(|| date.clone()),
                        scheduled_time: (seed.flags & 4 != 0 && seed.flags & 8 != 0)
                            .then(|| "09:30".into()),
                        status: task_status.into(),
                        completed_at: (task_status == "completed").then(completed_at).flatten(),
                        created_at: NOW.into(),
                        updated_at: NOW.into(),
                    });
                    data.check_items.push(BackupCheckItem {
                        id: format!("check-{suffix}"),
                        task_id: task_id.clone(),
                        title: format!("Check {}", seed.title),
                        position: 0,
                        completed_at: (seed.flags & 16 != 0).then(completed_at).flatten(),
                        created_at: NOW.into(),
                        updated_at: NOW.into(),
                    });
                    data.recurrence_rules.push(BackupRecurrenceRule {
                        id: rule_id.clone(),
                        task_template_id: task_id.clone(),
                        pattern,
                        local_time: (seed.flags & 32 != 0).then(|| "09:30".into()),
                        timezone: ["UTC", "Asia/Shanghai", "Europe/London"]
                            [usize::from(seed.flags % 3)]
                        .into(),
                        starts_on: date.clone(),
                        ends_on: (seed.flags & 64 != 0).then(|| "2026-08-31".into()),
                        status: ["active", "paused", "ended"][usize::from((seed.flags / 7) % 3)]
                            .into(),
                        version: u32::from(seed.flags % 5) + 1,
                        created_at: NOW.into(),
                        updated_at: NOW.into(),
                    });
                    data.task_instances.push(BackupTaskInstance {
                        id: instance_id.clone(),
                        recurrence_rule_id: rule_id,
                        rule_version: u32::from(seed.flags % 5) + 1,
                        scheduled_date: date.clone(),
                        scheduled_at: (seed.flags & 8 != 0).then(|| NOW.into()),
                        snapshot_title: seed.title.clone(),
                        snapshot_project_id: (seed.flags & 2 != 0).then(|| project_id.clone()),
                        status: instance_status.into(),
                        completed_at: (instance_status == "completed")
                            .then(completed_at)
                            .flatten(),
                        source_instance_id: (index > 0 && seed.flags & 128 != 0)
                            .then(|| format!("instance-{:02}", index - 1)),
                        created_at: NOW.into(),
                        updated_at: NOW.into(),
                    });
                    data.focus_sessions.push(BackupFocusSession {
                        id: format!("focus-{suffix}"),
                        task_id: (seed.flags & 1 != 0).then(|| task_id.clone()),
                        task_instance_id: (seed.flags & 2 != 0).then_some(instance_id),
                        project_id: (seed.flags & 4 != 0).then_some(project_id),
                        planned_seconds: seed.planned_seconds,
                        actual_seconds: i64::from(seed.actual_seconds_seed)
                            % (seed.planned_seconds + 1),
                        interruption_count: i64::from(seed.flags % 6),
                        completion_kind: ["deadline", "early", "cancelled"]
                            [usize::from(seed.flags % 3)]
                        .into(),
                        started_at: NOW.into(),
                        ended_at: NOW.into(),
                        created_at: NOW.into(),
                    });
                    data.notes.push(BackupNote {
                        id: format!("note-{suffix}"),
                        body: format!("Note for {}", seed.title),
                        note_date: date,
                        created_at: NOW.into(),
                        updated_at: NOW.into(),
                    });
                    data.weekly_goals.push(BackupWeeklyGoal {
                        id: format!("goal-{suffix}"),
                        week_starts_on: "2026-07-20".into(),
                        title: format!("Goal {}", seed.title),
                        category: ["completed_tasks", "focus_minutes", "active_days"]
                            [usize::from(seed.flags % 3)]
                        .into(),
                        target_count: i64::from(seed.flags % 20) + 1,
                        completed_count: i64::from(seed.flags % 20),
                        position: index as i64,
                        created_at: NOW.into(),
                        updated_at: NOW.into(),
                    });
                    data.preferences.push(BackupPreference {
                        key: format!("preference-{suffix}"),
                        value: json!({
                            "enabled": seed.flags & 1 != 0,
                            "label": seed.title,
                            "level": seed.flags % 4,
                        }),
                        updated_at: NOW.into(),
                    });
                }

                if include_active_focus && !branches.is_empty() {
                    let uses_instance = active_uses_instance;
                    let running = branches[0].flags & 1 == 0;
                    let planned_seconds = branches[0].planned_seconds;
                    data.active_focus = Some(BackupActiveFocus {
                        task_id: (!uses_instance).then(|| "task-00".into()),
                        task_instance_id: uses_instance.then(|| "instance-00".into()),
                        state: if running { "running" } else { "paused" }.into(),
                        planned_seconds,
                        remaining_seconds: i64::from(branches[0].actual_seconds_seed)
                            % (planned_seconds + 1),
                        started_at: NOW.into(),
                        target_ends_at: running.then(|| "2026-07-21T10:00:00Z".into()),
                        paused_at: (!running).then(|| "2026-07-21T08:30:00Z".into()),
                        interruption_count: i64::from(branches[0].flags % 6),
                        updated_at: NOW.into(),
                    });
                }

                data
            })
    }

    #[test]
    fn exports_every_portable_business_collection_and_parses_it() {
        let database = populated_database();
        let service = BackupService::new(&database);

        let json = service
            .export_json_at(Utc.with_ymd_and_hms(2026, 7, 21, 9, 0, 0).unwrap())
            .unwrap();
        let parsed = BackupService::parse_json(&json).unwrap();

        assert_eq!(parsed.envelope.format_version, 2);
        assert_eq!(parsed.summary.counts.projects, 1);
        assert_eq!(parsed.summary.counts.tasks, 1);
        assert_eq!(parsed.summary.counts.check_items, 1);
        assert_eq!(parsed.summary.counts.recurrence_rules, 1);
        assert_eq!(parsed.summary.counts.task_instances, 1);
        assert_eq!(parsed.summary.counts.focus_sessions, 1);
        assert_eq!(parsed.summary.counts.active_focus, 1);
        assert_eq!(parsed.summary.counts.notes, 1);
        assert_eq!(parsed.summary.counts.weekly_goals, 1);
        assert_eq!(parsed.summary.counts.preferences, 1);
        assert_eq!(parsed.summary.counts.memos, 1);
        assert_eq!(parsed.summary.counts.memo_tags, 1);
        assert_eq!(parsed.summary.counts.memo_tag_links, 1);
        assert_eq!(parsed.summary.counts.memo_reminders, 1);
        assert!(json.find("\"projects\"").unwrap() < json.find("\"tasks\"").unwrap());
    }

    #[test]
    fn empty_database_exports_a_valid_versioned_envelope() {
        let database = Database::open_in_memory().unwrap();
        let json = BackupService::new(&database)
            .export_json_at(Utc.with_ymd_and_hms(2026, 7, 21, 9, 0, 0).unwrap())
            .unwrap();
        let parsed = BackupService::parse_json(&json).unwrap();

        assert_eq!(parsed.summary.counts.total, 0);
        assert_eq!(parsed.summary.earliest_date, None);
        assert_eq!(parsed.summary.latest_date, None);
    }

    #[test]
    fn parser_reports_format_and_version_errors_separately() {
        assert_eq!(
            BackupService::parse_json("not-json").unwrap_err().code,
            "BACKUP_FORMAT_INVALID"
        );
        assert_eq!(
            BackupService::parse_json(r#"{"formatVersion":3}"#)
                .unwrap_err()
                .code,
            "BACKUP_VERSION_UNSUPPORTED"
        );
        assert_eq!(
            BackupService::parse_json(r#"{"formatVersion":"1"}"#)
                .unwrap_err()
                .field
                .as_deref(),
            Some("formatVersion")
        );
    }

    #[test]
    fn parser_converts_version_one_memo_collections_to_empty_sets() {
        let database = populated_database();
        let current_json = BackupService::new(&database)
            .export_json_at(Utc.with_ymd_and_hms(2026, 7, 21, 9, 0, 0).unwrap())
            .unwrap();
        let mut legacy: serde_json::Value = serde_json::from_str(&current_json).unwrap();
        legacy["formatVersion"] = json!(1);
        let data = legacy["data"].as_object_mut().unwrap();
        for field in ["memos", "memoTags", "memoTagLinks", "memoReminders"] {
            data.remove(field);
        }

        let parsed = BackupService::parse_json(&legacy.to_string()).unwrap();

        assert_eq!(parsed.envelope.format_version, 1);
        assert!(parsed.envelope.data.memos.is_empty());
        assert!(parsed.envelope.data.memo_tags.is_empty());
        assert!(parsed.envelope.data.memo_tag_links.is_empty());
        assert!(parsed.envelope.data.memo_reminders.is_empty());
        assert_eq!(parsed.summary.counts.total, 10);
    }

    #[test]
    fn restores_all_business_data_and_preserves_device_state() {
        let source = populated_database();
        let backup = BackupService::parse_json(
            &BackupService::new(&source)
                .export_json_at(Utc.with_ymd_and_hms(2026, 7, 21, 9, 0, 0).unwrap())
                .unwrap(),
        )
        .unwrap();
        let target = target_database();
        let directory = tempfile::tempdir().unwrap();

        let restored = BackupService::new(&target)
            .restore(
                backup,
                Path::new("selected.json"),
                directory.path(),
                Utc.with_ymd_and_hms(2026, 7, 21, 10, 0, 0).unwrap(),
            )
            .unwrap();

        let snapshot = BackupService::inspect_path(Path::new(&restored.snapshot_path)).unwrap();
        assert_eq!(snapshot.envelope.data.projects[0].id, "old-project");
        assert_eq!(snapshot.envelope.data.memos[0].id, "old-memo");
        target
            .read(|connection| {
                assert_eq!(
                    connection.query_row(
                        "SELECT name FROM projects WHERE id = 'project-1'",
                        [],
                        |row| row.get::<_, String>(0),
                    )?,
                    "Backup"
                );
                assert_eq!(
                    connection.query_row(
                        "SELECT title FROM memos WHERE id = 'memo-1'",
                        [],
                        |row| row.get::<_, String>(0),
                    )?,
                    "Portable memo"
                );
                assert_eq!(
                    connection.query_row(
                        "SELECT COUNT(*) FROM memo_tag_links WHERE memo_id = 'memo-1'",
                        [],
                        |row| row.get::<_, i64>(0),
                    )?,
                    1
                );
                assert_eq!(
                    connection.query_row(
                        "SELECT next_scheduled_for FROM memo_reminders WHERE memo_id = 'memo-1'",
                        [],
                        |row| row.get::<_, String>(0),
                    )?,
                    "2026-07-22T01:30:00Z"
                );
                assert_eq!(
                    connection.query_row(
                        "SELECT COUNT(*) FROM memos WHERE id = 'old-memo'",
                        [],
                        |row| row.get::<_, i64>(0),
                    )?,
                    0
                );
                assert_eq!(
                    connection.query_row("SELECT COUNT(*) FROM window_state", [], |row| {
                        row.get::<_, i64>(0)
                    })?,
                    1
                );
                assert_eq!(
                    connection.query_row("SELECT COUNT(*) FROM backup_history", [], |row| {
                        row.get::<_, i64>(0)
                    })?,
                    1
                );
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn failed_restore_rolls_back_business_data_and_keeps_snapshot() {
        let source = populated_database();
        let backup = BackupService::parse_json(
            &BackupService::new(&source)
                .export_json_at(Utc.with_ymd_and_hms(2026, 7, 21, 9, 0, 0).unwrap())
                .unwrap(),
        )
        .unwrap();
        let target = target_database();
        target
            .write(|transaction| {
                transaction.execute_batch(
                    "CREATE TRIGGER reject_restored_memo
                     BEFORE INSERT ON memos
                     WHEN NEW.id = 'memo-1'
                     BEGIN
                         SELECT RAISE(ABORT, 'injected restore failure');
                     END;",
                )
            })
            .unwrap();
        let directory = tempfile::tempdir().unwrap();

        let error = BackupService::new(&target)
            .restore(
                backup,
                Path::new("selected.json"),
                directory.path(),
                Utc.with_ymd_and_hms(2026, 7, 21, 10, 0, 0).unwrap(),
            )
            .unwrap_err();

        assert_eq!(error.code, "BACKUP_RESTORE_FAILED");
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
        target
            .read(|connection| {
                assert_eq!(
                    connection.query_row(
                        "SELECT name FROM projects WHERE id = 'old-project'",
                        [],
                        |row| row.get::<_, String>(0),
                    )?,
                    "Existing data"
                );
                assert_eq!(
                    connection.query_row(
                        "SELECT title FROM memos WHERE id = 'old-memo'",
                        [],
                        |row| row.get::<_, String>(0),
                    )?,
                    "Existing memo"
                );
                assert_eq!(
                    connection.query_row("SELECT COUNT(*) FROM backup_history", [], |row| {
                        row.get::<_, i64>(0)
                    })?,
                    1
                );
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn exports_to_a_selected_path_and_records_history() {
        let database = populated_database();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("manual.json");

        let exported = BackupService::new(&database)
            .export_to_path(&path, Utc.with_ymd_and_hms(2026, 7, 21, 10, 0, 0).unwrap())
            .unwrap();

        assert_eq!(exported.summary.counts.total, 14);
        assert_eq!(
            BackupService::inspect_path(&path).unwrap().summary,
            exported.summary
        );
        database
            .read(|connection| {
                assert_eq!(
                    connection.query_row("SELECT kind FROM backup_history", [], |row| row
                        .get::<_, String>(0),)?,
                    "manual"
                );
                Ok(())
            })
            .unwrap();
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        /// Property P9, validates requirements R8.2-R8.5.
        #[test]
        fn p9_export_parse_and_import_round_trip_preserves_business_data(
            data in valid_backup_data(),
        ) {
            let exported_at = Utc.with_ymd_and_hms(2026, 7, 21, 9, 0, 0).unwrap();
            let (json, expected_summary) = serialize_envelope(BackupEnvelope {
                format_version: CURRENT_BACKUP_FORMAT_VERSION,
                exported_at: format_timestamp(exported_at),
                data: data.clone(),
            }).unwrap();
            let parsed = BackupService::parse_json(&json).unwrap();
            prop_assert_eq!(&parsed.summary, &expected_summary);

            let database = Database::open_in_memory().unwrap();
            let directory = tempfile::tempdir().unwrap();
            BackupService::new(&database)
                .restore(
                    parsed,
                    Path::new("generated.json"),
                    directory.path(),
                    Utc.with_ymd_and_hms(2026, 7, 21, 10, 0, 0).unwrap(),
                )
                .unwrap();

            let restored_json = BackupService::new(&database)
                .export_json_at(exported_at)
                .unwrap();
            let restored = BackupService::parse_json(&restored_json).unwrap();

            prop_assert_eq!(restored.envelope.data, data);
            prop_assert_eq!(restored.summary, expected_summary);
        }

        /// Property M7, validates requirements R8.2-R8.5.
        #[test]
        fn property_m7_backup_round_trip_preserves_normalized_memo_business_model(
            data in valid_memo_backup_data(),
        ) {
            let exported_at = Utc.with_ymd_and_hms(2026, 7, 21, 9, 0, 0).unwrap();
            let restored_at = Utc.with_ymd_and_hms(2026, 7, 21, 10, 0, 0).unwrap();
            let seed_json = serialize_envelope(BackupEnvelope {
                format_version: CURRENT_BACKUP_FORMAT_VERSION,
                exported_at: format_timestamp(exported_at),
                data: data.clone(),
            }).unwrap().0;
            let source = Database::open_in_memory().unwrap();
            let source_directory = tempfile::tempdir().unwrap();
            BackupService::new(&source)
                .restore(
                    BackupService::parse_json(&seed_json).unwrap(),
                    Path::new("generated-seed.json"),
                    source_directory.path(),
                    restored_at,
                )
                .unwrap();

            let exported = BackupService::new(&source)
                .export_json_at(exported_at)
                .unwrap();
            let parsed = BackupService::parse_json(&exported).unwrap();
            let target = Database::open_in_memory().unwrap();
            let target_directory = tempfile::tempdir().unwrap();
            BackupService::new(&target)
                .restore(
                    parsed,
                    Path::new("generated-export.json"),
                    target_directory.path(),
                    restored_at,
                )
                .unwrap();

            let restored_export = BackupService::new(&target)
                .export_json_at(exported_at)
                .unwrap();
            let restored = BackupService::parse_json(&restored_export).unwrap();

            prop_assert_eq!(
                normalized_memo_backup(restored.envelope.data),
                normalized_memo_backup(data),
            );
        }
    }

    fn target_database() -> Database {
        let database = Database::open_in_memory().unwrap();
        database
            .write(|transaction| {
                transaction.execute(
                    "INSERT INTO projects(id, name, description, color, icon, status, started_on, created_at, updated_at) VALUES ('old-project', 'Existing data', '', 'mint', 'archive', 'active', '2026-07-01', ?1, ?1)",
                    [NOW],
                )?;
                transaction.execute(
                    "INSERT INTO memos(id, title, body, created_at, updated_at) VALUES ('old-memo', 'Existing memo', 'Keep on rollback', ?1, ?1)",
                    [NOW],
                )?;
                transaction.execute(
                    "INSERT INTO window_state(window_label, x, y, width, height, scale_factor, maximized, updated_at) VALUES ('main', 0, 0, 1200, 800, 1, 0, ?1)",
                    [NOW],
                )?;
                Ok(())
            })
            .unwrap();
        database
    }

    fn populated_database() -> Database {
        let database = Database::open_in_memory().unwrap();
        database
            .write(|tx| {
                tx.execute(
                    "INSERT INTO projects(id, name, description, color, icon, status, started_on, target_on, created_at, updated_at) VALUES ('project-1', 'Backup', '', 'mint', 'archive', 'active', '2026-07-21', '2026-07-31', ?1, ?1)",
                    [NOW],
                )?;
                tx.execute(
                    "INSERT INTO tasks(id, project_id, title, category, priority, scheduled_date, scheduled_time, status, created_at, updated_at) VALUES ('task-1', 'project-1', 'Export data', 'work', 3, '2026-07-22', '09:30', 'pending', ?1, ?1)",
                    [NOW],
                )?;
                tx.execute(
                    "INSERT INTO task_check_items(id, task_id, title, position, created_at, updated_at) VALUES ('check-1', 'task-1', 'Validate JSON', 0, ?1, ?1)",
                    [NOW],
                )?;
                tx.execute(
                    "INSERT INTO recurrence_rules(id, task_template_id, pattern_json, timezone, starts_on, status, version, created_at, updated_at) VALUES ('rule-1', 'task-1', '{\"kind\":\"daily\",\"interval\":1}', 'Asia/Shanghai', '2026-07-21', 'active', 1, ?1, ?1)",
                    [NOW],
                )?;
                tx.execute(
                    "INSERT INTO task_instances(id, recurrence_rule_id, rule_version, scheduled_date, snapshot_title, snapshot_project_id, status, created_at, updated_at) VALUES ('instance-1', 'rule-1', 1, '2026-07-22', 'Export data', 'project-1', 'pending', ?1, ?1)",
                    [NOW],
                )?;
                tx.execute(
                    "INSERT INTO focus_sessions(id, task_id, project_id, planned_seconds, actual_seconds, interruption_count, completion_kind, started_at, ended_at, created_at) VALUES ('focus-1', 'task-1', 'project-1', 1500, 1200, 1, 'early', ?1, '2026-07-21T08:20:00Z', ?1)",
                    [NOW],
                )?;
                tx.execute(
                    "INSERT INTO active_focus(singleton_id, task_id, state, planned_seconds, remaining_seconds, started_at, target_ends_at, interruption_count, updated_at) VALUES (1, 'task-1', 'running', 1500, 1200, ?1, '2026-07-21T08:25:00Z', 0, ?1)",
                    [NOW],
                )?;
                tx.execute(
                    "INSERT INTO notes(id, body, note_date, created_at, updated_at) VALUES ('note-1', 'Portable data', '2026-07-21', ?1, ?1)",
                    [NOW],
                )?;
                tx.execute(
                    "INSERT INTO weekly_goals(id, week_starts_on, title, target_count, completed_count, position, created_at, updated_at, category) VALUES ('goal-1', '2026-07-20', 'Finish backup', 3, 1, 0, ?1, ?1, 'completed_tasks')",
                    [NOW],
                )?;
                tx.execute(
                    "INSERT INTO preferences(key, value_json, updated_at) VALUES ('generalPreferences', ?1, ?2)",
                    params![r#"{"language":"system","appearance":"system","theme":"mint","backgroundRunning":true}"#, NOW],
                )?;
                tx.execute(
                    "INSERT INTO memos(id, title, body, pinned_at, created_at, updated_at) VALUES ('memo-1', 'Portable memo', 'Memo backup body', ?1, ?1, ?1)",
                    [NOW],
                )?;
                tx.execute(
                    "INSERT INTO memo_tags(id, name, normalized_name, created_at) VALUES ('memo-tag-1', 'Release', 'release', ?1)",
                    [NOW],
                )?;
                tx.execute(
                    "INSERT INTO memo_tag_links(memo_id, tag_id) VALUES ('memo-1', 'memo-tag-1')",
                    [],
                )?;
                tx.execute(
                    "INSERT INTO memo_reminders(id, memo_id, schedule_kind, frequency, interval_value, weekdays_json, local_time, starts_on, timezone, next_scheduled_for, status, created_at, updated_at) VALUES ('memo-reminder-1', 'memo-1', 'recurring', 'weekly', 2, '[1,3]', '09:30', '2026-07-21', 'Asia/Shanghai', '2026-07-22T01:30:00Z', 'active', ?1, ?1)",
                    [NOW],
                )?;
                Ok(())
            })
            .unwrap();
        database
    }
}

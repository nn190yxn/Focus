use std::path::Path;

use arrive_focus_core::{
    repositories::database::Database, services::backup_service::BackupService,
};
use chrono::{TimeZone, Utc};
use rusqlite::{params, OptionalExtension};
use serde_json::json;

const NOW: &str = "2026-07-21T08:00:00Z";

#[test]
fn validation_rejects_unknown_versions_and_broken_references() {
    let source_directory = tempfile::tempdir().unwrap();
    let source = Database::open(source_directory.path().join("source.sqlite3")).unwrap();
    seed_source(&source);
    let json = export_json(&source);

    let mut unknown_version: serde_json::Value = serde_json::from_str(&json).unwrap();
    unknown_version["formatVersion"] = json!(99);
    let version_error = BackupService::parse_json(&unknown_version.to_string()).unwrap_err();
    assert_eq!(version_error.code, "BACKUP_VERSION_UNSUPPORTED");
    assert_eq!(version_error.field.as_deref(), Some("formatVersion"));

    let mut broken_reference: serde_json::Value = serde_json::from_str(&json).unwrap();
    broken_reference["data"]["tasks"][0]["projectId"] = json!("missing-project");
    let reference_error = BackupService::parse_json(&broken_reference.to_string()).unwrap_err();
    assert_eq!(reference_error.code, "BACKUP_REFERENCE_INVALID");
    assert_eq!(reference_error.field.as_deref(), Some("tasks.projectId"));

    let mut broken_memo_reference: serde_json::Value = serde_json::from_str(&json).unwrap();
    broken_memo_reference["data"]["memoTagLinks"][0]["tagId"] = json!("missing-tag");
    let memo_reference_error =
        BackupService::parse_json(&broken_memo_reference.to_string()).unwrap_err();
    assert_eq!(memo_reference_error.code, "BACKUP_REFERENCE_INVALID");
    assert_eq!(
        memo_reference_error.field.as_deref(),
        Some("memoTagLinks.tagId")
    );
}

#[test]
fn version_one_restore_replaces_existing_memos_with_empty_legacy_collections() {
    let source_directory = tempfile::tempdir().unwrap();
    let source = Database::open(source_directory.path().join("source.sqlite3")).unwrap();
    seed_source(&source);
    let mut legacy: serde_json::Value = serde_json::from_str(&export_json(&source)).unwrap();
    legacy["formatVersion"] = json!(1);
    let data = legacy["data"].as_object_mut().unwrap();
    for field in ["memos", "memoTags", "memoTagLinks", "memoReminders"] {
        data.remove(field);
    }
    let backup = BackupService::parse_json(&legacy.to_string()).unwrap();
    assert_eq!(backup.envelope.format_version, 1);
    assert_eq!(backup.summary.counts.memos, 0);

    let target_directory = tempfile::tempdir().unwrap();
    let target = Database::open(target_directory.path().join("target.sqlite3")).unwrap();
    seed_target(&target);
    BackupService::new(&target)
        .restore(
            backup,
            Path::new("legacy.json"),
            &target_directory.path().join("backups"),
            Utc.with_ymd_and_hms(2026, 7, 21, 10, 0, 0).unwrap(),
        )
        .unwrap();

    assert_eq!(
        project_name(&target, "incoming-project").as_deref(),
        Some("Incoming data")
    );
    assert_eq!(memo_title(&target, "existing-memo"), None);
    assert_eq!(memo_count(&target), 0);
}

#[test]
fn restore_replaces_business_data_and_creates_a_readable_snapshot() {
    let source_directory = tempfile::tempdir().unwrap();
    let source = Database::open(source_directory.path().join("source.sqlite3")).unwrap();
    seed_source(&source);
    let backup = BackupService::parse_json(&export_json(&source)).unwrap();

    let target_directory = tempfile::tempdir().unwrap();
    let target_path = target_directory.path().join("target.sqlite3");
    let target = Database::open(&target_path).unwrap();
    seed_target(&target);
    let snapshot_directory = target_directory.path().join("backups");

    let result = BackupService::new(&target)
        .restore(
            backup,
            Path::new("selected.json"),
            &snapshot_directory,
            Utc.with_ymd_and_hms(2026, 7, 21, 10, 0, 0).unwrap(),
        )
        .unwrap();

    let snapshot = BackupService::inspect_path(Path::new(&result.snapshot_path)).unwrap();
    assert_eq!(snapshot.envelope.data.projects.len(), 1);
    assert_eq!(snapshot.envelope.data.projects[0].id, "existing-project");
    assert_eq!(snapshot.envelope.data.memos.len(), 1);
    assert_eq!(snapshot.envelope.data.memos[0].id, "existing-memo");

    let restored = BackupService::parse_json(&export_json(&target)).unwrap();
    assert_eq!(restored.envelope.data.projects[0].id, "incoming-project");
    assert_eq!(
        restored.envelope.data.tasks[0].project_id.as_deref(),
        Some("incoming-project")
    );
    assert_eq!(restored.envelope.data.memos[0].id, "incoming-memo");
    assert_eq!(restored.envelope.data.memo_tags[0].id, "incoming-tag");
    assert_eq!(restored.envelope.data.memo_tag_links.len(), 1);
    assert_eq!(
        restored.envelope.data.memo_reminders[0].id,
        "incoming-reminder"
    );
    assert_eq!(restored.summary.counts.total, 6);

    drop(target);
    let reopened = Database::open(&target_path).unwrap();
    assert_eq!(
        project_name(&reopened, "incoming-project").as_deref(),
        Some("Incoming data")
    );
    assert_eq!(project_name(&reopened, "existing-project"), None);
    assert_eq!(
        memo_title(&reopened, "incoming-memo").as_deref(),
        Some("Incoming memo")
    );
    assert_eq!(memo_title(&reopened, "existing-memo"), None);
}

#[test]
fn failed_restore_rolls_back_data_and_preserves_the_pre_restore_snapshot() {
    let source_directory = tempfile::tempdir().unwrap();
    let source = Database::open(source_directory.path().join("source.sqlite3")).unwrap();
    seed_source(&source);
    let backup = BackupService::parse_json(&export_json(&source)).unwrap();

    let target_directory = tempfile::tempdir().unwrap();
    let target = Database::open(target_directory.path().join("target.sqlite3")).unwrap();
    seed_target(&target);
    target
        .write(|transaction| {
            transaction.execute_batch(
                "CREATE TRIGGER reject_incoming_memo
                 BEFORE INSERT ON memos
                 WHEN NEW.id = 'incoming-memo'
                 BEGIN
                     SELECT RAISE(ABORT, 'injected restore failure');
                 END;",
            )
        })
        .unwrap();

    let error = BackupService::new(&target)
        .restore(
            backup,
            Path::new("selected.json"),
            &target_directory.path().join("backups"),
            Utc.with_ymd_and_hms(2026, 7, 21, 10, 0, 0).unwrap(),
        )
        .unwrap_err();

    assert_eq!(error.code, "BACKUP_RESTORE_FAILED");
    assert_eq!(
        project_name(&target, "existing-project").as_deref(),
        Some("Existing data")
    );
    assert_eq!(project_name(&target, "incoming-project"), None);
    assert_eq!(
        memo_title(&target, "existing-memo").as_deref(),
        Some("Existing memo")
    );
    assert_eq!(memo_title(&target, "incoming-memo"), None);

    let snapshot_path = target
        .read(|connection| {
            connection.query_row(
                "SELECT path FROM backup_history WHERE kind = 'pre_restore'",
                [],
                |row| row.get::<_, String>(0),
            )
        })
        .unwrap();
    let snapshot = BackupService::inspect_path(Path::new(&snapshot_path)).unwrap();
    assert_eq!(snapshot.envelope.data.projects.len(), 1);
    assert_eq!(snapshot.envelope.data.projects[0].id, "existing-project");
    assert_eq!(snapshot.envelope.data.memos.len(), 1);
    assert_eq!(snapshot.envelope.data.memos[0].id, "existing-memo");
}

fn export_json(database: &Database) -> String {
    BackupService::new(database)
        .export_json_at(Utc.with_ymd_and_hms(2026, 7, 21, 9, 0, 0).unwrap())
        .unwrap()
}

fn seed_source(database: &Database) {
    database
        .write(|transaction| {
            transaction.execute(
                "INSERT INTO projects(id, name, description, color, icon, status, started_on, created_at, updated_at) VALUES ('incoming-project', 'Incoming data', '', 'mint', 'folder', 'active', '2026-07-21', ?1, ?1)",
                [NOW],
            )?;
            transaction.execute(
                "INSERT INTO tasks(id, project_id, title, category, priority, status, created_at, updated_at) VALUES ('incoming-task', 'incoming-project', 'Restore backup', 'work', 2, 'pending', ?1, ?1)",
                [NOW],
            )?;
            transaction.execute(
                "INSERT INTO memos(id, title, body, pinned_at, created_at, updated_at) VALUES ('incoming-memo', 'Incoming memo', 'Portable memo body', ?1, ?1, ?1)",
                [NOW],
            )?;
            transaction.execute(
                "INSERT INTO memo_tags(id, name, normalized_name, created_at) VALUES ('incoming-tag', 'Portable', 'portable', ?1)",
                [NOW],
            )?;
            transaction.execute(
                "INSERT INTO memo_tag_links(memo_id, tag_id) VALUES ('incoming-memo', 'incoming-tag')",
                [],
            )?;
            transaction.execute(
                "INSERT INTO memo_reminders(id, memo_id, schedule_kind, frequency, interval_value, weekdays_json, local_time, starts_on, timezone, next_scheduled_for, status, created_at, updated_at) VALUES ('incoming-reminder', 'incoming-memo', 'recurring', 'weekly', 1, '[2]', '09:30', '2026-07-21', 'Asia/Shanghai', '2026-07-21T01:30:00Z', 'active', ?1, ?1)",
                [NOW],
            )?;
            Ok(())
        })
        .unwrap();
}

fn seed_target(database: &Database) {
    database
        .write(|transaction| {
            transaction.execute(
                "INSERT INTO projects(id, name, description, color, icon, status, started_on, created_at, updated_at) VALUES ('existing-project', 'Existing data', '', 'blue', 'folder', 'active', '2026-07-01', ?1, ?1)",
                [NOW],
            )?;
            transaction.execute(
                "INSERT INTO window_state(window_label, x, y, width, height, scale_factor, maximized, updated_at) VALUES ('main', 0, 0, 1200, 800, 1, 0, ?1)",
                [NOW],
            )?;
            transaction.execute(
                "INSERT INTO memos(id, title, body, created_at, updated_at) VALUES ('existing-memo', 'Existing memo', 'Keep on restore failure', ?1, ?1)",
                [NOW],
            )?;
            Ok(())
        })
        .unwrap();
}

fn project_name(database: &Database, project_id: &str) -> Option<String> {
    database
        .read(|connection| {
            connection
                .query_row(
                    "SELECT name FROM projects WHERE id = ?1",
                    params![project_id],
                    |row| row.get(0),
                )
                .optional()
        })
        .unwrap()
}

fn memo_title(database: &Database, memo_id: &str) -> Option<String> {
    database
        .read(|connection| {
            connection
                .query_row(
                    "SELECT title FROM memos WHERE id = ?1",
                    params![memo_id],
                    |row| row.get(0),
                )
                .optional()
        })
        .unwrap()
}

fn memo_count(database: &Database) -> i64 {
    database
        .read(|connection| connection.query_row("SELECT COUNT(*) FROM memos", [], |row| row.get(0)))
        .unwrap()
}

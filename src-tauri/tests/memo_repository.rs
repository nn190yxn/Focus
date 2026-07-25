use arrive_focus_core::{
    domain::memo::{MemoInput, MemoListQuery, MemoReminderFrequency, MemoReminderInput},
    repositories::{database::Database, memo_repository::MemoRepository},
    services::memo_service::{MemoService, NormalizedMemoTag},
};
use chrono::{TimeZone, Utc};

fn input(title: &str, body: &str, tags: &[&str]) -> MemoInput {
    MemoInput {
        title: title.into(),
        body: body.into(),
        tags: tags.iter().map(|tag| (*tag).into()).collect(),
        pinned: false,
        reminder: None,
    }
}

#[test]
fn repository_crud_search_tags_reminder_and_delete_round_trip() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("memos.sqlite3");
    let database = Database::open(&path).unwrap();
    let repository = MemoRepository::new(&database);
    let created_at = Utc.with_ymd_and_hms(2026, 7, 23, 10, 0, 0).unwrap();
    let first_input = input("Budget 100%", "Review literal search", &["Finance", "Work"]);
    let core = MemoService::create("memo-1".into(), &first_input, created_at).unwrap();
    let tags = MemoService::normalize_tags(&first_input.tags).unwrap();
    let created = repository
        .create(&core, &tags, None, "Untitled memo")
        .unwrap();
    let finance_id = created
        .tags
        .iter()
        .find(|tag| tag.name == "Finance")
        .unwrap()
        .id
        .clone();

    let found = repository
        .list(
            &MemoListQuery {
                search: "%".into(),
                tag_id: Some(finance_id),
            },
            "Untitled memo",
        )
        .unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].id, "memo-1");

    let update_input = input("", "\n  Updated display line", &["Personal"]);
    let updated_core = MemoService::update(
        &core,
        &update_input,
        Utc.with_ymd_and_hms(2026, 7, 23, 11, 0, 0).unwrap(),
    )
    .unwrap();
    let updated_tags = MemoService::normalize_tags(&update_input.tags).unwrap();
    let updated = repository
        .update(&updated_core, &updated_tags, None, "Untitled memo")
        .unwrap();
    assert_eq!(updated.display_title, "Updated display line");
    assert_eq!(repository.list_tags().unwrap()[0].name, "Personal");

    database
        .write(|transaction| {
            transaction.execute(
                "INSERT INTO memo_reminders(id, memo_id, schedule_kind, frequency, interval_value,
                    monthly_day, local_time, starts_on, timezone, next_scheduled_for, status,
                    created_at, updated_at)
                 VALUES ('reminder-1', 'memo-1', 'recurring', 'monthly', 1, 31, '09:30',
                    '2026-07-23', 'Asia/Shanghai', '2026-07-31T01:30:00Z', 'active',
                    '2026-07-23T11:00:00Z', '2026-07-23T11:00:00Z')",
                [],
            )?;
            Ok(())
        })
        .unwrap();
    let detail = repository.get("memo-1", "Untitled memo").unwrap().unwrap();
    assert_eq!(
        detail.reminder.unwrap().schedule,
        MemoReminderInput::Recurring {
            frequency: MemoReminderFrequency::Monthly,
            interval: 1,
            weekdays: Vec::new(),
            monthly_day: Some(31),
            local_time: "09:30".into(),
            starts_on: "2026-07-23".into(),
            ends_on: None,
            timezone: "Asia/Shanghai".into(),
        }
    );

    repository.remove("memo-1").unwrap();
    assert!(repository.get("memo-1", "Untitled memo").unwrap().is_none());
    assert!(repository.list_tags().unwrap().is_empty());
    drop(database);

    let reopened = Database::open(&path).unwrap();
    assert!(MemoRepository::new(&reopened)
        .get("memo-1", "Untitled memo")
        .unwrap()
        .is_none());
}

#[test]
fn failed_related_write_rolls_back_after_database_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("rollback.sqlite3");
    let database = Database::open(&path).unwrap();
    let repository = MemoRepository::new(&database);
    let original_input = input("Original", "Body", &["Work"]);
    let original_core = MemoService::create(
        "memo-1".into(),
        &original_input,
        Utc.with_ymd_and_hms(2026, 7, 23, 10, 0, 0).unwrap(),
    )
    .unwrap();
    let original_tags = MemoService::normalize_tags(&original_input.tags).unwrap();
    repository
        .create(&original_core, &original_tags, None, "Untitled memo")
        .unwrap();
    database
        .write(|transaction| {
            transaction.execute_batch(
                "CREATE TRIGGER reject_blocked_memo_tag
                 BEFORE INSERT ON memo_tags
                 WHEN NEW.normalized_name = 'blocked'
                 BEGIN
                     SELECT RAISE(ABORT, 'injected integration failure');
                 END;",
            )
        })
        .unwrap();
    let changed_input = input("Changed", "Changed body", &[]);
    let changed_core = MemoService::update(
        &original_core,
        &changed_input,
        Utc.with_ymd_and_hms(2026, 7, 23, 11, 0, 0).unwrap(),
    )
    .unwrap();
    let blocked_tags = vec![NormalizedMemoTag {
        name: "Blocked".into(),
        normalized_name: "blocked".into(),
    }];

    let error = repository
        .update(&changed_core, &blocked_tags, None, "Untitled memo")
        .unwrap_err();
    assert_eq!(error.code, "MEMO_SAVE_FAILED");
    drop(database);

    let reopened = Database::open(&path).unwrap();
    let preserved = MemoRepository::new(&reopened)
        .get("memo-1", "Untitled memo")
        .unwrap()
        .unwrap();
    assert_eq!(preserved.title, "Original");
    assert_eq!(preserved.body, "Body");
    assert_eq!(preserved.tags[0].name, "Work");
}

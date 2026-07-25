#[cfg(any(feature = "desktop-app", test))]
use chrono::Utc;

#[cfg(any(feature = "desktop-app", test))]
use crate::{
    domain::memo::{MemoInput, MemoRecord},
    repositories::{database::Database, memo_repository::MemoRepository},
    services::{
        memo_reminder_service::MemoReminderService,
        memo_service::{MemoCoreRecord, MemoService},
    },
    DomainError,
};

#[cfg(feature = "desktop-app")]
use crate::{
    domain::memo::{MemoListQuery, MemoSummary, MemoTagSummary},
    CommandResult,
};

#[cfg(any(feature = "desktop-app", test))]
const UNTITLED_MEMO_LABEL: &str = "Untitled memo";

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn memo_list(
    database: tauri::State<'_, Database>,
    query: MemoListQuery,
) -> CommandResult<Vec<MemoSummary>> {
    result(MemoRepository::new(&database).list(&query, UNTITLED_MEMO_LABEL))
}

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn memo_get(database: tauri::State<'_, Database>, id: String) -> CommandResult<MemoRecord> {
    result(
        MemoRepository::new(&database)
            .get(&id, UNTITLED_MEMO_LABEL)
            .and_then(|memo| memo.ok_or_else(memo_not_found)),
    )
}

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn memo_create(
    database: tauri::State<'_, Database>,
    app: tauri::AppHandle,
    input: MemoInput,
) -> CommandResult<MemoRecord> {
    result_after_memo_change(&app, create_memo(&database, &input))
}

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn memo_update(
    database: tauri::State<'_, Database>,
    app: tauri::AppHandle,
    id: String,
    input: MemoInput,
) -> CommandResult<MemoRecord> {
    result_after_memo_change(&app, update_memo(&database, &id, &input))
}

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn memo_remove(
    database: tauri::State<'_, Database>,
    app: tauri::AppHandle,
    id: String,
) -> CommandResult<()> {
    result_after_memo_change(&app, remove_memo(&database, &id))
}

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn memo_tag_list(database: tauri::State<'_, Database>) -> CommandResult<Vec<MemoTagSummary>> {
    result(MemoRepository::new(&database).list_tags())
}

#[cfg(any(feature = "desktop-app", test))]
fn create_memo(database: &Database, input: &MemoInput) -> Result<MemoRecord, DomainError> {
    let now = Utc::now();
    let tags = MemoService::normalize_tags(&input.tags)?;
    let memo = MemoService::create(uuid::Uuid::new_v4().to_string(), input, now)?;
    let reminder = MemoReminderService::prepare_rule(&memo.id, None, input.reminder.as_ref(), now)?;
    MemoRepository::new(database).create(&memo, &tags, reminder.as_ref(), UNTITLED_MEMO_LABEL)
}

#[cfg(any(feature = "desktop-app", test))]
fn update_memo(
    database: &Database,
    id: &str,
    input: &MemoInput,
) -> Result<MemoRecord, DomainError> {
    let repository = MemoRepository::new(database);
    let current = repository
        .get(id, UNTITLED_MEMO_LABEL)?
        .ok_or_else(memo_not_found)?;
    let now = Utc::now();
    let reminder = MemoReminderService::prepare_rule(
        id,
        current.reminder.as_ref(),
        input.reminder.as_ref(),
        now,
    )?;
    let current = MemoCoreRecord {
        id: current.id,
        title: current.title,
        body: current.body,
        pinned_at: current.pinned_at,
        created_at: current.created_at,
        updated_at: current.updated_at,
    };
    let tags = MemoService::normalize_tags(&input.tags)?;
    let memo = MemoService::update(&current, input, now)?;
    repository.update(&memo, &tags, reminder.as_ref(), UNTITLED_MEMO_LABEL)
}

#[cfg(any(feature = "desktop-app", test))]
fn remove_memo(database: &Database, id: &str) -> Result<(), DomainError> {
    MemoRepository::new(database).remove(id)
}

#[cfg(any(feature = "desktop-app", test))]
fn memo_not_found() -> DomainError {
    DomainError {
        code: "MEMO_NOT_FOUND".into(),
        message: "memo was not found".into(),
        field: None,
    }
}

#[cfg(feature = "desktop-app")]
fn result<T>(value: Result<T, DomainError>) -> CommandResult<T> {
    CommandResult::from_result(module_path!(), value, 1)
}

#[cfg(feature = "desktop-app")]
fn result_after_memo_change<T>(
    app: &tauri::AppHandle,
    value: Result<T, DomainError>,
) -> CommandResult<T> {
    result(crate::desktop::memo_events::after_memo_change(
        value,
        || {
            crate::desktop::memo_events::emit_memo_changed(app);
        },
    ))
}

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use super::*;

    fn input(title: &str, body: &str, tags: &[&str], pinned: bool) -> MemoInput {
        MemoInput {
            title: title.into(),
            body: body.into(),
            tags: tags.iter().map(|tag| (*tag).into()).collect(),
            pinned,
            reminder: None,
        }
    }

    #[test]
    fn command_helpers_create_update_and_remove_authoritative_records() {
        let database = Database::open_in_memory().unwrap();

        let created = create_memo(
            &database,
            &input("  Launch plan  ", "First body", &[" Work ", "work"], true),
        )
        .unwrap();
        assert_eq!(created.title, "Launch plan");
        assert_eq!(created.tags.len(), 1);
        assert!(created.pinned_at.is_some());

        let updated = update_memo(
            &database,
            &created.id,
            &input("Review plan", "Updated body", &["Personal"], false),
        )
        .unwrap();
        assert_eq!(updated.title, "Review plan");
        assert_eq!(updated.tags[0].name, "Personal");
        assert_eq!(updated.pinned_at, None);

        remove_memo(&database, &created.id).unwrap();
        assert_eq!(
            MemoRepository::new(&database)
                .get(&created.id, UNTITLED_MEMO_LABEL)
                .unwrap(),
            None
        );
    }

    #[test]
    fn command_helpers_preserve_stable_errors_on_failed_writes() {
        let database = Database::open_in_memory().unwrap();
        let invalid = input("Title", "Body", &[""], false);

        assert_eq!(
            create_memo(&database, &invalid).unwrap_err().code,
            "MEMO_TAG_INVALID"
        );
        assert_eq!(
            update_memo(&database, "missing", &input("Title", "Body", &[], false))
                .unwrap_err()
                .code,
            "MEMO_NOT_FOUND"
        );
        assert_eq!(
            remove_memo(&database, "missing").unwrap_err().code,
            "MEMO_NOT_FOUND"
        );
    }

    #[test]
    fn command_helpers_persist_replace_and_cancel_reminders() {
        let database = Database::open_in_memory().unwrap();
        let mut create_input = input("Title", "Body", &[], false);
        create_input.reminder = Some(crate::domain::memo::MemoReminderInput::Once {
            scheduled_local: (Utc::now() + Duration::days(1))
                .format("%Y-%m-%dT%H:%M")
                .to_string(),
            timezone: "UTC".into(),
        });

        let created = create_memo(&database, &create_input).unwrap();
        let original = created.reminder.unwrap();
        assert_eq!(
            original.status,
            crate::domain::memo::MemoReminderStatus::Active
        );
        assert!(original.next_scheduled_for.is_some());

        let mut update_input = input("Title", "Body", &[], false);
        update_input.reminder = Some(crate::domain::memo::MemoReminderInput::Once {
            scheduled_local: (Utc::now() + Duration::days(2))
                .format("%Y-%m-%dT%H:%M")
                .to_string(),
            timezone: "UTC".into(),
        });
        let updated = update_memo(&database, &created.id, &update_input).unwrap();
        let replaced = updated.reminder.unwrap();
        assert_eq!(replaced.id, original.id);
        assert_ne!(replaced.next_scheduled_for, original.next_scheduled_for);

        let cancelled =
            update_memo(&database, &created.id, &input("Title", "Body", &[], false)).unwrap();
        assert_eq!(cancelled.reminder, None);
    }
}

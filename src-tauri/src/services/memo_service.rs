use std::collections::HashSet;

use chrono::{DateTime, Utc};

use crate::{domain::memo::MemoInput, DomainError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoCoreRecord {
    pub id: String,
    pub title: String,
    pub body: String,
    pub pinned_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedMemoTag {
    pub name: String,
    pub normalized_name: String,
}

pub struct MemoService;

impl MemoService {
    pub fn normalize_tags(tags: &[String]) -> Result<Vec<NormalizedMemoTag>, DomainError> {
        let mut seen = HashSet::with_capacity(tags.len());
        let mut normalized = Vec::with_capacity(tags.len());
        for tag in tags {
            let name = tag.trim();
            if name.is_empty() || name.chars().count() > 30 {
                return Err(DomainError {
                    code: "MEMO_TAG_INVALID".into(),
                    message: "memo tag must contain between 1 and 30 characters".into(),
                    field: Some("tags".into()),
                });
            }
            let normalized_name = name.to_lowercase();
            if seen.insert(normalized_name.clone()) {
                normalized.push(NormalizedMemoTag {
                    name: name.to_owned(),
                    normalized_name,
                });
            }
        }
        if normalized.len() > 10 {
            return Err(DomainError {
                code: "MEMO_TAG_LIMIT_EXCEEDED".into(),
                message: "memo must contain at most 10 unique tags".into(),
                field: Some("tags".into()),
            });
        }
        Ok(normalized)
    }

    pub fn create(
        id: String,
        input: &MemoInput,
        now: DateTime<Utc>,
    ) -> Result<MemoCoreRecord, DomainError> {
        input.validate_at(now)?;
        let timestamp = now.to_rfc3339();
        Ok(MemoCoreRecord {
            id,
            title: input.title.trim().to_owned(),
            body: input.body.clone(),
            pinned_at: input.pinned.then(|| timestamp.clone()),
            created_at: timestamp.clone(),
            updated_at: timestamp,
        })
    }

    pub fn get(record: Option<MemoCoreRecord>) -> Result<MemoCoreRecord, DomainError> {
        record.ok_or_else(memo_not_found)
    }

    pub fn update(
        current: &MemoCoreRecord,
        input: &MemoInput,
        now: DateTime<Utc>,
    ) -> Result<MemoCoreRecord, DomainError> {
        input.validate_at(now)?;
        let timestamp = now.to_rfc3339();
        Ok(MemoCoreRecord {
            id: current.id.clone(),
            title: input.title.trim().to_owned(),
            body: input.body.clone(),
            pinned_at: if input.pinned {
                current
                    .pinned_at
                    .clone()
                    .or_else(|| Some(timestamp.clone()))
            } else {
                None
            },
            created_at: current.created_at.clone(),
            updated_at: timestamp,
        })
    }

    pub fn display_title(title: &str, body: &str, untitled_label: &str) -> String {
        let title = title.trim();
        if !title.is_empty() {
            return title.to_owned();
        }

        body.lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .map(|line| line.chars().take(40).collect())
            .unwrap_or_else(|| untitled_label.to_owned())
    }
}

fn memo_not_found() -> DomainError {
    DomainError {
        code: "MEMO_NOT_FOUND".into(),
        message: "memo was not found".into(),
        field: None,
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn input(title: &str, body: &str, pinned: bool) -> MemoInput {
        MemoInput {
            title: title.into(),
            body: body.into(),
            tags: Vec::new(),
            pinned,
            reminder: None,
        }
    }

    #[test]
    fn create_normalizes_title_and_initializes_a_single_timestamp() {
        let now = Utc.with_ymd_and_hms(2026, 7, 23, 10, 30, 0).unwrap();
        let memo = MemoService::create(
            "memo-1".into(),
            &input("  Launch notes  ", " Keep body spacing ", true),
            now,
        )
        .unwrap();

        assert_eq!(memo.id, "memo-1");
        assert_eq!(memo.title, "Launch notes");
        assert_eq!(memo.body, " Keep body spacing ");
        assert_eq!(memo.pinned_at.as_deref(), Some(memo.created_at.as_str()));
        assert_eq!(memo.created_at, memo.updated_at);
    }

    #[test]
    fn update_preserves_identity_creation_and_original_pin_time() {
        let created = Utc.with_ymd_and_hms(2026, 7, 23, 10, 0, 0).unwrap();
        let current =
            MemoService::create("memo-1".into(), &input("First", "Body", true), created).unwrap();
        let updated = MemoService::update(
            &current,
            &input("Second", "Changed", true),
            Utc.with_ymd_and_hms(2026, 7, 23, 11, 0, 0).unwrap(),
        )
        .unwrap();

        assert_eq!(updated.id, current.id);
        assert_eq!(updated.created_at, current.created_at);
        assert_eq!(updated.pinned_at, current.pinned_at);
        assert_ne!(updated.updated_at, current.updated_at);

        let unpinned = MemoService::update(
            &updated,
            &input("Second", "Changed", false),
            Utc.with_ymd_and_hms(2026, 7, 23, 12, 0, 0).unwrap(),
        )
        .unwrap();
        assert_eq!(unpinned.pinned_at, None);
    }

    #[test]
    fn get_returns_a_stable_not_found_error() {
        let error = MemoService::get(None).unwrap_err();
        assert_eq!(error.code, "MEMO_NOT_FOUND");
        assert_eq!(error.field, None);
    }

    #[test]
    fn display_title_uses_title_body_line_and_localized_fallback_in_order() {
        assert_eq!(
            MemoService::display_title("  Explicit title  ", "Body", "Untitled"),
            "Explicit title"
        );
        let derived = MemoService::display_title(
            "",
            "\n   \n  第一行内容超过四十个字符以验证使用Unicode字符安全截断并忽略后续内容ABCDEFGHIJ\nSecond",
            "Untitled",
        );
        assert_eq!(
            derived,
            "第一行内容超过四十个字符以验证使用Unicode字符安全截断并忽略后续内容ABC"
        );
        assert_eq!(derived.chars().count(), 40);
        assert_eq!(
            MemoService::display_title(" ", "\n ", "无标题备忘录"),
            "无标题备忘录"
        );
    }

    #[test]
    fn tag_normalization_trims_deduplicates_and_preserves_first_spelling() {
        let tags =
            MemoService::normalize_tags(&[" Work ".into(), "work".into(), "PERSONAL".into()])
                .unwrap();

        assert_eq!(
            tags,
            vec![
                NormalizedMemoTag {
                    name: "Work".into(),
                    normalized_name: "work".into(),
                },
                NormalizedMemoTag {
                    name: "PERSONAL".into(),
                    normalized_name: "personal".into(),
                },
            ]
        );
    }
}

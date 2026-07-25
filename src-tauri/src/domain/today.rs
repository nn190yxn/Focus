use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TodaySourceKind {
    Task,
    RecurringInstance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TodayItemKind {
    OrdinaryTask,
    ProjectTask,
    RecurringInstance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TodayItemStatus {
    Pending,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodayProjectSummary {
    pub id: String,
    pub name: String,
    pub color: String,
    pub icon: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodayDigestItem {
    pub source_kind: TodaySourceKind,
    pub source_id: String,
    pub item_kind: TodayItemKind,
    pub recurrence_rule_id: Option<String>,
    pub title: String,
    pub category: String,
    pub priority: i64,
    pub scheduled_date: String,
    pub scheduled_time: Option<String>,
    pub status: TodayItemStatus,
    pub completed_at: Option<String>,
    pub project: Option<TodayProjectSummary>,
    pub is_overdue: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodayDigest {
    pub date: String,
    pub items: Vec<TodayDigestItem>,
}

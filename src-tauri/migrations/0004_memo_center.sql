CREATE TABLE memos (
    id TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL CHECK(length(title) <= 200),
    body TEXT NOT NULL CHECK(length(body) <= 20000),
    pinned_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE memo_tags (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL CHECK(length(name) BETWEEN 1 AND 30),
    normalized_name TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL
);

CREATE TABLE memo_tag_links (
    memo_id TEXT NOT NULL REFERENCES memos(id) ON DELETE CASCADE,
    tag_id TEXT NOT NULL REFERENCES memo_tags(id) ON DELETE CASCADE,
    PRIMARY KEY (memo_id, tag_id)
);

CREATE TABLE memo_reminders (
    id TEXT PRIMARY KEY NOT NULL,
    memo_id TEXT NOT NULL UNIQUE REFERENCES memos(id) ON DELETE CASCADE,
    schedule_kind TEXT NOT NULL CHECK(schedule_kind IN ('once', 'recurring')),
    frequency TEXT CHECK(frequency IN ('daily', 'weekdays', 'weekly', 'monthly')),
    interval_value INTEGER CHECK(interval_value IS NULL OR interval_value > 0),
    weekdays_json TEXT,
    monthly_day INTEGER CHECK(monthly_day IS NULL OR monthly_day BETWEEN 1 AND 31),
    local_time TEXT NOT NULL,
    starts_on TEXT NOT NULL,
    ends_on TEXT,
    timezone TEXT NOT NULL,
    next_scheduled_for TEXT,
    status TEXT NOT NULL CHECK(status IN ('active', 'completed', 'cancelled')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX memos_sort_idx ON memos(pinned_at, updated_at);
CREATE INDEX memo_tag_links_tag_idx ON memo_tag_links(tag_id, memo_id);
CREATE INDEX memo_reminders_due_idx ON memo_reminders(status, next_scheduled_for);

ALTER TABLE notification_deliveries RENAME TO notification_deliveries_legacy;
DROP INDEX notification_deliveries_status_idx;

CREATE TABLE notification_deliveries (
    id TEXT PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL CHECK(kind IN ('focusCompleted', 'taskDue', 'recurringTaskDue', 'memoReminder')),
    source_id TEXT NOT NULL,
    scheduled_for TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('pending', 'sent', 'failed')),
    sound_enabled INTEGER NOT NULL CHECK(sound_enabled IN (0, 1)),
    created_at TEXT NOT NULL,
    sent_at TEXT,
    last_error TEXT,
    UNIQUE(kind, source_id, scheduled_for)
);

INSERT INTO notification_deliveries(
    id, kind, source_id, scheduled_for, status, sound_enabled, created_at, sent_at, last_error
)
SELECT id, kind, source_id, scheduled_for, status, sound_enabled, created_at, sent_at, last_error
FROM notification_deliveries_legacy;

DROP TABLE notification_deliveries_legacy;
CREATE INDEX notification_deliveries_status_idx ON notification_deliveries(status, created_at);

CREATE TABLE notification_deliveries (
    id TEXT PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL CHECK(kind IN ('focusCompleted', 'taskDue', 'recurringTaskDue')),
    source_id TEXT NOT NULL,
    scheduled_for TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('pending', 'sent', 'failed')),
    sound_enabled INTEGER NOT NULL CHECK(sound_enabled IN (0, 1)),
    created_at TEXT NOT NULL,
    sent_at TEXT,
    last_error TEXT,
    UNIQUE(kind, source_id, scheduled_for)
);

CREATE INDEX notification_deliveries_status_idx
    ON notification_deliveries(status, created_at);

CREATE TABLE projects (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL CHECK(length(name) BETWEEN 1 AND 80),
    description TEXT NOT NULL DEFAULT '' CHECK(length(description) <= 2000),
    color TEXT NOT NULL,
    icon TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('active', 'paused', 'completed', 'archived')),
    started_on TEXT NOT NULL,
    target_on TEXT CHECK(target_on IS NULL OR target_on >= started_on),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX projects_status_idx ON projects(status);

CREATE TABLE tasks (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
    title TEXT NOT NULL CHECK(length(title) BETWEEN 1 AND 200),
    category TEXT NOT NULL,
    priority INTEGER NOT NULL DEFAULT 0 CHECK(priority BETWEEN 0 AND 3),
    scheduled_date TEXT,
    scheduled_time TEXT,
    status TEXT NOT NULL CHECK(status IN ('pending', 'completed', 'removed')),
    completed_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK((status = 'completed' AND completed_at IS NOT NULL) OR (status != 'completed' AND completed_at IS NULL))
);

CREATE INDEX tasks_date_status_idx ON tasks(scheduled_date, status);
CREATE INDEX tasks_project_idx ON tasks(project_id);

CREATE TABLE task_check_items (
    id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    title TEXT NOT NULL CHECK(length(title) BETWEEN 1 AND 200),
    position INTEGER NOT NULL CHECK(position >= 0),
    completed_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(task_id, position)
);

CREATE TABLE recurrence_rules (
    id TEXT PRIMARY KEY NOT NULL,
    task_template_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE RESTRICT,
    pattern_json TEXT NOT NULL CHECK(json_valid(pattern_json)),
    local_time TEXT,
    timezone TEXT NOT NULL,
    starts_on TEXT NOT NULL,
    ends_on TEXT CHECK(ends_on IS NULL OR ends_on >= starts_on),
    status TEXT NOT NULL CHECK(status IN ('active', 'paused', 'ended')),
    version INTEGER NOT NULL CHECK(version > 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX recurrence_rules_status_idx ON recurrence_rules(status);

CREATE TABLE task_instances (
    id TEXT PRIMARY KEY NOT NULL,
    recurrence_rule_id TEXT NOT NULL REFERENCES recurrence_rules(id) ON DELETE CASCADE,
    rule_version INTEGER NOT NULL CHECK(rule_version > 0),
    scheduled_date TEXT NOT NULL,
    scheduled_at TEXT,
    snapshot_title TEXT NOT NULL CHECK(length(snapshot_title) BETWEEN 1 AND 200),
    snapshot_project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
    status TEXT NOT NULL CHECK(status IN ('pending', 'completed', 'skipped', 'rescheduled')),
    completed_at TEXT,
    source_instance_id TEXT REFERENCES task_instances(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(recurrence_rule_id, scheduled_date)
);

CREATE INDEX task_instances_date_status_idx ON task_instances(scheduled_date, status);

CREATE TABLE focus_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT REFERENCES tasks(id) ON DELETE SET NULL,
    task_instance_id TEXT REFERENCES task_instances(id) ON DELETE SET NULL,
    project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
    planned_seconds INTEGER NOT NULL CHECK(planned_seconds > 0),
    actual_seconds INTEGER NOT NULL CHECK(actual_seconds >= 0),
    interruption_count INTEGER NOT NULL DEFAULT 0 CHECK(interruption_count >= 0),
    completion_kind TEXT NOT NULL CHECK(completion_kind IN ('deadline', 'early', 'cancelled')),
    started_at TEXT NOT NULL,
    ended_at TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX focus_sessions_project_started_idx ON focus_sessions(project_id, started_at);

CREATE TABLE active_focus (
    singleton_id INTEGER PRIMARY KEY CHECK(singleton_id = 1),
    task_id TEXT REFERENCES tasks(id) ON DELETE SET NULL,
    task_instance_id TEXT REFERENCES task_instances(id) ON DELETE SET NULL,
    state TEXT NOT NULL CHECK(state IN ('running', 'paused')),
    planned_seconds INTEGER NOT NULL CHECK(planned_seconds > 0),
    remaining_seconds INTEGER NOT NULL CHECK(remaining_seconds >= 0),
    started_at TEXT NOT NULL,
    target_ends_at TEXT,
    paused_at TEXT,
    interruption_count INTEGER NOT NULL DEFAULT 0 CHECK(interruption_count >= 0),
    updated_at TEXT NOT NULL
);

CREATE TABLE preferences (
    key TEXT PRIMARY KEY NOT NULL,
    value_json TEXT NOT NULL CHECK(json_valid(value_json)),
    updated_at TEXT NOT NULL
);

CREATE TABLE window_state (
    window_label TEXT PRIMARY KEY NOT NULL,
    x REAL NOT NULL,
    y REAL NOT NULL,
    width REAL NOT NULL CHECK(width > 0),
    height REAL NOT NULL CHECK(height > 0),
    monitor_id TEXT,
    scale_factor REAL NOT NULL DEFAULT 1 CHECK(scale_factor > 0),
    maximized INTEGER NOT NULL DEFAULT 0 CHECK(maximized IN (0, 1)),
    updated_at TEXT NOT NULL
);

CREATE TABLE widget_layout (
    id TEXT PRIMARY KEY NOT NULL,
    mode TEXT NOT NULL CHECK(mode IN ('compact', 'standard', 'expanded')),
    desktop_mode TEXT NOT NULL CHECK(desktop_mode IN ('desktop', 'floating')),
    locked INTEGER NOT NULL DEFAULT 0 CHECK(locked IN (0, 1)),
    opacity REAL NOT NULL DEFAULT 1 CHECK(opacity BETWEEN 0.2 AND 1),
    modules_json TEXT NOT NULL CHECK(json_valid(modules_json)),
    last_visible_at TEXT,
    updated_at TEXT NOT NULL
);

CREATE TABLE notes (
    id TEXT PRIMARY KEY NOT NULL,
    body TEXT NOT NULL CHECK(length(body) <= 4000),
    note_date TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX notes_date_idx ON notes(note_date);

CREATE TABLE weekly_goals (
    id TEXT PRIMARY KEY NOT NULL,
    week_starts_on TEXT NOT NULL,
    title TEXT NOT NULL CHECK(length(title) BETWEEN 1 AND 200),
    target_count INTEGER NOT NULL CHECK(target_count > 0),
    completed_count INTEGER NOT NULL DEFAULT 0 CHECK(completed_count BETWEEN 0 AND target_count),
    position INTEGER NOT NULL CHECK(position >= 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(week_starts_on, position)
);

CREATE TABLE backup_history (
    id TEXT PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL CHECK(kind IN ('manual', 'automatic', 'pre_restore')),
    path TEXT NOT NULL,
    format_version INTEGER NOT NULL CHECK(format_version > 0),
    checksum TEXT NOT NULL,
    created_at TEXT NOT NULL
);

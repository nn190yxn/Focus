ALTER TABLE weekly_goals
ADD COLUMN category TEXT NOT NULL DEFAULT 'completed_tasks'
CHECK(category IN ('completed_tasks', 'focus_minutes', 'active_days'));

CREATE INDEX weekly_goals_week_category_idx
ON weekly_goals(week_starts_on, category);

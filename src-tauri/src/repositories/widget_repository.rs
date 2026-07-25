use chrono::{DateTime, Utc};
use rusqlite::OptionalExtension;

use crate::{
    domain::widget::{
        WidgetConfig, WidgetConfigInput, WidgetMode, WidgetModule, WidgetSize, WIDGET_LAYOUT_ID,
        WIDGET_WINDOW_LABEL,
    },
    repositories::database::Database,
    DomainError,
};

pub struct WidgetRepository<'a> {
    database: &'a Database,
}

struct StoredWidgetConfig {
    size: String,
    mode: String,
    locked: bool,
    opacity: f64,
    modules_json: String,
    last_visible_at: Option<String>,
    updated_at: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    monitor_id: Option<String>,
    scale_factor: f64,
}

impl<'a> WidgetRepository<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn get(&self) -> Result<Option<WidgetConfig>, DomainError> {
        let stored = self.database.read(|connection| {
            connection
                .query_row(
                    "SELECT layout.mode, layout.desktop_mode, layout.locked, layout.opacity,
                        layout.modules_json, layout.last_visible_at, layout.updated_at,
                        window.x, window.y, window.width, window.height, window.monitor_id,
                        window.scale_factor
                 FROM widget_layout layout
                 JOIN window_state window ON window.window_label = ?2
                 WHERE layout.id = ?1",
                    (WIDGET_LAYOUT_ID, WIDGET_WINDOW_LABEL),
                    |row| {
                        Ok(StoredWidgetConfig {
                            size: row.get(0)?,
                            mode: row.get(1)?,
                            locked: row.get(2)?,
                            opacity: row.get(3)?,
                            modules_json: row.get(4)?,
                            last_visible_at: row.get(5)?,
                            updated_at: row.get(6)?,
                            x: row.get(7)?,
                            y: row.get(8)?,
                            width: row.get(9)?,
                            height: row.get(10)?,
                            monitor_id: row.get(11)?,
                            scale_factor: row.get(12)?,
                        })
                    },
                )
                .optional()
        })?;
        stored.map(parse_stored).transpose()
    }

    pub fn save(&self, config: &WidgetConfig) -> Result<(), DomainError> {
        let input = &config.input;
        let modules_json = serde_json::to_string(&input.modules).map_err(config_error)?;
        self.database.write(|transaction| {
            transaction.execute(
                "INSERT INTO widget_layout(id, mode, desktop_mode, locked, opacity, modules_json, last_visible_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(id) DO UPDATE SET mode = excluded.mode, desktop_mode = excluded.desktop_mode,
                    locked = excluded.locked, opacity = excluded.opacity, modules_json = excluded.modules_json,
                    last_visible_at = excluded.last_visible_at, updated_at = excluded.updated_at",
                (
                    WIDGET_LAYOUT_ID,
                    input.size.as_str(),
                    input.mode.as_str(),
                    input.locked,
                    input.opacity,
                    modules_json,
                    config.last_visible_at.map(|value| value.to_rfc3339()),
                    config.updated_at.to_rfc3339(),
                ),
            )?;
            transaction.execute(
                "INSERT INTO window_state(window_label, x, y, width, height, monitor_id, scale_factor, maximized, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8)
                 ON CONFLICT(window_label) DO UPDATE SET x = excluded.x, y = excluded.y,
                    width = excluded.width, height = excluded.height, monitor_id = excluded.monitor_id,
                    scale_factor = excluded.scale_factor, maximized = 0, updated_at = excluded.updated_at",
                (
                    WIDGET_WINDOW_LABEL,
                    input.x,
                    input.y,
                    input.width,
                    input.height,
                    input.monitor_id.as_deref(),
                    input.scale_factor,
                    config.updated_at.to_rfc3339(),
                ),
            )?;
            Ok(())
        })
    }
}

fn parse_stored(stored: StoredWidgetConfig) -> Result<WidgetConfig, DomainError> {
    let config = WidgetConfig {
        input: WidgetConfigInput {
            size: WidgetSize::parse(&stored.size)?,
            mode: WidgetMode::parse(&stored.mode)?,
            locked: stored.locked,
            opacity: stored.opacity,
            modules: serde_json::from_str::<Vec<WidgetModule>>(&stored.modules_json)
                .map_err(config_error)?,
            x: stored.x,
            y: stored.y,
            width: stored.width,
            height: stored.height,
            monitor_id: stored.monitor_id,
            scale_factor: stored.scale_factor,
        },
        last_visible_at: stored
            .last_visible_at
            .map(|value| parse_timestamp(&value))
            .transpose()?,
        updated_at: parse_timestamp(&stored.updated_at)?,
    };
    config.input.validate()?;
    Ok(config)
}

fn parse_timestamp(value: &str) -> Result<DateTime<Utc>, DomainError> {
    DateTime::parse_from_rfc3339(value)
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .map_err(config_error)
}

fn config_error(error: impl std::fmt::Display) -> DomainError {
    DomainError {
        code: "WIDGET_CONFIG_CORRUPTED".into(),
        message: error.to_string(),
        field: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn configuration_round_trips_across_layout_and_window_tables() {
        let database = Database::open_in_memory().unwrap();
        let repository = WidgetRepository::new(&database);
        let now = Utc.with_ymd_and_hms(2026, 7, 19, 12, 0, 0).unwrap();
        let input = WidgetConfigInput {
            size: WidgetSize::Expanded,
            width: 470.0,
            height: 680.0,
            x: -320.0,
            y: 72.0,
            monitor_id: Some("monitor-2".into()),
            scale_factor: 1.25,
            ..Default::default()
        };
        let mut config = WidgetConfig::new(input, now).unwrap();
        config.last_visible_at = Some(now);

        repository.save(&config).unwrap();

        assert_eq!(repository.get().unwrap(), Some(config));
    }
}

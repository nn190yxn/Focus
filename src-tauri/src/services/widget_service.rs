use chrono::{DateTime, Utc};

use crate::{
    domain::widget::{WidgetConfig, WidgetConfigInput},
    repositories::{database::Database, widget_repository::WidgetRepository},
    DomainError,
};

pub struct WidgetService<'a> {
    database: &'a Database,
}

impl<'a> WidgetService<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn get(&self) -> Result<WidgetConfig, DomainError> {
        self.get_at(Utc::now())
    }

    pub fn update(&self, input: WidgetConfigInput) -> Result<WidgetConfig, DomainError> {
        self.update_at(input, Utc::now())
    }

    pub fn mark_visible(&self) -> Result<WidgetConfig, DomainError> {
        self.mark_visible_at(Utc::now())
    }

    pub fn unlock(&self) -> Result<WidgetConfig, DomainError> {
        self.unlock_at(Utc::now())
    }

    pub(crate) fn get_at(&self, now: DateTime<Utc>) -> Result<WidgetConfig, DomainError> {
        let repository = WidgetRepository::new(self.database);
        if let Some(config) = repository.get()? {
            return Ok(config);
        }
        let config = WidgetConfig::new(WidgetConfigInput::default(), now)?;
        repository.save(&config)?;
        Ok(config)
    }

    pub(crate) fn update_at(
        &self,
        input: WidgetConfigInput,
        now: DateTime<Utc>,
    ) -> Result<WidgetConfig, DomainError> {
        input.validate()?;
        let repository = WidgetRepository::new(self.database);
        let last_visible_at = repository.get()?.and_then(|config| config.last_visible_at);
        let config = WidgetConfig {
            input,
            last_visible_at,
            updated_at: now,
        };
        repository.save(&config)?;
        Ok(config)
    }

    pub(crate) fn mark_visible_at(&self, now: DateTime<Utc>) -> Result<WidgetConfig, DomainError> {
        let mut config = self.get_at(now)?;
        config.last_visible_at = Some(now);
        config.updated_at = now;
        WidgetRepository::new(self.database).save(&config)?;
        Ok(config)
    }

    pub(crate) fn unlock_at(&self, now: DateTime<Utc>) -> Result<WidgetConfig, DomainError> {
        let mut input = self.get_at(now)?.input;
        input.locked = false;
        self.update_at(input, now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::widget::{WidgetMode, WidgetSize};
    use chrono::TimeZone;

    fn at(hour: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 7, 19, hour, 0, 0).unwrap()
    }

    #[test]
    fn get_creates_a_stable_default_and_visibility_survives_updates() {
        let database = Database::open_in_memory().unwrap();
        let service = WidgetService::new(&database);
        let initial = service.get_at(at(9)).unwrap();
        assert_eq!(initial.input.size, WidgetSize::Standard);
        assert_eq!(initial.input.mode, WidgetMode::Desktop);
        assert_eq!(initial.last_visible_at, None);

        let visible = service.mark_visible_at(at(10)).unwrap();
        assert_eq!(visible.last_visible_at, Some(at(10)));

        let mut input = visible.input;
        input.opacity = 0.72;
        let updated = service.update_at(input, at(11)).unwrap();
        assert_eq!(updated.input.opacity, 0.72);
        assert_eq!(updated.last_visible_at, Some(at(10)));
        assert_eq!(service.get_at(at(12)).unwrap(), updated);
    }

    #[test]
    fn unlock_clears_and_persists_the_locked_state() {
        let database = Database::open_in_memory().unwrap();
        let service = WidgetService::new(&database);
        let mut input = service.get_at(at(9)).unwrap().input;
        input.locked = true;
        service.update_at(input, at(10)).unwrap();

        let unlocked = service.unlock_at(at(11)).unwrap();

        assert!(!unlocked.input.locked);
        assert_eq!(unlocked.updated_at, at(11));
        assert!(!service.get_at(at(12)).unwrap().input.locked);
    }
}

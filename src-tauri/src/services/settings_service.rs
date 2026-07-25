use crate::{
    domain::settings::{GeneralPreferences, GeneralPreferencesPatch},
    repositories::{database::Database, preferences_repository::PreferencesRepository},
    DomainError,
};

pub struct SettingsService<'a> {
    database: &'a Database,
}

impl<'a> SettingsService<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn get(&self) -> Result<GeneralPreferences, DomainError> {
        PreferencesRepository::new(self.database).get_general()
    }

    pub fn update(
        &self,
        patch: GeneralPreferencesPatch,
    ) -> Result<GeneralPreferences, DomainError> {
        let repository = PreferencesRepository::new(self.database);
        repository.set_general(repository.get_general()?.apply(patch))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::settings::{AppearancePreference, ThemePreference};

    #[test]
    fn updates_selected_preferences_and_persists_the_result() {
        let database = Database::open_in_memory().unwrap();
        let service = SettingsService::new(&database);

        let updated = service
            .update(GeneralPreferencesPatch {
                appearance: Some(AppearancePreference::Dark),
                theme: Some(ThemePreference::Blush),
                background_running: Some(false),
                ..GeneralPreferencesPatch::default()
            })
            .unwrap();

        assert_eq!(updated.appearance, AppearancePreference::Dark);
        assert_eq!(updated.theme, ThemePreference::Blush);
        assert!(!updated.background_running);
        assert_eq!(service.get().unwrap(), updated);
    }
}

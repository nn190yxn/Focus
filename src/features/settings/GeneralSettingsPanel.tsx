import { useState } from "react";

import { Badge, Panel } from "../../components/ui";
import { useI18n } from "../../i18n/I18nContext";
import { themeNames } from "../../theme/theme";
import type { GeneralPreferences, GeneralPreferencesPatch } from "./types";

export function GeneralSettingsPanel({
  preferences,
  onSave,
}: {
  preferences: GeneralPreferences;
  onSave: (patch: GeneralPreferencesPatch) => Promise<GeneralPreferences>;
}) {
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const { t } = useI18n();

  async function save(patch: GeneralPreferencesPatch) {
    if (saving) return;
    setSaving(true);
    setError(null);
    try {
      await onSave(patch);
    } catch {
      setError(t("settings.saveError"));
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="settings-workspace">
      <Panel className="settings-panel">
        <div className="settings-panel__heading">
          <div><span className="eyebrow">{t("settings.general.eyebrow")}</span><h2>{t("settings.general.title")}</h2></div>
          <Badge tone={saving ? "warning" : "success"}>{t(saving ? "settings.saving" : "settings.synced")}</Badge>
        </div>
        <p className="settings-panel__description">{t("settings.general.description")}</p>
        <label className="settings-select-row">
          <span><strong>{t("settings.language.title")}</strong><small>{t("settings.language.description")}</small></span>
          <select aria-label={t("settings.language.label")} value={preferences.language} disabled={saving} onChange={(event) => void save({ language: event.target.value as GeneralPreferences["language"] })}>
            <option value="system">{t("settings.language.system")}</option>
            <option value="zhCn">{t("settings.language.zhCn")}</option>
            <option value="en">{t("settings.language.en")}</option>
          </select>
        </label>
        <label className="settings-select-row">
          <span><strong>{t("settings.appearance.title")}</strong><small>{t("settings.appearance.description")}</small></span>
          <select aria-label={t("settings.appearance.label")} value={preferences.appearance} disabled={saving} onChange={(event) => void save({ appearance: event.target.value as GeneralPreferences["appearance"] })}>
            <option value="system">{t("settings.appearance.system")}</option>
            <option value="light">{t("settings.appearance.light")}</option>
            <option value="dark">{t("settings.appearance.dark")}</option>
          </select>
        </label>
        <label className="settings-select-row">
          <span><strong>{t("settings.theme.title")}</strong><small>{t("settings.theme.description")}</small></span>
          <select aria-label={t("settings.theme.label")} value={preferences.theme} disabled={saving} onChange={(event) => void save({ theme: event.target.value as GeneralPreferences["theme"] })}>
            {themeNames.map((theme) => <option key={theme} value={theme}>{t(`theme.${theme}`)}</option>)}
          </select>
        </label>
        <div className="settings-row">
          <div><strong>{t("settings.background.title")}</strong><span>{t("settings.background.description")}</span></div>
          <label className="settings-switch">
            <input type="checkbox" aria-label={t("settings.background.label")} checked={preferences.backgroundRunning} disabled={saving} onChange={(event) => void save({ backgroundRunning: event.target.checked })} />
            <span aria-hidden="true" />
          </label>
        </div>
        {error ? <p className="settings-error" role="alert">{error}</p> : null}
      </Panel>
    </div>
  );
}

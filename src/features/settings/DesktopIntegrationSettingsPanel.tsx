import { useEffect, useState } from "react";

import { Badge, Button, Panel } from "../../components/ui";
import { useI18n } from "../../i18n/I18nContext";
import type { MessageKey } from "../../i18n/messages";
import { isTauriRuntime } from "../../lib/commandClient";
import { domainErrorMessage } from "../../lib/domainError";
import { desktopIntegrationClient, type DesktopIntegrationClient } from "./desktopIntegrationClient";
import type { DesktopIntegrationSettings, ShortcutBindings, ShortcutPreferences } from "./types";

const previewSettings: DesktopIntegrationSettings = {
  shortcuts: {
    enabled: false,
    bindings: {
      showMainWindow: "Ctrl+Alt+A",
      toggleFocus: "Ctrl+Alt+Space",
      createQuickTask: "Ctrl+Alt+N",
      unlockWidget: "Ctrl+Alt+U",
    },
  },
  autostartEnabled: false,
  shortcutError: null,
};

const shortcutFields: { key: keyof ShortcutBindings; labelKey: MessageKey; descriptionKey: MessageKey }[] = [
  { key: "showMainWindow", labelKey: "settings.desktop.showMain", descriptionKey: "settings.desktop.showMainDescription" },
  { key: "toggleFocus", labelKey: "settings.desktop.toggleFocus", descriptionKey: "settings.desktop.toggleFocusDescription" },
  { key: "createQuickTask", labelKey: "settings.desktop.quickTask", descriptionKey: "settings.desktop.quickTaskDescription" },
  { key: "unlockWidget", labelKey: "settings.desktop.unlockWidget", descriptionKey: "settings.desktop.unlockWidgetDescription" },
];

export function DesktopIntegrationSettingsPanel({ client = desktopIntegrationClient }: { client?: DesktopIntegrationClient }) {
  const { t } = useI18n();
  const desktopRuntime = isTauriRuntime();
  const [settings, setSettings] = useState<DesktopIntegrationSettings | null>(desktopRuntime ? null : previewSettings);
  const [bindings, setBindings] = useState<ShortcutBindings>(previewSettings.shortcuts.bindings);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!desktopRuntime) return;
    let active = true;
    void client.getSettings().then((result) => {
      if (!active) return;
      if (result.ok) {
        setSettings(result.data);
        setBindings(result.data.shortcuts.bindings);
        setError(result.data.shortcutError);
      } else {
        setError(domainErrorMessage(result.error, t));
      }
    });
    return () => { active = false; };
  }, [client, desktopRuntime]);

  async function saveShortcuts(shortcuts: ShortcutPreferences) {
    if (!settings || saving) return;
    if (!desktopRuntime) {
      setSettings({ ...settings, shortcuts, shortcutError: null });
      setBindings(shortcuts.bindings);
      setError(null);
      return;
    }
    setSaving(true);
    setError(null);
    const result = await client.updateShortcuts(shortcuts);
    if (result.ok) {
      setSettings({ ...settings, shortcuts: result.data, shortcutError: null });
      setBindings(result.data.bindings);
    } else {
      setBindings(settings.shortcuts.bindings);
      setError(domainErrorMessage(result.error, t));
    }
    setSaving(false);
  }

  async function setAutostart(enabled: boolean) {
    if (!settings || saving) return;
    if (!desktopRuntime) {
      setSettings({ ...settings, autostartEnabled: enabled });
      return;
    }
    setSaving(true);
    setError(null);
    const result = await client.setAutostart(enabled);
    if (result.ok) setSettings({ ...settings, autostartEnabled: result.data });
    else setError(domainErrorMessage(result.error, t));
    setSaving(false);
  }

  if (!settings) return <Panel className="settings-panel"><p>{t("settings.desktop.loading")}</p></Panel>;

  return (
    <div className="settings-workspace">
      <Panel className="settings-panel">
        <div className="settings-panel__heading">
          <div><span className="eyebrow">{t("settings.desktop.loginEyebrow")}</span><h2>{t("settings.desktop.loginTitle")}</h2></div>
          <Badge tone={settings.autostartEnabled ? "success" : "neutral"}>{t(settings.autostartEnabled ? "common.enabled" : "settings.desktop.manual")}</Badge>
        </div>
        <p className="settings-panel__description">{t("settings.desktop.loginDescription")}</p>
        <div className="settings-row">
          <div><strong>{t("settings.desktop.autostart")}</strong><span>{t("settings.desktop.autostartDescription")}</span></div>
          <label className="settings-switch">
            <input type="checkbox" aria-label={t("settings.desktop.autostartLabel")} checked={settings.autostartEnabled} disabled={saving} onChange={(event) => void setAutostart(event.target.checked)} />
            <span aria-hidden="true" />
          </label>
        </div>
      </Panel>

      <Panel className="settings-panel">
        <div className="settings-panel__heading">
          <div><span className="eyebrow">{t("settings.desktop.shortcutsEyebrow")}</span><h2>{t("settings.desktop.shortcutsTitle")}</h2></div>
          <Badge tone={settings.shortcuts.enabled ? "success" : "neutral"}>{t(settings.shortcuts.enabled ? "settings.desktop.running" : "common.disabled")}</Badge>
        </div>
        <p className="settings-panel__description">{t("settings.desktop.shortcutsDescription")}</p>
        <div className="settings-row">
          <div><strong>{t("settings.desktop.shortcutsEnabled")}</strong><span>{t("settings.desktop.shortcutsEnabledDescription")}</span></div>
          <label className="settings-switch">
            <input type="checkbox" aria-label={t("settings.desktop.shortcutsEnabledLabel")} checked={settings.shortcuts.enabled} disabled={saving} onChange={(event) => void saveShortcuts({ enabled: event.target.checked, bindings })} />
            <span aria-hidden="true" />
          </label>
        </div>
        <div className="shortcut-grid">
          {shortcutFields.map((field) => (
            <label className="shortcut-field" key={field.key}>
              <span><strong>{t(field.labelKey)}</strong><small>{t(field.descriptionKey)}</small></span>
              <input aria-label={t("settings.desktop.shortcutLabel", { action: t(field.labelKey) })} value={bindings[field.key]} disabled={saving} spellCheck={false} onChange={(event) => setBindings({ ...bindings, [field.key]: event.target.value })} />
            </label>
          ))}
        </div>
        <div className="settings-actions">
          <Button tone="primary" disabled={saving} onClick={() => void saveShortcuts({ enabled: settings.shortcuts.enabled, bindings })}>{t("settings.desktop.saveShortcuts")}</Button>
          <span>{t("settings.desktop.example")}</span>
        </div>
        {error ? <p className="settings-error" role="alert">{error}</p> : null}
      </Panel>
    </div>
  );
}

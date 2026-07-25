import { useEffect, useState } from "react";

import { Badge, Button, Panel } from "../../components/ui";
import { useI18n, type I18nValue } from "../../i18n/I18nContext";
import { isTauriRuntime } from "../../lib/commandClient";
import { domainErrorMessage } from "../../lib/domainError";
import { notificationClient, type NotificationClient } from "./notificationClient";
import type { NotificationPreferences, NotificationSettings } from "./types";

const previewSettings: NotificationSettings = {
  preferences: { notificationsEnabled: true, soundEnabled: true },
  permissionState: "granted",
};

export function NotificationSettingsPanel({ client = notificationClient }: { client?: NotificationClient }) {
  const { t } = useI18n();
  const desktopRuntime = isTauriRuntime();
  const [settings, setSettings] = useState<NotificationSettings | null>(desktopRuntime ? null : previewSettings);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!desktopRuntime) return;
    let active = true;
    void client.getSettings().then((result) => {
      if (!active) return;
      if (result.ok) setSettings(result.data);
      else setError(domainErrorMessage(result.error, t));
    });
    return () => { active = false; };
  }, [client, desktopRuntime]);

  async function update(preferences: NotificationPreferences) {
    if (!settings || saving) return;
    if (!desktopRuntime) {
      setSettings({ ...settings, preferences });
      return;
    }
    setSaving(true);
    setError(null);
    const result = await client.updatePreferences(preferences);
    if (result.ok) setSettings({ ...settings, preferences: result.data });
    else setError(domainErrorMessage(result.error, t));
    setSaving(false);
  }

  async function openSystemSettings() {
    if (!desktopRuntime) return;
    const result = await client.openSystemSettings();
    if (!result.ok) setError(domainErrorMessage(result.error, t));
  }

  if (!settings) return <Panel className="settings-panel"><p>{t("settings.notification.loading")}</p></Panel>;

  const permission = permissionPresentation(settings.permissionState, t);
  return (
    <div className="settings-workspace">
      <Panel className="settings-panel">
        <div className="settings-panel__heading">
          <div><span className="eyebrow">{t("settings.notification.eyebrow")}</span><h2>{t("settings.notification.title")}</h2></div>
          <Badge tone={permission.tone}>{permission.label}</Badge>
        </div>
        <p className="settings-panel__description">{t("settings.notification.description")}</p>
        <div className="settings-row">
          <div><strong>{t("settings.notification.system")}</strong><span>{t("settings.notification.systemDescription")}</span></div>
          <label className="settings-switch">
            <input type="checkbox" aria-label={t("settings.notification.systemLabel")} checked={settings.preferences.notificationsEnabled} disabled={saving} onChange={(event) => void update({ ...settings.preferences, notificationsEnabled: event.target.checked })} />
            <span aria-hidden="true" />
          </label>
        </div>
        <div className="settings-row">
          <div><strong>{t("settings.notification.sound")}</strong><span>{t("settings.notification.soundDescription")}</span></div>
          <label className="settings-switch">
            <input type="checkbox" aria-label={t("settings.notification.soundLabel")} checked={settings.preferences.soundEnabled} disabled={saving || !settings.preferences.notificationsEnabled} onChange={(event) => void update({ ...settings.preferences, soundEnabled: event.target.checked })} />
            <span aria-hidden="true" />
          </label>
        </div>
        <div className="settings-permission">
          <div><strong>{t("settings.notification.permission")}</strong><span>{permission.description}</span></div>
          <Button tone="secondary" onClick={() => void openSystemSettings()} disabled={!desktopRuntime}>{t("settings.notification.open")}</Button>
        </div>
        {error ? <p className="settings-error" role="alert">{error}</p> : null}
      </Panel>
    </div>
  );
}

function permissionPresentation(state: NotificationSettings["permissionState"], t: I18nValue["t"]): {
  label: string;
  description: string;
  tone: "success" | "warning" | "neutral";
} {
  if (state === "granted") return { label: t("settings.notification.granted"), description: t("settings.notification.grantedDescription"), tone: "success" };
  if (state === "denied") return { label: t("settings.notification.denied"), description: t("settings.notification.deniedDescription"), tone: "warning" };
  return { label: t("settings.notification.unknown"), description: t("settings.notification.unknownDescription"), tone: "neutral" };
}

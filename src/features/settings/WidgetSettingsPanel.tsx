import { useEffect, useState } from "react";

import { Badge, Button, Panel } from "../../components/ui";
import { useI18n } from "../../i18n/I18nContext";
import type { MessageKey } from "../../i18n/messages";
import { isTauriRuntime } from "../../lib/commandClient";
import { domainErrorMessage } from "../../lib/domainError";
import { defaultWidgetConfig, type WidgetConfig, type WidgetModule, type WidgetSize } from "../widget/types";
import { widgetClient } from "../widget/widgetClient";

const dimensions: Record<WidgetSize, { width: number; height: number }> = {
  compact: { width: 320, height: 132 },
  standard: { width: 360, height: 420 },
  expanded: { width: 440, height: 640 },
};

const modules: WidgetModule[] = ["clock", "currentFocus", "todayProgress", "tasks", "quickActions", "projectProgress", "weeklyGoals", "noteEntry"];

export function WidgetSettingsPanel() {
  const { t } = useI18n();
  const desktopRuntime = isTauriRuntime();
  const [config, setConfig] = useState<WidgetConfig>(defaultWidgetConfig);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!desktopRuntime) return;
    let active = true;
    void widgetClient.getConfig().then((result) => {
      if (!active) return;
      if (result.ok) setConfig(result.data);
      else setError(domainErrorMessage(result.error, t));
    });
    return () => { active = false; };
  }, [desktopRuntime]);

  async function update(patch: Partial<WidgetConfig>) {
    if (saving) return;
    const candidate = { ...config, ...patch };
    if (candidate.modules.length === 0) {
      setError(t("settings.widget.moduleError"));
      return;
    }
    if (!desktopRuntime) {
      setConfig(candidate);
      return;
    }
    setSaving(true);
    setError(null);
    const result = await widgetClient.updateConfig(candidate);
    if (result.ok) setConfig(result.data);
    else setError(domainErrorMessage(result.error, t));
    setSaving(false);
  }

  async function showWidget() {
    if (!desktopRuntime) return;
    const result = await widgetClient.show();
    if (result.ok) setConfig(result.data);
    else setError(domainErrorMessage(result.error, t));
  }

  function toggleModule(module: WidgetModule, enabled: boolean) {
    const next = enabled
      ? [...config.modules, module]
      : config.modules.filter((candidate) => candidate !== module);
    void update({ modules: next });
  }

  return (
    <div className="settings-workspace">
      <Panel className="settings-panel">
        <div className="settings-panel__heading">
          <div><span className="eyebrow">{t("settings.widget.eyebrow")}</span><h2>{t("settings.widget.title")}</h2></div>
          <Badge tone={config.locked ? "neutral" : "success"}>{t(config.locked ? "settings.widget.locked" : "settings.widget.editable")}</Badge>
        </div>
        <p className="settings-panel__description">{t("settings.widget.description")}</p>
        <label className="settings-select-row">
          <span><strong>{t("settings.widget.mode")}</strong><small>{t("settings.widget.modeDescription")}</small></span>
          <select aria-label={t("settings.widget.modeLabel")} value={config.mode} disabled={saving} onChange={(event) => void update({ mode: event.target.value as WidgetConfig["mode"] })}>
            <option value="desktop">{t("settings.widget.desktop")}</option>
            <option value="floating">{t("settings.widget.floating")}</option>
          </select>
        </label>
        <label className="settings-select-row">
          <span><strong>{t("settings.widget.size")}</strong><small>{t("settings.widget.sizeDescription")}</small></span>
          <select aria-label={t("settings.widget.sizeLabel")} value={config.size} disabled={saving} onChange={(event) => {
            const size = event.target.value as WidgetSize;
            void update({ size, ...dimensions[size] });
          }}>
            <option value="compact">{t("settings.widget.compact")}</option>
            <option value="standard">{t("settings.widget.standard")}</option>
            <option value="expanded">{t("settings.widget.expanded")}</option>
          </select>
        </label>
        <label className="settings-range-row">
          <span><strong>{t("settings.widget.opacity")}</strong><small>{Math.round(config.opacity * 100)}%</small></span>
          <input aria-label={t("settings.widget.opacityLabel")} type="range" min="20" max="100" step="5" value={Math.round(config.opacity * 100)} disabled={saving} onChange={(event) => void update({ opacity: Number(event.target.value) / 100 })} />
        </label>
        <div className="settings-row">
          <div><strong>{t("settings.widget.lock")}</strong><span>{t("settings.widget.lockDescription")}</span></div>
          <label className="settings-switch">
            <input type="checkbox" aria-label={t("settings.widget.lockLabel")} checked={config.locked} disabled={saving} onChange={(event) => void update({ locked: event.target.checked })} />
            <span aria-hidden="true" />
          </label>
        </div>
        <fieldset className="settings-modules">
          <legend>{t("settings.widget.modules")}</legend>
          {modules.map((module) => (
            <label key={module}>
              <input type="checkbox" checked={config.modules.includes(module)} disabled={saving} onChange={(event) => toggleModule(module, event.target.checked)} />
              <span>{t(`settings.widget.module.${module}` as MessageKey)}</span>
            </label>
          ))}
        </fieldset>
        <div className="settings-actions">
          <Button tone="secondary" disabled={!desktopRuntime || saving} onClick={() => void showWidget()}>{t("settings.widget.show")}</Button>
          <span>{t(config.mode === "desktop" ? "settings.widget.desktopHint" : "settings.widget.floatingHint")}</span>
        </div>
        {error ? <p className="settings-error" role="alert">{error}</p> : null}
      </Panel>
    </div>
  );
}

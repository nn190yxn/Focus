import { Badge, Panel } from "../../components/ui";
import { useI18n } from "../../i18n/I18nContext";

const releaseUrl = "https://github.com/nn190yxn/Focus/releases/latest";

export function UpdateSettingsPanel() {
  const { t } = useI18n();

  return (
    <div className="settings-workspace">
      <Panel className="settings-panel">
        <div className="settings-panel__heading">
          <div><span className="eyebrow">{t("settings.update.eyebrow")}</span><h2>{t("settings.update.title")}</h2></div>
          <Badge tone="warning">{t("settings.update.manualBadge")}</Badge>
        </div>
        <p className="settings-panel__description">{t("settings.update.description")}</p>
        <div className="settings-data-card">
          <strong>{t("settings.update.currentVersion")}</strong>
          <span>{t("settings.update.manualStatus")}</span>
          <p>{t("settings.update.unsignedNotice")}</p>
          <div>
            <a className="button button--primary" href={releaseUrl} target="_blank" rel="noreferrer">{t("settings.update.openRelease")}</a>
          </div>
        </div>
      </Panel>
    </div>
  );
}

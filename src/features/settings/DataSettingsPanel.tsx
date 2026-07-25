import { useState } from "react";

import { Badge, Button, Dialog, Panel, Toast } from "../../components/ui";
import { useI18n } from "../../i18n/I18nContext";
import { isTauriRuntime } from "../../lib/commandClient";
import { domainErrorMessage } from "../../lib/domainError";
import { backupClient, type BackupClient, type BackupInspection } from "./backupClient";

export function DataSettingsPanel({ client = backupClient }: { client?: BackupClient }) {
  const { t, formatDate } = useI18n();
  const desktopRuntime = isTauriRuntime();
  const [inspection, setInspection] = useState<BackupInspection | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  async function exportBackup() {
    if (!desktopRuntime || busy) return;
    setBusy(true);
    setError(null);
    setSuccess(null);
    try {
      const result = await client.exportBackup();
      if (result.ok && result.data) setSuccess(t("settings.data.exportSuccess"));
      if (!result.ok) setError(domainErrorMessage(result.error, t));
    } catch {
      setError(t("settings.data.operationFailed"));
    } finally {
      setBusy(false);
    }
  }

  async function inspectBackup() {
    if (!desktopRuntime || busy) return;
    setBusy(true);
    setError(null);
    setSuccess(null);
    try {
      const result = await client.inspectBackup();
      if (result.ok && result.data) setInspection(result.data);
      if (!result.ok) setError(domainErrorMessage(result.error, t));
    } catch {
      setError(t("settings.data.operationFailed"));
    } finally {
      setBusy(false);
    }
  }

  async function restoreBackup() {
    if (!inspection || busy) return;
    setBusy(true);
    setError(null);
    try {
      const result = await client.restoreBackup(inspection.token);
      if (result.ok) {
        setInspection(null);
        setSuccess(t("settings.data.restoreSuccess"));
      } else {
        setError(domainErrorMessage(result.error, t));
      }
    } catch {
      setError(t("settings.data.operationFailed"));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="settings-workspace">
      <Panel className="settings-panel">
        <div className="settings-panel__heading">
          <div><span className="eyebrow">{t("settings.data.eyebrow")}</span><h2>{t("settings.data.title")}</h2></div>
          <Badge tone="success">{t("settings.data.localOnly")}</Badge>
        </div>
        <p className="settings-panel__description">{t("settings.data.description")}</p>
        <div className="settings-data-card">
          <strong>{t("settings.data.backupTitle")}</strong>
          <span>{t("settings.data.backupDescription")}</span>
          <div>
            <Button tone="secondary" disabled={!desktopRuntime || busy} onClick={() => void exportBackup()}>{t("settings.data.export")}</Button>
            <Button tone="ghost" disabled={!desktopRuntime || busy} onClick={() => void inspectBackup()}>{t("settings.data.restore")}</Button>
          </div>
        </div>
        {error ? <Toast tone="danger">{error}</Toast> : null}
        {success ? <Toast tone="success">{success}</Toast> : null}
      </Panel>
      <Dialog open={inspection !== null} title={t("settings.data.confirmTitle")} onClose={() => !busy && setInspection(null)}>
        {inspection ? (
          <div className="backup-confirmation">
            <p>{t("settings.data.confirmDescription")}</p>
            <dl>
              <div><dt>{t("settings.data.version")}</dt><dd>{inspection.formatVersion}</dd></div>
              <div><dt>{t("settings.data.exportedAt")}</dt><dd>{formatDate(inspection.exportedAt, { dateStyle: "medium", timeStyle: "short" })}</dd></div>
              <div><dt>{t("settings.data.tasks")}</dt><dd>{inspection.summary.counts.tasks + inspection.summary.counts.taskInstances}</dd></div>
              <div><dt>{t("settings.data.focusSessions")}</dt><dd>{inspection.summary.counts.focusSessions}</dd></div>
              <div><dt>{t("settings.data.totalRecords")}</dt><dd>{inspection.summary.counts.total}</dd></div>
              <div><dt>{t("settings.data.dateRange")}</dt><dd>{inspection.summary.earliestDate && inspection.summary.latestDate ? `${inspection.summary.earliestDate} – ${inspection.summary.latestDate}` : t("settings.data.noDateRange")}</dd></div>
            </dl>
            <p className="backup-confirmation__warning">{t("settings.data.replaceWarning")}</p>
            <div className="backup-confirmation__actions">
              <Button tone="ghost" disabled={busy} onClick={() => setInspection(null)}>{t("common.cancel")}</Button>
              <Button tone="danger" disabled={busy} onClick={() => void restoreBackup()}>{t("settings.data.confirmRestore")}</Button>
            </div>
          </div>
        ) : null}
      </Dialog>
    </div>
  );
}

import { useEffect, useRef, useState } from "react";

import { Badge, Button, Dialog, Panel, Progress, Toast } from "../../components/ui";
import { useI18n } from "../../i18n/I18nContext";
import { isTauriRuntime } from "../../lib/commandClient";
import {
  updateClient,
  type UpdateClient,
  type UpdateDownloadProgress,
  type UpdateMetadata,
} from "./updateClient";

type UpdatePhase = "idle" | "checking" | "upToDate" | "available" | "downloading" | "ready" | "installing" | "error";

export function UpdateSettingsPanel({ client = updateClient }: { client?: UpdateClient }) {
  const { t, formatDate } = useI18n();
  const desktopRuntime = isTauriRuntime();
  const checkStarted = useRef(false);
  const [phase, setPhase] = useState<UpdatePhase>("idle");
  const [available, setAvailable] = useState<UpdateMetadata | null>(null);
  const [progress, setProgress] = useState<UpdateDownloadProgress | null>(null);
  const [confirmingInstall, setConfirmingInstall] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const busy = phase === "checking" || phase === "downloading" || phase === "installing";

  useEffect(() => {
    if (!desktopRuntime || checkStarted.current) return;
    checkStarted.current = true;
    void checkForUpdate();
  }, [desktopRuntime]);

  async function checkForUpdate() {
    if (!desktopRuntime || busy || phase === "ready") return;
    setPhase("checking");
    setError(null);
    setProgress(null);
    try {
      const response = await client.check();
      if (!response.ok) {
        setPhase("error");
        setError(t("settings.update.checkFailed"));
        return;
      }
      setAvailable(response.data);
      setPhase(response.data ? "available" : "upToDate");
    } catch {
      setPhase("error");
      setError(t("settings.update.checkFailed"));
    }
  }

  async function downloadUpdate() {
    if (!available || busy) return;
    setPhase("downloading");
    setError(null);
    setProgress({ downloaded: 0, contentLength: null });
    try {
      const response = await client.download(setProgress);
      if (!response.ok) {
        setPhase("available");
        setError(t("settings.update.downloadFailed"));
        return;
      }
      setProgress(response.data);
      setPhase("ready");
      setConfirmingInstall(true);
    } catch {
      setPhase("available");
      setError(t("settings.update.downloadFailed"));
    }
  }

  async function installUpdate() {
    if (phase !== "ready") return;
    setPhase("installing");
    setError(null);
    try {
      const response = await client.install();
      if (!response.ok) {
        setPhase("ready");
        setError(t("settings.update.installFailed"));
      }
    } catch {
      setPhase("ready");
      setError(t("settings.update.installFailed"));
    }
  }

  const percentage = progress?.contentLength
    ? Math.round((progress.downloaded / progress.contentLength) * 100)
    : 0;

  return (
    <div className="settings-workspace">
      <Panel className="settings-panel">
        <div className="settings-panel__heading">
          <div><span className="eyebrow">{t("settings.update.eyebrow")}</span><h2>{t("settings.update.title")}</h2></div>
          <Badge tone={phase === "available" || phase === "ready" ? "accent" : "success"}>
            {phase === "available" || phase === "ready" ? t("settings.update.available") : t("settings.update.signed")}
          </Badge>
        </div>
        <p className="settings-panel__description">{t("settings.update.description")}</p>
        <div className="settings-data-card">
          <strong>{available ? t("settings.update.versionAvailable", { version: available.version }) : t("settings.update.currentVersion")}</strong>
          <span role="status" aria-live="polite">
            {!desktopRuntime ? t("settings.update.desktopOnly") : null}
            {desktopRuntime && phase === "idle" ? t("settings.update.readyToCheck") : null}
            {phase === "checking" ? t("settings.update.checking") : null}
            {phase === "upToDate" ? t("settings.update.upToDate") : null}
            {phase === "available" ? t("settings.update.downloadPrompt") : null}
            {phase === "downloading" ? t("settings.update.downloading", { percentage }) : null}
            {phase === "ready" ? t("settings.update.readyToInstall") : null}
            {phase === "installing" ? t("settings.update.installing") : null}
          </span>
          {available?.publishedAt ? <span>{formatDate(new Date(available.publishedAt * 1000), { dateStyle: "medium" })}</span> : null}
          {available?.notes ? <p>{available.notes}</p> : null}
          {phase === "downloading" ? <Progress label={t("settings.update.downloadProgress")} value={percentage} /> : null}
          <div>
            <Button tone="secondary" disabled={!desktopRuntime || busy || phase === "ready"} onClick={() => void checkForUpdate()}>{t("settings.update.check")}</Button>
            {phase === "available" ? <Button tone="primary" onClick={() => void downloadUpdate()}>{t("settings.update.download")}</Button> : null}
            {phase === "ready" ? <Button tone="primary" onClick={() => setConfirmingInstall(true)}>{t("settings.update.install")}</Button> : null}
          </div>
        </div>
        {error ? <Toast tone="danger">{error}</Toast> : null}
      </Panel>
      <Dialog open={confirmingInstall} title={t("settings.update.confirmTitle")} onClose={() => phase !== "installing" && setConfirmingInstall(false)}>
        <p>{t("settings.update.confirmDescription")}</p>
        <div className="backup-confirmation__actions">
          <Button tone="ghost" disabled={phase === "installing"} onClick={() => setConfirmingInstall(false)}>{t("common.cancel")}</Button>
          <Button autoFocus tone="primary" disabled={phase === "installing"} onClick={() => void installUpdate()}>{t("settings.update.confirmInstall")}</Button>
        </div>
      </Dialog>
    </div>
  );
}

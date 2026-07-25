import { useEffect, useState } from "react";

import { Button } from "../../components/ui";
import { useI18n } from "../../i18n/I18nContext";
import { isTauriRuntime } from "../../lib/commandClient";
import { notificationClient, type NotificationClient } from "../settings/notificationClient";

export function MemoReminderPermissionNotice({
  active,
  runtime = isTauriRuntime(),
  client = notificationClient,
}: {
  active: boolean;
  runtime?: boolean;
  client?: NotificationClient;
}) {
  const { t } = useI18n();
  const [denied, setDenied] = useState(false);

  useEffect(() => {
    if (!active || !runtime) return;
    let mounted = true;
    void client.getSettings().then((result) => {
      if (mounted && result.ok) setDenied(result.data.permissionState === "denied");
    });
    return () => { mounted = false; };
  }, [active, client, runtime]);

  if (!active || !denied) return null;
  return (
    <div className="memo-reminder-permission" role="status">
      <span>{t("memo.reminderPermissionDenied")}</span>
      <Button tone="secondary" onClick={() => void client.openSystemSettings()}>{t("settings.notification.open")}</Button>
    </div>
  );
}

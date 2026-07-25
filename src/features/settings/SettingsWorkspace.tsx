import { DesktopIntegrationSettingsPanel } from "./DesktopIntegrationSettingsPanel";
import { DataSettingsPanel } from "./DataSettingsPanel";
import { GeneralSettingsPanel } from "./GeneralSettingsPanel";
import { NotificationSettingsPanel } from "./NotificationSettingsPanel";
import type { GeneralPreferences, GeneralPreferencesPatch } from "./types";
import { UpdateSettingsPanel } from "./UpdateSettingsPanel";
import { WidgetSettingsPanel } from "./WidgetSettingsPanel";

export function SettingsWorkspace({
  general,
  onSaveGeneral,
}: {
  general: GeneralPreferences;
  onSaveGeneral: (patch: GeneralPreferencesPatch) => Promise<GeneralPreferences>;
}) {
  return (
    <div className="settings-page">
      <GeneralSettingsPanel preferences={general} onSave={onSaveGeneral} />
      <NotificationSettingsPanel />
      <DesktopIntegrationSettingsPanel />
      <WidgetSettingsPanel />
      <DataSettingsPanel />
      <UpdateSettingsPanel />
    </div>
  );
}

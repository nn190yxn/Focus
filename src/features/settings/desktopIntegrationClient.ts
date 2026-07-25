import { invokeCommand, type CommandResult } from "../../lib/commandClient";
import type { DesktopIntegrationSettings, ShortcutPreferences } from "./types";

export interface DesktopIntegrationClient {
  getSettings(): Promise<CommandResult<DesktopIntegrationSettings>>;
  updateShortcuts(shortcuts: ShortcutPreferences): Promise<CommandResult<ShortcutPreferences>>;
  setAutostart(enabled: boolean): Promise<CommandResult<boolean>>;
}

export const desktopIntegrationClient: DesktopIntegrationClient = {
  getSettings: () => invokeCommand("desktop_integration_get_settings"),
  updateShortcuts: (shortcuts) => invokeCommand("desktop_integration_update_shortcuts", { shortcuts }),
  setAutostart: (enabled) => invokeCommand("desktop_integration_set_autostart", { enabled }),
};

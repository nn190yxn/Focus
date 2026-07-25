import { invokeCommand, type CommandResult } from "../../lib/commandClient";
import type { NotificationPreferences, NotificationSettings } from "./types";

export interface NotificationClient {
  getSettings(): Promise<CommandResult<NotificationSettings>>;
  updatePreferences(preferences: NotificationPreferences): Promise<CommandResult<NotificationPreferences>>;
  openSystemSettings(): Promise<CommandResult<null>>;
}

export const notificationClient: NotificationClient = {
  getSettings: () => invokeCommand("notification_get_settings"),
  updatePreferences: (preferences) => invokeCommand("notification_update_preferences", { preferences }),
  openSystemSettings: () => invokeCommand("notification_open_settings"),
};

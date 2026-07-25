import type { ThemeName } from "../../theme/theme";

export type LanguagePreference = "system" | "zhCn" | "en";
export type AppearancePreference = "system" | "light" | "dark";

export interface GeneralPreferences {
  language: LanguagePreference;
  appearance: AppearancePreference;
  theme: ThemeName;
  backgroundRunning: boolean;
}

export type GeneralPreferencesPatch = Partial<GeneralPreferences>;

export const defaultGeneralPreferences: GeneralPreferences = {
  language: "system",
  appearance: "system",
  theme: "mint",
  backgroundRunning: true,
};

export type NotificationPermissionState = "granted" | "denied" | "unknown";

export interface NotificationPreferences {
  notificationsEnabled: boolean;
  soundEnabled: boolean;
}

export interface NotificationSettings {
  preferences: NotificationPreferences;
  permissionState: NotificationPermissionState;
}

export interface ShortcutBindings {
  showMainWindow: string;
  toggleFocus: string;
  createQuickTask: string;
  unlockWidget: string;
}

export interface ShortcutPreferences {
  enabled: boolean;
  bindings: ShortcutBindings;
}

export interface DesktopIntegrationSettings {
  shortcuts: ShortcutPreferences;
  autostartEnabled: boolean;
  shortcutError: string | null;
}

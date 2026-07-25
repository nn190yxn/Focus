import { invokeCommand, type CommandResult } from "../../lib/commandClient";
import type { GeneralPreferences, GeneralPreferencesPatch } from "./types";

export type SettingsClient = {
  get: () => Promise<CommandResult<GeneralPreferences>>;
  update: (patch: GeneralPreferencesPatch) => Promise<CommandResult<GeneralPreferences>>;
};

export const settingsClient: SettingsClient = {
  get: () => invokeCommand("settings_get"),
  update: (patch) => invokeCommand("settings_update", { patch }),
};

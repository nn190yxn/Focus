import { invokeCommand, type CommandResult } from "../../lib/commandClient";
import type { WidgetConfig, WidgetConfigInput } from "./types";

export const widgetClient = {
  getConfig: (): Promise<CommandResult<WidgetConfig>> => invokeCommand("widget_get_config"),
  updateConfig: (input: WidgetConfigInput): Promise<CommandResult<WidgetConfig>> => invokeCommand("widget_update_config", { input }),
  show: (): Promise<CommandResult<WidgetConfig>> => invokeCommand("widget_show"),
  unlock: (): Promise<CommandResult<WidgetConfig>> => invokeCommand("widget_unlock"),
};

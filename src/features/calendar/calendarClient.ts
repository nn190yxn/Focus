import { invokeCommand, type CommandResult } from "../../lib/commandClient";
import type { CalendarPeriodResult, CalendarQuery } from "./types";

export interface CalendarCommandClient {
  getPeriod(query: CalendarQuery): Promise<CommandResult<CalendarPeriodResult>>;
}

export const calendarClient: CalendarCommandClient = {
  getPeriod: (query) => invokeCommand<CalendarPeriodResult>("calendar_get_period", { query }),
};

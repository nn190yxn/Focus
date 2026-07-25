import { invokeCommand, type CommandResult } from "../../lib/commandClient";
import type { CalendarQuery } from "./types";
import type { StatisticsSummary } from "./statisticsTypes";

export interface StatisticsCommandClient {
  getSummary(query: CalendarQuery): Promise<CommandResult<StatisticsSummary>>;
}

export const statisticsClient: StatisticsCommandClient = {
  getSummary: (query) => invokeCommand<StatisticsSummary>("statistics_get_summary", { query }),
};

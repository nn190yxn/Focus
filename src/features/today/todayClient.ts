import { invokeCommand } from "../../lib/commandClient";
import type { TodayDigest } from "./types";

export const todayClient = {
  getDigest(date: string) {
    return invokeCommand<TodayDigest>("today_get_digest", { date });
  },
};

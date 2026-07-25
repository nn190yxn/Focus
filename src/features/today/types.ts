import type { TaskCategory } from "../tasks/types";

export type TodaySourceKind = "task" | "recurringInstance";
export type TodayItemKind = "ordinaryTask" | "projectTask" | "recurringInstance";

export interface TodayProjectSummary {
  id: string;
  name: string;
  color: string;
  icon: string;
  status: "active" | "paused" | "completed" | "archived";
}

export interface TodayDigestItem {
  sourceKind: TodaySourceKind;
  sourceId: string;
  itemKind: TodayItemKind;
  recurrenceRuleId: string | null;
  title: string;
  category: TaskCategory;
  priority: number;
  scheduledDate: string;
  scheduledTime: string | null;
  status: "pending" | "completed";
  completedAt: string | null;
  project: TodayProjectSummary | null;
  isOverdue: boolean;
  createdAt: string;
}

export interface TodayDigest {
  date: string;
  items: TodayDigestItem[];
}

export type RecurrencePattern =
  | { kind: "daily"; interval: number }
  | { kind: "weekdays" }
  | { kind: "weekly"; interval: number; weekdays: number[] }
  | { kind: "monthly"; interval: number; dayOfMonth: number };

export type RecurrenceStatus = "active" | "paused" | "ended";

export interface RecurrenceRule {
  id: string;
  taskTemplateId: string;
  pattern: RecurrencePattern;
  localTime: string | null;
  timezone: string;
  startsOn: string;
  endsOn: string | null;
  status: RecurrenceStatus;
  version: number;
}

export type RecurrenceRuleInput = Pick<RecurrenceRule, "pattern" | "localTime" | "timezone" | "startsOn" | "endsOn">;

export type RecurrenceChangeScope =
  | { scope: "thisInstance"; instanceId: string }
  | { scope: "future"; effectiveOn: string };

export interface GenerationSummary {
  ruleId: string;
  scheduledCount: number;
  affectedCount: number;
}

export interface TaskInstanceRecord {
  id: string;
  recurrenceRuleId: string;
  ruleVersion: number;
  scheduledDate: string;
  scheduledAt: string | null;
  snapshotTitle: string;
  snapshotProjectId: string | null;
  status: "pending" | "completed" | "skipped" | "rescheduled";
  completedAt: string | null;
  sourceInstanceId: string | null;
  createdAt: string;
  updatedAt: string;
}

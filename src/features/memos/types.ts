export type MemoReminderFrequency = "daily" | "weekdays" | "weekly" | "monthly";

export type MemoReminderStatus = "active" | "completed" | "cancelled";

export type MemoReminderSchedule =
  | {
      kind: "once";
      scheduledLocal: string;
      timezone: string;
    }
  | {
      kind: "recurring";
      frequency: MemoReminderFrequency;
      interval: number;
      weekdays: number[];
      monthlyDay: number | null;
      localTime: string;
      startsOn: string;
      endsOn: string | null;
      timezone: string;
    };

export interface MemoTag {
  id: string;
  name: string;
}

export interface MemoTagSummary extends MemoTag {
  memoCount: number;
}

export interface MemoReminder {
  id: string;
  memoId: string;
  schedule: MemoReminderSchedule;
  nextScheduledFor: string | null;
  status: MemoReminderStatus;
  createdAt: string;
  updatedAt: string;
}

export interface MemoInput {
  title: string;
  body: string;
  tags: string[];
  pinned: boolean;
  reminder: MemoReminderSchedule | null;
}

export interface MemoRecord {
  id: string;
  title: string;
  body: string;
  displayTitle: string;
  tags: MemoTag[];
  pinnedAt: string | null;
  reminder: MemoReminder | null;
  createdAt: string;
  updatedAt: string;
}

export interface MemoSummary {
  id: string;
  displayTitle: string;
  bodyPreview: string;
  tags: MemoTag[];
  pinnedAt: string | null;
  reminder: MemoReminder | null;
  updatedAt: string;
}

export interface MemoListQuery {
  search: string;
  tagId: string | null;
}

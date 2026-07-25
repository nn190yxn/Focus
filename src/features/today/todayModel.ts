import { zhCNMessages } from "../../i18n/messages";
import type { CheckItemInput, TaskCategory, TaskListItem, TaskVisualState } from "../tasks/types";
import { summarizePattern } from "../recurrence/recurrenceSummary";
import type { RecurrencePattern } from "../recurrence/types";
import type { TodayDigestItem, TodaySourceKind } from "./types";

export type WorkspaceTask = TaskListItem & {
  visualState: TaskVisualState;
  checkItems: CheckItemInput[];
  sourceKind: TodaySourceKind;
  sourceId: string;
  recurrenceRuleId: string | null;
  recurrenceLabel: string | null;
};

export type WeekDay = {
  date: string;
  dayLabel: string;
  dateLabel: string;
};

export type TaskSection = {
  category: TaskCategory;
  label: string;
  tasks: WorkspaceTask[];
};

const categories: TaskCategory[] = ["work", "study", "health", "life"];

export function localDateString(date = new Date()): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

export function buildWeekDays(selectedDate: string, locale = "zh-CN"): WeekDay[] {
  const selected = parseLocalDate(selectedDate);
  const mondayOffset = (selected.getDay() + 6) % 7;
  const monday = new Date(selected);
  monday.setDate(selected.getDate() - mondayOffset);
  return Array.from({ length: 7 }, (_, index) => {
    const date = new Date(monday);
    date.setDate(monday.getDate() + index);
    return {
      date: localDateString(date),
      dayLabel: new Intl.DateTimeFormat(locale, { weekday: "short" }).format(date),
      dateLabel: String(date.getDate()),
    };
  });
}

export function decorateTaskList(items: TaskListItem[], selectedDate: string): WorkspaceTask[] {
  let hasCurrentTask = false;
  return items.map((item) => {
    let visualState: TaskVisualState = "normal";
    if (item.task.status === "completed") visualState = "completed";
    else if (item.project?.status === "paused") visualState = "paused";
    else if (item.task.scheduledDate && item.task.scheduledDate < selectedDate) visualState = "overdue";
    else if (!hasCurrentTask) {
      visualState = "current";
      hasCurrentTask = true;
    }
    return { ...item, visualState, checkItems: [], sourceKind: "task", sourceId: item.task.id, recurrenceRuleId: null, recurrenceLabel: null };
  });
}

export function decorateTodayDigest(items: TodayDigestItem[]): WorkspaceTask[] {
  let hasCurrentTask = false;
  return items.map((item) => {
    let visualState: TaskVisualState = "normal";
    if (item.status === "completed") visualState = "completed";
    else if (item.project?.status === "paused") visualState = "paused";
    else if (item.isOverdue) visualState = "overdue";
    else if (!hasCurrentTask) {
      visualState = "current";
      hasCurrentTask = true;
    }
    return {
      sourceKind: item.sourceKind,
      sourceId: item.sourceId,
      recurrenceRuleId: item.recurrenceRuleId,
      recurrenceLabel: item.sourceKind === "recurringInstance" ? zhCNMessages["task.recurring"] : null,
      visualState,
      checkItems: [],
      project: item.project,
      task: {
        id: item.sourceId,
        projectId: item.project?.id ?? null,
        title: item.title,
        category: item.category,
        priority: item.priority,
        scheduledDate: item.scheduledDate,
        scheduledTime: item.scheduledTime,
        status: item.status,
        completedAt: item.completedAt,
        createdAt: item.createdAt,
        updatedAt: item.createdAt,
      },
    };
  });
}

export function recurrenceBadge(pattern: RecurrencePattern): string {
  return summarizePattern(pattern);
}

export function taskSections(
  tasks: WorkspaceTask[],
  completion: "all" | "pending" | "completed",
  labels: Record<TaskCategory, string> = {
    work: zhCNMessages["task.category.work"],
    study: zhCNMessages["task.category.study"],
    health: zhCNMessages["task.category.health"],
    life: zhCNMessages["task.category.life"],
  },
): TaskSection[] {
  const visible = tasks.filter((item) => completion === "all" || (completion === "completed") === (item.task.status === "completed"));
  return categories.map((category) => ({
    category,
    label: labels[category],
    tasks: visible.filter((item) => item.task.category === category),
  })).filter((section) => section.tasks.length > 0);
}

export function scheduledTasks(tasks: WorkspaceTask[]): WorkspaceTask[] {
  return tasks
    .filter((item) => item.task.status !== "completed" && item.task.scheduledTime)
    .sort((left, right) => (left.task.scheduledTime ?? "").localeCompare(right.task.scheduledTime ?? ""));
}

function parseLocalDate(value: string): Date {
  const [year, month, day] = value.split("-").map(Number);
  return new Date(year, month - 1, day, 12);
}

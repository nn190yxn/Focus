import type { FocusState, FocusTarget } from "../focus/types";
import type { TodayDigestItem } from "../today/types";
import type { WidgetSize } from "./types";

const taskLimits: Record<WidgetSize, number> = {
  compact: 1,
  standard: 5,
  expanded: 10,
};

export function selectWidgetTasks(items: TodayDigestItem[], size: WidgetSize) {
  if (size === "compact") {
    const nextPending = items.find((item) => item.status === "pending");
    return nextPending ? [nextPending] : items.slice(0, 1);
  }
  return items.slice(0, taskLimits[size]);
}

export function calculateTodayProgress(items: TodayDigestItem[]) {
  const completed = items.filter((item) => item.status === "completed").length;
  return {
    completed,
    total: items.length,
    percentage: items.length === 0 ? 0 : Math.round((completed / items.length) * 100),
  };
}

export function focusTargetForItem(item: TodayDigestItem): FocusTarget {
  return item.sourceKind === "recurringInstance"
    ? { taskId: null, taskInstanceId: item.sourceId }
    : { taskId: item.sourceId, taskInstanceId: null };
}

export function focusedItem(items: TodayDigestItem[], state: FocusState) {
  if (state.state === "ready") return null;
  return items.find((item) =>
    state.taskInstanceId
      ? item.sourceKind === "recurringInstance" && item.sourceId === state.taskInstanceId
      : item.sourceKind === "task" && item.sourceId === state.taskId,
  ) ?? null;
}

export function remainingFocusSeconds(state: FocusState, now: Date) {
  if (state.state === "ready") return 0;
  if (state.state === "paused") return state.remainingSeconds;
  return Math.max(0, Math.ceil((new Date(state.targetEndsAt).getTime() - now.getTime()) / 1000));
}

export function formatWidgetClock(date: Date, locale = "zh-CN") {
  return new Intl.DateTimeFormat(locale, {
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  }).format(date);
}

export function formatWidgetDate(date: Date, locale = "zh-CN") {
  return new Intl.DateTimeFormat(locale, {
    month: "long",
    day: "numeric",
    weekday: "short",
  }).format(date);
}

export function formatFocusDuration(seconds: number) {
  const minutes = Math.floor(seconds / 60);
  const remainder = seconds % 60;
  return `${String(minutes).padStart(2, "0")}:${String(remainder).padStart(2, "0")}`;
}

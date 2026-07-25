import { describe, expect, it } from "vitest";

import type { FocusState } from "../focus/types";
import type { TodayDigestItem } from "../today/types";
import {
  calculateTodayProgress,
  focusTargetForItem,
  focusedItem,
  formatFocusDuration,
  remainingFocusSeconds,
  selectWidgetTasks,
} from "./widgetModel";

function item(id: string, status: TodayDigestItem["status"] = "pending"): TodayDigestItem {
  return {
    sourceKind: "task",
    sourceId: id,
    itemKind: "ordinaryTask",
    recurrenceRuleId: null,
    title: id,
    category: "work",
    priority: 1,
    scheduledDate: "2026-07-19",
    scheduledTime: null,
    status,
    completedAt: status === "completed" ? "2026-07-19T08:00:00Z" : null,
    project: null,
    isOverdue: false,
    createdAt: "2026-07-19T07:00:00Z",
  };
}

describe("widgetModel", () => {
  it("uses the next pending task for compact and size capacities for larger layouts", () => {
    const items = [item("done", "completed"), ...Array.from({ length: 11 }, (_, index) => item(`task-${index}`))];
    expect(selectWidgetTasks(items, "compact").map((entry) => entry.sourceId)).toEqual(["task-0"]);
    expect(selectWidgetTasks(items, "standard")).toHaveLength(5);
    expect(selectWidgetTasks(items, "expanded")).toHaveLength(10);
  });

  it("calculates stable empty and populated progress", () => {
    expect(calculateTodayProgress([])).toEqual({ completed: 0, total: 0, percentage: 0 });
    expect(calculateTodayProgress([item("one", "completed"), item("two")])).toEqual({
      completed: 1,
      total: 2,
      percentage: 50,
    });
  });

  it("maps ordinary and recurring sources to exclusive focus targets", () => {
    const task = item("task");
    const recurring = { ...item("instance"), sourceKind: "recurringInstance" as const };
    expect(focusTargetForItem(task)).toEqual({ taskId: "task", taskInstanceId: null });
    expect(focusTargetForItem(recurring)).toEqual({ taskId: null, taskInstanceId: "instance" });
  });

  it("resolves active focus and derives remaining display time", () => {
    const items = [item("task")];
    const state: FocusState = {
      state: "running",
      taskId: "task",
      taskInstanceId: null,
      plannedSeconds: 1500,
      remainingSeconds: 1500,
      startedAt: "2026-07-19T10:00:00Z",
      interruptionCount: 0,
      serverTime: "2026-07-19T10:00:00Z",
      targetEndsAt: "2026-07-19T10:25:00Z",
    };
    expect(focusedItem(items, state)?.sourceId).toBe("task");
    expect(remainingFocusSeconds(state, new Date("2026-07-19T10:03:30Z"))).toBe(1290);
    expect(formatFocusDuration(1290)).toBe("21:30");
  });
});

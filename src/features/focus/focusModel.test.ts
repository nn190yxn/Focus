// @vitest-environment jsdom
import { describe, expect, it } from "vitest";

import { decorateTaskList, decorateTodayDigest } from "../today/todayModel";
import { availableFocusTasks, focusTargetForTask, formatFocusTime, isEditableTarget, remainingSeconds } from "./focusModel";
import type { FocusState } from "./types";

const task = decorateTaskList([{
  project: null,
  task: { id: "task-1", projectId: null, title: "整理提纲", category: "work", priority: 1, scheduledDate: "2026-07-19", scheduledTime: null, status: "pending", completedAt: null, createdAt: "2026-07-19T08:00:00Z", updatedAt: "2026-07-19T08:00:00Z" },
}], "2026-07-19")[0];

describe("focusModel", () => {
  it("maps ordinary tasks and recurrence instances to exclusive targets", () => {
    const recurring = decorateTodayDigest([{ sourceKind: "recurringInstance", sourceId: "instance-1", itemKind: "recurringInstance", recurrenceRuleId: "rule-1", title: "每日回顾", category: "work", priority: 2, scheduledDate: "2026-07-19", scheduledTime: "18:00", status: "pending", completedAt: null, project: null, isOverdue: false, createdAt: "2026-07-19T08:00:00Z" }])[0];

    expect(focusTargetForTask(task)).toEqual({ taskId: "task-1", taskInstanceId: null });
    expect(focusTargetForTask(recurring)).toEqual({ taskId: null, taskInstanceId: "instance-1" });
  });

  it("filters unavailable tasks and formats timer values", () => {
    const pausedTask = { ...task, sourceId: "paused", visualState: "paused" as const };
    const completedTask = { ...task, sourceId: "done", task: { ...task.task, id: "done", status: "completed" as const } };

    expect(availableFocusTasks([task, pausedTask, completedTask])).toEqual([task]);
    expect(formatFocusTime(1_501)).toBe("25:01");
    expect(formatFocusTime(-2)).toBe("00:00");
  });

  it("derives and clamps running time from the persisted deadline", () => {
    const state: FocusState = { state: "running", taskId: "task-1", taskInstanceId: null, plannedSeconds: 900, remainingSeconds: 800, startedAt: "2026-07-19T10:00:00Z", targetEndsAt: "2026-07-19T10:15:00Z", interruptionCount: 0, serverTime: "2026-07-19T10:00:00Z" };

    expect(remainingSeconds(state, Date.parse("2026-07-19T10:00:10Z"))).toBe(890);
    expect(remainingSeconds(state, Date.parse("2026-07-19T09:00:00Z"))).toBe(900);
    expect(remainingSeconds(state, Date.parse("2026-07-19T10:20:00Z"))).toBe(0);
  });

  it("recognizes controls that must keep the space key", () => {
    expect(isEditableTarget(document.createElement("input"))).toBe(true);
    expect(isEditableTarget(document.createElement("select"))).toBe(true);
    expect(isEditableTarget(document.createElement("div"))).toBe(false);
  });
});

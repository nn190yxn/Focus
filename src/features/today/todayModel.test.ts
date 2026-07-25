import { describe, expect, it } from "vitest";

import type { TaskListItem, TaskProjectSummary } from "../tasks/types";
import { buildWeekDays, decorateTaskList, scheduledTasks, taskSections } from "./todayModel";

describe("todayModel", () => {
  it("builds a stable Monday-to-Sunday week across month boundaries", () => {
    expect(buildWeekDays("2026-08-01").map((item) => item.date)).toEqual([
      "2026-07-27", "2026-07-28", "2026-07-29", "2026-07-30", "2026-07-31", "2026-08-01", "2026-08-02",
    ]);
  });

  it("derives completed, paused, overdue, current, and normal states", () => {
    const decorated = decorateTaskList([
      makeItem("done", "work", "completed", "2026-07-18", null),
      makeItem("paused", "study", "pending", "2026-07-18", "paused"),
      makeItem("late", "health", "pending", "2026-07-17", null),
      makeItem("current", "life", "pending", "2026-07-18", null),
      makeItem("normal", "work", "pending", "2026-07-18", null),
    ], "2026-07-18");

    expect(decorated.map((item) => item.visualState)).toEqual(["completed", "paused", "overdue", "current", "normal"]);
  });

  it("groups completion-filtered tasks by category and sorts the schedule", () => {
    const tasks = decorateTaskList([
      makeItem("later", "work", "pending", "2026-07-18", null, "14:00"),
      makeItem("earlier", "work", "pending", "2026-07-18", null, "09:30"),
      makeItem("done", "life", "completed", "2026-07-18", null, "08:00"),
    ], "2026-07-18");

    expect(taskSections(tasks, "pending").map((section) => [section.category, section.tasks.length])).toEqual([["work", 2]]);
    expect(scheduledTasks(tasks).map((item) => item.task.id)).toEqual(["earlier", "later"]);
  });
});

function makeItem(id: string, category: TaskListItem["task"]["category"], status: TaskListItem["task"]["status"], scheduledDate: string, projectStatus: TaskProjectSummary["status"] | null, scheduledTime = "10:00"): TaskListItem {
  return {
    project: projectStatus ? { id: `${id}-project`, name: "项目", color: "#4eaa98", icon: "AF", status: projectStatus } : null,
    task: { id, projectId: null, title: id, category, priority: 0, scheduledDate, scheduledTime, status, completedAt: status === "completed" ? `${scheduledDate}T10:00:00Z` : null, createdAt: `${scheduledDate}T08:00:00Z`, updatedAt: `${scheduledDate}T08:00:00Z` },
  };
}

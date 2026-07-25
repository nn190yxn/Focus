// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";

import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { TaskRow } from "./TaskRow";
import type { TaskListItem, TaskVisualState } from "./types";

const item: TaskListItem = {
  project: { id: "focus", name: "抵达 Focus", color: "#4eaa98", icon: "AF", status: "active" },
  task: {
    id: "task-1",
    projectId: "focus",
    title: "实现任务行",
    category: "work",
    priority: 3,
    scheduledDate: "2026-07-18",
    scheduledTime: "10:30",
    status: "pending",
    completedAt: null,
    createdAt: "2026-07-18T08:00:00Z",
    updatedAt: "2026-07-18T08:00:00Z",
  },
};

describe("TaskRow", () => {
  it.each<[TaskVisualState, string]>([
    ["normal", "计划"],
    ["current", "当前"],
    ["completed", "已完成"],
    ["overdue", "已逾期"],
    ["paused", "已暂停"],
  ])("renders the %s visual state", (state, label) => {
    const { container } = render(<TaskRow item={item} state={state} onOpen={() => undefined} onToggleCompleted={() => undefined} />);
    expect(container.querySelector(".task-row")).toHaveAttribute("data-state", state);
    expect(screen.getByText(label)).toBeInTheDocument();
  });

  it("keeps open, complete, and focus actions independent", () => {
    const onOpen = vi.fn();
    const onToggleCompleted = vi.fn();
    const onStartFocus = vi.fn();
    render(<TaskRow item={item} state="current" onOpen={onOpen} onToggleCompleted={onToggleCompleted} onStartFocus={onStartFocus} />);

    fireEvent.click(screen.getByRole("button", { name: "打开任务：实现任务行" }));
    fireEvent.click(screen.getByRole("button", { name: "完成任务：实现任务行" }));
    fireEvent.click(screen.getByRole("button", { name: "专注任务：实现任务行" }));

    expect(onOpen).toHaveBeenCalledWith("task-1");
    expect(onToggleCompleted).toHaveBeenCalledWith("task-1", true);
    expect(onStartFocus).toHaveBeenCalledWith("task-1");
  });

  it("exposes every task action in the keyboard focus order", () => {
    render(<TaskRow item={item} state="current" onOpen={() => undefined} onToggleCompleted={() => undefined} onStartFocus={() => undefined} />);

    const actions = [
      screen.getByRole("button", { name: "打开任务：实现任务行" }),
      screen.getByRole("button", { name: "完成任务：实现任务行" }),
      screen.getByRole("button", { name: "专注任务：实现任务行" }),
    ];
    for (const action of actions) {
      expect(action).toHaveProperty("tabIndex", 0);
      action.focus();
      expect(action).toHaveFocus();
    }
  });
});

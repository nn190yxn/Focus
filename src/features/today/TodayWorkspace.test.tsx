// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";

import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { decorateTaskList, decorateTodayDigest } from "./todayModel";
import { TodayWorkspace } from "./TodayWorkspace";

const tasks = decorateTaskList([
  {
    project: { id: "focus", name: "抵达 Focus", color: "#4eaa98", icon: "AF", status: "active" },
    task: { id: "pending", projectId: "focus", title: "完成工作台", category: "work", priority: 3, scheduledDate: "2026-07-18", scheduledTime: "10:30", status: "pending", completedAt: null, createdAt: "2026-07-18T08:00:00Z", updatedAt: "2026-07-18T08:00:00Z" },
  },
  {
    project: null,
    task: { id: "completed", projectId: null, title: "晨间散步", category: "health", priority: 0, scheduledDate: "2026-07-18", scheduledTime: null, status: "completed", completedAt: "2026-07-18T07:00:00Z", createdAt: "2026-07-18T06:00:00Z", updatedAt: "2026-07-18T07:00:00Z" },
  },
], "2026-07-18");

describe("TodayWorkspace", () => {
  afterEach(() => vi.useRealTimers());

  it("renders task sections, schedule, goals, and notes", () => {
    render(<TodayWorkspace tasks={tasks} onCreate={() => undefined} onEdit={() => undefined} onToggleCompleted={() => undefined} onStartFocus={() => undefined} />);

    expect(screen.getByRole("heading", { name: "工作" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "健康" })).toBeInTheDocument();
    expect(screen.getByLabelText("本周目标")).toBeInTheDocument();
    expect(screen.getByRole("textbox", { name: "随手便签" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "打开任务：完成工作台" })).toBeInTheDocument();
  });

  it("filters completion state and exposes quick actions", () => {
    const onCreate = vi.fn();
    const onStartFocus = vi.fn();
    render(<TodayWorkspace tasks={tasks} onCreate={onCreate} onEdit={() => undefined} onToggleCompleted={() => undefined} onStartFocus={onStartFocus} />);

    fireEvent.click(screen.getByRole("radio", { name: "待完成" }));
    expect(screen.queryByText("晨间散步")).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "专注任务：完成工作台" }));
    fireEvent.click(screen.getByRole("button", { name: /快速创建/ }));

    expect(onStartFocus).toHaveBeenCalledWith("pending");
    expect(onCreate).toHaveBeenCalledOnce();
  });

  it("shows recurrence identity and dispatches instance actions", () => {
    const onSkip = vi.fn();
    const onDelay = vi.fn();
    const onReschedule = vi.fn();
    const recurring = decorateTodayDigest([{ sourceKind: "recurringInstance", sourceId: "instance-1", itemKind: "recurringInstance", recurrenceRuleId: "rule-1", title: "每日回顾", category: "work", priority: 2, scheduledDate: "2026-07-18", scheduledTime: "18:00", status: "pending", completedAt: null, project: tasks[0].project, isOverdue: true, createdAt: "2026-07-17T08:00:00Z" }]);
    render(<TodayWorkspace tasks={recurring} onCreate={() => undefined} onEdit={() => undefined} onToggleCompleted={() => undefined} onStartFocus={() => undefined} onSkipInstance={onSkip} onDelayInstance={onDelay} onRescheduleInstance={onReschedule} />);

    expect(screen.getByText("重复计划")).toBeInTheDocument();
    expect(screen.getByText("已逾期")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "延后任务：每日回顾" }));
    fireEvent.click(screen.getByRole("button", { name: "顺延任务到明天：每日回顾" }));
    fireEvent.click(screen.getByRole("button", { name: "跳过任务：每日回顾" }));
    expect(onDelay).toHaveBeenCalledWith("instance-1");
    expect(onReschedule).toHaveBeenCalledWith("instance-1");
    expect(onSkip).toHaveBeenCalledWith("instance-1");
  });

  it("auto-saves a note after 500ms of inactivity", async () => {
    vi.useFakeTimers();
    const onSaveNote = vi.fn(async () => undefined);
    render(<TodayWorkspace tasks={tasks} noteDate="2026-07-18" noteBody="旧内容" onSaveNote={onSaveNote} onCreate={() => undefined} onEdit={() => undefined} onToggleCompleted={() => undefined} onStartFocus={() => undefined} />);

    fireEvent.change(screen.getByRole("textbox", { name: "随手便签" }), { target: { value: "新的临时想法" } });
    await act(async () => vi.advanceTimersByTime(499));
    expect(onSaveNote).not.toHaveBeenCalled();
    await act(async () => vi.advanceTimersByTime(1));

    expect(onSaveNote).toHaveBeenCalledOnce();
    expect(onSaveNote).toHaveBeenCalledWith("新的临时想法");
    expect(screen.getByRole("status")).toHaveTextContent("已保存");
  });

  it("saves a note immediately with Ctrl+Enter", async () => {
    vi.useFakeTimers();
    const onSaveNote = vi.fn(async () => undefined);
    render(<TodayWorkspace tasks={tasks} noteDate="2026-07-18" onSaveNote={onSaveNote} onCreate={() => undefined} onEdit={() => undefined} onToggleCompleted={() => undefined} onStartFocus={() => undefined} />);
    const note = screen.getByRole("textbox", { name: "随手便签" });

    fireEvent.change(note, { target: { value: "立即记录" } });
    fireEvent.keyDown(note, { key: "Enter", ctrlKey: true });
    await act(async () => Promise.resolve());
    await act(async () => vi.advanceTimersByTime(500));

    expect(onSaveNote).toHaveBeenCalledOnce();
    expect(onSaveNote).toHaveBeenCalledWith("立即记录");
  });

  it("keeps newer note input when an earlier save completes", async () => {
    vi.useFakeTimers();
    let finishFirstSave: (() => void) | undefined;
    const firstSave = new Promise<void>((resolve) => { finishFirstSave = resolve; });
    const onSaveNote = vi.fn()
      .mockImplementationOnce(() => firstSave)
      .mockResolvedValueOnce(undefined);
    const view = render(<TodayWorkspace tasks={tasks} noteDate="2026-07-18" noteBody="旧内容" onSaveNote={onSaveNote} onCreate={() => undefined} onEdit={() => undefined} onToggleCompleted={() => undefined} onStartFocus={() => undefined} />);
    const note = screen.getByRole("textbox", { name: "随手便签" });

    fireEvent.change(note, { target: { value: "第一次输入" } });
    await act(async () => vi.advanceTimersByTime(500));
    fireEvent.change(note, { target: { value: "继续输入的新内容" } });
    view.rerender(<TodayWorkspace tasks={tasks} noteDate="2026-07-18" noteBody="第一次输入" onSaveNote={onSaveNote} onCreate={() => undefined} onEdit={() => undefined} onToggleCompleted={() => undefined} onStartFocus={() => undefined} />);
    await act(async () => finishFirstSave?.());

    expect(note).toHaveValue("继续输入的新内容");
    await act(async () => vi.advanceTimersByTime(500));
    expect(onSaveNote).toHaveBeenLastCalledWith("继续输入的新内容");
  });

  it("saves the current note with the visible save button", async () => {
    vi.useFakeTimers();
    const onSaveNote = vi.fn(async () => undefined);
    render(<TodayWorkspace tasks={tasks} noteDate="2026-07-18" onSaveNote={onSaveNote} onCreate={() => undefined} onEdit={() => undefined} onToggleCompleted={() => undefined} onStartFocus={() => undefined} />);

    fireEvent.change(screen.getByRole("textbox", { name: "随手便签" }), { target: { value: "手动保存" } });
    fireEvent.click(screen.getByRole("button", { name: "保存记录" }));
    await act(async () => Promise.resolve());
    await act(async () => vi.advanceTimersByTime(500));

    expect(onSaveNote).toHaveBeenCalledOnce();
    expect(onSaveNote).toHaveBeenCalledWith("手动保存");
  });

  it("submits a weekly goal with category and target", async () => {
    const onSaveWeeklyGoal = vi.fn(async () => undefined);
    render(<TodayWorkspace tasks={tasks} weekStartsOn="2026-07-13" onSaveWeeklyGoal={onSaveWeeklyGoal} onCreate={() => undefined} onEdit={() => undefined} onToggleCompleted={() => undefined} onStartFocus={() => undefined} />);

    fireEvent.change(screen.getByRole("textbox", { name: "目标名称" }), { target: { value: "保持深度工作" } });
    fireEvent.change(screen.getByRole("combobox", { name: "目标分类" }), { target: { value: "focusMinutes" } });
    fireEvent.change(screen.getByRole("spinbutton", { name: "目标数量" }), { target: { value: "180" } });
    fireEvent.click(screen.getByRole("button", { name: "添加" }));

    await waitFor(() => expect(onSaveWeeklyGoal).toHaveBeenCalledWith({ id: null, weekStartsOn: "2026-07-13", title: "保持深度工作", category: "focusMinutes", targetCount: 180 }));
  });
});

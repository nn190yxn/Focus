// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";

import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  defaultWidgetConfig,
  type WidgetConfig,
  type WidgetModeFallbackEvent,
} from "../features/widget/types";
import type { TodayDigest, TodayDigestItem } from "../features/today/types";
import { WidgetApp } from "./WidgetApp";

const mocks = vi.hoisted(() => ({
  getConfig: vi.fn(),
  updateConfig: vi.fn(),
  hideWidget: vi.fn(),
  getDigest: vi.fn(),
  getFocusState: vi.fn(),
  startFocus: vi.fn(),
  pauseFocus: vi.fn(),
  resumeFocus: vi.fn(),
  completeTask: vi.fn(),
  completeInstance: vi.fn(),
  delayInstance: vi.fn(),
  getSettings: vi.fn(),
  unlisten: vi.fn(),
  listeners: new Map<string, (event: { payload: unknown }) => void>(),
}));

vi.mock("../lib/commandClient", () => ({
  isTauriRuntime: () => true,
}));

vi.mock("../features/widget/widgetClient", () => ({
  widgetClient: { getConfig: mocks.getConfig, updateConfig: mocks.updateConfig, hide: mocks.hideWidget },
}));

vi.mock("../features/settings/settingsClient", () => ({
  settingsClient: { get: mocks.getSettings },
}));

vi.mock("../features/today/todayClient", () => ({
  todayClient: { getDigest: mocks.getDigest },
}));

vi.mock("../features/focus/focusClient", () => ({
  focusClient: {
    getState: mocks.getFocusState,
    start: mocks.startFocus,
    pause: mocks.pauseFocus,
    resume: mocks.resumeFocus,
  },
}));

vi.mock("../features/tasks/taskClient", () => ({
  taskClient: { setCompleted: mocks.completeTask },
}));

vi.mock("../features/recurrence/recurrenceClient", () => ({
  recurrenceClient: { complete: mocks.completeInstance, delayToday: mocks.delayInstance },
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn((event: string, listener: (event: { payload: unknown }) => void) => {
    mocks.listeners.set(event, listener);
    return Promise.resolve(mocks.unlisten);
  }),
}));

beforeEach(() => {
  vi.clearAllMocks();
  vi.spyOn(window.navigator, "languages", "get").mockReturnValue(["zh-CN"]);
  mocks.listeners.clear();
  mocks.getConfig.mockResolvedValue({ ok: true, data: defaultWidgetConfig, version: 1 });
  mocks.hideWidget.mockResolvedValue({ ok: true, data: null, version: 1 });
  mocks.getSettings.mockResolvedValue({
    ok: true,
    data: { language: "system", appearance: "system", theme: "mint", backgroundRunning: true },
    version: 1,
  });
  mocks.getDigest.mockResolvedValue({ ok: true, data: digest(), version: 1 });
  mocks.getFocusState.mockResolvedValue({
    ok: true,
    data: { state: "ready", serverTime: "2026-07-19T10:00:00Z" },
    version: 1,
  });
  mocks.completeTask.mockResolvedValue({ ok: true, data: {}, version: 1 });
  mocks.completeInstance.mockResolvedValue({ ok: true, data: {}, version: 1 });
  mocks.delayInstance.mockResolvedValue({ ok: true, data: {}, version: 1 });
  mocks.startFocus.mockResolvedValue({
    ok: true,
    data: {
      state: "running",
      taskId: "task-1",
      taskInstanceId: null,
      plannedSeconds: 1500,
      remainingSeconds: 1500,
      startedAt: "2026-07-19T10:00:00Z",
      interruptionCount: 0,
      serverTime: "2026-07-19T10:00:00Z",
      targetEndsAt: "2026-07-19T10:25:00Z",
    },
    version: 1,
  });
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("WidgetApp", () => {
  it("loads persisted configuration and follows configuration events", async () => {
    mocks.getConfig.mockResolvedValue({
      ok: true,
      data: { ...defaultWidgetConfig, size: "compact", opacity: 0.65 },
      version: 1,
    });

    const { container, unmount } = render(<WidgetApp />);
    const widget = container.querySelector("main");
    await waitFor(() => expect(widget).toHaveClass("widget--compact"));
    expect(widget).toHaveStyle({ "--widget-opacity": "65%" });

    act(() => {
      mocks.listeners.get("widget://config-changed")?.({
        payload: { ...defaultWidgetConfig, size: "expanded", locked: true, opacity: 0.8 },
      });
    });
    expect(widget).toHaveClass("widget--expanded");
    expect(widget).toHaveAttribute("data-locked", "true");
    expect(widget).toHaveStyle({ "--widget-opacity": "80%" });

    unmount();
    expect(mocks.unlisten).toHaveBeenCalledTimes(7);
  });

  it("refreshes focus and today data after a backup restore", async () => {
    render(<WidgetApp />);
    await waitFor(() => expect(mocks.listeners.has("backup://restored")).toBe(true));
    await waitFor(() => expect(mocks.getDigest).toHaveBeenCalledTimes(1));
    expect(mocks.getFocusState).toHaveBeenCalledTimes(1);

    act(() => {
      mocks.listeners.get("backup://restored")?.({ payload: {} });
    });

    await waitFor(() => expect(mocks.getDigest).toHaveBeenCalledTimes(2));
    expect(mocks.getFocusState).toHaveBeenCalledTimes(2);
  });

  it("refreshes today data after recurrence generation", async () => {
    render(<WidgetApp />);
    await waitFor(() => expect(mocks.listeners.has("today://changed")).toBe(true));
    await waitFor(() => expect(mocks.getDigest).toHaveBeenCalledTimes(1));

    act(() => {
      mocks.listeners.get("today://changed")?.({ payload: {} });
    });

    await waitFor(() => expect(mocks.getDigest).toHaveBeenCalledTimes(2));
    expect(mocks.getFocusState).toHaveBeenCalledTimes(1);
  });

  it("follows focus state changes from another window", async () => {
    render(<WidgetApp />);
    await waitFor(() => expect(mocks.listeners.has("focus://state-changed")).toBe(true));

    act(() => {
      mocks.listeners.get("focus://state-changed")?.({
        payload: {
          state: "paused",
          taskId: "task-1",
          taskInstanceId: null,
          plannedSeconds: 1500,
          remainingSeconds: 900,
          startedAt: "2026-07-19T10:00:00Z",
          interruptionCount: 1,
          serverTime: "2026-07-19T10:10:00Z",
          pausedAt: "2026-07-19T10:10:00Z",
        },
      });
    });

    expect(screen.getByText("已暂停")).toBeInTheDocument();
    expect(screen.getByText("15:00")).toBeInTheDocument();
  });

  it("loads persisted appearance and follows shared settings events", async () => {
    mocks.getSettings.mockResolvedValue({
      ok: true,
      data: { language: "zhCn", appearance: "dark", theme: "office", backgroundRunning: true },
      version: 1,
    });
    const { container } = render(<WidgetApp />);
    const widget = container.querySelector("main");

    await waitFor(() => expect(widget).toHaveAttribute("data-theme", "office"));
    expect(widget).toHaveAttribute("data-mode", "dark");

    act(() => {
      mocks.listeners.get("settings://changed")?.({
        payload: { language: "en", appearance: "light", theme: "blush", backgroundRunning: false },
      });
    });
    expect(widget).toHaveAttribute("data-theme", "blush");
    expect(widget).toHaveAttribute("data-mode", "light");
    expect(widget).toHaveAttribute("data-locale", "en-US");
    expect(container).toHaveTextContent("Today's progress");
  });

  it("shows the floating fallback status from the shell event", async () => {
    mocks.getConfig.mockResolvedValue({
      ok: true,
      data: defaultWidgetConfig,
      version: 1,
    });
    const { findByRole } = render(<WidgetApp />);
    await waitFor(() => expect(mocks.listeners.has("widget://mode-fallback")).toBe(true));

    act(() => {
      mocks.listeners.get("widget://mode-fallback")?.({
        payload: {
          fromMode: "desktop",
          toMode: "floating",
          reason: "HOST_NOT_FOUND",
        } satisfies WidgetModeFallbackEvent,
      });
    });

    expect(await findByRole("status")).toHaveTextContent("已切换到普通浮窗");

    act(() => {
      mocks.listeners.get("widget://mode-restored")?.({ payload: null });
    });
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
  });

  it("marks the header as a drag region and locks through persisted config", async () => {
    mocks.getConfig.mockResolvedValue({ ok: true, data: defaultWidgetConfig, version: 1 });
    mocks.updateConfig.mockResolvedValue({
      ok: true,
      data: { ...defaultWidgetConfig, locked: true },
      version: 1,
    });
    const { findByRole, container } = render(<WidgetApp />);

    expect(container.querySelector("header")).toHaveAttribute("data-tauri-drag-region");
    fireEvent.click(await findByRole("button", { name: "锁定小组件" }));

    await waitFor(() =>
      expect(mocks.updateConfig).toHaveBeenCalledWith({
        ...defaultWidgetConfig,
        locked: true,
      }),
    );
    await waitFor(() => expect(container.querySelector("main")).toHaveAttribute("data-locked", "true"));
  });

  it("hides the widget from its header close button", async () => {
    render(<WidgetApp />);

    fireEvent.click(await screen.findByRole("button", { name: "关闭小组件" }));

    await waitFor(() => expect(mocks.hideWidget).toHaveBeenCalledTimes(1));
  });

  it("renders five detailed tasks in standard mode and one pending task in compact mode", async () => {
    mocks.getDigest.mockResolvedValue({ ok: true, data: digest(7), version: 1 });
    const { container } = render(<WidgetApp />);
    await waitFor(() => expect(container.querySelectorAll(".widget-task")).toHaveLength(5));
    expect(container).toHaveTextContent("今日抵达");
    expect(container).toHaveTextContent("抵达项目");
    expect(container).toHaveTextContent("重复");

    act(() => {
      mocks.listeners.get("widget://config-changed")?.({
        payload: { ...defaultWidgetConfig, size: "compact" },
      });
    });
    expect(container.querySelectorAll(".widget-task")).toHaveLength(0);
    expect(container.querySelector(".widget__compact-task")).toBeInTheDocument();
  });

  it("dispatches task completion, recurring delay, and 25-minute focus actions", async () => {
    const prompt = vi.spyOn(window, "prompt").mockReturnValue("15:30");
    const { container } = render(<WidgetApp />);
    await waitFor(() => expect(container.querySelectorAll(".widget-task")).toHaveLength(5));
    const taskRows = container.querySelectorAll<HTMLElement>(".widget-task");

    fireEvent.click(within(taskRows[0]).getByRole("button", { name: "完成任务：任务 1" }));
    await waitFor(() => expect(mocks.completeTask).toHaveBeenCalledWith("task-1", true));

    fireEvent.click(within(taskRows[1]).getByRole("button", { name: "延后任务：任务 2" }));
    await waitFor(() => expect(mocks.delayInstance).toHaveBeenCalledWith("task-2", "15:30"));

    fireEvent.click(within(taskRows[0]).getByRole("button", { name: "专注任务：任务 1" }));
    await waitFor(() =>
      expect(mocks.startFocus).toHaveBeenCalledWith({ taskId: "task-1", taskInstanceId: null }, 25),
    );
    prompt.mockRestore();
  });
});

function digest(count = 5): TodayDigest {
  return {
    date: "2026-07-19",
    items: Array.from({ length: count }, (_, index) => item(index + 1)),
  };
}

function item(index: number): TodayDigestItem {
  const recurring = index === 2;
  return {
    sourceKind: recurring ? "recurringInstance" : "task",
    sourceId: `task-${index}`,
    itemKind: recurring ? "recurringInstance" : "projectTask",
    recurrenceRuleId: recurring ? "rule-1" : null,
    title: `任务 ${index}`,
    category: "work",
    priority: 2,
    scheduledDate: "2026-07-19",
    scheduledTime: `${String(9 + index).padStart(2, "0")}:00`,
    status: "pending",
    completedAt: null,
    project: recurring ? null : {
      id: "project-1",
      name: "抵达项目",
      color: "#45a88b",
      icon: "target",
      status: "active",
    },
    isOverdue: false,
    createdAt: "2026-07-19T08:00:00Z",
  };
}

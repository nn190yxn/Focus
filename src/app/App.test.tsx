// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";

import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const eventListeners = vi.hoisted(() => new Map<string, (event: { payload: unknown }) => void>());
const eventUnlisteners = vi.hoisted(() => new Map<string, ReturnType<typeof vi.fn>>());
const settingsMocks = vi.hoisted(() => ({ get: vi.fn(), update: vi.fn() }));
const todayMocks = vi.hoisted(() => ({ getDigest: vi.fn() }));
const projectMocks = vi.hoisted(() => ({ list: vi.fn() }));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (event: string, callback: (event: { payload: unknown }) => void) => {
    eventListeners.set(event, callback);
    const unlisten = vi.fn(() => eventListeners.delete(event));
    eventUnlisteners.set(event, unlisten);
    return unlisten;
  }),
}));

vi.mock("../features/memos/MemoWorkspace", () => ({
  MemoWorkspace: ({ dataRevision, openRequest, initialQuery, onQueryChange }: { dataRevision: number; openRequest: { memoId: string; sequence: number } | null; initialQuery: { search: string; tagId: string | null }; onQueryChange: (query: { search: string; tagId: string | null }) => void }) => (
    <div aria-label="备忘录工作区" data-revision={dataRevision} data-memo-id={openRequest?.memoId} data-open-sequence={openRequest?.sequence}>
      <input aria-label="测试备忘录搜索" value={initialQuery.search} onChange={(event) => onQueryChange({ ...initialQuery, search: event.target.value })} />
    </div>
  ),
}));

vi.mock("../features/today/todayClient", () => ({
  todayClient: {
    getDigest: todayMocks.getDigest,
  },
}));

vi.mock("../features/focus/focusClient", () => ({
  focusClient: {
    reconcile: vi.fn(async () => ({
      ok: true,
      data: { state: { state: "ready", serverTime: "2026-07-19T10:00:00Z" }, completedSession: null },
      version: 1,
    })),
  },
}));

vi.mock("../features/settings/settingsClient", () => ({
  settingsClient: settingsMocks,
}));

vi.mock("../features/projects/projectClient", () => ({
  projectClient: {
    list: projectMocks.list,
  },
}));

import { App, applicationTitle } from "./App";

beforeEach(() => {
  vi.spyOn(window.navigator, "languages", "get").mockReturnValue(["zh-CN"]);
  settingsMocks.get.mockResolvedValue({
    ok: true,
    data: { language: "system", appearance: "system", theme: "mint", backgroundRunning: true },
    version: 1,
  });
  settingsMocks.update.mockResolvedValue({
    ok: true,
    data: { language: "system", appearance: "dark", theme: "noir", backgroundRunning: true },
    version: 1,
  });
  todayMocks.getDigest.mockImplementation(async (date: string) => ({
    ok: true,
    data: { date, items: [] },
    version: 1,
  }));
  projectMocks.list.mockResolvedValue({ ok: true, data: [], version: 1 });
});

afterEach(() => {
  vi.restoreAllMocks();
  eventListeners.clear();
  eventUnlisteners.clear();
  Reflect.deleteProperty(window, "__TAURI_INTERNALS__");
});

describe("App", () => {
  it("renders the application identity", () => {
    expect(applicationTitle).toBe("抵达 Focus");
  });

  it("renders an interactive primary navigation within the startup budget", () => {
    const startedAt = performance.now();
    render(<App />);

    const today = screen.getByRole("button", { name: "今日" });
    expect(performance.now() - startedAt).toBeLessThan(3_000);
    expect(today).toBeEnabled();
    today.focus();
    expect(today).toHaveFocus();
  });

  it("places memos between today and projects and opens its route", () => {
    render(<App />);
    const navigation = screen.getByRole("navigation", { name: "主导航" });
    const labels = Array.from(navigation.querySelectorAll("button span"), (item) => item.textContent);
    expect(labels).toEqual(["今日", "备忘录", "项目", "专注", "日历", "设置"]);

    const memos = screen.getByRole("button", { name: "备忘录" });
    fireEvent.click(memos);

    expect(screen.getByRole("heading", { name: "备忘录" })).toBeInTheDocument();
    expect(screen.getByLabelText("备忘录工作区")).toBeInTheDocument();
    expect(memos).toHaveAttribute("aria-current", "page");
    expect(screen.queryByLabelText("本周日期")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "新建任务" })).not.toBeInTheDocument();
  });

  it("retains memo filters while navigating between main pages", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "备忘录" }));
    fireEvent.change(screen.getByLabelText("测试备忘录搜索"), { target: { value: "季度复盘" } });

    fireEvent.click(screen.getByRole("button", { name: "项目" }));
    fireEvent.click(screen.getByRole("button", { name: "备忘录" }));

    expect(screen.getByLabelText("测试备忘录搜索")).toHaveValue("季度复盘");
  });

  it("forwards memo changes and repeated open requests to the memo route", async () => {
    Object.defineProperty(window, "__TAURI_INTERNALS__", { configurable: true, value: {} });
    render(<App />);
    await waitFor(() => expect(eventListeners.has("memo://changed")).toBe(true));
    await waitFor(() => expect(eventListeners.has("memo://open-requested")).toBe(true));

    act(() => eventListeners.get("memo://changed")?.({ payload: null }));
    act(() => eventListeners.get("memo://open-requested")?.({ payload: "memo-1" }));

    const workspace = screen.getByLabelText("备忘录工作区");
    expect(screen.getByRole("heading", { name: "备忘录" })).toBeInTheDocument();
    expect(workspace).toHaveAttribute("data-revision", "1");
    expect(workspace).toHaveAttribute("data-memo-id", "memo-1");
    expect(workspace).toHaveAttribute("data-open-sequence", "1");

    act(() => eventListeners.get("memo://open-requested")?.({ payload: "memo-1" }));
    expect(workspace).toHaveAttribute("data-open-sequence", "2");
  });

  it("releases memo event listeners when the app unmounts", async () => {
    Object.defineProperty(window, "__TAURI_INTERNALS__", { configurable: true, value: {} });
    const { unmount } = render(<App />);
    await waitFor(() => expect(eventUnlisteners.has("memo://open-requested")).toBe(true));

    unmount();

    expect(eventUnlisteners.get("memo://changed")).toHaveBeenCalledOnce();
    expect(eventUnlisteners.get("memo://open-requested")).toHaveBeenCalledOnce();
  });

  it("opens quick task and focus views from tray events", async () => {
    Object.defineProperty(window, "__TAURI_INTERNALS__", { configurable: true, value: {} });
    render(<App />);
    await waitFor(() => expect(eventListeners.has("tray://quick-task")).toBe(true));

    act(() => eventListeners.get("tray://quick-task")?.({ payload: null }));
    expect(screen.getByRole("dialog", { name: "创建任务" })).toBeInTheDocument();

    await act(async () => {
      eventListeners.get("tray://open-focus")?.({ payload: null });
      await Promise.resolve();
    });
    expect(screen.getByRole("heading", { name: "专注" })).toBeInTheDocument();
  });

  it("refreshes project summaries and the current digest after today data changes", async () => {
    Object.defineProperty(window, "__TAURI_INTERNALS__", { configurable: true, value: {} });
    render(<App />);
    await waitFor(() => expect(eventListeners.has("today://changed")).toBe(true));
    await waitFor(() => expect(todayMocks.getDigest).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(projectMocks.list).toHaveBeenCalledTimes(1));

    act(() => eventListeners.get("today://changed")?.({ payload: null }));

    await waitFor(() => expect(todayMocks.getDigest).toHaveBeenCalledTimes(2));
    await waitFor(() => expect(projectMocks.list).toHaveBeenCalledTimes(2));
  });

  it("opens the calendar workspace from primary navigation", () => {
    render(<App />);
    const calendar = screen.getByRole("button", { name: "日历" });
    calendar.focus();
    fireEvent.click(calendar);

    expect(screen.getByRole("heading", { name: "日历" })).toBeInTheDocument();
    expect(screen.getByRole("radiogroup", { name: "日历视图" })).toBeInTheDocument();
    expect(calendar).toHaveFocus();
    expect(calendar).toHaveAttribute("aria-current", "page");
    expect(screen.queryByLabelText("本周日期")).not.toBeInTheDocument();
  });

  it("applies shared settings events to the main window theme", async () => {
    Object.defineProperty(window, "__TAURI_INTERNALS__", { configurable: true, value: {} });
    const { container } = render(<App />);
    await waitFor(() => expect(eventListeners.has("settings://changed")).toBe(true));

    act(() => {
      eventListeners.get("settings://changed")?.({
        payload: { language: "en", appearance: "dark", theme: "noir", backgroundRunning: false },
      });
    });

    expect(container.querySelector(".app-shell")).toHaveAttribute("data-theme", "noir");
    expect(container.querySelector(".app-shell")).toHaveAttribute("data-mode", "dark");
    expect(container.querySelector(".app-shell")).toHaveAttribute("data-locale", "en-US");
    expect(screen.getByRole("heading", { name: "Today" })).toBeInTheDocument();
    expect(document.documentElement).toHaveAttribute("lang", "en-US");
  });
});

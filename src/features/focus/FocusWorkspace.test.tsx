// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";

import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { decorateTaskList } from "../today/todayModel";
import { FocusWorkspace } from "./FocusWorkspace";

const tasks = decorateTaskList([{
  project: { id: "focus", name: "抵达 Focus", color: "#4eaa98", icon: "AF", status: "active" },
  task: { id: "task-1", projectId: "focus", title: "完成专注空间", category: "work", priority: 3, scheduledDate: "2026-07-19", scheduledTime: "10:30", status: "pending", completedAt: null, createdAt: "2026-07-19T08:00:00Z", updatedAt: "2026-07-19T08:00:00Z" },
}], "2026-07-19");

afterEach(() => {
  vi.useRealTimers();
});

describe("FocusWorkspace", () => {
  it("selects duration and transitions through running and paused states", async () => {
    render(<FocusWorkspace tasks={tasks} initialTask={tasks[0]} />);

    expect(screen.getByLabelText("剩余时间 25:00")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "15 分钟" }));
    expect(screen.getByLabelText("剩余时间 15:00")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "开始专注" }));
    await screen.findByText("专注进行中");
    await waitFor(() => expect(screen.getByRole("button", { name: "暂停" })).toBeEnabled());
    fireEvent.keyDown(window, { code: "Space" });
    await screen.findByText("已暂停");
    fireEvent.click(screen.getByRole("button", { name: "继续" }));
    await screen.findByText("专注进行中");
  });

  it("keeps space key behavior inside task inputs", async () => {
    render(<FocusWorkspace tasks={tasks} initialTask={tasks[0]} />);

    const taskSelect = screen.getByRole("combobox", { name: "选择任务" });
    fireEvent.keyDown(taskSelect, { code: "Space" });
    expect(screen.getByText("准备就绪")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "开始专注" })).toBeEnabled();

    fireEvent.click(screen.getByRole("button", { name: "自定义" }));
    fireEvent.change(screen.getByRole("spinbutton", { name: /分钟数/ }), { target: { value: "181" } });
    expect(screen.getByRole("button", { name: "开始专注" })).toBeDisabled();
  });

  it("confirms early completion and records the session", async () => {
    render(<FocusWorkspace tasks={tasks} initialTask={tasks[0]} />);
    fireEvent.click(screen.getByRole("button", { name: "开始专注" }));
    await screen.findByText("专注进行中");
    fireEvent.click(screen.getByRole("button", { name: "提前完成" }));
    expect(screen.getByRole("dialog", { name: "提前完成本轮专注？" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "确认完成" }));

    await waitFor(() => expect(screen.getByText("准备就绪")).toBeInTheDocument());
    expect(screen.getByText("完成专注空间", { selector: ".focus-sessions strong" })).toBeInTheDocument();
  });

  it("finishes naturally when the persisted deadline is reached", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-07-19T10:00:00Z"));
    render(<FocusWorkspace tasks={tasks} initialTask={tasks[0]} />);
    fireEvent.click(screen.getByRole("button", { name: "自定义" }));
    fireEvent.change(screen.getByRole("spinbutton", { name: /分钟数/ }), { target: { value: "1" } });
    fireEvent.click(screen.getByRole("button", { name: "开始专注" }));

    await act(async () => {
      await vi.advanceTimersByTimeAsync(60_500);
    });

    expect(screen.getByText("准备就绪")).toBeInTheDocument();
    expect(screen.getByText("完成专注空间", { selector: ".focus-sessions strong" })).toBeInTheDocument();
  });
});

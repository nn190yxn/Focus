// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { TaskEditor, validateTaskInput } from "./TaskEditor";
import type { TaskInput, TaskProjectSummary } from "./types";

const projects: TaskProjectSummary[] = [
  { id: "focus", name: "抵达 Focus", color: "#4eaa98", icon: "AF", status: "active" },
  { id: "paused", name: "暂停项目", color: "#c18471", icon: "PA", status: "paused" },
];

const validInput: TaskInput = {
  projectId: null,
  title: "整理任务编辑器",
  category: "work",
  priority: 2,
  scheduledDate: "2026-07-18",
  scheduledTime: "10:30",
  checkItems: [],
};

describe("validateTaskInput", () => {
  it("rejects past dates, orphan times, and empty check items", () => {
    expect(validateTaskInput({ ...validInput, scheduledDate: "2026-07-17" }, "2026-07-18")).toHaveProperty("scheduledDate");
    expect(validateTaskInput({ ...validInput, scheduledDate: null }, "2026-07-18")).toHaveProperty("scheduledTime");
    expect(validateTaskInput({ ...validInput, checkItems: [{ title: " ", completed: false }] }, "2026-07-18")).toHaveProperty("checkItems");
  });
});

describe("TaskEditor", () => {
  it("edits project and check items before submitting a valid task", async () => {
    const onSubmit = vi.fn();
    render(<TaskEditor today="2026-07-18" projects={projects} initialValue={validInput} submitLabel="创建任务" onCancel={() => undefined} onSubmit={onSubmit} />);

    fireEvent.change(screen.getByLabelText("所属项目"), { target: { value: "focus" } });
    fireEvent.click(screen.getByRole("button", { name: "添加检查项" }));
    fireEvent.change(screen.getByLabelText("检查项 1"), { target: { value: "完成组件测试" } });
    fireEvent.click(screen.getByRole("button", { name: "创建任务" }));

    await waitFor(() => expect(onSubmit).toHaveBeenCalledOnce());
    expect(onSubmit).toHaveBeenCalledWith(expect.objectContaining({
      projectId: "focus",
      checkItems: [{ title: "完成组件测试", completed: false }],
    }), null);
  });

  it("keeps invalid input and exposes an accessible error", () => {
    const onSubmit = vi.fn();
    render(<TaskEditor today="2026-07-18" projects={projects} onCancel={() => undefined} onSubmit={onSubmit} />);

    fireEvent.click(screen.getByRole("button", { name: "保存任务" }));

    expect(screen.getByText("请输入 1 至 200 个字符的任务标题")).toBeInTheDocument();
    expect(screen.getByLabelText("任务标题")).toHaveAttribute("aria-invalid", "true");
    expect(onSubmit).not.toHaveBeenCalled();
  });

  it("keeps controls keyboard-focusable and submits through form semantics", async () => {
    const onSubmit = vi.fn();
    render(<TaskEditor today="2026-07-18" projects={projects} initialValue={validInput} onCancel={() => undefined} onSubmit={onSubmit} />);

    const title = screen.getByRole("textbox", { name: "任务标题" });
    const submit = screen.getByRole("button", { name: "保存任务" });
    title.focus();
    expect(title).toHaveFocus();
    submit.focus();
    expect(submit).toHaveFocus();

    fireEvent.submit(submit.closest("form")!);
    await waitFor(() => expect(onSubmit).toHaveBeenCalledWith(validInput, null));
  });

  it("submits a structured recurrence rule with the task", async () => {
    const onSubmit = vi.fn();
    render(<TaskEditor today="2026-07-18" projects={projects} initialValue={validInput} onCancel={() => undefined} onSubmit={onSubmit} />);

    fireEvent.click(screen.getByRole("checkbox", { name: "设置每天、每周或每月重复任务" }));
    fireEvent.change(screen.getByLabelText("重复频率"), { target: { value: "weekly" } });
    fireEvent.click(screen.getByText("周三"));
    fireEvent.click(screen.getByRole("button", { name: "保存任务" }));

    await waitFor(() => expect(onSubmit).toHaveBeenCalledWith(validInput, expect.objectContaining({
      pattern: { kind: "weekly", interval: 1, weekdays: [1, 3] },
      startsOn: "2026-07-18",
    })));
  });
});

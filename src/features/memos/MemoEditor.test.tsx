// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";

import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { I18nProvider } from "../../i18n/I18nContext";
import { MemoEditor } from "./MemoEditor";
import type { MemoRecord } from "./types";

const memo: MemoRecord = {
  id: "memo-1",
  title: "项目复盘",
  body: "整理下一步",
  displayTitle: "项目复盘",
  tags: [{ id: "tag-1", name: "工作" }],
  pinnedAt: null,
  reminder: null,
  createdAt: "2026-07-24T07:00:00Z",
  updatedAt: "2026-07-24T08:00:00Z",
};

function renderEditor(overrides: Partial<React.ComponentProps<typeof MemoEditor>> = {}) {
  const props = {
    memo,
    saving: false,
    saveError: null,
    deleting: false,
    deleteError: null,
    onSave: vi.fn(),
    onDelete: vi.fn(),
    ...overrides,
  };
  const view = render(<I18nProvider locale="zh-CN"><MemoEditor {...props} /></I18nProvider>);
  return { ...props, ...view };
}

afterEach(() => vi.useRealTimers());

describe("MemoEditor metadata", () => {
  it("trims a new tag and removes an existing association", () => {
    const props = renderEditor();
    fireEvent.change(screen.getByRole("textbox", { name: "添加备忘录标签" }), { target: { value: "  计划  " } });
    fireEvent.click(screen.getByRole("button", { name: "添加" }));
    expect(props.onSave).toHaveBeenCalledWith(expect.objectContaining({ tags: ["工作", "计划"] }));
    expect(screen.getByText("已添加标签“计划”。")).toHaveAttribute("role", "status");

    const removeTag = screen.getByRole("button", { name: "移除标签：工作" });
    expect(removeTag).toHaveAttribute("title", "移除标签：工作");
    fireEvent.click(removeTag);
    expect(props.onSave).toHaveBeenLastCalledWith(expect.objectContaining({ tags: ["计划"] }));
  });

  it("reuses case-insensitive attached tags without another save", () => {
    const props = renderEditor({ memo: { ...memo, tags: [{ id: "tag-1", name: "Work" }] } });
    fireEvent.change(screen.getByRole("textbox", { name: "添加备忘录标签" }), { target: { value: "work" } });
    fireEvent.keyDown(screen.getByRole("textbox", { name: "添加备忘录标签" }), { key: "Enter" });
    expect(props.onSave).not.toHaveBeenCalled();
    expect(screen.getByText("标签“work”已经添加。")).toHaveAttribute("role", "status");
  });

  it("reports the ten-tag limit and toggles pin state", () => {
    const tags = Array.from({ length: 10 }, (_, index) => ({ id: `tag-${index}`, name: `标签${index}` }));
    const props = renderEditor({ memo: { ...memo, tags } });
    expect(screen.getByRole("textbox", { name: "添加备忘录标签" })).toBeDisabled();
    expect(screen.getByText("每条备忘录最多添加 10 个标签。")).toHaveAttribute("role", "status");

    fireEvent.click(screen.getByRole("button", { name: "置顶" }));
    expect(props.onSave).toHaveBeenCalledWith(expect.objectContaining({ pinned: true }));
  });

  it("shows, modifies, and cancels an active reminder", () => {
    const reminder = {
      id: "reminder-1",
      memoId: memo.id,
      schedule: { kind: "recurring" as const, frequency: "daily" as const, interval: 1, weekdays: [], monthlyDay: null, localTime: "09:00", startsOn: "2026-07-25", endsOn: null, timezone: "Asia/Shanghai" },
      nextScheduledFor: "2026-07-25T01:00:00Z",
      status: "active" as const,
      createdAt: memo.createdAt,
      updatedAt: memo.updatedAt,
    };
    const props = renderEditor({ memo: { ...memo, reminder } });
    expect(screen.getByLabelText("当前提醒")).toHaveTextContent("每天 09:00");

    const editReminder = screen.getByRole("button", { name: "修改提醒" });
    editReminder.focus();
    fireEvent.click(editReminder);
    expect(screen.getByRole("dialog", { name: "备忘录提醒" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "取消" }));
    expect(editReminder).toHaveFocus();
    fireEvent.click(screen.getByRole("button", { name: "取消提醒" }));
    expect(props.onSave).toHaveBeenCalledWith(expect.objectContaining({ reminder: null }));
  });

  it("keeps the editor keyboard order aligned with the visible workflow", () => {
    const { container } = renderEditor();
    const controls = Array.from(container.querySelectorAll<HTMLElement>("input, textarea, button"));
    const indexOf = (element: HTMLElement) => controls.indexOf(element);
    const title = screen.getByRole("textbox", { name: "标题" });
    const pin = screen.getByRole("button", { name: "置顶" });
    const tags = screen.getByRole("textbox", { name: "添加备忘录标签" });
    const body = screen.getByRole("textbox", { name: "正文" });
    const reminder = screen.getByRole("button", { name: "设置提醒" });
    const remove = screen.getByRole("button", { name: "删除备忘录" });
    const save = screen.getByRole("button", { name: "保存备忘录" });

    expect([title, pin, tags, body, reminder, remove, save].every((element) => indexOf(element) >= 0)).toBe(true);
    expect(indexOf(title)).toBeLessThan(indexOf(pin));
    expect(indexOf(pin)).toBeLessThan(indexOf(tags));
    expect(indexOf(tags)).toBeLessThan(indexOf(body));
    expect(indexOf(body)).toBeLessThan(indexOf(reminder));
    expect(indexOf(reminder)).toBeLessThan(indexOf(remove));
    expect(indexOf(remove)).toBeLessThan(indexOf(save));
  });

  it("saves title and plain text body explicitly and with Ctrl+Enter", () => {
    const props = renderEditor({ memo: null });
    fireEvent.change(screen.getByRole("textbox", { name: "标题" }), { target: { value: "新想法" } });
    fireEvent.change(screen.getByRole("textbox", { name: "正文" }), { target: { value: "保留纯文本内容" } });
    fireEvent.click(screen.getByRole("button", { name: "保存备忘录" }));
    expect(props.onSave).toHaveBeenLastCalledWith(expect.objectContaining({ title: "新想法", body: "保留纯文本内容" }));

    fireEvent.keyDown(screen.getByRole("textbox", { name: "正文" }), { key: "Enter", ctrlKey: true });
    expect(props.onSave).toHaveBeenCalledTimes(2);
  });

  it("limits title and body by Unicode characters", () => {
    renderEditor({ memo: null });
    const title = screen.getByRole("textbox", { name: "标题" });
    const body = screen.getByRole("textbox", { name: "正文" });

    fireEvent.change(title, { target: { value: `${"想".repeat(199)}💡超出` } });
    fireEvent.change(body, { target: { value: `${"文".repeat(19_999)}💡超出` } });

    expect(title).toHaveValue(`${"想".repeat(199)}💡`);
    expect(body).toHaveValue(`${"文".repeat(19_999)}💡`);
    expect(screen.getByText("200/200 个字符")).toBeInTheDocument();
    expect(screen.getByText("20000/20000 个字符")).toBeInTheDocument();
  });

  it("auto-saves the latest draft after 500ms while a request is in progress", () => {
    vi.useFakeTimers();
    const props = renderEditor({ saving: true });
    const title = screen.getByRole("textbox", { name: "标题" });

    fireEvent.change(title, { target: { value: "第一版" } });
    act(() => vi.advanceTimersByTime(499));
    expect(props.onSave).not.toHaveBeenCalled();
    act(() => vi.advanceTimersByTime(1));
    expect(props.onSave).toHaveBeenLastCalledWith(expect.objectContaining({ title: "第一版" }));

    fireEvent.change(title, { target: { value: "最终版" } });
    act(() => vi.advanceTimersByTime(500));
    expect(props.onSave).toHaveBeenLastCalledWith(expect.objectContaining({ title: "最终版" }));
    expect(title).toHaveValue("最终版");
  });
});

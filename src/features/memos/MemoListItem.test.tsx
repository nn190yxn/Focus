// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";

import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { I18nProvider, useI18n } from "../../i18n/I18nContext";
import { MemoListItem, memoReminderSummary } from "./MemoListItem";
import type { MemoReminderSchedule, MemoSummary } from "./types";

const memo: MemoSummary = {
  id: "memo-1",
  displayTitle: "项目复盘",
  bodyPreview: "整理本周的进展、阻塞和下一步行动。",
  tags: [
    { id: "tag-1", name: "工作" },
    { id: "tag-2", name: "复盘" },
    { id: "tag-3", name: "计划" },
    { id: "tag-4", name: "重要" },
  ],
  pinnedAt: "2026-07-24T07:00:00Z",
  reminder: {
    id: "reminder-1",
    memoId: "memo-1",
    schedule: {
      kind: "recurring",
      frequency: "weekly",
      interval: 1,
      weekdays: [1, 5],
      monthlyDay: null,
      localTime: "09:00",
      startsOn: "2026-07-20",
      endsOn: null,
      timezone: "Asia/Shanghai",
    },
    nextScheduledFor: "2026-07-27T01:00:00Z",
    status: "active",
    createdAt: "2026-07-24T07:00:00Z",
    updatedAt: "2026-07-24T07:00:00Z",
  },
  updatedAt: "2026-07-24T08:00:00Z",
};

describe("MemoListItem", () => {
  it("shows title, two-line content source, three tags, pin, reminder, and update metadata", () => {
    const onSelect = vi.fn();
    const { container } = render(
      <I18nProvider locale="zh-CN">
        <MemoListItem memo={memo} selected onSelect={onSelect} now={new Date("2026-07-24T08:00:00Z")} />
      </I18nProvider>,
    );

    const item = screen.getByRole("button", { name: "打开备忘录：项目复盘" });
    expect(item).toHaveAttribute("aria-current", "true");
    expect(item).toHaveTextContent("置顶");
    expect(item).toHaveTextContent("整理本周的进展、阻塞和下一步行动。");
    expect(screen.getByLabelText("备忘录标签").children).toHaveLength(4);
    expect(item).toHaveTextContent("另有 1 个");
    expect(item).toHaveTextContent(/每周.*周一.*周五.*09:00/);
    expect(container.querySelector(".memo-list-item__preview")).toHaveTextContent("整理本周的进展");

    fireEvent.click(item);
    expect(onSelect).toHaveBeenCalledWith("memo-1");
  });

  it("uses explicit completed reminder text", () => {
    render(
      <I18nProvider locale="zh-CN">
        <MemoListItem memo={{ ...memo, reminder: { ...memo.reminder!, status: "completed", nextScheduledFor: null } }} selected={false} onSelect={() => undefined} />
      </I18nProvider>,
    );

    expect(screen.getByRole("button", { name: "打开备忘录：项目复盘" })).toHaveTextContent("已提醒");
  });

  it.each([
    [{ kind: "once", scheduledLocal: "2026-07-24T09:30:00", timezone: "UTC" }, /今天.*09:30/],
    [{ kind: "recurring", frequency: "daily", interval: 2, weekdays: [], monthlyDay: null, localTime: "08:00", startsOn: "2026-07-20", endsOn: null, timezone: "UTC" }, "每 2 天 08:00"],
    [{ kind: "recurring", frequency: "weekdays", interval: 1, weekdays: [], monthlyDay: null, localTime: "09:00", startsOn: "2026-07-20", endsOn: null, timezone: "UTC" }, "工作日 09:00"],
    [{ kind: "recurring", frequency: "monthly", interval: 1, weekdays: [], monthlyDay: 15, localTime: "10:00", startsOn: "2026-07-20", endsOn: null, timezone: "UTC" }, "每月 15 日 10:00"],
  ] as [MemoReminderSchedule, string | RegExp][])("formats reminder schedule %#", (schedule, expected) => {
    let summary = "";
    function SummaryProbe() {
      const i18n = useI18n();
      summary = memoReminderSummary(
        { ...memo.reminder!, schedule, nextScheduledFor: schedule.kind === "once" ? `${schedule.scheduledLocal}Z` : memo.reminder!.nextScheduledFor },
        new Date("2026-07-24T08:00:00Z"),
        i18n,
      );
      return null;
    }

    render(<I18nProvider locale="zh-CN"><SummaryProbe /></I18nProvider>);
    expect(summary).toMatch(expected);
  });
});

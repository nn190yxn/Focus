// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { I18nProvider } from "../../i18n/I18nContext";
import { MemoReminderEditor } from "./MemoReminderEditor";

function renderEditor(schedule: React.ComponentProps<typeof MemoReminderEditor>["schedule"] = null) {
  const onSave = vi.fn();
  render(
    <I18nProvider locale="zh-CN">
      <MemoReminderEditor open schedule={schedule} saving={false} now={new Date("2026-07-24T08:00:00Z")} onClose={vi.fn()} onSave={onSave} />
    </I18nProvider>,
  );
  return onSave;
}

describe("MemoReminderEditor", () => {
  it("switches from a one-time reminder to progressive recurring fields", () => {
    renderEditor();
    expect(screen.getByLabelText("本地日期和时间")).toHaveAttribute("type", "datetime-local");

    fireEvent.click(screen.getByRole("radio", { name: "重复提醒" }));
    expect(screen.getByLabelText("重复频率")).toHaveValue("daily");
    expect(screen.queryByRole("group", { name: "执行星期" })).not.toBeInTheDocument();
    expect(screen.queryByLabelText("每月日期")).not.toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("重复频率"), { target: { value: "weekly" } });
    expect(screen.getByRole("group", { name: "执行星期" })).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("重复频率"), { target: { value: "monthly" } });
    expect(screen.getByLabelText("每月日期")).toBeInTheDocument();
  });

  it("emits a recurring schedule with local fields and IANA timezone", () => {
    const onSave = renderEditor();
    fireEvent.click(screen.getByRole("radio", { name: "重复提醒" }));
    fireEvent.change(screen.getByLabelText("重复频率"), { target: { value: "monthly" } });
    fireEvent.change(screen.getByLabelText("间隔"), { target: { value: "2" } });
    fireEvent.change(screen.getByLabelText("每月日期"), { target: { value: "31" } });
    fireEvent.change(screen.getByLabelText("本地执行时间"), { target: { value: "09:30" } });
    fireEvent.change(screen.getByLabelText("开始日期"), { target: { value: "2026-07-25" } });
    fireEvent.change(screen.getByLabelText("结束日期（可选）"), { target: { value: "2026-12-31" } });
    fireEvent.change(screen.getByLabelText("IANA 时区"), { target: { value: "Asia/Shanghai" } });
    fireEvent.click(screen.getByRole("button", { name: "保存提醒" }));

    expect(onSave).toHaveBeenCalledWith(expect.objectContaining({
      kind: "recurring",
      frequency: "monthly",
      interval: 2,
      monthlyDay: 31,
      localTime: "09:30",
      startsOn: "2026-07-25",
      endsOn: "2026-12-31",
      timezone: "Asia/Shanghai",
    }));
  });

  it("rejects a past one-time reminder and focuses its field", async () => {
    const onSave = renderEditor();
    const dateTime = screen.getByLabelText("本地日期和时间");
    fireEvent.change(dateTime, { target: { value: "2026-07-24T07:59" } });
    fireEvent.change(screen.getByLabelText("IANA 时区"), { target: { value: "UTC" } });
    fireEvent.click(screen.getByRole("button", { name: "保存提醒" }));

    expect(onSave).not.toHaveBeenCalled();
    expect(screen.getByRole("alert")).toHaveTextContent("一次提醒时间必须晚于当前时间");
    await waitFor(() => expect(dateTime).toHaveFocus());
  });

  it.each(["daily", "weekdays", "weekly", "monthly"] as const)("saves the %s frequency with only its relevant fields", (frequency) => {
    const onSave = renderEditor();
    fireEvent.click(screen.getByRole("radio", { name: "重复提醒" }));
    fireEvent.change(screen.getByLabelText("重复频率"), { target: { value: frequency } });
    if (frequency === "monthly") fireEvent.change(screen.getByLabelText("每月日期"), { target: { value: "15" } });
    fireEvent.change(screen.getByLabelText("本地执行时间"), { target: { value: "09:30" } });
    fireEvent.change(screen.getByLabelText("开始日期"), { target: { value: "2026-07-25" } });
    fireEvent.change(screen.getByLabelText("IANA 时区"), { target: { value: "UTC" } });
    fireEvent.click(screen.getByRole("button", { name: "保存提醒" }));

    expect(onSave).toHaveBeenCalledWith(expect.objectContaining({
      frequency,
      weekdays: frequency === "weekly" ? [1] : [],
      monthlyDay: frequency === "monthly" ? 15 : null,
    }));
  });

  it("loads an existing schedule for modification", () => {
    renderEditor({
      kind: "recurring",
      frequency: "weekly",
      interval: 2,
      weekdays: [1, 5],
      monthlyDay: null,
      localTime: "10:15",
      startsOn: "2026-07-25",
      endsOn: null,
      timezone: "Asia/Shanghai",
    });

    expect(screen.getByRole("radio", { name: "重复提醒" })).toHaveAttribute("aria-checked", "true");
    expect(screen.getByLabelText("重复频率")).toHaveValue("weekly");
    expect(screen.getByLabelText("本地执行时间")).toHaveValue("10:15");
    expect(screen.getAllByRole("checkbox").filter((input) => (input as HTMLInputElement).checked)).toHaveLength(2);
  });
});

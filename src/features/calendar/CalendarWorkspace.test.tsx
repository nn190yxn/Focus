// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";

import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it } from "vitest";

import { CalendarWorkspace } from "./CalendarWorkspace";

function CalendarHarness() {
  const [selectedDate, setSelectedDate] = useState("2026-07-20");
  return <CalendarWorkspace selectedDate={selectedDate} onSelectDate={setSelectedDate} runtime={false} />;
}

describe("CalendarWorkspace", () => {
  it("switches between month, week, and year views", () => {
    render(<CalendarHarness />);

    expect(screen.getByRole("heading", { name: "2026 年 7 月" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "这一段时间的投入" })).toBeInTheDocument();
    expect(screen.getByText("50", { selector: ".statistics-metrics strong" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("radio", { name: "周" }));
    expect(screen.getByRole("heading", { name: "7 月 20 日 - 7 月 26 日" })).toBeInTheDocument();
    expect(screen.getAllByRole("button", { name: /项计划/ })).toHaveLength(7);

    fireEvent.click(screen.getByRole("radio", { name: "年" }));
    expect(screen.getByRole("heading", { name: "2026 年" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "12 月" })).toBeInTheDocument();
  });

  it("filters preview activity by category and project", () => {
    render(<CalendarHarness />);

    expect(screen.getByText("整理本周实现节奏")).toBeInTheDocument();
    fireEvent.change(screen.getByRole("combobox", { name: "任务分类" }), { target: { value: "study" } });
    expect(screen.queryByText("整理本周实现节奏")).not.toBeInTheDocument();
    expect(screen.getByText("完成每日复盘")).toBeInTheDocument();

    fireEvent.change(screen.getByRole("combobox", { name: "所属项目" }), { target: { value: "focus" } });
    expect(screen.queryByText("完成每日复盘")).not.toBeInTheDocument();
    expect(screen.getByText("这一天还没有完成记录。")).toBeInTheDocument();
  });

  it("navigates to the next month and updates the selected date", () => {
    render(<CalendarHarness />);
    fireEvent.click(screen.getByRole("button", { name: "下一周期" }));

    expect(screen.getByRole("heading", { name: "2026 年 8 月" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "8 月 1 日 · 星期六" })).toBeInTheDocument();
  });
});

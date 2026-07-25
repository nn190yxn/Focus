// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";

import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it } from "vitest";

import { Button, Dialog, SegmentedControl } from "./ui";

function DialogHarness() {
  const [open, setOpen] = useState(false);
  return (
    <>
      <Button onClick={() => setOpen(true)}>打开设置</Button>
      <Dialog open={open} title="键盘设置" onClose={() => setOpen(false)}>
        <Button>第一个操作</Button>
        <Button>最后一个操作</Button>
      </Dialog>
    </>
  );
}

describe("Dialog", () => {
  it("manages initial focus, traps Tab, closes with Escape, and restores focus", () => {
    render(<DialogHarness />);
    const trigger = screen.getByRole("button", { name: "打开设置" });

    trigger.focus();
    fireEvent.click(trigger);

    const close = screen.getByRole("button", { name: "关闭对话框" });
    const last = screen.getByRole("button", { name: "最后一个操作" });
    expect(screen.getByRole("dialog", { name: "键盘设置" })).toBeInTheDocument();
    expect(close).toHaveFocus();

    close.focus();
    fireEvent.keyDown(close, { key: "Tab", shiftKey: true });
    expect(last).toHaveFocus();
    fireEvent.keyDown(last, { key: "Tab" });
    expect(close).toHaveFocus();

    fireEvent.keyDown(close, { key: "Escape" });
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(trigger).toHaveFocus();
  });

  it("uses a unique accessible title for each open dialog", () => {
    render(
      <>
        <Dialog open title="第一个对话框" onClose={() => undefined}><span>内容一</span></Dialog>
        <Dialog open title="第二个对话框" onClose={() => undefined}><span>内容二</span></Dialog>
      </>,
    );

    const dialogs = screen.getAllByRole("dialog");
    expect(dialogs[0]).toHaveAccessibleName("第一个对话框");
    expect(dialogs[1]).toHaveAccessibleName("第二个对话框");
    expect(dialogs[0].getAttribute("aria-labelledby")).not.toBe(dialogs[1].getAttribute("aria-labelledby"));
  });

  it("prioritizes explicit autofocus over earlier keyboard controls", () => {
    function AutofocusHarness() {
      const [open, setOpen] = useState(false);
      return (
        <>
          <Button onClick={() => setOpen(true)}>新建项目</Button>
          <Dialog open={open} title="新建项目" onClose={() => setOpen(false)}>
            <input aria-label="项目名称" autoFocus />
            <Button disabled>不可用操作</Button>
          </Dialog>
        </>
      );
    }

    render(<AutofocusHarness />);
    const trigger = screen.getByRole("button", { name: "新建项目" });
    trigger.focus();
    fireEvent.click(trigger);

    expect(screen.getByRole("textbox", { name: "项目名称" })).toHaveFocus();
    expect(screen.getByRole("button", { name: "不可用操作" })).toBeDisabled();
    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });
    expect(trigger).toHaveFocus();
  });
});

describe("SegmentedControl", () => {
  it("uses roving focus and changes selection with arrow, Home, and End keys", () => {
    function Harness() {
      const [value, setValue] = useState("today");
      return <SegmentedControl label="日期范围" options={[{ value: "today", label: "今天" }, { value: "week", label: "本周" }, { value: "month", label: "本月" }]} value={value} onChange={setValue} />;
    }

    render(<Harness />);
    const today = screen.getByRole("radio", { name: "今天" });
    const week = screen.getByRole("radio", { name: "本周" });
    const month = screen.getByRole("radio", { name: "本月" });

    expect(today).toHaveAttribute("tabindex", "0");
    expect(week).toHaveAttribute("tabindex", "-1");
    today.focus();
    fireEvent.keyDown(today, { key: "ArrowRight" });
    expect(week).toHaveFocus();
    expect(week).toHaveAttribute("aria-checked", "true");

    fireEvent.keyDown(week, { key: "End" });
    expect(month).toHaveFocus();
    expect(month).toHaveAttribute("aria-checked", "true");
    fireEvent.keyDown(month, { key: "Home" });
    expect(today).toHaveFocus();
    expect(today).toHaveAttribute("aria-checked", "true");
  });
});

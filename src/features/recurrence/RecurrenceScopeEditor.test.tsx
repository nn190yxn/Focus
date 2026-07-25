// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { RecurrenceScopeEditor } from "./RecurrenceScopeEditor";
import type { RecurrenceRule } from "./types";

const rule: RecurrenceRule = { id: "rule-1", taskTemplateId: "task-1", pattern: { kind: "daily", interval: 1 }, localTime: "09:00", timezone: "Asia/Shanghai", startsOn: "2026-07-18", endsOn: null, status: "active", version: 2 };

describe("RecurrenceScopeEditor", () => {
  it("applies one-instance and future scopes with the expected version", async () => {
    const onSubmit = vi.fn();
    render(<RecurrenceScopeEditor instanceId="instance-1" effectiveOn="2026-07-20" rule={rule} onCancel={() => undefined} onSubmit={onSubmit} onSetStatus={() => undefined} />);

    fireEvent.change(screen.getByLabelText("重复执行时间"), { target: { value: "10:30" } });
    fireEvent.click(screen.getByRole("button", { name: "应用修改" }));
    await waitFor(() => expect(onSubmit).toHaveBeenLastCalledWith(expect.objectContaining({ localTime: "10:30", version: 2 }), { scope: "thisInstance", instanceId: "instance-1" }));

    fireEvent.click(screen.getByRole("radio", { name: /修改未来计划/ }));
    fireEvent.click(screen.getByRole("button", { name: "应用修改" }));
    await waitFor(() => expect(onSubmit).toHaveBeenLastCalledWith(expect.objectContaining({ version: 3 }), { scope: "future", effectiveOn: "2026-07-20" }));
  });

  it("exposes pause and end rule controls", () => {
    const onSetStatus = vi.fn();
    render(<RecurrenceScopeEditor instanceId="instance-1" effectiveOn="2026-07-20" rule={rule} onCancel={() => undefined} onSubmit={() => undefined} onSetStatus={onSetStatus} />);

    fireEvent.click(screen.getByRole("button", { name: "暂停规则" }));
    fireEvent.click(screen.getByRole("button", { name: "结束规则" }));
    expect(onSetStatus).toHaveBeenNthCalledWith(1, "paused");
    expect(onSetStatus).toHaveBeenNthCalledWith(2, "ended");
  });
});

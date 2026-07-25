import { beforeEach, describe, expect, it, vi } from "vitest";

const invokeCommand = vi.hoisted(() => vi.fn());

vi.mock("../../lib/commandClient", () => ({ invokeCommand }));

import { projectClient } from "./projectClient";

describe("projectClient", () => {
  beforeEach(() => invokeCommand.mockResolvedValue({ ok: true, data: null, version: 1 }));

  it("maps project reads and writes to typed Tauri commands", async () => {
    const input = { name: "Alpha", description: "Plan", color: "#4eaa98", icon: "AL", startedOn: "2026-07-21", targetOn: null };

    await projectClient.list("active", "2026-07-21");
    await projectClient.get("project-1", "2026-07-21");
    await projectClient.create(input);
    await projectClient.update("project-1", input);
    await projectClient.setStatus("project-1", "archived");

    expect(invokeCommand.mock.calls).toEqual([
      ["project_list", { status: "active", today: "2026-07-21" }],
      ["project_get", { id: "project-1", today: "2026-07-21" }],
      ["project_create", { input }],
      ["project_update", { id: "project-1", input }],
      ["project_set_status", { id: "project-1", status: "archived" }],
    ]);
  });
});

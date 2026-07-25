import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());

vi.mock("@tauri-apps/api/core", () => ({ invoke }));

import { invokeCommand } from "./commandClient";

describe("invokeCommand", () => {
  beforeEach(() => invoke.mockReset());

  it("returns successful command results unchanged", async () => {
    const response = { ok: true as const, data: "ready", version: 1 };
    invoke.mockResolvedValueOnce(response);

    await expect(invokeCommand("health")).resolves.toBe(response);
    expect(invoke).toHaveBeenCalledWith("health", undefined);
  });

  it("records the failed command and preserves a stable public error", async () => {
    invoke
      .mockRejectedValueOnce(new Error("command settings_get not found"))
      .mockResolvedValueOnce(undefined);

    await expect(invokeCommand("settings_get")).resolves.toEqual({
      ok: false,
      error: {
        code: "COMMAND_INVOCATION_FAILED",
        message: "command invocation failed: settings_get",
        field: "settings_get",
      },
    });
    expect(invoke).toHaveBeenNthCalledWith(2, "diagnostic_command_failure", {
      command: "settings_get",
      error: "command settings_get not found",
    });
  });

  it("still returns a stable error when diagnostic logging fails", async () => {
    invoke
      .mockRejectedValueOnce(new Error("IPC unavailable"))
      .mockRejectedValueOnce(new Error("diagnostic IPC unavailable"));

    const result = await invokeCommand("health");

    expect(result).toMatchObject({ ok: false, error: { field: "health" } });
    expect(invoke).toHaveBeenCalledTimes(2);
  });
});

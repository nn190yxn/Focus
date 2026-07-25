// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { NotificationClient } from "./notificationClient";
import { NotificationSettingsPanel } from "./NotificationSettingsPanel";

function desktopRuntime() {
  Object.defineProperty(window, "__TAURI_INTERNALS__", {
    configurable: true,
    value: {},
  });
}

describe("NotificationSettingsPanel", () => {
  it("loads settings and saves the sound preference", async () => {
    desktopRuntime();
    const updatePreferences = vi.fn(async (preferences) => ({ ok: true as const, data: preferences, version: 1 }));
    const client: NotificationClient = {
      getSettings: async () => ({ ok: true, data: { preferences: { notificationsEnabled: true, soundEnabled: true }, permissionState: "granted" }, version: 1 }),
      updatePreferences,
      openSystemSettings: async () => ({ ok: true, data: null, version: 1 }),
    };

    render(<NotificationSettingsPanel client={client} />);
    const sound = await screen.findByRole("checkbox", { name: "启用通知提示音" });
    fireEvent.click(sound);

    await waitFor(() => expect(updatePreferences).toHaveBeenCalledWith({ notificationsEnabled: true, soundEnabled: false }));
    expect(sound).not.toBeChecked();
    expect(screen.getByText("可发送")).toBeInTheDocument();
  });

  it("keeps the previous value when saving fails", async () => {
    desktopRuntime();
    const client: NotificationClient = {
      getSettings: async () => ({ ok: true, data: { preferences: { notificationsEnabled: true, soundEnabled: true }, permissionState: "denied" }, version: 1 }),
      updatePreferences: async () => ({ ok: false, error: { code: "DATABASE_ERROR", message: "C:\\Private\\focus.db 包含机密任务标题" } }),
      openSystemSettings: async () => ({ ok: true, data: null, version: 1 }),
    };

    render(<NotificationSettingsPanel client={client} />);
    const notifications = await screen.findByRole("checkbox", { name: "启用系统通知" });
    fireEvent.click(notifications);

    await screen.findByRole("alert");
    expect(notifications).toBeChecked();
    expect(screen.getByText("本地数据暂时不可用，请稍后重试。")).toBeInTheDocument();
    expect(screen.queryByText(/机密任务标题/)).not.toBeInTheDocument();
  });
});

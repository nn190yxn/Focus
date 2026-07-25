// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";

import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { I18nProvider } from "../../i18n/I18nContext";
import { MemoReminderPermissionNotice } from "./MemoReminderPermissionNotice";

describe("MemoReminderPermissionNotice", () => {
  it("explains denied permission and opens Windows settings", async () => {
    const openSystemSettings = vi.fn(async () => ({ ok: true as const, data: null, version: 1 }));
    const client = {
      getSettings: vi.fn(async () => ({ ok: true as const, data: { preferences: { notificationsEnabled: true, soundEnabled: true }, permissionState: "denied" as const }, version: 1 })),
      updatePreferences: vi.fn(),
      openSystemSettings,
    };
    render(<I18nProvider locale="zh-CN"><MemoReminderPermissionNotice active runtime client={client} /></I18nProvider>);

    expect(await screen.findByRole("status")).toHaveTextContent("提醒已保存；请在 Windows 设置中启用系统通知");
    fireEvent.click(screen.getByRole("button", { name: "打开 Windows 通知设置" }));
    expect(openSystemSettings).toHaveBeenCalledTimes(1);
  });
});

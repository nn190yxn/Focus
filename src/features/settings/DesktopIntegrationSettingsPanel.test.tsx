// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { DesktopIntegrationClient } from "./desktopIntegrationClient";
import { DesktopIntegrationSettingsPanel } from "./DesktopIntegrationSettingsPanel";
import type { DesktopIntegrationSettings } from "./types";

const settings: DesktopIntegrationSettings = {
  shortcuts: {
    enabled: true,
    bindings: {
      showMainWindow: "Ctrl+Alt+A",
      toggleFocus: "Ctrl+Alt+Space",
      createQuickTask: "Ctrl+Alt+N",
      unlockWidget: "Ctrl+Alt+U",
    },
  },
  autostartEnabled: false,
  shortcutError: null,
};

function desktopRuntime() {
  Object.defineProperty(window, "__TAURI_INTERNALS__", {
    configurable: true,
    value: {},
  });
}

afterEach(() => {
  Reflect.deleteProperty(window, "__TAURI_INTERNALS__");
});

describe("DesktopIntegrationSettingsPanel", () => {
  it("synchronizes autostart before showing the enabled state", async () => {
    desktopRuntime();
    const setAutostart = vi.fn(async () => ({ ok: true as const, data: true, version: 1 }));
    const client: DesktopIntegrationClient = {
      getSettings: async () => ({ ok: true, data: settings, version: 1 }),
      updateShortcuts: async (shortcuts) => ({ ok: true, data: shortcuts, version: 1 }),
      setAutostart,
    };

    render(<DesktopIntegrationSettingsPanel client={client} />);
    const autostart = await screen.findByRole("checkbox", { name: "启用开机自动启动" });
    fireEvent.click(autostart);

    await waitFor(() => expect(setAutostart).toHaveBeenCalledWith(true));
    expect(autostart).toBeChecked();
  });

  it("restores active shortcut values when registration conflicts", async () => {
    desktopRuntime();
    const client: DesktopIntegrationClient = {
      getSettings: async () => ({ ok: true, data: settings, version: 1 }),
      updateShortcuts: async () => ({ ok: false, error: { code: "SHORTCUT_CONFLICT", message: "快捷键与机密任务标题冲突", field: "toggleFocus" } }),
      setAutostart: async (enabled) => ({ ok: true, data: enabled, version: 1 }),
    };

    render(<DesktopIntegrationSettingsPanel client={client} />);
    const toggle = await screen.findByRole("textbox", { name: "开始或暂停专注快捷键" });
    fireEvent.change(toggle, { target: { value: "Ctrl+Shift+Space" } });
    fireEvent.click(screen.getByRole("button", { name: "保存快捷键" }));

    await screen.findByRole("alert");
    expect(toggle).toHaveValue("Ctrl+Alt+Space");
    expect(screen.getByText("快捷键已被其他应用占用，请更换组合。")).toBeInTheDocument();
    expect(screen.queryByText(/机密任务标题/)).not.toBeInTheDocument();
  });
});

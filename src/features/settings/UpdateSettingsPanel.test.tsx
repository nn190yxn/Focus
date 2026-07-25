// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { UpdateSettingsPanel } from "./UpdateSettingsPanel";
import type { UpdateClient, UpdateMetadata } from "./updateClient";

const availableUpdate: UpdateMetadata = {
  currentVersion: "0.1.0",
  version: "0.2.0",
  notes: "提升窗口恢复稳定性。",
  publishedAt: 1_784_592_000,
};

function desktopRuntime() {
  Object.defineProperty(window, "__TAURI_INTERNALS__", {
    configurable: true,
    value: {},
  });
}

function createClient(overrides: Partial<UpdateClient> = {}): UpdateClient {
  return {
    check: async () => ({ ok: true, data: availableUpdate, version: 1 }),
    download: async (onProgress) => {
      onProgress({ downloaded: 50, contentLength: 100 });
      return { ok: true, data: { downloaded: 100, contentLength: 100 }, version: 1 };
    },
    install: async () => ({ ok: true, data: undefined, version: 1 }),
    ...overrides,
  };
}

describe("UpdateSettingsPanel", () => {
  beforeEach(() => {
    Reflect.deleteProperty(window, "__TAURI_INTERNALS__");
  });

  it("keeps update controls unavailable in the browser preview", () => {
    const check = vi.fn(createClient().check);
    render(<UpdateSettingsPanel client={createClient({ check })} />);

    expect(screen.getByText("更新检查仅在 Windows 桌面应用中可用。")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "检查更新" })).toBeDisabled();
    expect(check).not.toHaveBeenCalled();
  });

  it("shows release details when a signed update is available", async () => {
    desktopRuntime();
    render(<UpdateSettingsPanel client={createClient()} />);

    expect(await screen.findByText("新版本 0.2.0")).toBeInTheDocument();
    expect(screen.getByText("提升窗口恢复稳定性。")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "确认下载" })).toBeEnabled();
  });

  it("downloads first and waits for installation confirmation", async () => {
    desktopRuntime();
    const install = vi.fn(createClient().install);
    render(<UpdateSettingsPanel client={createClient({ install })} />);

    fireEvent.click(await screen.findByRole("button", { name: "确认下载" }));
    expect(await screen.findByRole("dialog", { name: "确认安装更新" })).toBeInTheDocument();
    expect(screen.getByText("更新包已通过签名验证，可以安装。")).toBeInTheDocument();
    expect(install).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "取消" }));
    expect(screen.queryByRole("dialog", { name: "确认安装更新" })).not.toBeInTheDocument();
    expect(install).not.toHaveBeenCalled();
  });

  it("starts installation only after the user confirms", async () => {
    desktopRuntime();
    const install = vi.fn(createClient().install);
    render(<UpdateSettingsPanel client={createClient({ install })} />);

    fireEvent.click(await screen.findByRole("button", { name: "确认下载" }));
    fireEvent.click(await screen.findByRole("button", { name: "保存状态并安装" }));

    await waitFor(() => expect(install).toHaveBeenCalledTimes(1));
  });

  it("keeps the current version usable when update checking fails", async () => {
    desktopRuntime();
    render(<UpdateSettingsPanel client={createClient({
      check: async () => ({
        ok: false,
        error: { code: "UPDATE_CHECK_FAILED", message: "https://private.example/release failed" },
      }),
    })} />);

    expect(await screen.findByRole("alert")).toHaveTextContent("更新检查失败，当前版本可以继续使用。");
    expect(screen.queryByText(/private\.example/)).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "检查更新" })).toBeEnabled();
  });
});

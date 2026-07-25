// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { BackupClient, BackupInspection } from "./backupClient";
import { DataSettingsPanel } from "./DataSettingsPanel";

const inspection: BackupInspection = {
  token: "restore-token",
  path: "C:\\Backups\\arrive-focus.json",
  formatVersion: 1,
  exportedAt: "2026-07-21T09:00:00.000Z",
  summary: {
    counts: {
      projects: 1,
      tasks: 2,
      checkItems: 1,
      recurrenceRules: 1,
      taskInstances: 3,
      focusSessions: 4,
      activeFocus: 0,
      notes: 1,
      weeklyGoals: 1,
      preferences: 2,
      memos: 0,
      memoTags: 0,
      memoTagLinks: 0,
      memoReminders: 0,
      total: 16,
    },
    earliestDate: "2026-07-01",
    latestDate: "2026-07-21",
  },
};

function desktopRuntime() {
  Object.defineProperty(window, "__TAURI_INTERNALS__", {
    configurable: true,
    value: {},
  });
}

function createClient(overrides: Partial<BackupClient> = {}): BackupClient {
  return {
    exportBackup: async () => ({ ok: true, data: null, version: 1 }),
    inspectBackup: async () => ({ ok: true, data: inspection, version: 1 }),
    restoreBackup: async () => ({
      ok: true,
      data: {
        sourcePath: inspection.path,
        snapshotPath: "C:\\AppData\\backups\\pre-restore.json",
        summary: inspection.summary,
      },
      version: 1,
    }),
    ...overrides,
  };
}

describe("DataSettingsPanel", () => {
  it("exports after the native save dialog returns a path", async () => {
    desktopRuntime();
    const exportBackup = vi.fn(async () => ({
      ok: true as const,
      data: { path: "C:\\Backups\\arrive-focus.json", summary: inspection.summary },
      version: 1,
    }));
    render(<DataSettingsPanel client={createClient({ exportBackup })} />);

    fireEvent.click(screen.getByRole("button", { name: "导出备份" }));

    await waitFor(() => expect(exportBackup).toHaveBeenCalledTimes(1));
    expect(await screen.findByText("备份已导出。")).toBeInTheDocument();
  });

  it("shows the validated summary and restores only after confirmation", async () => {
    desktopRuntime();
    const restoreBackup = vi.fn(createClient().restoreBackup);
    render(<DataSettingsPanel client={createClient({ restoreBackup })} />);

    fireEvent.click(screen.getByRole("button", { name: "从备份恢复" }));

    expect(await screen.findByRole("dialog", { name: "确认恢复备份" })).toBeInTheDocument();
    expect(screen.getByText("16")).toBeInTheDocument();
    expect(screen.getByText("2026-07-01 – 2026-07-21")).toBeInTheDocument();
    expect(restoreBackup).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "确认替换并恢复" }));

    await waitFor(() => expect(restoreBackup).toHaveBeenCalledWith("restore-token"));
    expect(await screen.findByText("数据恢复完成，恢复前快照已保留。")).toBeInTheDocument();
  });

  it("keeps the confirmation open when the restore transaction fails", async () => {
    desktopRuntime();
    const client = createClient({
      restoreBackup: async () => ({
        ok: false,
        error: { code: "BACKUP_RESTORE_FAILED", message: "恢复 C:\\Private\\tasks.json 失败：机密任务标题" },
      }),
    });
    render(<DataSettingsPanel client={client} />);
    fireEvent.click(screen.getByRole("button", { name: "从备份恢复" }));
    fireEvent.click(await screen.findByRole("button", { name: "确认替换并恢复" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("数据恢复失败，原数据和恢复前快照已保留。");
    expect(screen.queryByText(/机密任务标题/)).not.toBeInTheDocument();
    expect(screen.getByRole("dialog", { name: "确认恢复备份" })).toBeInTheDocument();
  });
});

import { invokeCommand, type CommandResult } from "../../lib/commandClient";

export type BackupRecordCounts = {
  projects: number;
  tasks: number;
  checkItems: number;
  recurrenceRules: number;
  taskInstances: number;
  focusSessions: number;
  activeFocus: number;
  notes: number;
  weeklyGoals: number;
  preferences: number;
  memos: number;
  memoTags: number;
  memoTagLinks: number;
  memoReminders: number;
  total: number;
};

export type BackupImportSummary = {
  counts: BackupRecordCounts;
  earliestDate: string | null;
  latestDate: string | null;
};

export type BackupExportResult = {
  path: string;
  summary: BackupImportSummary;
};

export type BackupInspection = {
  token: string;
  path: string;
  formatVersion: number;
  exportedAt: string;
  summary: BackupImportSummary;
};

export type BackupRestoreResult = {
  sourcePath: string;
  snapshotPath: string;
  summary: BackupImportSummary;
};

export type BackupClient = {
  exportBackup: () => Promise<CommandResult<BackupExportResult | null>>;
  inspectBackup: () => Promise<CommandResult<BackupInspection | null>>;
  restoreBackup: (token: string) => Promise<CommandResult<BackupRestoreResult>>;
};

export const backupClient: BackupClient = {
  exportBackup: () => invokeCommand("backup_export"),
  inspectBackup: () => invokeCommand("backup_inspect"),
  restoreBackup: (token) => invokeCommand("backup_restore", { token }),
};

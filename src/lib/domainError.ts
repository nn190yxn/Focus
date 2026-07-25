import type { MessageKey } from "../i18n/messages";
import type { DomainError } from "./commandClient";

type Translate = (key: MessageKey) => string;

const EXACT_MESSAGE_KEYS: Readonly<Record<string, MessageKey>> = {
  COMMAND_INVOCATION_FAILED: "error.transport",
  TASK_TITLE_INVALID: "error.taskTitle",
  TASK_DATE_IN_PAST: "error.taskDatePast",
  PROJECT_NAME_INVALID: "error.projectName",
  PROJECT_HAS_HISTORY: "error.projectHasHistory",
  NOTE_BODY_INVALID: "error.noteBody",
  MEMO_TITLE_INVALID: "error.memoTitle",
  MEMO_BODY_INVALID: "error.memoBody",
  MEMO_TAG_INVALID: "error.memoTags",
  MEMO_TAG_LIMIT_EXCEEDED: "error.memoTags",
  MEMO_NOT_FOUND: "error.memoNotFound",
  MEMO_SAVE_FAILED: "error.memoSave",
  MEMO_DELETE_FAILED: "error.memoDelete",
  MEMO_REMINDER_TIME_INVALID: "error.memoReminder",
  MEMO_REMINDER_DATE_INVALID: "error.memoReminder",
  MEMO_REMINDER_INTERVAL_INVALID: "error.memoReminder",
  MEMO_REMINDER_WEEKDAYS_INVALID: "error.memoReminder",
  MEMO_REMINDER_MONTHLY_DAY_INVALID: "error.memoReminder",
  MEMO_REMINDER_TIMEZONE_INVALID: "error.memoReminder",
  MEMO_REMINDER_DATA_INVALID: "error.memoReminder",
  FOCUS_ALREADY_ACTIVE: "error.focusAlreadyActive",
  FOCUS_NOT_ACTIVE: "error.focusNotActive",
  FOCUS_PROJECT_PAUSED: "error.focusProjectPaused",
  NOTIFICATION_DENIED: "error.notificationDenied",
  SHORTCUT_INVALID: "error.shortcutInvalid",
  SHORTCUT_DUPLICATE: "error.shortcutInvalid",
  SHORTCUT_CONFLICT: "error.shortcutConflict",
  BACKUP_CONFIRMATION_INVALID: "error.backupConfirmation",
};

export function domainErrorMessage(error: DomainError, t: Translate): string {
  return t(EXACT_MESSAGE_KEYS[error.code] ?? categorizedMessageKey(error.code));
}

function categorizedMessageKey(code: string): MessageKey {
  if (code.endsWith("_NOT_FOUND") || code.endsWith("_MISSING")) return "error.notFound";
  if (code.startsWith("DATABASE_") || code.endsWith("_DATA_INVALID") || code.endsWith("_CORRUPTED")) return "error.storage";
  if (code.startsWith("BACKUP_")) {
    if (code.includes("FORMAT") || code.includes("VERSION") || code.includes("FIELD") || code.includes("REFERENCE") || code.includes("DUPLICATE") || code.includes("RECORD_LIMIT")) return "error.backupInvalid";
    if (code.includes("FILE") || code.includes("PATH")) return "error.backupFile";
    return "error.backupRestore";
  }
  if (code.startsWith("FOCUS_")) return "error.focus";
  if (code.startsWith("RECURRENCE_") || code.startsWith("INSTANCE_")) return "error.recurrence";
  if (code.startsWith("WIDGET_") || code.startsWith("MAIN_WINDOW_") || code.startsWith("TASKBAR_") || code.startsWith("TRAY_")) return "error.desktop";
  if (code.endsWith("_INVALID") || code.includes("DATE_RANGE") || code.includes("REQUIRES_DATE")) return "error.input";
  if (code.endsWith("_UNAVAILABLE") || code.endsWith("_CONFLICT") || code.endsWith("_REMOVED")) return "error.conflict";
  return "error.generic";
}

import { useEffect, useState } from "react";

import { Button, Dialog, SegmentedControl } from "../../components/ui";
import { useI18n } from "../../i18n/I18nContext";
import type { MessageKey } from "../../i18n/messages";
import type { MemoReminderFrequency, MemoReminderSchedule } from "./types";

type ReminderKind = MemoReminderSchedule["kind"];

export type MemoReminderEditorProps = {
  open: boolean;
  schedule: MemoReminderSchedule | null;
  saving: boolean;
  now?: Date;
  onClose: () => void;
  onSave: (schedule: MemoReminderSchedule) => void;
};

const weekdays = [1, 2, 3, 4, 5, 6, 7] as const;

export function MemoReminderEditor({ open, schedule, saving, now = new Date(), onClose, onSave }: MemoReminderEditorProps) {
  const { t, formatDate } = useI18n();
  const [kind, setKind] = useState<ReminderKind>(schedule?.kind ?? "once");
  const [once, setOnce] = useState(() => onceDefaults(schedule));
  const [recurring, setRecurring] = useState(() => recurringDefaults(schedule));
  const [validationError, setValidationError] = useState<MessageKey | null>(null);

  useEffect(() => {
    if (!open) return;
    setKind(schedule?.kind ?? "once");
    setOnce(onceDefaults(schedule));
    setRecurring(recurringDefaults(schedule));
    setValidationError(null);
  }, [open, schedule]);

  function toggleWeekday(day: number) {
    setRecurring((current) => ({
      ...current,
      weekdays: current.weekdays.includes(day)
        ? current.weekdays.filter((candidate) => candidate !== day)
        : [...current.weekdays, day].sort((left, right) => left - right),
    }));
    setValidationError(null);
  }

  function submit() {
    const next = kind === "once" ? { kind: "once" as const, ...once } : recurring;
    const error = validateSchedule(next, now);
    setValidationError(error?.message ?? null);
    if (error) {
      window.requestAnimationFrame(() => document.getElementById(`memo-reminder-${error.field}`)?.focus());
    } else {
      onSave(next);
    }
  }

  function changeFrequency(frequency: MemoReminderFrequency) {
    setRecurring((current) => ({
      ...current,
      frequency,
      interval: frequency === "weekdays" ? 1 : current.interval,
      weekdays: frequency === "weekly" ? (current.weekdays.length > 0 ? current.weekdays : [1]) : [],
      monthlyDay: frequency === "monthly" ? (current.monthlyDay ?? 1) : null,
    }));
    setValidationError(null);
  }

  return (
    <Dialog open={open} title={t("memo.reminderEditorTitle")} onClose={() => !saving && onClose()}>
      <div className="memo-reminder-editor">
        <SegmentedControl
          label={t("memo.reminderType")}
          options={[
            { value: "once", label: t("memo.reminderTypeOnce") },
            { value: "recurring", label: t("memo.reminderTypeRecurring") },
          ]}
          value={kind}
          onChange={(value) => { setKind(value); setValidationError(null); }}
        />
        {kind === "once" ? (
          <div className="memo-reminder-fields">
            <label><span>{t("memo.reminderDateTime")}</span><input id="memo-reminder-scheduledLocal" type="datetime-local" value={once.scheduledLocal} onChange={(event) => setOnce({ ...once, scheduledLocal: event.target.value })} /></label>
            <label><span>{t("memo.reminderTimezone")}</span><input id="memo-reminder-timezone" value={once.timezone} onChange={(event) => setOnce({ ...once, timezone: event.target.value })} /></label>
          </div>
        ) : (
          <div className="memo-reminder-fields">
            <label><span>{t("memo.reminderFrequency")}</span><select value={recurring.frequency} onChange={(event) => changeFrequency(event.target.value as MemoReminderFrequency)}>
              <option value="daily">{t("memo.frequencyDaily")}</option>
              <option value="weekdays">{t("memo.frequencyWeekdays")}</option>
              <option value="weekly">{t("memo.frequencyWeekly")}</option>
              <option value="monthly">{t("memo.frequencyMonthly")}</option>
            </select></label>
            <label><span>{t("memo.reminderInterval")}</span><input id="memo-reminder-interval" type="number" min="1" max="365" disabled={recurring.frequency === "weekdays"} value={recurring.interval} onChange={(event) => setRecurring({ ...recurring, interval: Number(event.target.value) })} /></label>
            {recurring.frequency === "weekly" ? (
              <fieldset id="memo-reminder-weekdays" className="memo-reminder-weekdays" tabIndex={-1}>
                <legend>{t("memo.reminderWeekdayLegend")}</legend>
                {weekdays.map((day) => <label key={day}><input type="checkbox" checked={recurring.weekdays.includes(day)} onChange={() => toggleWeekday(day)} />{weekdayLabel(day, formatDate)}</label>)}
              </fieldset>
            ) : null}
            {recurring.frequency === "monthly" ? <label><span>{t("memo.reminderMonthlyDay")}</span><input id="memo-reminder-monthlyDay" type="number" min="1" max="31" value={recurring.monthlyDay ?? ""} onChange={(event) => setRecurring({ ...recurring, monthlyDay: event.target.value ? Number(event.target.value) : null })} /></label> : null}
            <label><span>{t("memo.reminderTime")}</span><input id="memo-reminder-localTime" type="time" value={recurring.localTime} onChange={(event) => setRecurring({ ...recurring, localTime: event.target.value })} /></label>
            <label><span>{t("memo.reminderStartsOn")}</span><input id="memo-reminder-startsOn" type="date" value={recurring.startsOn} onChange={(event) => setRecurring({ ...recurring, startsOn: event.target.value })} /></label>
            <label><span>{t("memo.reminderEndsOn")}</span><input id="memo-reminder-endsOn" type="date" value={recurring.endsOn ?? ""} onChange={(event) => setRecurring({ ...recurring, endsOn: event.target.value || null })} /></label>
            <label><span>{t("memo.reminderTimezone")}</span><input id="memo-reminder-timezone" value={recurring.timezone} onChange={(event) => setRecurring({ ...recurring, timezone: event.target.value })} /></label>
          </div>
        )}
        {validationError ? <div className="memo-inline-error" role="alert">{t(validationError)}</div> : null}
        <div className="memo-reminder-editor__actions">
          <Button tone="ghost" disabled={saving} onClick={onClose}>{t("common.cancel")}</Button>
          <Button tone="primary" disabled={saving} onClick={submit}>{saving ? t("memo.saving") : t("memo.reminderSave")}</Button>
        </div>
      </div>
    </Dialog>
  );
}

function onceDefaults(schedule: MemoReminderSchedule | null) {
  if (schedule?.kind === "once") return { scheduledLocal: schedule.scheduledLocal, timezone: schedule.timezone };
  const date = new Date(Date.now() + 60 * 60 * 1000);
  return { scheduledLocal: localDateTime(date), timezone: localTimezone() };
}

function recurringDefaults(schedule: MemoReminderSchedule | null): Extract<MemoReminderSchedule, { kind: "recurring" }> {
  if (schedule?.kind === "recurring") return schedule;
  const now = new Date();
  return {
    kind: "recurring",
    frequency: "daily",
    interval: 1,
    weekdays: [],
    monthlyDay: null,
    localTime: `${pad(now.getHours())}:${pad(now.getMinutes())}`,
    startsOn: localDate(now),
    endsOn: null,
    timezone: localTimezone(),
  };
}

function localTimezone(): string {
  return Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC";
}

function localDateTime(date: Date): string {
  return `${localDate(date)}T${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

function localDate(date: Date): string {
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

function pad(value: number): string {
  return String(value).padStart(2, "0");
}

function weekdayLabel(day: number, formatDate: (value: string | Date, options?: Intl.DateTimeFormatOptions) => string): string {
  return formatDate(new Date(Date.UTC(2024, 0, day)), { weekday: "short", timeZone: "UTC" });
}

type ValidationError = { message: MessageKey; field: string };

function validateSchedule(schedule: MemoReminderSchedule, now: Date): ValidationError | null {
  if (!isIanaTimezone(schedule.timezone)) return { message: "memo.reminderValidationTimezone", field: "timezone" };
  if (schedule.kind === "once") {
    if (!/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}/.test(schedule.scheduledLocal)) return { message: "memo.reminderValidationDateTime", field: "scheduledLocal" };
    if (schedule.scheduledLocal.slice(0, 16) <= localDateTimeInTimezone(now, schedule.timezone)) return { message: "memo.reminderValidationFuture", field: "scheduledLocal" };
    return null;
  }
  if (!Number.isInteger(schedule.interval) || schedule.interval < 1 || schedule.interval > 365) return { message: "memo.reminderValidationInterval", field: "interval" };
  if (!/^\d{2}:\d{2}$/.test(schedule.localTime)) return { message: "memo.reminderValidationDateTime", field: "localTime" };
  if (!/^\d{4}-\d{2}-\d{2}$/.test(schedule.startsOn)) return { message: "memo.reminderValidationDateTime", field: "startsOn" };
  if (schedule.endsOn && schedule.endsOn < schedule.startsOn) return { message: "memo.reminderValidationEndDate", field: "endsOn" };
  if (schedule.frequency === "weekly" && schedule.weekdays.length === 0) return { message: "memo.reminderValidationWeekdays", field: "weekdays" };
  if (schedule.frequency === "monthly" && (!schedule.monthlyDay || schedule.monthlyDay < 1 || schedule.monthlyDay > 31)) return { message: "memo.reminderValidationMonthlyDay", field: "monthlyDay" };
  return null;
}

function isIanaTimezone(value: string): boolean {
  try {
    new Intl.DateTimeFormat("en", { timeZone: value }).format();
    return value.trim().length > 0;
  } catch {
    return false;
  }
}

function localDateTimeInTimezone(date: Date, timezone: string): string {
  const parts = new Intl.DateTimeFormat("en-CA", {
    timeZone: timezone,
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    hourCycle: "h23",
  }).formatToParts(date);
  const value = (type: Intl.DateTimeFormatPartTypes) => parts.find((part) => part.type === type)?.value ?? "";
  return `${value("year")}-${value("month")}-${value("day")}T${value("hour")}:${value("minute")}`;
}

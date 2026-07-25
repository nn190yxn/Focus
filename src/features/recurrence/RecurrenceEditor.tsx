import { useState } from "react";

import { Button } from "../../components/ui";
import { createI18n, useI18n, type I18nValue } from "../../i18n/I18nContext";
import { summarizeRecurrence } from "./recurrenceSummary";
import type { RecurrencePattern, RecurrenceRuleInput } from "./types";

type RecurrenceEditorProps = {
  initialValue: RecurrenceRuleInput;
  submitLabel?: string;
  showActions?: boolean;
  onCancel?: () => void;
  onChange?: (value: RecurrenceRuleInput) => void;
  onSubmit?: (value: RecurrenceRuleInput) => void | Promise<void>;
};

export function validateRecurrenceInput(value: RecurrenceRuleInput, t: I18nValue["t"] = createI18n("zh-CN").t): string | null {
  if (!value.startsOn) return t("recurrence.validation.start");
  if (value.endsOn && value.endsOn < value.startsOn) return t("recurrence.validation.end");
  if (value.pattern.kind === "weekly" && value.pattern.weekdays.length === 0) return t("recurrence.validation.weekday");
  if ("interval" in value.pattern && (!Number.isInteger(value.pattern.interval) || value.pattern.interval < 1)) return t("recurrence.validation.interval");
  if (value.pattern.kind === "monthly" && (value.pattern.dayOfMonth < 1 || value.pattern.dayOfMonth > 31)) return t("recurrence.validation.monthDay");
  if (!value.timezone.trim()) return t("recurrence.validation.timezone");
  return null;
}

export function RecurrenceEditor({ initialValue, submitLabel, showActions = true, onCancel, onChange, onSubmit }: RecurrenceEditorProps) {
  const { locale, t } = useI18n();
  const monday = new Date(2026, 6, 20, 12);
  const weekdays = Array.from({ length: 7 }, (_, index) => {
    const date = new Date(monday);
    date.setDate(monday.getDate() + index);
    return new Intl.DateTimeFormat(locale, { weekday: "short" }).format(date);
  });
  const [value, setValue] = useState(initialValue);
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  function update(next: RecurrenceRuleInput) {
    setValue(next);
    setError(null);
    onChange?.(next);
  }

  function changeKind(kind: RecurrencePattern["kind"]) {
    const pattern: RecurrencePattern = kind === "daily" ? { kind, interval: 1 }
      : kind === "weekdays" ? { kind }
        : kind === "weekly" ? { kind, interval: 1, weekdays: [1] }
          : { kind, interval: 1, dayOfMonth: Number(value.startsOn.slice(-2)) || 1 };
    update({ ...value, pattern });
  }

  function changeInterval(interval: number) {
    const pattern = value.pattern;
    if (pattern.kind === "daily" || pattern.kind === "weekly" || pattern.kind === "monthly") update({ ...value, pattern: { ...pattern, interval } });
  }

  function changeDayOfMonth(dayOfMonth: number) {
    const pattern = value.pattern;
    if (pattern.kind === "monthly") update({ ...value, pattern: { ...pattern, dayOfMonth } });
  }

  async function submit() {
    const nextError = validateRecurrenceInput(value, t);
    setError(nextError);
    if (nextError || !onSubmit) return;
    setSubmitting(true);
    try {
      await onSubmit(value);
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="recurrence-editor">
      <div className="recurrence-editor__summary" aria-live="polite"><span>{t("recurrence.summary")}</span><strong>{summarizeRecurrence(value, locale)}</strong></div>
      <div className="task-editor__grid">
        <label className="task-editor__field"><span>{t("recurrence.frequency")}</span><select aria-label={t("recurrence.frequency")} value={value.pattern.kind} onChange={(event) => changeKind(event.target.value as RecurrencePattern["kind"])}><option value="daily">{t("recurrence.daily")}</option><option value="weekdays">{t("recurrence.weekdays")}</option><option value="weekly">{t("recurrence.weekly")}</option><option value="monthly">{t("recurrence.monthly")}</option></select></label>
        {"interval" in value.pattern ? <label className="task-editor__field"><span>{t("recurrence.interval")}</span><input aria-label={t("recurrence.interval")} type="number" min="1" max="365" value={value.pattern.interval} onChange={(event) => changeInterval(Number(event.target.value))} /></label> : <div />}
        {value.pattern.kind === "monthly" ? <label className="task-editor__field"><span>{t("recurrence.monthDay")}</span><input aria-label={t("recurrence.monthDay")} type="number" min="1" max="31" value={value.pattern.dayOfMonth} onChange={(event) => changeDayOfMonth(Number(event.target.value))} /></label> : null}
        <label className="task-editor__field"><span>{t("recurrence.executionTime")}</span><input aria-label={t("recurrence.executionTimeLabel")} type="time" value={value.localTime ?? ""} onChange={(event) => update({ ...value, localTime: event.target.value || null })} /></label>
        <label className="task-editor__field"><span>{t("recurrence.startDate")}</span><input aria-label={t("recurrence.startDateLabel")} type="date" value={value.startsOn} onChange={(event) => update({ ...value, startsOn: event.target.value })} /></label>
        <label className="task-editor__field"><span>{t("recurrence.endDate")}</span><input aria-label={t("recurrence.endDateLabel")} type="date" min={value.startsOn} value={value.endsOn ?? ""} onChange={(event) => update({ ...value, endsOn: event.target.value || null })} /></label>
        <label className="task-editor__field task-editor__field--wide"><span>{t("recurrence.timezone")}</span><input aria-label={t("recurrence.timezone")} value={value.timezone} onChange={(event) => update({ ...value, timezone: event.target.value })} /></label>
      </div>
      {value.pattern.kind === "weekly" ? <fieldset className="recurrence-weekdays"><legend>{t("recurrence.executionWeekday")}</legend>{weekdays.map((label, index) => { const day = index + 1; const checked = value.pattern.kind === "weekly" && value.pattern.weekdays.includes(day); return <label key={day}><input type="checkbox" checked={checked} onChange={() => { if (value.pattern.kind !== "weekly") return; const selected = checked ? value.pattern.weekdays.filter((item) => item !== day) : [...value.pattern.weekdays, day]; update({ ...value, pattern: { ...value.pattern, weekdays: selected.sort((a, b) => a - b) } }); }} /><span>{label}</span></label>; })}</fieldset> : null}
      {error ? <small className="field__error" role="alert">{error}</small> : null}
      {showActions ? <footer className="task-editor__footer">{onCancel ? <Button type="button" tone="ghost" onClick={onCancel}>{t("common.cancel")}</Button> : null}<Button type="button" tone="primary" disabled={submitting} onClick={() => void submit()}>{submitting ? t("common.saving") : submitLabel ?? t("recurrence.save")}</Button></footer> : null}
    </div>
  );
}

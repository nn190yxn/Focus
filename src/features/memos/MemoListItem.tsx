import { Icon } from "../../components/Icon";
import { useI18n, type I18nValue } from "../../i18n/I18nContext";
import type { MemoReminder, MemoReminderSchedule, MemoSummary } from "./types";

export type MemoListItemProps = {
  memo: MemoSummary;
  selected: boolean;
  onSelect: (id: string) => void;
  now?: Date;
};

export function MemoListItem({ memo, selected, onSelect, now = new Date() }: MemoListItemProps) {
  const i18n = useI18n();
  const visibleTags = memo.tags.slice(0, 3);
  const remainingTags = memo.tags.length - visibleTags.length;

  return (
    <button
      type="button"
      className={`memo-list-item ${selected ? "active" : ""}`}
      aria-current={selected ? "true" : undefined}
      aria-label={i18n.t("memo.openRecord", { title: memo.displayTitle })}
      onClick={() => onSelect(memo.id)}
    >
      <span className="memo-list-item__heading">
        <strong>{memo.displayTitle}</strong>
        {memo.pinnedAt ? <span className="memo-list-item__status"><Icon name="pin" />{i18n.t("memo.pinned")}</span> : null}
      </span>
      <span className="memo-list-item__preview">{memo.bodyPreview || i18n.t("memo.noBodyPreview")}</span>
      {memo.tags.length > 0 ? (
        <span className="memo-list-item__tags" aria-label={i18n.t("memo.tagsLabel")}>
          {visibleTags.map((tag) => <span key={tag.id}>{tag.name}</span>)}
          {remainingTags > 0 ? <span>{i18n.t("memo.moreTags", { count: remainingTags })}</span> : null}
        </span>
      ) : null}
      <span className="memo-list-item__meta">
        {memo.reminder ? <span><Icon name="clock" />{memoReminderSummary(memo.reminder, now, i18n)}</span> : <span />}
        <span>{i18n.t("memo.updatedAt", { value: formatUpdatedAt(memo.updatedAt, i18n) })}</span>
      </span>
    </button>
  );
}

export function memoReminderSummary(reminder: MemoReminder, now: Date, i18n: I18nValue): string {
  if (reminder.status === "completed") return i18n.t("memo.reminderCompleted");
  if (reminder.status === "cancelled") return i18n.t("memo.reminderCancelled");

  if (reminder.schedule.kind === "once") {
    if (!reminder.nextScheduledFor) return i18n.t("memo.reminderScheduled");
    const scheduled = new Date(reminder.nextScheduledFor);
    const time = i18n.formatTime(scheduled);
    return localDateKey(scheduled) === localDateKey(now)
      ? i18n.t("memo.reminderToday", { time })
      : i18n.t("memo.reminderOnce", {
          date: i18n.formatDate(scheduled, { month: "short", day: "numeric" }),
          time,
        });
  }

  return memoReminderScheduleSummary(reminder.schedule, i18n);
}

export function memoReminderScheduleSummary(schedule: MemoReminderSchedule, i18n: I18nValue): string {
  if (schedule.kind === "once") {
    return i18n.t("memo.reminderOnce", {
      date: i18n.formatDate(schedule.scheduledLocal.slice(0, 10), { month: "short", day: "numeric" }),
      time: schedule.scheduledLocal.slice(11, 16),
    });
  }
  const { frequency, interval, localTime, monthlyDay, weekdays } = schedule;
  if (frequency === "weekdays") return i18n.t("memo.reminderWeekdays", { time: localTime });
  if (frequency === "daily") {
    return interval === 1
      ? i18n.t("memo.reminderDaily", { time: localTime })
      : i18n.t("memo.reminderEveryDays", { interval, time: localTime });
  }
  if (frequency === "weekly") {
    const days = weekdays.map((weekday) => weekdayLabel(weekday, i18n.locale)).join(i18n.t("memo.weekdaySeparator"));
    return interval === 1
      ? i18n.t("memo.reminderWeekly", { days, time: localTime })
      : i18n.t("memo.reminderEveryWeeks", { interval, days, time: localTime });
  }
  return interval === 1
    ? i18n.t("memo.reminderMonthly", { day: monthlyDay ?? 1, time: localTime })
    : i18n.t("memo.reminderEveryMonths", { interval, day: monthlyDay ?? 1, time: localTime });
}

function formatUpdatedAt(value: string, i18n: I18nValue): string {
  const date = new Date(value);
  return `${i18n.formatDate(date, { month: "short", day: "numeric" })} ${i18n.formatTime(date)}`;
}

function localDateKey(value: Date): string {
  return `${value.getFullYear()}-${value.getMonth()}-${value.getDate()}`;
}

function weekdayLabel(weekday: number, locale: string): string {
  return new Intl.DateTimeFormat(locale, { weekday: "short", timeZone: "UTC" }).format(new Date(Date.UTC(2024, 0, weekday)));
}

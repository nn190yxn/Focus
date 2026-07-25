import { createI18n } from "../../i18n/I18nContext";
import type { SupportedLocale } from "../../i18n/locale";
import type { RecurrencePattern, RecurrenceRuleInput } from "./types";

export function summarizeRecurrence(input: RecurrenceRuleInput, locale: SupportedLocale = "zh-CN"): string {
  const { t } = createI18n(locale);
  const frequency = summarizePattern(input.pattern, locale);
  const time = input.localTime ? ` ${input.localTime}` : t("recurrence.noTime");
  const range = input.endsOn
    ? t("recurrence.range", { start: input.startsOn, end: input.endsOn })
    : t("recurrence.rangeOpen", { start: input.startsOn });
  return `${frequency}${time}${range}`;
}

export function summarizePattern(pattern: RecurrencePattern, locale: SupportedLocale = "zh-CN"): string {
  const { t } = createI18n(locale);
  switch (pattern.kind) {
    case "daily":
      return pattern.interval === 1 ? t("recurrence.daily") : t("recurrence.everyDays", { count: pattern.interval });
    case "weekdays":
      return t("recurrence.everyWeekday");
    case "weekly": {
      const interval = pattern.interval === 1 ? t("recurrence.weekly") : t("recurrence.everyWeeks", { count: pattern.interval });
      const monday = new Date(2026, 6, 20, 12);
      const labels = [...pattern.weekdays].sort((a, b) => a - b).map((day) => {
        const date = new Date(monday);
        date.setDate(monday.getDate() + day - 1);
        return new Intl.DateTimeFormat(locale, { weekday: "short" }).format(date);
      });
      const days = locale === "zh-CN" ? labels.join("、") : new Intl.ListFormat(locale, { style: "short", type: "conjunction" }).format(labels);
      return days ? t("recurrence.weeklyDays", { interval, days }) : interval;
    }
    case "monthly":
      return t("recurrence.monthlyDay", {
        interval: pattern.interval === 1 ? t("recurrence.monthly") : t("recurrence.everyMonths", { count: pattern.interval }),
        day: pattern.dayOfMonth,
      });
  }
}

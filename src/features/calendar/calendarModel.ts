import { createI18n } from "../../i18n/I18nContext";
import type { SupportedLocale } from "../../i18n/locale";
import type { CalendarDay, CalendarPeriod, CalendarPeriodResult, CalendarProject } from "./types";

export function weekdayLabelsForLocale(locale: string): string[] {
  const monday = new Date(2026, 6, 20, 12);
  return Array.from({ length: 7 }, (_, index) => {
    const date = new Date(monday);
    date.setDate(monday.getDate() + index);
    return new Intl.DateTimeFormat(locale, { weekday: "short" }).format(date);
  });
}

export function parseLocalDate(value: string): Date {
  const [year, month, day] = value.split("-").map(Number);
  return new Date(year, month - 1, day, 12);
}

export function localDateValue(date: Date): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

export function shiftPeriod(anchor: string, period: CalendarPeriod, direction: -1 | 1): string {
  const date = parseLocalDate(anchor);
  if (period === "week") date.setDate(date.getDate() + direction * 7);
  if (period === "month") date.setMonth(date.getMonth() + direction, 1);
  if (period === "year") date.setFullYear(date.getFullYear() + direction, 0, 1);
  return localDateValue(date);
}

export function formatPeriodTitle(period: CalendarPeriod, anchor: string, locale = "zh-CN"): string {
  const date = parseLocalDate(anchor);
  if (period === "year") return formatDateParts(date, locale, { year: "numeric" });
  if (period === "month") return formatDateParts(date, locale, { year: "numeric", month: "long" });
  const monday = new Date(date);
  const offset = (monday.getDay() + 6) % 7;
  monday.setDate(monday.getDate() - offset);
  const sunday = new Date(monday);
  sunday.setDate(sunday.getDate() + 6);
  const formatter = new Intl.DateTimeFormat(locale, { month: "short", day: "numeric" });
  if (locale === "zh-CN") return `${formatParts(formatter, monday)} - ${formatParts(formatter, sunday)}`;
  return formatter.formatRange(monday, sunday);
}

export function monthCells(year: number, monthIndex: number, days: readonly CalendarDay[]): (CalendarDay | null)[] {
  const lookup = new Map(days.map((day) => [day.date, day]));
  const first = new Date(year, monthIndex, 1, 12);
  const leading = (first.getDay() + 6) % 7;
  const count = new Date(year, monthIndex + 1, 0, 12).getDate();
  const cells: (CalendarDay | null)[] = Array.from({ length: leading }, () => null);
  for (let day = 1; day <= count; day += 1) {
    cells.push(lookup.get(localDateValue(new Date(year, monthIndex, day, 12))) ?? emptyDay(localDateValue(new Date(year, monthIndex, day, 12))));
  }
  while (cells.length % 7 !== 0) cells.push(null);
  return cells;
}

export function dayActivity(day: CalendarDay): number {
  return day.plannedTasks.length + day.completedTasks.length + day.focusSessions.length;
}

export function formatDayHeading(value: string, locale = "zh-CN"): string {
  const date = parseLocalDate(value);
  const formatter = new Intl.DateTimeFormat(locale, { month: "long", day: "numeric", weekday: "long" });
  if (locale === "zh-CN") {
    const parts = formatter.formatToParts(date);
    const dateParts = parts.filter((part) => part.type !== "weekday").map((part) => part.value).join(" ").trim();
    const weekday = parts.find((part) => part.type === "weekday")?.value ?? "";
    return `${dateParts} · ${weekday}`;
  }
  return formatter.format(date);
}

export function formatFocusDuration(seconds: number, locale: SupportedLocale = "zh-CN"): string {
  const minutes = Math.floor(Math.max(0, seconds) / 60);
  return createI18n(locale).t(minutes > 0 ? "common.minutes" : "common.seconds", { count: minutes > 0 ? minutes : Math.max(0, seconds) });
}

export function buildPreviewPeriod(period: CalendarPeriod, anchor: string): CalendarPeriodResult {
  const [startsOn, endsOn] = periodBounds(period, anchor);
  const projects: CalendarProject[] = [
    { id: "focus", name: "抵达 Focus", color: "#4eaa98", icon: "AF", status: "active" },
    { id: "writing", name: "夏季写作计划", color: "#647fbd", icon: "WR", status: "active" },
  ];
  const days = dateRange(startsOn, endsOn).map(emptyDay);
  const selected = days.find((day) => day.date === anchor) ?? days[Math.min(2, days.length - 1)];
  if (selected) {
    selected.plannedTasks.push({
      sourceKind: "task", sourceId: "calendar-plan", title: "整理本周实现节奏", category: "work", project: projects[0],
      scheduledDate: selected.date, scheduledTime: "10:30", status: "pending", completedAt: null,
    });
    selected.completedTasks.push({
      sourceKind: "recurringInstance", sourceId: "calendar-done", title: "完成每日复盘", category: "study", project: projects[1],
      scheduledDate: selected.date, scheduledTime: "18:00", status: "completed", completedAt: `${selected.date}T11:10:00Z`,
    });
    selected.focusSessions.push({
      id: "calendar-focus", title: "日历聚合设计", category: "work", project: projects[0], actualSeconds: 3_000,
      completionKind: "deadline", startedAt: `${selected.date}T06:00:00Z`, endedAt: `${selected.date}T06:50:00Z`,
    });
  }
  days.filter((_, index) => index % Math.max(2, Math.floor(days.length / 12)) === 0).forEach((day, index) => {
    if (day === selected) return;
    day.plannedTasks.push({
      sourceKind: "task", sourceId: `preview-${index}`, title: "推进一项长期计划", category: index % 2 ? "study" : "work", project: projects[index % projects.length],
      scheduledDate: day.date, scheduledTime: index % 2 ? "14:00" : "09:30", status: "pending", completedAt: null,
    });
  });
  return { period, startsOn, endsOn, days, projects };
}

function periodBounds(period: CalendarPeriod, anchor: string): [string, string] {
  const date = parseLocalDate(anchor);
  if (period === "week") {
    const start = new Date(date);
    start.setDate(start.getDate() - ((start.getDay() + 6) % 7));
    const end = new Date(start);
    end.setDate(end.getDate() + 6);
    return [localDateValue(start), localDateValue(end)];
  }
  if (period === "month") {
    return [localDateValue(new Date(date.getFullYear(), date.getMonth(), 1, 12)), localDateValue(new Date(date.getFullYear(), date.getMonth() + 1, 0, 12))];
  }
  return [`${date.getFullYear()}-01-01`, `${date.getFullYear()}-12-31`];
}

function dateRange(startsOn: string, endsOn: string): string[] {
  const dates: string[] = [];
  const current = parseLocalDate(startsOn);
  const end = parseLocalDate(endsOn);
  while (current <= end) {
    dates.push(localDateValue(current));
    current.setDate(current.getDate() + 1);
  }
  return dates;
}

function emptyDay(date: string): CalendarDay {
  return { date, plannedTasks: [], completedTasks: [], focusSessions: [] };
}

function formatDateParts(date: Date, locale: string, options: Intl.DateTimeFormatOptions): string {
  const formatter = new Intl.DateTimeFormat(locale, options);
  return locale === "zh-CN" ? formatParts(formatter, date) : formatter.format(date);
}

function formatParts(formatter: Intl.DateTimeFormat, date: Date): string {
  return formatter.formatToParts(date).map((part) => part.value).join(" ").trim();
}

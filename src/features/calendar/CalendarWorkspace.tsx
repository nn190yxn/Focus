import { useEffect, useState, type CSSProperties } from "react";

import { Badge, Button, Panel, SegmentedControl } from "../../components/ui";
import { useI18n } from "../../i18n/I18nContext";
import type { MessageKey } from "../../i18n/messages";
import { isTauriRuntime } from "../../lib/commandClient";
import { domainErrorMessage } from "../../lib/domainError";
import { calendarClient, type CalendarCommandClient } from "./calendarClient";
import {
  buildPreviewPeriod,
  dayActivity,
  formatDayHeading,
  formatFocusDuration,
  formatPeriodTitle,
  localDateValue,
  monthCells,
  parseLocalDate,
  shiftPeriod,
  weekdayLabelsForLocale,
} from "./calendarModel";
import { StatisticsOverview } from "./StatisticsOverview";
import { statisticsClient, type StatisticsCommandClient } from "./statisticsClient";
import { buildStatisticsSummary } from "./statisticsModel";
import type { StatisticsSummary } from "./statisticsTypes";
import type { CalendarDay, CalendarPeriod, CalendarPeriodResult, CalendarTaskItem } from "./types";

const periods: CalendarPeriod[] = ["week", "month", "year"];
const categoryValues = ["", "work", "study", "health", "life"] as const;

type CalendarWorkspaceProps = {
  selectedDate: string;
  onSelectDate: (date: string) => void;
  runtime?: boolean;
  client?: CalendarCommandClient;
  statisticsCommandClient?: StatisticsCommandClient;
  onStartFocus?: () => void;
};

export function CalendarWorkspace({
  selectedDate,
  onSelectDate,
  runtime = isTauriRuntime(),
  client = calendarClient,
  statisticsCommandClient = statisticsClient,
  onStartFocus,
}: CalendarWorkspaceProps) {
  const { formatDate, locale, t } = useI18n();
  const periodOptions = periods.map((value) => ({ value, label: t(`calendar.period.${value}`) }));
  const categories = categoryValues.map((value) => ({ value, label: t(value ? `task.category.${value}` as MessageKey : "calendar.allCategories") }));
  const [period, setPeriod] = useState<CalendarPeriod>("month");
  const [anchor, setAnchor] = useState(selectedDate);
  const [category, setCategory] = useState("");
  const [projectId, setProjectId] = useState("");
  const [result, setResult] = useState<CalendarPeriodResult>(() => buildPreviewPeriod("month", selectedDate));
  const [statistics, setStatistics] = useState<StatisticsSummary>(() => buildStatisticsSummary(buildPreviewPeriod("month", selectedDate)));
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const today = localDateValue(new Date());

  useEffect(() => {
    let active = true;
    setLoading(true);
    setError(null);
    if (!runtime) {
      const preview = filterPreview(buildPreviewPeriod(period, anchor), category, projectId);
      setResult(preview);
      setStatistics(buildStatisticsSummary(preview));
      setLoading(false);
      return () => { active = false; };
    }
    const timezone = Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC";
    const query = { period, anchorDate: anchor, timezone, category: category || null, projectId: projectId || null };
    void Promise.all([client.getPeriod(query), statisticsCommandClient.getSummary(query)]).then(([calendarResponse, statisticsResponse]) => {
      if (!active) return;
      if (calendarResponse.ok) setResult(calendarResponse.data);
      else setError(domainErrorMessage(calendarResponse.error, t));
      if (statisticsResponse.ok) setStatistics(statisticsResponse.data);
      else setError(domainErrorMessage(statisticsResponse.error, t));
      setLoading(false);
    });
    return () => { active = false; };
  }, [anchor, category, client, period, projectId, runtime, statisticsCommandClient]);

  const selectedDay = result.days.find((day) => day.date === selectedDate) ?? emptyDay(selectedDate);
  const activeDays = result.days.filter((day) => dayActivity(day) > 0).length;

  function changePeriod(nextPeriod: CalendarPeriod) {
    setPeriod(nextPeriod);
    setAnchor(selectedDate);
  }

  function navigate(direction: -1 | 1) {
    const nextAnchor = shiftPeriod(anchor, period, direction);
    setAnchor(nextAnchor);
    onSelectDate(nextAnchor);
  }

  function goToday() {
    setAnchor(today);
    onSelectDate(today);
  }

  return (
    <div className="calendar-workspace">
      <Panel className="calendar-controls">
        <span className="eyebrow">{t("calendar.reviewRange")}</span>
        <h2>{t("calendar.perspective")}</h2>
        <SegmentedControl label={t("calendar.viewLabel")} value={period} options={periodOptions} onChange={changePeriod} />
        <label className="calendar-filter">
          <span>{t("calendar.category")}</span>
          <select aria-label={t("calendar.category")} value={category} onChange={(event) => setCategory(event.target.value)}>
            {categories.map((item) => <option key={item.value} value={item.value}>{item.label}</option>)}
          </select>
        </label>
        <label className="calendar-filter">
          <span>{t("calendar.project")}</span>
          <select aria-label={t("calendar.project")} value={projectId} onChange={(event) => setProjectId(event.target.value)}>
            <option value="">{t("calendar.allProjects")}</option>
            {result.projects.map((project) => <option key={project.id} value={project.id}>{project.name}</option>)}
          </select>
        </label>
        <div className="calendar-range-note">
          <span>{formatDate(result.startsOn)}</span>
          <i />
          <span>{formatDate(result.endsOn)}</span>
        </div>
        <div className="calendar-activity-note">
          <strong>{activeDays}</strong>
          <span>{t("calendar.activeDates")}</span>
        </div>
        <div className="calendar-legend" aria-label={t("calendar.legend")}>
          <span><i className="calendar-dot calendar-dot--planned" />{t("calendar.planned")}</span>
          <span><i className="calendar-dot calendar-dot--completed" />{t("calendar.completed")}</span>
          <span><i className="calendar-dot calendar-dot--focus" />{t("calendar.focus")}</span>
        </div>
      </Panel>

      <Panel className={`calendar-board calendar-board--${period}`} aria-busy={loading}>
        <header className="calendar-board__header">
          <div>
            <span className="eyebrow">{period === "year" ? "YEAR REVIEW" : "CALENDAR"}</span>
            <h2>{formatPeriodTitle(period, anchor, locale)}</h2>
          </div>
          <div className="calendar-navigation">
            <Button tone="ghost" aria-label={t("calendar.previous")} onClick={() => navigate(-1)}>←</Button>
            <Button tone="ghost" onClick={goToday}>{t("common.today")}</Button>
            <Button tone="ghost" aria-label={t("calendar.next")} onClick={() => navigate(1)}>→</Button>
          </div>
        </header>
        {error ? <p className="calendar-error" role="alert">{error}</p> : null}
        {period === "week" ? <WeekView days={result.days} selectedDate={selectedDate} today={today} onSelect={onSelectDate} /> : null}
        {period === "month" ? <MonthView anchor={anchor} days={result.days} selectedDate={selectedDate} today={today} onSelect={onSelectDate} /> : null}
        {period === "year" ? <YearView anchor={anchor} days={result.days} selectedDate={selectedDate} today={today} onSelect={onSelectDate} /> : null}
        {loading ? <div className="calendar-loading">{t("calendar.loading")}</div> : null}
      </Panel>

      <Panel className="calendar-detail">
        <span className="eyebrow">{t("calendar.selectedDate")}</span>
        <h2>{formatDayHeading(selectedDate, locale)}</h2>
        <DaySummary day={selectedDay} />
        <DetailSection title={t("calendar.planned")} empty={t("calendar.plannedEmpty")} items={selectedDay.plannedTasks} />
        <DetailSection title={t("calendar.completed")} empty={t("calendar.completedEmpty")} items={selectedDay.completedTasks} completed />
        <section className="calendar-detail-section">
          <header><h3>{t("calendar.focusSessions")}</h3><span>{selectedDay.focusSessions.length}</span></header>
          {selectedDay.focusSessions.length === 0 ? <p>{t("calendar.focusEmpty")}</p> : selectedDay.focusSessions.map((session) => (
            <article className="calendar-focus-entry" key={session.id}>
              <span className="calendar-focus-entry__time">{formatFocusDuration(session.actualSeconds, locale)}</span>
              <div><strong>{session.title}</strong><small>{t("calendar.completedAt", { time: formatClock(session.endedAt, locale, t("calendar.completedFallback")) })}{session.project ? ` · ${session.project.name}` : ""}</small></div>
            </article>
          ))}
        </section>
      </Panel>

      <StatisticsOverview summary={statistics} loading={loading} error={error} onStartFocus={onStartFocus} />
    </div>
  );
}

function WeekView({ days, selectedDate, today, onSelect }: CalendarViewProps) {
  const { locale } = useI18n();
  const weekdayLabels = weekdayLabelsForLocale(locale);
  return (
    <div className="calendar-week-grid">
      {days.map((day, index) => (
        <DayButton key={day.date} day={day} label={weekdayLabels[index]} selected={day.date === selectedDate} today={day.date === today} onSelect={onSelect} />
      ))}
    </div>
  );
}

function MonthView({ anchor, days, selectedDate, today, onSelect }: CalendarViewProps & { anchor: string }) {
  const { locale } = useI18n();
  const weekdayLabels = weekdayLabelsForLocale(locale);
  const date = parseLocalDate(anchor);
  const cells = monthCells(date.getFullYear(), date.getMonth(), days);
  return (
    <div className="calendar-month">
      <div className="calendar-weekdays">{weekdayLabels.map((label) => <span key={label}>{label}</span>)}</div>
      <div className="calendar-month-grid">
        {cells.map((day, index) => day ? <DayButton key={day.date} day={day} selected={day.date === selectedDate} today={day.date === today} onSelect={onSelect} /> : <span className="calendar-day-placeholder" key={`blank-${index}`} />)}
      </div>
    </div>
  );
}

function YearView({ anchor, days, selectedDate, today, onSelect }: CalendarViewProps & { anchor: string }) {
  const { locale, t } = useI18n();
  const weekdayLabels = weekdayLabelsForLocale(locale);
  const year = parseLocalDate(anchor).getFullYear();
  return (
    <div className="calendar-year-grid">
      {Array.from({ length: 12 }, (_, month) => (
        <section className="calendar-mini-month" key={month}>
          <h3>{formatMonthHeading(new Date(year, month, 1, 12), locale)}</h3>
          <div className="calendar-mini-weekdays">{weekdayLabels.map((label) => <span key={label}>{label}</span>)}</div>
          <div className="calendar-mini-grid">
            {monthCells(year, month, days).map((day, index) => day ? (
              <button key={day.date} type="button" className={dayClasses(day, day.date === selectedDate, day.date === today)} aria-label={dayLabel(day, locale, t)} onClick={() => onSelect(day.date)}>
                {parseLocalDate(day.date).getDate()}
              </button>
            ) : <span key={`blank-${index}`} />)}
          </div>
        </section>
      ))}
    </div>
  );
}

function DayButton({ day, label, selected, today, onSelect }: { day: CalendarDay; label?: string; selected: boolean; today: boolean; onSelect: (date: string) => void }) {
  const { locale, t } = useI18n();
  return (
    <button type="button" className={dayClasses(day, selected, today)} aria-pressed={selected} aria-label={dayLabel(day, locale, t)} onClick={() => onSelect(day.date)}>
      {label ? <span className="calendar-day__weekday">{label}</span> : null}
      <strong>{parseLocalDate(day.date).getDate()}</strong>
      <div className="calendar-day__marks" aria-hidden="true">
        {day.plannedTasks.length > 0 ? <i className="calendar-dot calendar-dot--planned" /> : null}
        {day.completedTasks.length > 0 ? <i className="calendar-dot calendar-dot--completed" /> : null}
        {day.focusSessions.length > 0 ? <i className="calendar-dot calendar-dot--focus" /> : null}
      </div>
      {dayActivity(day) > 0 ? <small>{t("calendar.records", { count: dayActivity(day) })}</small> : null}
    </button>
  );
}

function DaySummary({ day }: { day: CalendarDay }) {
  const { t } = useI18n();
  return (
    <div className="calendar-day-summary">
      <span><strong>{day.plannedTasks.length}</strong>{t("calendar.daySummaryPlanned")}</span>
      <span><strong>{day.completedTasks.length}</strong>{t("calendar.daySummaryCompleted")}</span>
      <span><strong>{day.focusSessions.length}</strong>{t("calendar.daySummaryFocus")}</span>
    </div>
  );
}

function DetailSection({ title, empty, items, completed = false }: { title: string; empty: string; items: CalendarTaskItem[]; completed?: boolean }) {
  const { locale, t } = useI18n();
  return (
    <section className="calendar-detail-section">
      <header><h3>{title}</h3><span>{items.length}</span></header>
      {items.length === 0 ? <p>{empty}</p> : items.map((item) => (
        <article className="calendar-task-entry" key={`${title}-${item.sourceKind}-${item.sourceId}`} style={{ "--calendar-project-color": item.project?.color ?? "var(--color-accent)" } as CSSProperties}>
          <i />
          <div><strong>{item.title}</strong><small>{item.project?.name ?? categoryLabel(item.category, t)}{item.sourceKind === "recurringInstance" ? ` · ${t("calendar.recurring")}` : ""}</small></div>
          <Badge tone={completed ? "success" : "neutral"}>{completed ? formatClock(item.completedAt, locale, t("calendar.completedFallback")) : item.scheduledTime ?? t("calendar.allDay")}</Badge>
        </article>
      ))}
    </section>
  );
}

type CalendarViewProps = { days: CalendarDay[]; selectedDate: string; today: string; onSelect: (date: string) => void };

function dayClasses(day: CalendarDay, selected: boolean, today: boolean): string {
  return ["calendar-day", dayActivity(day) > 0 ? "calendar-day--active" : "", selected ? "calendar-day--selected" : "", today ? "calendar-day--today" : ""].filter(Boolean).join(" ");
}

function dayLabel(day: CalendarDay, locale: string, t: ReturnType<typeof useI18n>["t"]): string {
  return t("calendar.dayLabel", { date: formatDayHeading(day.date, locale), planned: day.plannedTasks.length, completed: day.completedTasks.length, focus: day.focusSessions.length });
}

function formatClock(value: string | null, locale: string, fallback: string): string {
  if (!value) return fallback;
  return new Intl.DateTimeFormat(locale, { hour: "2-digit", minute: "2-digit", hour12: false }).format(new Date(value));
}

function formatMonthHeading(value: Date, locale: string): string {
  const parts = new Intl.DateTimeFormat(locale, { month: "numeric" }).formatToParts(value).filter((part) => part.type !== "literal" || part.value.trim());
  return locale === "zh-CN" ? parts.map((part) => part.value.trim()).join(" ") : parts.map((part) => part.value).join("");
}

function categoryLabel(value: string, t: ReturnType<typeof useI18n>["t"]): string {
  return (["work", "study", "health", "life"] as string[]).includes(value) ? t(`task.category.${value}` as MessageKey) : value;
}

function filterPreview(result: CalendarPeriodResult, category: string, projectId: string): CalendarPeriodResult {
  if (!category && !projectId) return result;
  const matchesTask = (item: CalendarTaskItem) => (!category || item.category === category) && (!projectId || item.project?.id === projectId);
  return {
    ...result,
    days: result.days.map((day) => ({
      ...day,
      plannedTasks: day.plannedTasks.filter(matchesTask),
      completedTasks: day.completedTasks.filter(matchesTask),
      focusSessions: day.focusSessions.filter((session) => (!category || session.category === category) && (!projectId || session.project?.id === projectId)),
    })),
  };
}

function emptyDay(date: string): CalendarDay {
  return { date, plannedTasks: [], completedTasks: [], focusSessions: [] };
}

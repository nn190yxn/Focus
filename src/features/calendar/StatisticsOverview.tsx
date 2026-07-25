import type { CSSProperties } from "react";

import { Button, Panel } from "../../components/ui";
import { useI18n } from "../../i18n/I18nContext";
import { trendBuckets } from "./statisticsModel";
import type { StatisticsSummary } from "./statisticsTypes";

type StatisticsOverviewProps = {
  summary: StatisticsSummary;
  loading?: boolean;
  error?: string | null;
  onStartFocus?: () => void;
};

export function StatisticsOverview({ summary, loading = false, error, onStartFocus }: StatisticsOverviewProps) {
  const { formatDate, locale, t } = useI18n();
  const buckets = trendBuckets(summary, locale);
  const maxFocus = Math.max(1, ...buckets.map((item) => item.focusSeconds));
  const maxCompleted = Math.max(1, ...buckets.map((item) => item.completedTaskCount));
  const hasActivity = summary.completedTaskCount > 0 || summary.effectiveSessionCount > 0;

  return (
    <Panel className="statistics-overview" aria-busy={loading}>
      <header className="statistics-overview__header">
        <div><span className="eyebrow">PERIOD REVIEW</span><h2>{t("statistics.title")}</h2></div>
        <span>{t("statistics.range", { start: formatDate(summary.startsOn), end: formatDate(summary.endsOn) })}</span>
      </header>

      <div className="statistics-metrics" aria-label={t("statistics.summaryLabel")}>
        <Metric label={t("statistics.completionRate")} value={`${summary.completionPercent}%`} detail={t("statistics.plannedCount", { count: summary.plannedTaskCount })} />
        <Metric label={t("statistics.completedTasks")} value={String(summary.completedTaskCount)} detail={t("statistics.completedDetail")} />
        <Metric label={t("statistics.focusDuration")} value={String(summary.focusMinutes)} detail={t("statistics.minutes")} />
        <Metric label={t("statistics.sessions")} value={String(summary.effectiveSessionCount)} detail={t("statistics.sessionsDetail")} />
        <Metric label={t("statistics.activeDays")} value={String(summary.activeDayCount)} detail={t("statistics.days")} />
      </div>

      {error ? <p className="statistics-error" role="alert">{error}</p> : null}
      {!hasActivity && !loading ? (
        <div className="statistics-empty">
          <div><strong>{t("statistics.emptyTitle")}</strong><p>{t("statistics.emptyDescription")}</p></div>
          {onStartFocus ? <Button tone="primary" onClick={onStartFocus}>{t("statistics.startFocus")}</Button> : null}
        </div>
      ) : (
        <div className="statistics-content">
          <section className="statistics-trend" aria-label={t("statistics.trendLabel")}>
            <header><div><h3>{t("statistics.trendTitle")}</h3><p>{t("statistics.trendDescription")}</p></div><div className="statistics-trend__legend"><span>{t("statistics.focus")}</span><span>{t("statistics.completed")}</span></div></header>
            <div className="statistics-trend__plot">
              {buckets.map((bucket) => (
                <div
                  className="statistics-trend__bucket"
                  key={bucket.date}
                  aria-label={t("statistics.bucketLabel", { date: formatDate(bucket.date.length === 7 ? `${bucket.date}-01` : bucket.date), minutes: Math.floor(bucket.focusSeconds / 60), count: bucket.completedTaskCount })}
                  style={{
                    "--focus-height": `${Math.max(bucket.focusSeconds > 0 ? 5 : 0, (bucket.focusSeconds / maxFocus) * 100)}%`,
                    "--task-height": `${Math.max(bucket.completedTaskCount > 0 ? 5 : 0, (bucket.completedTaskCount / maxCompleted) * 100)}%`,
                  } as CSSProperties}
                >
                  <div><i /><i /></div>
                  <span>{bucket.label}</span>
                </div>
              ))}
            </div>
          </section>

          <section className="statistics-projects">
            <header><h3>{t("statistics.projects")}</h3><p>{t("statistics.projectsDescription")}</p></header>
            {summary.projectInvestments.length === 0 ? <p className="statistics-projects__empty">{t("statistics.projectsEmpty")}</p> : summary.projectInvestments.map((investment) => (
              <article key={investment.project.id} style={{ "--project-color": investment.project.color } as CSSProperties}>
                <div><span>{investment.project.icon}</span><strong>{investment.project.name}</strong><small>{t("common.minutes", { count: Math.floor(investment.focusSeconds / 60) })}</small></div>
                <div className="statistics-projects__track"><i style={{ width: `${investment.focusPercent}%` }} /></div>
                <footer><span>{t("statistics.sessionCount", { count: investment.effectiveSessionCount })}</span><strong>{investment.focusPercent}%</strong></footer>
              </article>
            ))}
          </section>
        </div>
      )}
    </Panel>
  );
}

function Metric({ label, value, detail }: { label: string; value: string; detail: string }) {
  return <article><span>{label}</span><strong>{value}</strong><small>{detail}</small></article>;
}

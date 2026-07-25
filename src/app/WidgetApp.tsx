import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";

import { Badge, Button, Progress } from "../components/ui";
import { focusClient } from "../features/focus/focusClient";
import type { FocusState } from "../features/focus/types";
import { recurrenceClient } from "../features/recurrence/recurrenceClient";
import { settingsClient } from "../features/settings/settingsClient";
import { defaultGeneralPreferences, type GeneralPreferences } from "../features/settings/types";
import { taskClient } from "../features/tasks/taskClient";
import { todayClient } from "../features/today/todayClient";
import { localDateString } from "../features/today/todayModel";
import type { TodayDigest, TodayDigestItem } from "../features/today/types";
import {
  calculateTodayProgress,
  focusedItem,
  focusTargetForItem,
  formatFocusDuration,
  formatWidgetClock,
  formatWidgetDate,
  remainingFocusSeconds,
  selectWidgetTasks,
} from "../features/widget/widgetModel";
import {
  defaultWidgetConfig,
  type WidgetConfig,
  type WidgetModeFallbackEvent,
} from "../features/widget/types";
import { widgetClient } from "../features/widget/widgetClient";
import { createI18n, I18nProvider, useI18n } from "../i18n/I18nContext";
import { useResolvedLocale } from "../i18n/locale";
import { isTauriRuntime } from "../lib/commandClient";
import { domainErrorMessage } from "../lib/domainError";
import { resolveThemeTokens, themeStyle } from "../theme/theme";
import { useResolvedColorMode } from "../theme/useResolvedColorMode";

const readyFocus: FocusState = { state: "ready", serverTime: new Date(0).toISOString() };

export function WidgetApp() {
  const desktopRuntime = isTauriRuntime();
  const [generalSettings, setGeneralSettings] = useState(defaultGeneralPreferences);
  const mode = useResolvedColorMode(generalSettings.appearance);
  const locale = useResolvedLocale(generalSettings.language);
  const i18n = createI18n(locale);
  const tokens = resolveThemeTokens("widget", generalSettings.theme, mode);
  const [config, setConfig] = useState(defaultWidgetConfig);
  const [digest, setDigest] = useState<TodayDigest>(() => desktopRuntime
    ? { date: localDateString(), items: [] }
    : previewDigest(localDateString()));
  const [focusState, setFocusState] = useState<FocusState>(readyFocus);
  const [now, setNow] = useState(() => new Date());
  const [modeFallback, setModeFallback] = useState<WidgetModeFallbackEvent | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busyAction, setBusyAction] = useState<string | null>(null);

  useEffect(() => {
    if (!desktopRuntime) return;
    let disposed = false;
    let stopListening: (() => void) | undefined;
    void settingsClient.get().then((result) => {
      if (!disposed && result.ok) setGeneralSettings(result.data);
    });
    void listen<GeneralPreferences>("settings://changed", (event) => setGeneralSettings(event.payload))
      .then((unlisten) => {
        if (disposed) unlisten();
        else stopListening = unlisten;
      });
    return () => {
      disposed = true;
      stopListening?.();
    };
  }, [desktopRuntime]);

  useEffect(() => {
    if (!desktopRuntime) return;
    let disposed = false;
    let stopListening: (() => void) | undefined;
    void widgetClient.getConfig().then((result) => {
      if (!disposed && result.ok) setConfig(result.data);
    });
    void listen<WidgetConfig>("widget://config-changed", (event) => setConfig(event.payload)).then(
      (unlisten) => {
        if (disposed) unlisten();
        else stopListening = unlisten;
      },
    );
    return () => {
      disposed = true;
      stopListening?.();
    };
  }, [desktopRuntime]);

  useEffect(() => {
    if (!desktopRuntime) return;
    let disposed = false;
    const unlisteners: (() => void)[] = [];
    void listen<WidgetModeFallbackEvent>("widget://mode-fallback", (event) =>
      setModeFallback(event.payload),
    )
      .then((unlisten) => {
        if (disposed) unlisten();
        else unlisteners.push(unlisten);
      })
      .catch(() => undefined);
    void listen("widget://mode-restored", () => setModeFallback(null))
      .then((unlisten) => {
        if (disposed) unlisten();
        else unlisteners.push(unlisten);
      })
      .catch(() => undefined);
    return () => {
      disposed = true;
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, [desktopRuntime]);

  useEffect(() => {
    const clock = window.setInterval(() => setNow(new Date()), 1000);
    return () => window.clearInterval(clock);
  }, []);

  useEffect(() => {
    if (!desktopRuntime) return;
    let disposed = false;
    const refreshFocus = async () => {
      const result = await focusClient.getState();
      if (!disposed && result.ok) setFocusState(result.data);
    };
    const refreshToday = async () => {
      const result = await todayClient.getDigest(localDateString());
      if (!disposed) {
        if (result.ok) setDigest(result.data);
        else setError(domainErrorMessage(result.error, i18n.t));
      }
    };
    void refreshFocus();
    void refreshToday();
    const unlisteners: (() => void)[] = [];
    void listen("backup://restored", () => {
      void refreshFocus();
      void refreshToday();
    }).then((unlisten) => {
      if (disposed) unlisten();
      else unlisteners.push(unlisten);
    });
    void listen("today://changed", () => {
      void refreshToday();
    }).then((unlisten) => {
      if (disposed) unlisten();
      else unlisteners.push(unlisten);
    });
    void listen<FocusState>("focus://state-changed", (event) => {
      setFocusState(event.payload);
    }).then((unlisten) => {
      if (disposed) unlisten();
      else unlisteners.push(unlisten);
    });
    const focusPoll = window.setInterval(() => void refreshFocus(), 2000);
    return () => {
      disposed = true;
      unlisteners.forEach((unlisten) => unlisten());
      window.clearInterval(focusPoll);
    };
  }, [desktopRuntime]);

  async function refreshDigest() {
    if (!desktopRuntime) return;
    const result = await todayClient.getDigest(localDateString());
    if (result.ok) setDigest(result.data);
    else setError(domainErrorMessage(result.error, i18n.t));
  }

  async function lockWidget() {
    if (!desktopRuntime) {
      setConfig((current) => ({ ...current, locked: true }));
      return;
    }
    const result = await widgetClient.updateConfig({ ...config, locked: true });
    if (result.ok) setConfig(result.data);
    else setError(domainErrorMessage(result.error, i18n.t));
  }

  async function completeTask(item: TodayDigestItem) {
    setBusyAction(`complete:${item.sourceId}`);
    setError(null);
    if (!desktopRuntime) {
      setDigest((current) => ({
        ...current,
        items: current.items.map((candidate) => candidate.sourceId === item.sourceId
          ? { ...candidate, status: "completed", completedAt: new Date().toISOString() }
          : candidate),
      }));
      setBusyAction(null);
      return;
    }
    const result = item.sourceKind === "recurringInstance"
      ? await recurrenceClient.complete(item.sourceId)
      : await taskClient.setCompleted(item.sourceId, true);
    if (result.ok) await refreshDigest();
    else setError(domainErrorMessage(result.error, i18n.t));
    setBusyAction(null);
  }

  async function startFocus(item: TodayDigestItem) {
    setBusyAction(`focus:${item.sourceId}`);
    setError(null);
    if (!desktopRuntime) {
      const startedAt = new Date();
      setFocusState({
        state: "running",
        ...focusTargetForItem(item),
        plannedSeconds: 1500,
        remainingSeconds: 1500,
        startedAt: startedAt.toISOString(),
        interruptionCount: 0,
        serverTime: startedAt.toISOString(),
        targetEndsAt: new Date(startedAt.getTime() + 1_500_000).toISOString(),
      });
      setBusyAction(null);
      return;
    }
    const result = await focusClient.start(focusTargetForItem(item), 25);
    if (result.ok) setFocusState(result.data);
    else setError(domainErrorMessage(result.error, i18n.t));
    setBusyAction(null);
  }

  async function toggleFocus() {
    if (focusState.state === "ready") return;
    setBusyAction("focus-control");
    setError(null);
    if (!desktopRuntime) {
      const remainingSeconds = remainingFocusSeconds(focusState, new Date());
      if (focusState.state === "running") {
        setFocusState({ ...focusState, state: "paused", remainingSeconds, pausedAt: new Date().toISOString() });
      } else {
        setFocusState({
          ...focusState,
          state: "running",
          targetEndsAt: new Date(Date.now() + remainingSeconds * 1000).toISOString(),
        });
      }
      setBusyAction(null);
      return;
    }
    const result = focusState.state === "running" ? await focusClient.pause() : await focusClient.resume();
    if (result.ok) setFocusState(result.data);
    else setError(domainErrorMessage(result.error, i18n.t));
    setBusyAction(null);
  }

  async function delayTask(item: TodayDigestItem) {
    const suggestedTime = item.scheduledTime ?? "09:00";
    const localTime = window.prompt(i18n.t("widget.delayPrompt"), suggestedTime);
    if (!localTime) return;
    setBusyAction(`delay:${item.sourceId}`);
    setError(null);
    if (!desktopRuntime) {
      setDigest((current) => ({
        ...current,
        items: current.items.map((candidate) => candidate.sourceId === item.sourceId
          ? { ...candidate, scheduledTime: localTime }
          : candidate),
      }));
      setBusyAction(null);
      return;
    }
    const result = await recurrenceClient.delayToday(item.sourceId, localTime);
    if (result.ok) await refreshDigest();
    else setError(domainErrorMessage(result.error, i18n.t));
    setBusyAction(null);
  }

  const items = selectWidgetTasks(digest.items, config.size);
  const progress = calculateTodayProgress(digest.items);
  const activeItem = focusedItem(digest.items, focusState);
  const remainingSeconds = remainingFocusSeconds(focusState, now);
  const showActions = config.modules.includes("quickActions") && config.size !== "compact";

  return (
    <I18nProvider locale={locale}>
    <main
      className={`widget widget--${config.size}`}
      data-locked={config.locked}
      data-theme={generalSettings.theme}
      data-mode={mode}
      data-locale={locale}
      style={{ ...themeStyle(tokens), "--widget-opacity": `${Math.round(config.opacity * 100)}%` } as React.CSSProperties}
    >
      <header className="widget__drag" data-tauri-drag-region>
        <span data-tauri-drag-region>{i18n.t("widget.title")}</span>
        <div className="widget__header-actions">
          <Badge tone={focusState.state === "running" ? "accent" : "neutral"}>
            {i18n.t(focusState.state === "running" ? "widget.status.running" : focusState.state === "paused" ? "widget.status.paused" : "widget.status.today")}
          </Badge>
          {!config.locked && (
            <Button tone="ghost" aria-label={i18n.t("widget.lockLabel")} onClick={lockWidget}>
              {i18n.t(config.size === "compact" ? "widget.lockShort" : "widget.lock")}
            </Button>
          )}
        </div>
      </header>

      {modeFallback && (
        <p className="widget__mode-status" role="status">
          {i18n.t("widget.fallback")}
        </p>
      )}
      {error && <p className="widget__error" role="alert">{error}</p>}

      {config.size === "compact" ? (
        <CompactWidget
          clock={config.modules.includes("clock") ? formatWidgetClock(now, locale) : null}
          focusState={focusState}
          focusTitle={activeItem?.title ?? null}
          remainingSeconds={remainingSeconds}
          task={config.modules.includes("tasks") ? items[0] : undefined}
        />
      ) : (
        <>
          <section className="widget__overview">
            {config.modules.includes("clock") && (
              <div>
                <time className="widget__clock">{formatWidgetClock(now, locale)}</time>
                <p className="widget__date">{formatWidgetDate(now, locale)}</p>
              </div>
            )}
            {config.modules.includes("todayProgress") && (
              <div className="widget__today-progress">
                <span>{i18n.t("widget.progress.title")}</span>
                <strong>{progress.completed}<small> / {progress.total}</small></strong>
                <Progress label={i18n.t("widget.progress.label")} value={progress.percentage} />
              </div>
            )}
          </section>

          {config.modules.includes("currentFocus") && (
            <section className="widget__focus-card" data-state={focusState.state}>
              <div>
                <small>{i18n.t(focusState.state === "ready" ? "widget.focus.current" : focusState.state === "paused" ? "widget.focus.paused" : "widget.focus.running")}</small>
                <strong>{activeItem?.title ?? i18n.t(focusState.state === "ready" ? "widget.focus.choose" : "widget.focus.inProgress")}</strong>
              </div>
              {focusState.state !== "ready" && (
                <div className="widget__focus-time">
                  <span>{formatFocusDuration(remainingSeconds)}</span>
                  <Button tone="ghost" disabled={busyAction === "focus-control"} onClick={toggleFocus}>
                    {i18n.t(focusState.state === "running" ? "widget.focus.pause" : "widget.focus.resume")}
                  </Button>
                </div>
              )}
            </section>
          )}

          {config.modules.includes("tasks") && (
            <section className="widget__tasks" aria-label={i18n.t("widget.tasks.label")}>
              <div className="widget__section-title">
                <h2>{i18n.t("widget.tasks.title")}</h2>
                <span>{items.length} / {digest.items.length}</span>
              </div>
              {items.length === 0 ? (
                <p className="widget__empty">{i18n.t("widget.tasks.empty")}</p>
              ) : (
                <div className="widget__task-list">
                  {items.map((item) => (
                    <WidgetTask
                      key={`${item.sourceKind}:${item.sourceId}`}
                      item={item}
                      active={activeItem?.sourceId === item.sourceId}
                      showActions={showActions}
                      focusReady={focusState.state === "ready"}
                      busyAction={busyAction}
                      onComplete={() => completeTask(item)}
                      onFocus={() => startFocus(item)}
                      onDelay={() => delayTask(item)}
                    />
                  ))}
                </div>
              )}
            </section>
          )}
        </>
      )}
    </main>
    </I18nProvider>
  );
}

function CompactWidget({
  clock,
  focusState,
  focusTitle,
  remainingSeconds,
  task,
}: {
  clock: string | null;
  focusState: FocusState;
  focusTitle: string | null;
  remainingSeconds: number;
  task?: TodayDigestItem;
}) {
  const { t } = useI18n();
  return (
    <div className="widget__compact-body">
      {clock && <time className="widget__compact-clock">{clock}</time>}
      <div className="widget__compact-content">
        {focusState.state !== "ready" ? (
          <p className="widget__compact-focus"><strong>{formatFocusDuration(remainingSeconds)}</strong><span>{focusTitle ?? t("widget.focus.inProgress")}</span></p>
        ) : task ? (
          <p className="widget__compact-task"><span>{task.scheduledTime ?? t("widget.task.next")}</span><strong>{task.title}</strong></p>
        ) : (
          <p className="widget__compact-task"><span>{t("widget.tasks.title")}</span><strong>{t("widget.task.none")}</strong></p>
        )}
      </div>
    </div>
  );
}

function WidgetTask({
  item,
  active,
  showActions,
  focusReady,
  busyAction,
  onComplete,
  onFocus,
  onDelay,
}: {
  item: TodayDigestItem;
  active: boolean;
  showActions: boolean;
  focusReady: boolean;
  busyAction: string | null;
  onComplete: () => void;
  onFocus: () => void;
  onDelay: () => void;
}) {
  const { t } = useI18n();
  return (
    <article
      className="widget-task"
      data-active={active}
      data-completed={item.status === "completed"}
      data-overdue={item.isOverdue}
      style={{ borderLeftColor: item.project?.color ?? "transparent" }}
    >
      <button
        className="widget-task__check"
        aria-label={t("widget.task.completeLabel", { title: item.title })}
        aria-pressed={item.status === "completed"}
        disabled={item.status === "completed" || busyAction === `complete:${item.sourceId}`}
        onClick={onComplete}
      >
        {item.status === "completed" ? t("widget.task.completed") : ""}
      </button>
      <div className="widget-task__body">
        <div className="widget-task__title-line">
          <time>{item.scheduledTime ?? t(item.isOverdue ? "widget.task.overdue" : "widget.task.pending")}</time>
          <strong>{item.title}</strong>
        </div>
        <div className="widget-task__meta">
          {item.project && <span>{item.project.name}</span>}
          {item.sourceKind === "recurringInstance" && <span>{t("widget.task.recurring")}</span>}
          {item.isOverdue && <span className="widget-task__overdue">{t("widget.task.overdueStatus")}</span>}
          {active && <span className="widget-task__active">{t("widget.task.focusing")}</span>}
        </div>
      </div>
      {showActions && item.status === "pending" && (
        <div className="widget-task__actions">
          {item.sourceKind === "recurringInstance" && (
            <Button tone="ghost" aria-label={t("widget.task.delayLabel", { title: item.title })} disabled={busyAction === `delay:${item.sourceId}`} onClick={onDelay}>{t("widget.task.delay")}</Button>
          )}
          <Button tone="ghost" aria-label={t("widget.task.focusLabel", { title: item.title })} disabled={!focusReady || busyAction === `focus:${item.sourceId}`} onClick={onFocus}>{t("widget.task.focus")}</Button>
        </div>
      )}
    </article>
  );
}

function previewDigest(date: string): TodayDigest {
  const project = { id: "preview-project", name: "抵达桌面版", color: "#45a88b", icon: "target", status: "active" as const };
  const base = {
    category: "work" as const,
    priority: 2,
    scheduledDate: date,
    status: "pending" as const,
    completedAt: null,
    isOverdue: false,
    createdAt: `${date}T08:00:00Z`,
  };
  return {
    date,
    items: [
      { ...base, sourceKind: "task", sourceId: "preview-1", itemKind: "projectTask", recurrenceRuleId: null, title: "完成小组件三档布局", scheduledTime: "10:30", project },
      { ...base, sourceKind: "recurringInstance", sourceId: "preview-2", itemKind: "recurringInstance", recurrenceRuleId: "preview-rule", title: "整理今日工作记录", scheduledTime: "14:00", project: null },
      { ...base, sourceKind: "task", sourceId: "preview-3", itemKind: "ordinaryTask", recurrenceRuleId: null, title: "回顾本周专注目标", scheduledTime: "16:20", project: null },
      { ...base, sourceKind: "task", sourceId: "preview-4", itemKind: "ordinaryTask", recurrenceRuleId: null, title: "更新项目进度", scheduledTime: null, project },
      { ...base, sourceKind: "task", sourceId: "preview-5", itemKind: "ordinaryTask", recurrenceRuleId: null, title: "阅读 30 分钟", scheduledTime: "20:00", project: null },
    ],
  };
}

import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";

import { Icon, type IconName } from "../components/Icon";
import { Button, Dialog, SegmentedControl, Toast } from "../components/ui";
import { CalendarWorkspace } from "../features/calendar/CalendarWorkspace";
import { FocusWorkspace } from "../features/focus/FocusWorkspace";
import { MemoWorkspace, type MemoOpenRequest } from "../features/memos/MemoWorkspace";
import type { MemoListQuery } from "../features/memos/types";
import { ProjectWorkspace } from "../features/projects/ProjectWorkspace";
import { projectClient } from "../features/projects/projectClient";
import type { ProjectRecord, ProjectSummary } from "../features/projects/types";
import { RecurrenceScopeEditor } from "../features/recurrence/RecurrenceScopeEditor";
import { recurrenceClient } from "../features/recurrence/recurrenceClient";
import type { RecurrenceChangeScope, RecurrenceRule, RecurrenceRuleInput } from "../features/recurrence/types";
import { settingsClient } from "../features/settings/settingsClient";
import { SettingsWorkspace } from "../features/settings/SettingsWorkspace";
import { defaultGeneralPreferences, type GeneralPreferences, type GeneralPreferencesPatch } from "../features/settings/types";
import { TaskEditor } from "../features/tasks/TaskEditor";
import { taskClient } from "../features/tasks/taskClient";
import type { TaskDetail, TaskInput, TaskProjectSummary, TaskVisualState } from "../features/tasks/types";
import { TodayWorkspace } from "../features/today/TodayWorkspace";
import { planningClient } from "../features/today/planningClient";
import type { WeeklyGoal, WeeklyGoalCategory, WeeklyGoalInput } from "../features/today/planningTypes";
import { buildWeekDays, decorateTodayDigest, localDateString, recurrenceBadge, type WorkspaceTask } from "../features/today/todayModel";
import { todayClient } from "../features/today/todayClient";
import { createI18n, I18nProvider, type I18nValue } from "../i18n/I18nContext";
import { useResolvedLocale } from "../i18n/locale";
import { isTauriRuntime } from "../lib/commandClient";
import { domainErrorMessage } from "../lib/domainError";
import { colorModes, resolveThemeTokens, themeNames, themeStyle, type ThemeName } from "../theme/theme";
import { useResolvedColorMode } from "../theme/useResolvedColorMode";

const pages = [
  { id: "today", labelKey: "nav.today", icon: "today" },
  { id: "memos", labelKey: "nav.memos", icon: "memos" },
  { id: "projects", labelKey: "nav.projects", icon: "projects" },
  { id: "focus", labelKey: "nav.focus", icon: "focus" },
  { id: "calendar", labelKey: "nav.calendar", icon: "calendar" },
  { id: "settings", labelKey: "nav.settings", icon: "settings" },
] as const satisfies readonly { id: string; labelKey: `nav.${string}`; icon: IconName }[];

export const applicationTitle = "抵达 Focus";
const initialToday = localDateString();
const previewWeekStartsOn = buildWeekDays(initialToday)[0].date;

const previewTaskProjects: TaskProjectSummary[] = [
  { id: "focus", name: "抵达 Focus", color: "#4eaa98", icon: "AF", status: "active" },
  { id: "writing", name: "夏季写作计划", color: "#647fbd", icon: "WR", status: "active" },
  { id: "study", name: "系统设计复习", color: "#c18471", icon: "SD", status: "paused" },
];

const initialTasks: WorkspaceTask[] = [
  makeTask("model", "完成项目数据模型设计", "focus", "work", "10:30", "current", 3, ["确认领域边界", "整理实现顺序"]),
  makeTask("review", "回复设计评审意见", "writing", "work", "11:40", "normal", 2),
  makeTask("rules", "整理重复任务规则", "focus", "study", "14:00", "paused", 2),
  makeTask("walk", "晚间散步 30 分钟", null, "health", "19:30", "normal", 0),
  makeTask("overdue", "补充昨日测试记录", "focus", "work", "09:00", "overdue", 1, [], "2026-07-17"),
  makeTask("done", "完成任务筛选查询", "focus", "work", "08:20", "completed", 2),
];

const initialWeeklyGoals: WeeklyGoal[] = [
  makeWeeklyGoal("preview-tasks", "完成重点任务", "completedTasks", 5, 0),
  makeWeeklyGoal("preview-active", "保持活跃天数", "activeDays", 5, 1),
];

export function App() {
  const today = localDateString();
  const [page, setPage] = useState<(typeof pages)[number]["id"]>("today");
  const [generalSettings, setGeneralSettings] = useState(defaultGeneralPreferences);
  const [tasks, setTasks] = useState(initialTasks);
  const [projectOptions, setProjectOptions] = useState(previewTaskProjects);
  const [selectedDate, setSelectedDate] = useState(initialToday);
  const [editingTaskId, setEditingTaskId] = useState<string | "new" | null>(null);
  const [draftProjectId, setDraftProjectId] = useState<string | null>(null);
  const [focusedTask, setFocusedTask] = useState<WorkspaceTask | null>(null);
  const [loadingTasks, setLoadingTasks] = useState(false);
  const [taskError, setTaskError] = useState<string | null>(null);
  const [noteBody, setNoteBody] = useState("今天保持节奏，先把底层边界做扎实。");
  const [weeklyGoals, setWeeklyGoals] = useState(initialWeeklyGoals);
  const [noteLoading, setNoteLoading] = useState(false);
  const [goalsLoading, setGoalsLoading] = useState(false);
  const [goalDataRevision, setGoalDataRevision] = useState(0);
  const [backupDataRevision, setBackupDataRevision] = useState(0);
  const [todayDataRevision, setTodayDataRevision] = useState(0);
  const [memoDataRevision, setMemoDataRevision] = useState(0);
  const [memoOpenRequest, setMemoOpenRequest] = useState<MemoOpenRequest | null>(null);
  const [memoQuery, setMemoQuery] = useState<MemoListQuery>({ search: "", tagId: null });
  const [editingRecurringTask, setEditingRecurringTask] = useState<{ instanceId: string; effectiveOn: string; rule: RecurrenceRule; template: TaskDetail } | null>(null);
  const [editingRecurrence, setEditingRecurrence] = useState<{ instanceId: string; effectiveOn: string; rule: RecurrenceRule } | null>(null);
  const desktopRuntime = isTauriRuntime();
  const theme = generalSettings.theme;
  const mode = useResolvedColorMode(generalSettings.appearance);
  const locale = useResolvedLocale(generalSettings.language);
  const i18n = createI18n(locale);
  const tokens = resolveThemeTokens("main", theme, mode);
  const currentPage = pages.find((item) => item.id === page) ?? pages[0];
  const editingTask = editingTaskId && editingTaskId !== "new" ? tasks.find((item) => item.task.id === editingTaskId) : undefined;
  const week = buildWeekDays(selectedDate, locale);
  const weekStartsOn = week[0]?.date ?? selectedDate;
  const timezone = Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC";
  const visibleTasks = desktopRuntime ? tasks : tasks.filter((item) => item.task.scheduledDate === selectedDate || (selectedDate === today && item.visualState === "overdue"));

  useEffect(() => {
    if (!desktopRuntime) return;
    let disposed = false;
    let stopListening: (() => void) | undefined;
    void settingsClient.get().then((result) => {
      if (disposed) return;
      if (result.ok) setGeneralSettings(result.data);
      else setTaskError(domainErrorMessage(result.error, i18n.t));
    }).catch(() => {
      if (!disposed) setTaskError("读取应用设置失败，请稍后重试。");
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
    let active = true;
    void projectClient.list(null, today).then((result) => {
      if (!active) return;
      if (result.ok) setProjectOptions(result.data.map(projectSummaryToTaskProject));
      else setTaskError(domainErrorMessage(result.error, i18n.t));
    });
    return () => { active = false; };
  }, [backupDataRevision, desktopRuntime, todayDataRevision]);

  useEffect(() => {
    if (!desktopRuntime) return;
    let active = true;
    setLoadingTasks(true);
    void todayClient.getDigest(selectedDate).then((result) => {
      if (!active) return;
      if (result.ok) setTasks(decorateTodayDigest(result.data.items));
      else setTaskError(domainErrorMessage(result.error, i18n.t));
      setLoadingTasks(false);
    });
    return () => { active = false; };
  }, [backupDataRevision, desktopRuntime, selectedDate, todayDataRevision]);

  useEffect(() => {
    if (!desktopRuntime) return;
    let active = true;
    setNoteLoading(true);
    void planningClient.getNote(selectedDate).then((result) => {
      if (!active) return;
      if (result.ok) setNoteBody(result.data?.body ?? "");
      else setTaskError(domainErrorMessage(result.error, i18n.t));
      setNoteLoading(false);
    }).catch(() => {
      if (active) {
        setTaskError("读取便签失败，请稍后重试。");
        setNoteLoading(false);
      }
    });
    return () => { active = false; };
  }, [backupDataRevision, desktopRuntime, selectedDate]);

  useEffect(() => {
    if (!desktopRuntime) return;
    let active = true;
    const timer = window.setTimeout(() => {
      setGoalsLoading(true);
      void planningClient.listWeeklyGoals(weekStartsOn, timezone).then((result) => {
        if (!active) return;
        if (result.ok) setWeeklyGoals(result.data);
        else setTaskError(domainErrorMessage(result.error, i18n.t));
        setGoalsLoading(false);
      }).catch(() => {
        if (active) {
          setTaskError("计算周目标失败，请稍后重试。");
          setGoalsLoading(false);
        }
      });
    }, goalDataRevision === 0 ? 0 : 250);
    return () => {
      active = false;
      window.clearTimeout(timer);
    };
  }, [backupDataRevision, desktopRuntime, goalDataRevision, timezone, weekStartsOn]);

  useEffect(() => {
    if (desktopRuntime) return;
    const completedTasks = tasks.filter((item) => item.task.status === "completed").length;
    const activeDays = new Set(tasks.flatMap((item) => item.task.completedAt ? [item.task.completedAt.slice(0, 10)] : [])).size;
    setWeeklyGoals((goals) => goals.map((goal) => ({
      ...goal,
      completedCount: Math.min(goal.targetCount, goal.category === "completedTasks" ? completedTasks : goal.category === "activeDays" ? activeDays : goal.completedCount),
    })));
  }, [desktopRuntime, tasks]);

  useEffect(() => {
    if (!desktopRuntime) return;
    let disposed = false;
    const unlisteners: (() => void)[] = [];
    void listen("tray://quick-task", () => {
      setPage("today");
      setSelectedDate(today);
      openNewTask();
    }).then((unlisten) => disposed ? unlisten() : unlisteners.push(unlisten));
    void listen("tray://open-focus", () => setPage("focus"))
      .then((unlisten) => disposed ? unlisten() : unlisteners.push(unlisten));
    void listen("focus://completed", () => setGoalDataRevision((value) => value + 1))
      .then((unlisten) => disposed ? unlisten() : unlisteners.push(unlisten));
    void listen("backup://restored", () => {
      setBackupDataRevision((value) => value + 1);
      setMemoDataRevision((value) => value + 1);
    })
      .then((unlisten) => disposed ? unlisten() : unlisteners.push(unlisten));
    void listen("memo://changed", () => setMemoDataRevision((value) => value + 1))
      .then((unlisten) => disposed ? unlisten() : unlisteners.push(unlisten));
    void listen<string>("memo://open-requested", (event) => {
      setPage("memos");
      setMemoOpenRequest((current) => ({ memoId: event.payload, sequence: (current?.sequence ?? 0) + 1 }));
    }).then((unlisten) => disposed ? unlisten() : unlisteners.push(unlisten));
    void listen("today://changed", () => {
      const nextToday = localDateString();
      setSelectedDate((current) => current === today ? nextToday : current);
      setTodayDataRevision((value) => value + 1);
      setGoalDataRevision((value) => value + 1);
    }).then((unlisten) => disposed ? unlisten() : unlisteners.push(unlisten));
    return () => {
      disposed = true;
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, [desktopRuntime]);

  async function saveGeneralSettings(patch: GeneralPreferencesPatch): Promise<GeneralPreferences> {
    if (!desktopRuntime) {
      const updated = { ...generalSettings, ...patch };
      setGeneralSettings(updated);
      return updated;
    }
    const result = await settingsClient.update(patch);
    if (!result.ok) throw new Error(domainErrorMessage(result.error, i18n.t));
    setGeneralSettings(result.data);
    return result.data;
  }

  async function saveTask(input: TaskInput, recurrence: RecurrenceRuleInput | null) {
    const project = projectOptions.find((item) => item.id === input.projectId) ?? null;
    if (desktopRuntime) {
      const result = editingTask
        ? await taskClient.update(editingTask.task.id, input, today)
        : await taskClient.create(input, today);
      if (!result.ok) {
        setTaskError(domainErrorMessage(result.error, i18n.t));
        return;
      }
      if (recurrence) {
        const rule: RecurrenceRule = { id: crypto.randomUUID(), taskTemplateId: result.data.task.id, ...recurrence, status: "active", version: 1 };
        const recurrenceResult = await recurrenceClient.create(rule, recurrence.startsOn, recurrence.endsOn ?? recurrence.startsOn);
        if (!recurrenceResult.ok) {
          setTaskError(domainErrorMessage(recurrenceResult.error, i18n.t));
          setEditingTaskId(null);
          setDraftProjectId(null);
          await refreshDigest();
          setGoalDataRevision((value) => value + 1);
          return;
        }
      }
      const nextItem: WorkspaceTask = {
        sourceKind: "task",
        sourceId: result.data.task.id,
        recurrenceRuleId: null,
        recurrenceLabel: null,
        task: result.data.task,
        project,
        checkItems: result.data.checkItems.map((item) => ({ id: item.id, title: item.title, completed: Boolean(item.completedAt) })),
        visualState: project?.status === "paused" ? "paused" : "normal",
      };
      setTasks((items) => editingTask ? items.map((item) => item.task.id === nextItem.task.id ? nextItem : item) : [...items, nextItem]);
      setEditingTaskId(null);
      setDraftProjectId(null);
      await refreshDigest();
      setGoalDataRevision((value) => value + 1);
      return;
    }
    if (editingTask) {
      setTasks((items) => items.map((item) => item.task.id === editingTask.task.id ? {
        ...item,
        project,
        task: { ...item.task, ...input, updatedAt: new Date().toISOString() },
        checkItems: input.checkItems,
        visualState: project?.status === "paused" ? "paused" : item.visualState,
        recurrenceLabel: recurrence ? recurrenceBadge(recurrence.pattern) : item.recurrenceLabel,
      } : item));
    } else {
      const id = crypto.randomUUID();
      const now = new Date().toISOString();
      setTasks((items) => [...items, {
        sourceKind: "task",
        sourceId: id,
        recurrenceRuleId: recurrence ? `preview-rule-${id}` : null,
        recurrenceLabel: recurrence ? recurrenceBadge(recurrence.pattern) : null,
        project,
        visualState: project?.status === "paused" ? "paused" : "normal",
        checkItems: input.checkItems,
        task: { id, ...input, status: "pending", completedAt: null, createdAt: now, updatedAt: now },
      }]);
    }
    setEditingTaskId(null);
    setDraftProjectId(null);
  }

  async function toggleTaskCompleted(id: string, completed: boolean) {
    const item = tasks.find((candidate) => candidate.sourceId === id);
    if (item?.sourceKind === "recurringInstance") {
      if (!completed) return;
      const result = await recurrenceClient.complete(id);
      if (!result.ok) setTaskError(domainErrorMessage(result.error, i18n.t));
      else {
        await refreshDigest();
        setGoalDataRevision((value) => value + 1);
      }
      return;
    }
    if (desktopRuntime) {
      const result = await taskClient.setCompleted(id, completed);
      if (!result.ok) {
        setTaskError(domainErrorMessage(result.error, i18n.t));
        return;
      }
    }
    setTasks((items) => items.map((item) => item.task.id === id ? {
      ...item,
      visualState: completed ? "completed" : item.project?.status === "paused" ? "paused" : "normal",
      task: { ...item.task, status: completed ? "completed" : "pending", completedAt: completed ? new Date().toISOString() : null },
    } : item));
    setGoalDataRevision((value) => value + 1);
  }

  async function openTask(id: string) {
    const source = tasks.find((item) => item.sourceId === id);
    if (source?.sourceKind === "recurringInstance" && source.recurrenceRuleId) {
      const result = await recurrenceClient.get(source.recurrenceRuleId);
      if (!result.ok) {
        setTaskError(domainErrorMessage(result.error, i18n.t));
        return;
      }
      const templateResult = await taskClient.get(result.data.taskTemplateId);
      if (!templateResult.ok) {
        setTaskError(domainErrorMessage(templateResult.error, i18n.t));
        return;
      }
      setEditingRecurringTask({ instanceId: id, effectiveOn: source.task.scheduledDate ?? selectedDate, rule: result.data, template: templateResult.data });
      return;
    }
    if (desktopRuntime) {
      const result = await taskClient.get(id);
      if (!result.ok) {
        setTaskError(domainErrorMessage(result.error, i18n.t));
        return;
      }
      setTasks((items) => items.map((item) => item.task.id === id ? {
        ...item,
        task: result.data.task,
        checkItems: result.data.checkItems.map((checkItem) => ({ id: checkItem.id, title: checkItem.title, completed: Boolean(checkItem.completedAt) })),
      } : item));
    }
    setEditingTaskId(id);
  }

  function startFocus(id: string) {
    setFocusedTask(tasks.find((item) => item.sourceId === id) ?? null);
    setPage("focus");
  }

  function openNewTask(projectId: string | null = null) {
    setDraftProjectId(projectId);
    setEditingTaskId("new");
  }

  function startProjectFocus(task: WorkspaceTask["task"], project: ProjectRecord) {
    const projectSummary = projectSummaryToTaskProject({ project });
    setFocusedTask({
      sourceKind: "task",
      sourceId: task.id,
      recurrenceRuleId: null,
      recurrenceLabel: null,
      task,
      project: projectSummary,
      checkItems: [],
      visualState: project.status === "paused" ? "paused" : task.status === "completed" ? "completed" : "normal",
    });
    setPage("focus");
  }

  async function refreshDigest() {
    if (!desktopRuntime) return;
    const result = await todayClient.getDigest(selectedDate);
    if (result.ok) setTasks(decorateTodayDigest(result.data.items));
    else setTaskError(domainErrorMessage(result.error, i18n.t));
  }

  async function skipInstance(id: string) {
    const result = await recurrenceClient.skip(id);
    if (!result.ok) setTaskError(domainErrorMessage(result.error, i18n.t));
    else {
      await refreshDigest();
      setGoalDataRevision((value) => value + 1);
    }
  }

  async function delayInstance(id: string) {
    const current = tasks.find((item) => item.sourceId === id)?.task.scheduledTime ?? "09:00";
    const localTime = window.prompt("延后到今天几点？请输入 HH:MM", current);
    if (!localTime) return;
    const result = await recurrenceClient.delayToday(id, localTime);
    if (!result.ok) setTaskError(domainErrorMessage(result.error, i18n.t));
    else {
      await refreshDigest();
      setGoalDataRevision((value) => value + 1);
    }
  }

  async function rescheduleInstance(id: string) {
    const result = await recurrenceClient.rescheduleTomorrow(id);
    if (!result.ok) setTaskError(domainErrorMessage(result.error, i18n.t));
    else {
      await refreshDigest();
      setGoalDataRevision((value) => value + 1);
    }
  }

  async function saveNote(body: string) {
    if (desktopRuntime) {
      const result = await planningClient.saveNote({ noteDate: selectedDate, body });
      if (!result.ok) throw new Error(domainErrorMessage(result.error, i18n.t));
      setNoteBody(result.data.body);
      return;
    }
    setNoteBody(body);
  }

  async function saveWeeklyGoal(input: WeeklyGoalInput) {
    if (desktopRuntime) {
      const result = await planningClient.saveWeeklyGoal(input, timezone);
      if (!result.ok) throw new Error(domainErrorMessage(result.error, i18n.t));
      setWeeklyGoals((goals) => [...goals.filter((goal) => goal.id !== result.data.id), result.data].sort((left, right) => left.position - right.position));
      return;
    }
    const progress = previewGoalProgress(input.category, tasks);
    setWeeklyGoals((goals) => [...goals, {
      ...input,
      id: crypto.randomUUID(),
      completedCount: Math.min(input.targetCount, progress),
      position: goals.length,
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    }]);
  }

  async function updateRecurrence(rule: RecurrenceRule, scope: RecurrenceChangeScope) {
    const effectiveOn = scope.scope === "future" ? scope.effectiveOn : editingRecurrence?.effectiveOn ?? selectedDate;
    const result = await recurrenceClient.update(rule, scope, rule.endsOn ?? addOneYear(effectiveOn));
    if (!result.ok) {
      setTaskError(domainErrorMessage(result.error, i18n.t));
      return;
    }
    setEditingRecurrence(null);
    await refreshDigest();
  }

  async function saveRecurringTask(input: TaskInput) {
    if (!editingRecurringTask) return;
    if (desktopRuntime) {
      const validationDate = editingRecurringTask.template.task.scheduledDate ?? today;
      const templateResult = await taskClient.update(editingRecurringTask.rule.taskTemplateId, input, validationDate);
      if (!templateResult.ok) {
        setTaskError(domainErrorMessage(templateResult.error, i18n.t));
        return;
      }
      const proposed = { ...editingRecurringTask.rule, version: editingRecurringTask.rule.version + 1 };
      const recurrenceResult = await recurrenceClient.update(proposed, { scope: "future", effectiveOn: editingRecurringTask.effectiveOn }, proposed.endsOn ?? addOneYear(editingRecurringTask.effectiveOn));
      if (!recurrenceResult.ok) {
        setTaskError(domainErrorMessage(recurrenceResult.error, i18n.t));
        return;
      }
      setEditingRecurringTask(null);
      await refreshDigest();
      setGoalDataRevision((value) => value + 1);
      return;
    }
    setEditingRecurringTask(null);
  }

  async function setRecurrenceStatus(status: "paused" | "ended") {
    if (!editingRecurrence) return;
    const result = await recurrenceClient.setStatus(editingRecurrence.rule.id, status);
    if (!result.ok) {
      setTaskError(domainErrorMessage(result.error, i18n.t));
      return;
    }
    setEditingRecurrence(null);
    await refreshDigest();
  }

  return (
    <I18nProvider locale={locale}>
    <div className="app-shell" data-theme={theme} data-mode={mode} data-locale={locale} style={themeStyle(tokens)}>
      <aside className="sidebar">
        <div className="brand"><span className="brand__mark">{i18n.t("app.brandMark")}</span><div><strong>{i18n.t("app.title")}</strong><small>Arrive Focus</small></div></div>
        <nav aria-label={i18n.t("nav.label")}>
          {pages.map((item) => <button key={item.id} className={page === item.id ? "active" : ""} aria-current={page === item.id ? "page" : undefined} onClick={() => setPage(item.id)}><Icon name={item.icon} /><span>{i18n.t(item.labelKey)}</span></button>)}
        </nav>
        <div className="sidebar__footer">
          <div className="companion"><span>AF</span><p>{i18n.t("app.tagline")}</p></div>
          <SegmentedControl label={i18n.t("theme.colorMode")} value={mode} options={colorModes.map((value) => ({ value, label: i18n.t(value === "light" ? "common.light" : "common.dark") }))} onChange={(appearance) => void saveGeneralSettings({ appearance }).catch(() => setTaskError(i18n.t("common.saveFailed")))} />
        </div>
      </aside>

      <main className="main-content">
        <header className="page-header">
          <div>{page !== "memos" ? <span className="eyebrow">{formatDateHeading(selectedDate, i18n)}</span> : null}<h1>{i18n.t(currentPage.labelKey)}</h1><p>{pageDescription(page, focusedTask, i18n)}</p></div>
          <div className="header-actions">
            <select aria-label={i18n.t("theme.label")} value={theme} onChange={(event) => void saveGeneralSettings({ theme: event.target.value as ThemeName }).catch(() => setTaskError(i18n.t("common.saveFailed")))}>
              {themeNames.map((name) => <option key={name} value={name}>{i18n.t(`theme.${name}`)}</option>)}
            </select>
            {page !== "memos" ? <Button tone="primary" onClick={() => openNewTask()}><Icon name="plus" />{i18n.t("task.new")}</Button> : null}
          </div>
        </header>

        {page !== "focus" && page !== "calendar" && page !== "memos" ? (
          <div className="week-strip" aria-label={i18n.t("week.label")}>
            {week.map((item) => <button key={item.date} className={item.date === selectedDate ? "active" : ""} aria-pressed={item.date === selectedDate} onClick={() => setSelectedDate(item.date)}><span>{item.dayLabel}</span><strong>{item.dateLabel}</strong>{item.date === today ? <small>{i18n.t("common.today")}</small> : null}</button>)}
          </div>
        ) : null}

        {page === "today" ? <TodayWorkspace tasks={visibleTasks} loading={loadingTasks} noteDate={selectedDate} weekStartsOn={weekStartsOn} noteBody={noteBody} weeklyGoals={weeklyGoals} planningLoading={noteLoading || goalsLoading} onSaveNote={saveNote} onSaveWeeklyGoal={saveWeeklyGoal} onCreate={() => openNewTask()} onEdit={(id) => void openTask(id)} onToggleCompleted={(id, completed) => void toggleTaskCompleted(id, completed)} onStartFocus={startFocus} onSkipInstance={(id) => void skipInstance(id)} onDelayInstance={(id) => void delayInstance(id)} onRescheduleInstance={(id) => void rescheduleInstance(id)} /> : null}
        {page === "memos" ? <MemoWorkspace dataRevision={memoDataRevision} openRequest={memoOpenRequest} initialQuery={memoQuery} onQueryChange={setMemoQuery} /> : null}
        {page === "projects" ? <ProjectWorkspace today={today} runtime={desktopRuntime} onProjectsChange={(summaries) => setProjectOptions(summaries.map(projectSummaryToTaskProject))} onAddTask={(project) => openNewTask(project.id)} onStartFocus={startProjectFocus} onTaskChange={async () => { await refreshDigest(); setGoalDataRevision((value) => value + 1); }} /> : null}
        {page === "focus" ? <FocusWorkspace tasks={tasks} initialTask={focusedTask} /> : null}
        {page === "settings" ? <SettingsWorkspace general={generalSettings} onSaveGeneral={saveGeneralSettings} /> : null}
        {page === "calendar" ? <CalendarWorkspace selectedDate={selectedDate} onSelectDate={setSelectedDate} runtime={desktopRuntime} onStartFocus={() => setPage("focus")} /> : null}
      </main>

      <Dialog open={editingTaskId !== null} title={i18n.t(editingTask ? "task.edit" : "task.create")} onClose={() => { setEditingTaskId(null); setDraftProjectId(null); }}>
        {editingTaskId ? <TaskEditor key={`${editingTaskId}-${draftProjectId ?? "none"}`} today={today} projects={projectOptions} initialValue={editingTask ? taskToInput(editingTask) : { ...emptyTaskInput, projectId: draftProjectId, scheduledDate: selectedDate < today ? today : selectedDate }} submitLabel={i18n.t(editingTask ? "task.save" : "task.create")} onCancel={() => { setEditingTaskId(null); setDraftProjectId(null); }} onSubmit={saveTask} /> : null}
      </Dialog>
      <Dialog open={editingRecurringTask !== null} title={i18n.t("task.edit")} onClose={() => setEditingRecurringTask(null)}>
        {editingRecurringTask ? <div className="recurrence-scope-editor">
          <div className="recurrence-rule-actions"><span>{i18n.t("task.recurring")}</span><Button tone="ghost" onClick={() => { setEditingRecurrence({ instanceId: editingRecurringTask.instanceId, effectiveOn: editingRecurringTask.effectiveOn, rule: editingRecurringTask.rule }); setEditingRecurringTask(null); }}>{i18n.t("task.recurrenceEdit")}</Button></div>
          <TaskEditor key={`${editingRecurringTask.instanceId}-${editingRecurringTask.template.task.updatedAt}`} today={editingRecurringTask.template.task.scheduledDate ?? today} projects={projectOptions} initialValue={taskDetailToInput(editingRecurringTask.template)} submitLabel={i18n.t("task.save")} showRecurrence={false} onCancel={() => setEditingRecurringTask(null)} onSubmit={(input) => saveRecurringTask(input)} />
        </div> : null}
      </Dialog>
      <Dialog open={editingRecurrence !== null} title={i18n.t("task.recurrenceEdit")} onClose={() => setEditingRecurrence(null)}>
        {editingRecurrence ? <RecurrenceScopeEditor instanceId={editingRecurrence.instanceId} effectiveOn={editingRecurrence.effectiveOn} rule={editingRecurrence.rule} onCancel={() => setEditingRecurrence(null)} onSubmit={updateRecurrence} onSetStatus={setRecurrenceStatus} /> : null}
      </Dialog>
      {taskError ? <div onClick={() => setTaskError(null)}><Toast tone="danger">{taskError}</Toast></div> : null}
    </div>
    </I18nProvider>
  );
}

const emptyTaskInput: TaskInput = { projectId: null, title: "", category: "work", priority: 0, scheduledDate: null, scheduledTime: null, checkItems: [] };

function taskToInput(item: WorkspaceTask): TaskInput {
  const { projectId, title, category, priority, scheduledDate, scheduledTime } = item.task;
  return { projectId, title, category, priority, scheduledDate, scheduledTime, checkItems: item.checkItems };
}

function taskDetailToInput(detail: TaskDetail): TaskInput {
  const { projectId, title, category, priority, scheduledDate, scheduledTime } = detail.task;
  return { projectId, title, category, priority, scheduledDate, scheduledTime, checkItems: detail.checkItems.map((item) => ({ id: item.id, title: item.title, completed: Boolean(item.completedAt) })) };
}

function makeTask(id: string, title: string, projectId: string | null, category: TaskInput["category"], scheduledTime: string, visualState: TaskVisualState, priority: number, checks: string[] = [], scheduledDate = initialToday): WorkspaceTask {
  const timestamp = `${scheduledDate}T08:00:00.000Z`;
  return {
    sourceKind: "task",
    sourceId: id,
    recurrenceRuleId: null,
    recurrenceLabel: null,
    project: previewTaskProjects.find((project) => project.id === projectId) ?? null,
    visualState,
    checkItems: checks.map((checkTitle, index) => ({ id: `${id}-check-${index}`, title: checkTitle, completed: false })),
    task: {
      id,
      projectId,
      title,
      category,
      priority,
      scheduledDate,
      scheduledTime,
      status: visualState === "completed" ? "completed" : "pending",
      completedAt: visualState === "completed" ? timestamp : null,
      createdAt: timestamp,
      updatedAt: timestamp,
    },
  };
}

function makeWeeklyGoal(id: string, title: string, category: WeeklyGoalCategory, targetCount: number, position: number): WeeklyGoal {
  const timestamp = new Date().toISOString();
  return { id, title, category, targetCount, position, weekStartsOn: previewWeekStartsOn, completedCount: 0, createdAt: timestamp, updatedAt: timestamp };
}

function previewGoalProgress(category: WeeklyGoalCategory, tasks: WorkspaceTask[]): number {
  if (category === "completedTasks") return tasks.filter((item) => item.task.status === "completed").length;
  if (category === "activeDays") return new Set(tasks.flatMap((item) => item.task.completedAt ? [item.task.completedAt.slice(0, 10)] : [])).size;
  return 0;
}

function addOneYear(value: string): string {
  const [year, month, day] = value.split("-").map(Number);
  return localDateString(new Date(year + 1, month - 1, day, 12));
}

function formatDateHeading(value: string, i18n: I18nValue): string {
  return i18n.formatDate(value, { year: "numeric", month: "long", day: "numeric", weekday: "long" });
}

function pageDescription(page: (typeof pages)[number]["id"], focusedTask: WorkspaceTask | null, i18n: I18nValue): string {
  if (page === "today") return i18n.t("page.todayDescription");
  if (page === "memos") return i18n.t("page.memoDescription");
  if (page === "calendar") return i18n.t("page.calendarDescription");
  if (page === "focus") return focusedTask ? i18n.t("page.focusTaskDescription", { title: focusedTask.task.title }) : i18n.t("page.focusDescription");
  return i18n.t("page.futureDescription");
}

function projectSummaryToTaskProject(summary: Pick<ProjectSummary, "project">): TaskProjectSummary {
  const { id, name, color, icon, status } = summary.project;
  return { id, name, color, icon, status };
}

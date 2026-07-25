import { useEffect, useState, type CSSProperties, type FormEvent } from "react";

import { Badge, Button, Dialog, Panel, Progress, SegmentedControl } from "../../components/ui";
import { useI18n } from "../../i18n/I18nContext";
import type { MessageKey } from "../../i18n/messages";
import { isTauriRuntime } from "../../lib/commandClient";
import { domainErrorMessage } from "../../lib/domainError";
import { taskClient } from "../tasks/taskClient";
import type { TaskRecord } from "../tasks/types";
import { localDateString } from "../today/todayModel";
import { projectClient, type ProjectClient } from "./projectClient";
import type { ProjectDetail, ProjectInput, ProjectRecord, ProjectStatus, ProjectSummary } from "./types";

type ProjectTab = "overview" | "tasks" | "activity" | "statistics";

type ProjectTaskActions = {
  setCompleted(id: string, completed: boolean): ReturnType<typeof taskClient.setCompleted>;
};

export type ProjectWorkspaceProps = {
  today?: string;
  runtime?: boolean;
  client?: ProjectClient;
  taskActions?: ProjectTaskActions;
  onProjectsChange?: (projects: ProjectSummary[]) => void;
  onAddTask?: (project: ProjectRecord) => void;
  onStartFocus?: (task: TaskRecord, project: ProjectRecord) => void;
  onTaskChange?: () => void | Promise<void>;
};

const filterValues = ["all", "active", "paused", "completed", "archived"] as const;
const tabValues: ProjectTab[] = ["overview", "tasks", "activity", "statistics"];

export function ProjectWorkspace({
  today = localDateString(),
  runtime = isTauriRuntime(),
  client = projectClient,
  taskActions = taskClient,
  onProjectsChange,
  onAddTask,
  onStartFocus,
  onTaskChange,
}: ProjectWorkspaceProps) {
  const { t } = useI18n();
  const [projects, setProjects] = useState<ProjectSummary[]>(runtime ? [] : previewProjects);
  const [filter, setFilter] = useState<ProjectStatus | "all">("all");
  const [selectedId, setSelectedId] = useState<string | null>(runtime ? null : previewProjects[0].project.id);
  const [detail, setDetail] = useState<ProjectDetail | null>(runtime ? null : previewDetails[previewProjects[0].project.id]);
  const [tab, setTab] = useState<ProjectTab>("overview");
  const [dialogMode, setDialogMode] = useState<"create" | "edit" | null>(null);
  const [loading, setLoading] = useState(runtime);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const selected = projects.find((summary) => summary.project.id === selectedId) ?? null;
  const selectedDetail = detail?.summary.project.id === selectedId ? detail : null;
  const visibleProjects = runtime ? projects : projects.filter((summary) => filter === "all" || summary.project.status === filter);
  const filterOptions = filterValues.map((value) => ({
    value,
    label: t(value === "all" ? "common.all" : value === "active" ? "project.status.active" : `project.filter.${value}` as MessageKey),
  }));
  const tabOptions = tabValues.map((value) => ({ value, label: t(`project.tab.${value}`) }));

  useEffect(() => {
    if (!runtime) return;
    let active = true;
    setLoading(true);
    setError(null);
    void client.list(filter === "all" ? null : filter, today).then((result) => {
      if (!active) return;
      if (result.ok) {
        setProjects(result.data);
        setSelectedId((current) => result.data.some((summary) => summary.project.id === current) ? current : result.data[0]?.project.id ?? null);
        if (filter === "all") onProjectsChange?.(result.data);
      } else {
        setError(domainErrorMessage(result.error, t));
      }
      setLoading(false);
    });
    return () => { active = false; };
  }, [client, filter, runtime, t, today]);

  useEffect(() => {
    if (!runtime || !selectedId) {
      if (runtime) setDetail(null);
      return;
    }
    let active = true;
    setLoading(true);
    void client.get(selectedId, today).then((result) => {
      if (!active) return;
      if (result.ok) setDetail(result.data);
      else setError(domainErrorMessage(result.error, t));
      setLoading(false);
    });
    return () => { active = false; };
  }, [client, runtime, selectedId, t, today]);

  async function reloadProjects(preferredId: string | null = selectedId, status: ProjectStatus | "all" = filter) {
    const result = await client.list(status === "all" ? null : status, today);
    if (!result.ok) {
      setError(domainErrorMessage(result.error, t));
      return;
    }
    setProjects(result.data);
    if (status === "all") onProjectsChange?.(result.data);
    const nextId = result.data.some((summary) => summary.project.id === preferredId) ? preferredId : result.data[0]?.project.id ?? null;
    setSelectedId(nextId);
    if (!nextId) {
      setDetail(null);
      return;
    }
    const detailResult = await client.get(nextId, today);
    if (detailResult.ok) setDetail(detailResult.data);
    else setError(domainErrorMessage(detailResult.error, t));
  }

  async function saveProject(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const formData = new FormData(event.currentTarget);
    const input: ProjectInput = {
      name: String(formData.get("name") ?? "").trim(),
      description: String(formData.get("description") ?? ""),
      color: String(formData.get("color") ?? "#4eaa98"),
      icon: String(formData.get("icon") ?? "AF").trim(),
      startedOn: String(formData.get("startedOn") ?? today),
      targetOn: String(formData.get("targetOn") ?? "") || null,
    };
    setSaving(true);
    setError(null);
    if (!runtime) {
      const now = new Date().toISOString();
      const project: ProjectRecord = dialogMode === "edit" && selected
        ? { ...selected.project, ...input, updatedAt: now }
        : { ...input, id: crypto.randomUUID(), status: "active", createdAt: now, updatedAt: now };
      const summary = makePreviewSummary(project);
      setProjects((items) => dialogMode === "edit" ? items.map((item) => item.project.id === project.id ? { ...item, project } : item) : [...items, summary]);
      setDetail((current) => current?.summary.project.id === project.id ? { ...current, summary: { ...current.summary, project } } : { summary, tasks: [] });
      setSelectedId(project.id);
      setDialogMode(null);
      setSaving(false);
      return;
    }
    const result = dialogMode === "edit" && selected
      ? await client.update(selected.project.id, input)
      : await client.create(input);
    if (!result.ok) {
      setError(domainErrorMessage(result.error, t));
      setSaving(false);
      return;
    }
    setDialogMode(null);
    setFilter("all");
    await reloadProjects(result.data.id, "all");
    setSaving(false);
  }

  async function changeStatus(status: ProjectStatus) {
    if (!selected) return;
    setSaving(true);
    setError(null);
    if (!runtime) {
      setProjects((items) => items.map((item) => item.project.id === selected.project.id ? { ...item, project: { ...item.project, status } } : item));
      setDetail((current) => current ? { ...current, summary: { ...current.summary, project: { ...current.summary.project, status } } } : current);
      if (status === "archived") setFilter("all");
      setSaving(false);
      return;
    }
    const result = await client.setStatus(selected.project.id, status);
    if (!result.ok) setError(domainErrorMessage(result.error, t));
    else {
      const nextFilter = status === "archived" ? "all" : filter;
      if (status === "archived") setFilter("all");
      await reloadProjects(selected.project.id, nextFilter);
    }
    setSaving(false);
  }

  async function toggleTask(task: TaskRecord) {
    const completed = task.status !== "completed";
    if (!runtime) {
      setDetail((current) => current ? { ...current, tasks: current.tasks.map((item) => item.id === task.id ? { ...item, status: completed ? "completed" : "pending", completedAt: completed ? new Date().toISOString() : null } : item) } : current);
      return;
    }
    const result = await taskActions.setCompleted(task.id, completed);
    if (!result.ok) {
      setError(domainErrorMessage(result.error, t));
      return;
    }
    await reloadProjects(selectedId);
    await onTaskChange?.();
  }

  return (
    <div className="projects-workspace">
      <aside className="project-browser">
        <div className="project-browser__header"><div><span className="eyebrow">{t("project.direction")}</span><h2>{t("project.title")}</h2></div><Button tone="primary" onClick={() => setDialogMode("create")}>{t("project.new")}</Button></div>
        <SegmentedControl label={t("project.filterLabel")} options={filterOptions} value={filter} onChange={setFilter} />
        <div className="project-browser__list">
          {loading && visibleProjects.length === 0 ? <p className="project-browser__state">{t("project.loading")}</p> : null}
          {!loading && visibleProjects.length === 0 ? <p className="project-browser__state">{t("project.empty")}</p> : null}
          {visibleProjects.map((summary) => {
            const project = summary.project;
            return (
              <button key={project.id} className={`project-nav-card ${project.id === selectedId ? "active" : ""}`} aria-pressed={project.id === selectedId} onClick={() => setSelectedId(project.id)} style={{ "--project-color": project.color } as CSSProperties}>
                <span className="project-nav-card__color" /><span className="project-nav-card__body"><strong>{project.name}</strong><small>{t(`project.status.${project.status}`)} · {summary.aggregation.completionPercent}%</small><Progress label={t("project.progressLabel", { name: project.name })} value={summary.aggregation.completionPercent} /></span>
              </button>
            );
          })}
        </div>
      </aside>

      <section className="project-detail">
        {error ? <p className="field__error" role="alert">{error}</p> : null}
        {selected ? <><header className="project-detail__header" style={{ "--project-color": selected.project.color } as CSSProperties}>
          <div className="project-symbol">{selected.project.icon.slice(0, 2).toUpperCase()}</div>
          <div><Badge tone={selected.project.status === "active" ? "accent" : "neutral"}>{t(`project.status.${selected.project.status}`)}</Badge><h2>{selected.project.name}</h2><p>{selected.project.description}</p></div>
          <Button tone="ghost" onClick={() => setDialogMode("edit")}>{t("common.edit")}</Button>
        </header>
        <SegmentedControl label={t("project.detailTabs")} options={tabOptions} value={tab} onChange={setTab} />

        {tab === "overview" ? <ProjectOverview summary={selectedDetail?.summary ?? selected} saving={saving} onStatusChange={changeStatus} /> : null}
        {tab === "tasks" ? <ProjectTasks detail={selectedDetail} onAddTask={() => onAddTask?.(selected.project)} onToggleTask={toggleTask} /> : null}
        {tab === "activity" ? <ProjectActivity detail={selectedDetail} /> : null}
        {tab === "statistics" ? <ProjectStatistics summary={selectedDetail?.summary ?? selected} /> : null}</> : <Panel className="project-tab-panel"><p>{loading ? t("project.loading") : t("project.emptySelection")}</p></Panel>}
      </section>

      <aside className="project-insights">
        {selected ? <><Panel><span className="eyebrow">{t("project.next")}</span><h2>{selectedDetail?.summary.nextTaskTitle ?? selected.nextTaskTitle ?? t("project.firstTask")}</h2><p>{t("project.nextHint")}</p><Button tone="primary" disabled={!selectedDetail?.tasks.some((task) => task.status === "pending") || !onStartFocus} onClick={() => { const task = selectedDetail?.tasks.find((item) => item.status === "pending"); if (task) onStartFocus?.(task, selected.project); }}>{t("project.startFocus")}</Button></Panel>
        <DeadlinePanel summary={selectedDetail?.summary ?? selected} /></> : null}
      </aside>

      <Dialog open={dialogMode !== null} title={t(dialogMode === "create" ? "project.create" : "project.edit")} onClose={() => setDialogMode(null)}>
        <form className="project-form" onSubmit={saveProject}>
          <label><span>{t("project.name")}</span><input name="name" required maxLength={80} defaultValue={dialogMode === "edit" ? selected?.project.name : ""} /></label>
          <label><span>{t("project.description")}</span><textarea name="description" maxLength={2000} defaultValue={dialogMode === "edit" ? selected?.project.description : ""} /></label>
          <label><span>{t("project.color")}</span><input name="color" type="color" defaultValue={dialogMode === "edit" ? selected?.project.color : "#4eaa98"} /></label>
          <label><span>{t("project.icon")}</span><input name="icon" required maxLength={16} defaultValue={dialogMode === "edit" ? selected?.project.icon : "AF"} /></label>
          <label><span>{t("project.startDate")}</span><input name="startedOn" type="date" required defaultValue={dialogMode === "edit" ? selected?.project.startedOn : today} /></label>
          <label><span>{t("project.targetDate")}</span><input name="targetOn" type="date" min={dialogMode === "edit" ? selected?.project.startedOn : today} defaultValue={dialogMode === "edit" ? selected?.project.targetOn ?? "" : ""} /></label>
          {error ? <small className="field__error" role="alert">{error}</small> : null}
          <footer><Button type="button" tone="ghost" onClick={() => setDialogMode(null)}>{t("common.cancel")}</Button><Button type="submit" tone="primary" disabled={saving}>{saving ? t("common.saving") : t("project.save")}</Button></footer>
        </form>
      </Dialog>
    </div>
  );
}

function ProjectOverview({ summary, saving, onStatusChange }: { summary: ProjectSummary; saving: boolean; onStatusChange: (status: ProjectStatus) => void }) {
  const { t } = useI18n();
  const { project, aggregation } = summary;
  const focusMinutes = Math.floor(aggregation.focusSeconds / 60);
  return <div className="project-overview"><Panel><span className="eyebrow">{t("project.taskProgress")}</span><div className="project-stat"><strong>{aggregation.completionPercent}%</strong><span>{t("project.completedCount", { completed: aggregation.completedTaskCount, total: aggregation.totalTaskCount })}</span></div><Progress label={t("project.taskProgressLabel")} value={aggregation.completionPercent} /></Panel><Panel><span className="eyebrow">{t("project.focusInvestment")}</span><div className="project-stat"><strong>{Math.floor(focusMinutes / 60)}h</strong><span>{t("common.minutes", { count: focusMinutes % 60 })}</span></div></Panel><Panel className="project-brief"><span className="eyebrow">{t("project.description")}</span><p>{project.description || t("project.noDescription")}</p><div className="project-status-actions">{project.status === "active" ? <Button tone="ghost" disabled={saving} onClick={() => onStatusChange("paused")}>{t("project.pause")}</Button> : null}{project.status === "paused" || project.status === "completed" ? <Button tone="ghost" disabled={saving} onClick={() => onStatusChange("active")}>{t("project.resume")}</Button> : null}{project.status === "active" || project.status === "paused" ? <Button tone="ghost" disabled={saving} onClick={() => onStatusChange("completed")}>{t("project.complete")}</Button> : null}{project.status !== "archived" ? <Button tone="ghost" disabled={saving} onClick={() => onStatusChange("archived")}>{t("project.archive")}</Button> : null}</div></Panel></div>;
}

function ProjectTasks({ detail, onAddTask, onToggleTask }: { detail: ProjectDetail | null; onAddTask: () => void; onToggleTask: (task: TaskRecord) => void }) {
  const { t } = useI18n();
  const tasks = detail?.tasks ?? [];
  const pending = tasks.filter((task) => task.status === "pending").length;
  return <Panel className="project-tab-panel"><div className="section-heading"><div><span className="eyebrow">{t("project.activeTasks")}</span><h2>{t("project.pendingCount", { count: pending })}</h2></div><Button tone="primary" onClick={onAddTask}>{t("project.addTask")}</Button></div>{tasks.length === 0 ? <p>{t("project.noTasks")}</p> : tasks.filter((task) => task.status !== "removed").map((task) => <article className="project-task-item" key={task.id}><button className={`task-check ${task.status === "completed" ? "done" : ""}`} aria-label={t(task.status === "completed" ? "task.restoreLabel" : "task.completeLabel", { title: task.title })} onClick={() => onToggleTask(task)} /><div><strong>{task.title}</strong><small>{task.scheduledDate ?? t("project.unscheduled")}</small></div><Badge>{t(task.status === "completed" ? "task.state.completed" : "task.state.normal")}</Badge></article>)}</Panel>;
}

function ProjectActivity({ detail }: { detail: ProjectDetail | null }) {
  const { formatDate, t } = useI18n();
  const tasks = [...(detail?.tasks ?? [])].filter((task) => task.status !== "removed").sort((left, right) => right.updatedAt.localeCompare(left.updatedAt)).slice(0, 8);
  return <Panel className="project-tab-panel"><span className="eyebrow">{t("project.recentActivity")}</span>{tasks.length === 0 ? <p>{t("project.noActivity")}</p> : tasks.map((task) => <div className="activity-item" key={task.id}><span /><div><strong>{task.title}</strong><small>{t(task.status === "completed" ? "project.activityCompleted" : "project.activityUpdated")} · {formatDate(task.updatedAt, { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" })}</small></div></div>)}</Panel>;
}

function ProjectStatistics({ summary }: { summary: ProjectSummary }) {
  const { t } = useI18n();
  const focusMinutes = Math.floor(summary.aggregation.focusSeconds / 60);
  const average = summary.aggregation.completedTaskCount ? Math.round(focusMinutes / summary.aggregation.completedTaskCount) : 0;
  return <div className="project-statistics"><Panel><span className="eyebrow">{t("project.taskProgress")}</span><div className="project-stat"><strong>{summary.aggregation.activeTaskCount}</strong><span>{t("project.pendingCount", { count: summary.aggregation.activeTaskCount })}</span></div><Progress label={t("project.taskProgressLabel")} value={summary.aggregation.completionPercent} /></Panel><Panel><span className="eyebrow">{t("project.totalInvestment")}</span><div className="project-stat"><strong>{focusMinutes}</strong><span>{t("project.focusMinutes")}</span></div><p>{t("project.averageMinutes", { count: average })}</p></Panel></div>;
}

function DeadlinePanel({ summary }: { summary: ProjectSummary }) {
  const { t } = useI18n();
  const days = summary.deadlineDays;
  const displayDays = days === null ? "--" : Math.abs(days);
  const message = summary.deadlineState === "none" ? t("project.deadlineNone") : summary.deadlineState === "overdue" ? t("project.deadlineOverdue") : summary.deadlineState === "atRisk" ? t("project.deadlineAtRisk") : t("project.riskStable");
  return <Panel><span className="eyebrow">{t("project.deadlineRisk")}</span><div className="deadline-number">{displayDays}<small>{days === null ? "" : t("project.days")}</small></div><p>{message}</p></Panel>;
}

const previewProjectRecords: ProjectRecord[] = [
  { id: "focus", name: "抵达 Focus", description: "构建安静、可靠的 Windows 本地专注系统。", status: "active", color: "#4eaa98", icon: "AF", startedOn: "2026-07-18", targetOn: "2026-08-18", createdAt: "2026-07-18T08:00:00.000Z", updatedAt: "2026-07-21T08:00:00.000Z" },
  { id: "writing", name: "夏季写作计划", description: "完成十二篇主题随笔，并形成稳定写作节奏。", status: "active", color: "#647fbd", icon: "WR", startedOn: "2026-07-01", targetOn: "2026-09-01", createdAt: "2026-07-01T08:00:00.000Z", updatedAt: "2026-07-20T08:00:00.000Z" },
  { id: "study", name: "系统设计复习", description: "整理核心模式与实践案例。", status: "paused", color: "#c18471", icon: "SD", startedOn: "2026-07-05", targetOn: "2026-10-10", createdAt: "2026-07-05T08:00:00.000Z", updatedAt: "2026-07-19T08:00:00.000Z" },
];

function makePreviewSummary(project: ProjectRecord): ProjectSummary {
  return { project, aggregation: { activeTaskCount: 0, completedTaskCount: 0, totalTaskCount: 0, completionPercent: 0, focusSeconds: 0 }, nextTaskTitle: null, nextTaskDate: null, deadlineState: project.targetOn ? "onTrack" : "none", deadlineDays: project.targetOn ? 31 : null };
}

const previewProjects = previewProjectRecords.map((project, index) => ({
  ...makePreviewSummary(project),
  aggregation: { activeTaskCount: [13, 8, 11][index], completedTaskCount: [11, 4, 7][index], totalTaskCount: [24, 12, 18][index], completionPercent: [45, 33, 38][index], focusSeconds: [47_100, 25_200, 33_600][index] },
  nextTaskTitle: ["实现长期项目领域", "整理第三篇文章结构", "复习一致性模型"][index],
}));

const previewDetails: Record<string, ProjectDetail> = Object.fromEntries(previewProjects.map((summary) => [summary.project.id, { summary, tasks: [] }]));

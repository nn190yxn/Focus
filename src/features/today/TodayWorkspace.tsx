import { useEffect, useRef, useState, type FormEvent } from "react";

import { Icon } from "../../components/Icon";
import { Badge, Button, Panel, Progress, SegmentedControl } from "../../components/ui";
import { useI18n } from "../../i18n/I18nContext";
import type { MessageKey } from "../../i18n/messages";
import { TaskRow } from "../tasks/TaskRow";
import type { WeeklyGoal, WeeklyGoalCategory, WeeklyGoalInput } from "./planningTypes";
import { scheduledTasks, taskSections, type WorkspaceTask } from "./todayModel";

type TodayWorkspaceProps = {
  tasks: WorkspaceTask[];
  loading?: boolean;
  onCreate: () => void;
  onEdit: (id: string) => void;
  onToggleCompleted: (id: string, completed: boolean) => void;
  onStartFocus: (id: string) => void;
  onSkipInstance?: (id: string) => void;
  onDelayInstance?: (id: string) => void;
  onRescheduleInstance?: (id: string) => void;
  noteDate?: string;
  weekStartsOn?: string;
  noteBody?: string;
  weeklyGoals?: WeeklyGoal[];
  planningLoading?: boolean;
  onSaveNote?: (body: string) => Promise<void>;
  onSaveWeeklyGoal?: (input: WeeklyGoalInput) => Promise<void>;
};

const completionValues = ["all", "pending", "completed"] as const;
const goalCategories: WeeklyGoalCategory[] = ["completedTasks", "focusMinutes", "activeDays"];

export function TodayWorkspace({ tasks, loading = false, onCreate, onEdit, onToggleCompleted, onStartFocus, onSkipInstance, onDelayInstance, onRescheduleInstance, noteDate = "", weekStartsOn = "", noteBody = "", weeklyGoals = [], planningLoading = false, onSaveNote = async () => undefined, onSaveWeeklyGoal = async () => undefined }: TodayWorkspaceProps) {
  const { t } = useI18n();
  const [completion, setCompletion] = useState<(typeof completionValues)[number]>("all");
  const completionOptions = completionValues.map((value) => ({ value, label: t(value === "all" ? "common.all" : `today.filter.${value}` as MessageKey) }));
  const categoryLabels = { work: t("task.category.work"), study: t("task.category.study"), health: t("task.category.health"), life: t("task.category.life") };
  const completedCount = tasks.filter((item) => item.task.status === "completed").length;
  const sections = taskSections(tasks, completion, categoryLabels);
  const schedule = scheduledTasks(tasks);
  const goalCompleted = weeklyGoals.reduce((total, goal) => total + goal.completedCount, 0);
  const goalTarget = weeklyGoals.reduce((total, goal) => total + goal.targetCount, 0);
  const goalProgress = goalTarget === 0 ? 0 : Math.round((goalCompleted / goalTarget) * 100);

  return (
    <div className="today-grid" aria-busy={loading}>
      <Panel className="goal-panel">
        <span className="eyebrow">{t("today.direction")}</span>
        <h2>{t("today.directionTitle")}</h2>
        <Progress label={t("today.goalProgress")} value={goalProgress} />
        <p className="metric"><strong>{goalCompleted}</strong> / {goalTarget} {t("today.progress")}</p>
        <div className="weekly-goals" aria-label={t("today.weeklyGoals")}>
          {planningLoading ? <p role="status">{t("today.goalsLoading")}</p> : null}
          {!planningLoading && weeklyGoals.length === 0 ? <p>{t("today.goalsEmpty")}</p> : null}
          {weeklyGoals.map((goal) => <div key={goal.id}><span>{goal.title}<small>{t(`today.goal.${goal.category}`)}</small></span><strong>{goal.completedCount} / {goal.targetCount}</strong></div>)}
        </div>
        <WeeklyGoalForm weekStartsOn={weekStartsOn} onSave={onSaveWeeklyGoal} />
      </Panel>

      <section className="task-column">
        <div className="section-heading today-task-heading">
          <div><span className="eyebrow">{t("today.important")}</span><h2>{t("today.tasks")}</h2></div>
          <Badge tone="accent">{t("today.completedCount", { completed: completedCount, total: tasks.length })}</Badge>
        </div>
        <SegmentedControl label={t("today.filterLabel")} options={completionOptions} value={completion} onChange={setCompletion} />
        {loading ? <Panel className="task-empty" role="status">{t("today.loading")}</Panel> : null}
        {!loading && sections.length === 0 ? <Panel className="task-empty"><strong>{t("today.emptyTitle")}</strong><p>{t("today.emptyDescription")}</p><Button tone="primary" onClick={onCreate}>{t("task.create")}</Button></Panel> : null}
        {!loading ? sections.map((section) => (
          <section className="task-section" key={section.category} aria-labelledby={`task-section-${section.category}`}>
            <header><h3 id={`task-section-${section.category}`}>{section.label}</h3><span>{t("common.items", { count: section.tasks.length })}</span></header>
            <div className="task-list">
              {section.tasks.map((item) => <TaskRow key={`${item.sourceKind}:${item.sourceId}`} item={item} state={item.visualState} recurrenceLabel={item.sourceKind === "recurringInstance" ? t("task.recurring") : item.recurrenceLabel} completionLocked={item.sourceKind === "recurringInstance"} onOpen={onEdit} onToggleCompleted={onToggleCompleted} onStartFocus={onStartFocus} onSkip={item.sourceKind === "recurringInstance" ? onSkipInstance : undefined} onDelay={item.sourceKind === "recurringInstance" ? onDelayInstance : undefined} onRescheduleTomorrow={item.sourceKind === "recurringInstance" ? onRescheduleInstance : undefined} />)}
            </div>
          </section>
        )) : null}
        <Button className="add-task" tone="ghost" onClick={onCreate}><Icon name="plus" />{t("today.quickCreate")}</Button>
      </section>

      <aside className="day-column">
        <Panel>
          <span className="eyebrow">{t("today.next")}</span><h2>{t("today.schedule")}</h2>
          {schedule.length === 0 ? <p className="schedule-empty">{t("today.scheduleEmpty")}</p> : schedule.map((item) => (
            <button className="schedule-item" key={item.task.id} onClick={() => onEdit(item.task.id)}>
              <time>{item.task.scheduledTime}</time><span style={{ background: item.project?.color }} /><div><strong>{item.task.title}</strong><small>{item.project?.name ?? t("today.personalTask")}</small></div>
            </button>
          ))}
        </Panel>
        <Panel className="note-panel">
          <DailyNoteEditor noteDate={noteDate} body={noteBody} onSave={onSaveNote} />
          <span className="note-tape" />
        </Panel>
      </aside>
    </div>
  );
}

function DailyNoteEditor({ noteDate, body, onSave }: { noteDate: string; body: string; onSave: (body: string) => Promise<void> }) {
  const { t } = useI18n();
  const [draft, setDraft] = useState(body);
  const [status, setStatus] = useState<"idle" | "saving" | "saved" | "error">("idle");
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const activeDate = useRef(noteDate);
  const draftValue = useRef(body);
  const lastSavedValue = useRef(body);

  useEffect(() => {
    const dateChanged = activeDate.current !== noteDate;
    activeDate.current = noteDate;
    if (!dateChanged && (body === draftValue.current || draftValue.current !== lastSavedValue.current)) return;
    if (timer.current) clearTimeout(timer.current);
    timer.current = null;
    draftValue.current = body;
    lastSavedValue.current = body;
    setDraft(body);
    setStatus("idle");
  }, [body, noteDate]);

  useEffect(() => () => {
    if (timer.current) clearTimeout(timer.current);
  }, []);

  async function save(value: string) {
    if (timer.current) clearTimeout(timer.current);
    timer.current = null;
    setStatus("saving");
    try {
      await onSave(value);
      lastSavedValue.current = value;
      setStatus("saved");
    } catch {
      setStatus("error");
    }
  }

  function update(value: string) {
    draftValue.current = value;
    setDraft(value);
    setStatus("idle");
    if (timer.current) clearTimeout(timer.current);
    timer.current = setTimeout(() => void save(value), 500);
  }

  return <>
    <span className="eyebrow">{t("today.note.eyebrow")}</span>
    <label><span>{t("today.note.title")}</span><textarea value={draft} maxLength={4000} aria-label={t("today.note.label")} onChange={(event) => update(event.target.value)} onKeyDown={(event) => {
      if (event.ctrlKey && event.key === "Enter") {
        event.preventDefault();
        void save(event.currentTarget.value);
      }
    }} /></label>
    <div className="note-actions"><small role="status">{t(`today.note.${status}`)}</small><Button type="button" tone="ghost" disabled={status === "saving"} onClick={() => void save(draft)}>{t("today.note.save")}</Button></div>
  </>;
}

function WeeklyGoalForm({ weekStartsOn, onSave }: { weekStartsOn: string; onSave: (input: WeeklyGoalInput) => Promise<void> }) {
  const { t } = useI18n();
  const [title, setTitle] = useState("");
  const [category, setCategory] = useState<WeeklyGoalCategory>("completedTasks");
  const [targetCount, setTargetCount] = useState("5");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState(false);

  async function submit(event: FormEvent) {
    event.preventDefault();
    const target = Number(targetCount);
    if (!title.trim() || !Number.isInteger(target) || target <= 0) return;
    setSaving(true);
    setError(false);
    try {
      await onSave({ id: null, weekStartsOn, title: title.trim(), category, targetCount: target });
      setTitle("");
    } catch {
      setError(true);
    } finally {
      setSaving(false);
    }
  }

  return <form className="weekly-goal-form" aria-label={t("today.goal.formLabel")} onSubmit={(event) => void submit(event)}>
    <input aria-label={t("today.goal.name")} placeholder={t("today.goal.placeholder")} maxLength={200} value={title} onChange={(event) => setTitle(event.target.value)} />
    <div>
      <select aria-label={t("today.goal.category")} value={category} onChange={(event) => setCategory(event.target.value as WeeklyGoalCategory)}>
        {goalCategories.map((value) => <option key={value} value={value}>{t(`today.goal.${value}`)}</option>)}
      </select>
      <input aria-label={t("today.goal.count")} type="number" min="1" value={targetCount} onChange={(event) => setTargetCount(event.target.value)} />
      <Button type="submit" tone="secondary" disabled={saving || !title.trim()}>{saving ? t("common.saving") : t("common.add")}</Button>
    </div>
    {error ? <small role="alert">{t("today.goal.error")}</small> : null}
  </form>;
}

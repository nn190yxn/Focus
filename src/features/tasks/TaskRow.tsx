import { Badge, Button } from "../../components/ui";
import { useI18n } from "../../i18n/I18nContext";
import type { MessageKey } from "../../i18n/messages";
import type { TaskListItem, TaskVisualState } from "./types";

type TaskRowProps = {
  item: TaskListItem;
  state?: TaskVisualState;
  compact?: boolean;
  onOpen: (id: string) => void;
  onToggleCompleted: (id: string, completed: boolean) => void;
  onStartFocus?: (id: string) => void;
  recurrenceLabel?: string | null;
  completionLocked?: boolean;
  onSkip?: (id: string) => void;
  onDelay?: (id: string) => void;
  onRescheduleTomorrow?: (id: string) => void;
};

const stateLabelKeys: Record<TaskVisualState, MessageKey> = {
  normal: "task.state.normal",
  current: "task.state.current",
  completed: "task.state.completed",
  overdue: "task.state.overdue",
  paused: "task.state.paused",
};

export function TaskRow({ item, state = item.task.status === "completed" ? "completed" : "normal", compact = false, onOpen, onToggleCompleted, onStartFocus, recurrenceLabel, completionLocked = false, onSkip, onDelay, onRescheduleTomorrow }: TaskRowProps) {
  const { t } = useI18n();
  const { task, project } = item;
  const completed = state === "completed" || task.status === "completed";
  const projectName = project?.name ?? t(`task.category.${task.category}`);
  const priorityKey = `task.priority.${Math.min(3, Math.max(0, task.priority))}` as MessageKey;

  return (
    <article className={`task-row task-row--${state} ${compact ? "task-row--compact" : ""}`} data-state={state} style={{ "--task-project-color": project?.color ?? "var(--color-accent)" } as React.CSSProperties}>
      <button className="task-check" aria-label={t(completed ? "task.restoreLabel" : "task.completeLabel", { title: task.title })} aria-pressed={completed} disabled={completed && completionLocked} onClick={() => onToggleCompleted(task.id, !completed)} />
      <time dateTime={task.scheduledTime ?? undefined}>{task.scheduledTime ?? t("task.pendingTime")}</time>
      <button className="task-row__main" aria-label={t("task.openLabel", { title: task.title })} onClick={() => onOpen(task.id)}>
        <strong>{task.title}</strong>
        <span><i />{projectName} · {t(priorityKey)}{recurrenceLabel ? <> · <b>{recurrenceLabel}</b></> : null}</span>
      </button>
      <div className="task-row__status">
        <Badge tone={state === "overdue" ? "danger" : state === "completed" ? "success" : state === "current" ? "accent" : state === "paused" ? "warning" : "neutral"}>{t(stateLabelKeys[state])}</Badge>
        {onStartFocus && !completed ? <Button className="task-row__focus" type="button" tone="ghost" aria-label={t(state === "paused" ? "task.resumeLabel" : "task.focusLabel", { title: task.title })} onClick={() => onStartFocus(task.id)}>{t(state === "paused" ? "task.resume" : "task.focus")}</Button> : null}
        {onDelay && !completed ? <Button className="task-row__minor" type="button" tone="ghost" aria-label={t("task.delayLabel", { title: task.title })} onClick={() => onDelay(task.id)}>{t("task.delay")}</Button> : null}
        {onRescheduleTomorrow && !completed ? <Button className="task-row__minor" type="button" tone="ghost" aria-label={t("task.tomorrowLabel", { title: task.title })} onClick={() => onRescheduleTomorrow(task.id)}>{t("task.tomorrow")}</Button> : null}
        {onSkip && !completed ? <Button className="task-row__minor" type="button" tone="ghost" aria-label={t("task.skipLabel", { title: task.title })} onClick={() => onSkip(task.id)}>{t("task.skip")}</Button> : null}
      </div>
    </article>
  );
}

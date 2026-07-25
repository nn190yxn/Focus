import type { WorkspaceTask } from "../today/todayModel";
import type { FocusState, FocusTarget } from "./types";

export function availableFocusTasks(tasks: WorkspaceTask[]): WorkspaceTask[] {
  return tasks.filter((item) => item.task.status === "pending" && item.visualState !== "paused");
}

export function focusTaskKey(task: WorkspaceTask): string {
  return `${task.sourceKind}:${task.sourceId}`;
}

export function focusTargetForTask(task: WorkspaceTask): FocusTarget {
  return task.sourceKind === "recurringInstance"
    ? { taskId: null, taskInstanceId: task.sourceId }
    : { taskId: task.sourceId, taskInstanceId: null };
}

export function remainingSeconds(state: FocusState, now = Date.now()): number {
  if (state.state === "ready") return 0;
  if (state.state === "paused") return clampSeconds(state.remainingSeconds, state.plannedSeconds);
  return clampSeconds(Math.ceil((Date.parse(state.targetEndsAt) - now) / 1000), state.plannedSeconds);
}

export function formatFocusTime(seconds: number): string {
  const safeSeconds = Math.max(0, Math.floor(seconds));
  const minutes = Math.floor(safeSeconds / 60);
  const remainder = safeSeconds % 60;
  return `${String(minutes).padStart(2, "0")}:${String(remainder).padStart(2, "0")}`;
}

export function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  return target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT", "BUTTON"].includes(target.tagName);
}

function clampSeconds(value: number, plannedSeconds: number): number {
  return Math.min(plannedSeconds, Math.max(0, value));
}

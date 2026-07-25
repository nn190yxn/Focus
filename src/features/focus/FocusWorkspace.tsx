import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";

import { Badge, Button, Dialog, Panel, Progress } from "../../components/ui";
import { useI18n } from "../../i18n/I18nContext";
import { isTauriRuntime } from "../../lib/commandClient";
import { domainErrorMessage } from "../../lib/domainError";
import type { WorkspaceTask } from "../today/todayModel";
import { focusClient } from "./focusClient";
import { availableFocusTasks, focusTargetForTask, focusTaskKey, formatFocusTime, isEditableTarget, remainingSeconds } from "./focusModel";
import type { FocusCompletionKind, FocusSession, FocusState } from "./types";

type DurationMode = "15" | "25" | "50" | "custom";

type FocusWorkspaceProps = {
  tasks: WorkspaceTask[];
  initialTask: WorkspaceTask | null;
};

const durationOptions = ["15", "25", "50"] as const;

class UserFacingFocusError extends Error {}

export function FocusWorkspace({ tasks, initialTask }: FocusWorkspaceProps) {
  const { formatTime, t } = useI18n();
  const focusTasks = availableFocusTasks(tasks);
  const [state, setState] = useState<FocusState>(() => readyState());
  const [selectedTaskKey, setSelectedTaskKey] = useState(() => initialTask ? focusTaskKey(initialTask) : focusTasks[0] ? focusTaskKey(focusTasks[0]) : "");
  const [durationMode, setDurationMode] = useState<DurationMode>("25");
  const [customMinutes, setCustomMinutes] = useState("25");
  const [now, setNow] = useState(Date.now());
  const [recentSessions, setRecentSessions] = useState<FocusSession[]>([]);
  const [loading, setLoading] = useState(() => isTauriRuntime());
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [confirmFinish, setConfirmFinish] = useState(false);
  const deadlineHandled = useRef(false);
  const desktopRuntime = isTauriRuntime();

  const activeTask = state.state === "ready" ? null : tasks.find((item) => {
    if (state.taskInstanceId) return item.sourceKind === "recurringInstance" && item.sourceId === state.taskInstanceId;
    return item.sourceKind === "task" && item.sourceId === state.taskId;
  }) ?? null;
  const selectedTask = focusTasks.find((item) => focusTaskKey(item) === selectedTaskKey) ?? focusTasks[0] ?? null;
  const shownTask = activeTask ?? selectedTask;
  const durationMinutes = durationMode === "custom" ? Number(customMinutes) : Number(durationMode);
  const durationValid = Number.isInteger(durationMinutes) && durationMinutes >= 1 && durationMinutes <= 180;
  const secondsLeft = remainingSeconds(state, now);
  const displayedSeconds = state.state === "ready" ? durationValid ? durationMinutes * 60 : 0 : secondsLeft;
  const progress = state.state === "ready" ? 0 : ((state.plannedSeconds - secondsLeft) / state.plannedSeconds) * 100;

  useEffect(() => {
    if (!desktopRuntime) return;
    let active = true;
    void focusClient.reconcile().then((result) => {
      if (!active) return;
      if (result.ok) {
        setState(result.data.state);
        if (result.data.completedSession) addRecentSession(result.data.completedSession);
      } else {
        setError(domainErrorMessage(result.error, t));
      }
      setLoading(false);
    }).catch(() => {
      if (active) {
        setError(t("focus.error.load"));
        setLoading(false);
      }
    });
    return () => { active = false; };
  }, [desktopRuntime]);

  useEffect(() => {
    if (!desktopRuntime) return;
    let disposed = false;
    let stopListening: (() => void) | undefined;
    void listen<FocusState>("focus://state-changed", (event) => {
      setState(event.payload);
      setNow(Date.now());
    }).then((unlisten) => {
      if (disposed) unlisten();
      else stopListening = unlisten;
    }).catch(() => setError(t("focus.error.sync")));
    return () => {
      disposed = true;
      stopListening?.();
    };
  }, [desktopRuntime]);

  useEffect(() => {
    if (!desktopRuntime) return;
    let disposed = false;
    let stopListening: (() => void) | undefined;
    void listen<FocusSession>("focus://completed", (event) => {
      addRecentSession(event.payload);
      setState(readyState());
      setNow(Date.now());
    }).then((unlisten) => {
      if (disposed) unlisten();
      else stopListening = unlisten;
    }).catch(() => setError(t("focus.error.listen")));
    return () => {
      disposed = true;
      stopListening?.();
    };
  }, [desktopRuntime]);

  useEffect(() => {
    if (state.state !== "running") return;
    setNow(Date.now());
    const timer = window.setInterval(() => setNow(Date.now()), 500);
    return () => window.clearInterval(timer);
  }, [state.state, state.state === "running" ? state.targetEndsAt : null]);

  useEffect(() => {
    if (initialTask && state.state === "ready") setSelectedTaskKey(focusTaskKey(initialTask));
  }, [initialTask, state.state]);

  useEffect(() => {
    if (state.state !== "ready" || selectedTask || !focusTasks[0]) return;
    setSelectedTaskKey(focusTaskKey(focusTasks[0]));
  }, [focusTasks, selectedTask, state.state]);

  useEffect(() => {
    if (state.state !== "running" || secondsLeft > 0) {
      deadlineHandled.current = false;
      return;
    }
    if (deadlineHandled.current) return;
    deadlineHandled.current = true;
    void reconcileFocus();
  }, [secondsLeft, state.state]);

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (event.code !== "Space" || event.repeat || isEditableTarget(event.target) || busy || loading || confirmFinish) return;
      if (state.state === "ready" && (!selectedTask || !durationValid)) return;
      event.preventDefault();
      if (state.state === "ready") void startFocus();
      else if (state.state === "running") void pauseFocus();
      else void resumeFocus();
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [busy, confirmFinish, durationValid, loading, selectedTask, state]);

  function addRecentSession(session: FocusSession) {
    setRecentSessions((items) => [session, ...items.filter((item) => item.id !== session.id)].slice(0, 5));
  }

  async function startFocus() {
    if (!selectedTask || !durationValid) return;
    await runAction(async () => {
      if (desktopRuntime) {
        const result = await focusClient.start(focusTargetForTask(selectedTask), durationMinutes);
        if (!result.ok) throw new UserFacingFocusError(domainErrorMessage(result.error, t));
        return result.data;
      }
      const startedAt = new Date();
      return {
        state: "running" as const,
        ...focusTargetForTask(selectedTask),
        plannedSeconds: durationMinutes * 60,
        remainingSeconds: durationMinutes * 60,
        startedAt: startedAt.toISOString(),
        targetEndsAt: new Date(startedAt.getTime() + durationMinutes * 60_000).toISOString(),
        interruptionCount: 0,
        serverTime: startedAt.toISOString(),
      };
    });
  }

  async function pauseFocus() {
    if (state.state !== "running") return;
    await runAction(async () => {
      if (desktopRuntime) {
        const result = await focusClient.pause();
        if (!result.ok) throw new UserFacingFocusError(domainErrorMessage(result.error, t));
        return result.data;
      }
      const pausedAt = new Date();
      return {
        state: "paused" as const,
        taskId: state.taskId,
        taskInstanceId: state.taskInstanceId,
        plannedSeconds: state.plannedSeconds,
        remainingSeconds: remainingSeconds(state, pausedAt.getTime()),
        startedAt: state.startedAt,
        pausedAt: pausedAt.toISOString(),
        interruptionCount: state.interruptionCount + 1,
        serverTime: pausedAt.toISOString(),
      };
    });
  }

  async function resumeFocus() {
    if (state.state !== "paused") return;
    await runAction(async () => {
      if (desktopRuntime) {
        const result = await focusClient.resume();
        if (!result.ok) throw new UserFacingFocusError(domainErrorMessage(result.error, t));
        return result.data;
      }
      const resumedAt = new Date();
      return {
        state: "running" as const,
        taskId: state.taskId,
        taskInstanceId: state.taskInstanceId,
        plannedSeconds: state.plannedSeconds,
        remainingSeconds: state.remainingSeconds,
        startedAt: state.startedAt,
        targetEndsAt: new Date(resumedAt.getTime() + state.remainingSeconds * 1000).toISOString(),
        interruptionCount: state.interruptionCount,
        serverTime: resumedAt.toISOString(),
      };
    });
  }

  async function resetFocus() {
    if (state.state === "ready") return;
    await runAction(async () => {
      if (desktopRuntime) {
        const result = await focusClient.reset();
        if (!result.ok) throw new UserFacingFocusError(domainErrorMessage(result.error, t));
        return result.data;
      }
      return readyState();
    });
  }

  async function finishFocus(completionKind: FocusCompletionKind = "early") {
    if (state.state === "ready") return;
    setConfirmFinish(false);
    setBusy(true);
    setError(null);
    try {
      let session: FocusSession;
      if (desktopRuntime) {
        const result = await focusClient.finish(completionKind);
        if (!result.ok) throw new UserFacingFocusError(domainErrorMessage(result.error, t));
        session = result.data;
      } else {
        session = previewSession(state, completionKind);
      }
      addRecentSession(session);
      setState(readyState());
    } catch (actionError) {
      setError(actionError instanceof UserFacingFocusError ? actionError.message : t("focus.error.action"));
    } finally {
      setBusy(false);
    }
  }

  async function reconcileFocus() {
    try {
      if (desktopRuntime) {
        const result = await focusClient.reconcile();
        if (!result.ok) throw new UserFacingFocusError(domainErrorMessage(result.error, t));
        setState(result.data.state);
        if (result.data.completedSession) addRecentSession(result.data.completedSession);
      } else if (state.state === "running") {
        addRecentSession(previewSession(state, "deadline"));
        setState(readyState());
      }
    } catch (actionError) {
      setError(actionError instanceof UserFacingFocusError ? actionError.message : t("focus.error.reconcile"));
    }
  }

  async function runAction(action: () => Promise<FocusState>) {
    setBusy(true);
    setError(null);
    try {
      const nextState = await action();
      setState(nextState);
      setNow(Date.now());
    } catch (actionError) {
      setError(actionError instanceof UserFacingFocusError ? actionError.message : t("focus.error.action"));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className={`focus-workspace focus-workspace--${state.state}`}>
      <Panel className="focus-stage" aria-busy={loading || busy}>
        <div className="focus-stage__ambient" aria-hidden="true" />
        <div className="focus-stage__topline">
          <Badge tone={state.state === "running" ? "success" : state.state === "paused" ? "warning" : "accent"}>{statusLabel(state, t)}</Badge>
          <span>{t("focus.shortcut", { action: t(state.state === "ready" ? "focus.action.start" : state.state === "running" ? "focus.action.pause" : "focus.action.resume") })}</span>
        </div>

        <div className="focus-stage__center">
          <div className="focus-companion" aria-hidden="true"><span>AF</span><i /><i /></div>
          <p className="eyebrow">{shownTask?.project?.name ?? t("focus.independentTask")}</p>
          <h2>{shownTask?.task.title ?? t("focus.chooseTitle")}</h2>
          <output className="focus-clock" aria-live="off" aria-label={t("focus.remainingLabel", { time: formatFocusTime(displayedSeconds) })}>{formatFocusTime(displayedSeconds)}</output>
          <Progress label={t("focus.progress")} value={progress} />
        </div>

        <div className="focus-controls">
          {state.state === "ready" ? (
            <Button tone="primary" disabled={loading || busy || !selectedTask || !durationValid} onClick={() => void startFocus()}>{t("focus.start")}</Button>
          ) : state.state === "running" ? (
            <Button tone="primary" disabled={busy} onClick={() => void pauseFocus()}>{t("focus.action.pause")}</Button>
          ) : (
            <Button tone="primary" disabled={busy} onClick={() => void resumeFocus()}>{t("focus.action.resume")}</Button>
          )}
          {state.state !== "ready" ? <Button tone="secondary" disabled={busy} onClick={() => setConfirmFinish(true)}>{t("focus.finishEarly")}</Button> : null}
          {state.state !== "ready" ? <Button tone="ghost" disabled={busy} onClick={() => void resetFocus()}>{t("focus.reset")}</Button> : null}
        </div>
        {error ? <p className="focus-error" role="alert">{error}</p> : null}
      </Panel>

      <aside className="focus-sidebar">
        <Panel>
          <span className="eyebrow">{t("focus.currentTask")}</span>
          {state.state === "ready" ? (
            <label className="focus-field"><span>{t("focus.chooseTask")}</span><select aria-label={t("focus.chooseTask")} value={selectedTask ? focusTaskKey(selectedTask) : ""} onChange={(event) => setSelectedTaskKey(event.target.value)} disabled={busy || focusTasks.length === 0}>
              {focusTasks.length === 0 ? <option value="">{t("focus.noTasks")}</option> : null}
              {focusTasks.map((item) => <option key={focusTaskKey(item)} value={focusTaskKey(item)}>{item.task.title}</option>)}
            </select></label>
          ) : (
            <div className="focus-current-task"><strong>{shownTask?.task.title ?? t("focus.focusing")}</strong><small>{shownTask?.sourceKind === "recurringInstance" ? t("task.recurring") : shownTask?.project?.name ?? t("focus.independentTask")}</small></div>
          )}

          <div className="focus-duration" aria-label={t("focus.duration")}>
            {durationOptions.map((minutes) => <button key={minutes} type="button" className={durationMode === minutes ? "active" : ""} disabled={state.state !== "ready"} onClick={() => setDurationMode(minutes)}>{t("focus.durationMinutes", { count: minutes })}</button>)}
            <button type="button" className={durationMode === "custom" ? "active" : ""} disabled={state.state !== "ready"} onClick={() => setDurationMode("custom")}>{t("focus.custom")}</button>
          </div>
          {durationMode === "custom" ? <label className="focus-field"><span>{t("focus.customMinutes")}</span><input type="number" min="1" max="180" step="1" value={customMinutes} disabled={state.state !== "ready"} aria-invalid={!durationValid} onChange={(event) => setCustomMinutes(event.target.value)} /></label> : null}
        </Panel>

        <Panel className="focus-sessions">
          <div className="panel__heading"><div><span className="eyebrow">{t("focus.recent")}</span><h3>{t("focus.records")}</h3></div><Badge>{recentSessions.length}</Badge></div>
          {recentSessions.length === 0 ? <p className="focus-empty">{t("focus.recordsEmpty")}</p> : recentSessions.map((session) => (
            <article key={session.id}><div><strong>{sessionLabel(session, tasks, t("focus.session"))}</strong><small>{formatTime(session.endedAt)}</small></div><span>{formatFocusTime(session.actualSeconds)}</span></article>
          ))}
        </Panel>
      </aside>

      <Dialog open={confirmFinish} title={t("focus.finishTitle")} onClose={() => setConfirmFinish(false)}>
        <p className="focus-dialog-copy">{t("focus.finishDescription")}</p>
        <footer className="focus-dialog-actions"><Button tone="ghost" onClick={() => setConfirmFinish(false)}>{t("focus.keepGoing")}</Button><Button tone="primary" onClick={() => void finishFocus("early")}>{t("focus.confirmFinish")}</Button></footer>
      </Dialog>
    </div>
  );
}

function readyState(): FocusState {
  return { state: "ready", serverTime: new Date().toISOString() };
}

function statusLabel(state: FocusState, t: ReturnType<typeof useI18n>["t"]): string {
  if (state.state === "running") return t("focus.status.running");
  if (state.state === "paused") return t("focus.status.paused");
  return t("focus.status.ready");
}

function previewSession(state: Exclude<FocusState, { state: "ready" }>, completionKind: FocusCompletionKind): FocusSession {
  const endedAt = completionKind === "deadline" && state.state === "running" ? state.targetEndsAt : new Date().toISOString();
  const actualSeconds = completionKind === "deadline" ? state.plannedSeconds : state.plannedSeconds - remainingSeconds(state);
  return {
    id: globalThis.crypto?.randomUUID?.() ?? `preview-${Date.now()}`,
    taskId: state.taskId,
    taskInstanceId: state.taskInstanceId,
    projectId: null,
    plannedSeconds: state.plannedSeconds,
    actualSeconds,
    interruptionCount: state.interruptionCount,
    completionKind,
    startedAt: state.startedAt,
    endedAt,
    createdAt: new Date().toISOString(),
  };
}

function sessionLabel(session: FocusSession, tasks: WorkspaceTask[], fallback: string): string {
  return tasks.find((item) => item.sourceId === (session.taskInstanceId ?? session.taskId))?.task.title ?? fallback;
}

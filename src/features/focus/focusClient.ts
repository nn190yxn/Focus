import { invokeCommand, type CommandResult } from "../../lib/commandClient";
import type { FocusCompletionKind, FocusReconcileResult, FocusSession, FocusState, FocusTarget } from "./types";

export type FocusCommandClient = {
  getState: () => Promise<CommandResult<FocusState>>;
  reconcile: () => Promise<CommandResult<FocusReconcileResult>>;
  start: (target: FocusTarget, plannedMinutes: number) => Promise<CommandResult<FocusState>>;
  pause: () => Promise<CommandResult<FocusState>>;
  resume: () => Promise<CommandResult<FocusState>>;
  reset: () => Promise<CommandResult<FocusState>>;
  finish: (completionKind: FocusCompletionKind) => Promise<CommandResult<FocusSession>>;
};

export const focusClient: FocusCommandClient = {
  getState: () => invokeCommand("focus_get_state"),
  reconcile: () => invokeCommand("focus_reconcile"),
  start: (target, plannedMinutes) => invokeCommand("focus_start", { target, plannedMinutes }),
  pause: () => invokeCommand("focus_pause"),
  resume: () => invokeCommand("focus_resume"),
  reset: () => invokeCommand("focus_reset"),
  finish: (completionKind) => invokeCommand("focus_finish", { completionKind }),
};

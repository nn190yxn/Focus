export type FocusCompletionKind = "deadline" | "early" | "cancelled";

export type FocusTarget = {
  taskId: string | null;
  taskInstanceId: string | null;
};

type FocusActiveBase = {
  taskId: string | null;
  taskInstanceId: string | null;
  plannedSeconds: number;
  remainingSeconds: number;
  startedAt: string;
  interruptionCount: number;
  serverTime: string;
};

export type FocusState =
  | { state: "ready"; serverTime: string }
  | (FocusActiveBase & { state: "running"; targetEndsAt: string })
  | (FocusActiveBase & { state: "paused"; pausedAt: string });

export type FocusSession = {
  id: string;
  taskId: string | null;
  taskInstanceId: string | null;
  projectId: string | null;
  plannedSeconds: number;
  actualSeconds: number;
  interruptionCount: number;
  completionKind: FocusCompletionKind;
  startedAt: string;
  endedAt: string;
  createdAt: string;
};

export type FocusReconcileResult = {
  state: FocusState;
  completedSession?: FocusSession;
};

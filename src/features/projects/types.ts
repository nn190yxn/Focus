import type { TaskRecord } from "../tasks/types";

export type ProjectStatus = "active" | "paused" | "completed" | "archived";

export interface ProjectInput {
  name: string;
  description: string;
  color: string;
  icon: string;
  startedOn: string;
  targetOn: string | null;
}

export interface ProjectRecord extends ProjectInput {
  id: string;
  status: ProjectStatus;
  createdAt: string;
  updatedAt: string;
}

export interface ProjectAggregation {
  activeTaskCount: number;
  completedTaskCount: number;
  totalTaskCount: number;
  completionPercent: number;
  focusSeconds: number;
}

export interface ProjectSummary {
  project: ProjectRecord;
  aggregation: ProjectAggregation;
  nextTaskTitle: string | null;
  nextTaskDate: string | null;
  deadlineState: "none" | "overdue" | "atRisk" | "onTrack";
  deadlineDays: number | null;
}

export interface ProjectDetail {
  summary: ProjectSummary;
  tasks: TaskRecord[];
}

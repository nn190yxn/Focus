export type TaskCategory = "work" | "study" | "health" | "life";
export type TaskCompletionFilter = "pending" | "completed";
export type TaskVisualState = "normal" | "current" | "completed" | "overdue" | "paused";

export interface CheckItemInput {
  id?: string;
  title: string;
  completed: boolean;
}

export interface TaskInput {
  projectId: string | null;
  title: string;
  category: TaskCategory;
  priority: number;
  scheduledDate: string | null;
  scheduledTime: string | null;
  checkItems: CheckItemInput[];
}

export interface TaskRecord {
  id: string;
  projectId: string | null;
  title: string;
  category: TaskCategory;
  priority: number;
  scheduledDate: string | null;
  scheduledTime: string | null;
  status: "pending" | "completed" | "removed";
  completedAt: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface CheckItemRecord {
  id: string;
  taskId: string;
  title: string;
  position: number;
  completedAt: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface TaskProjectSummary {
  id: string;
  name: string;
  color: string;
  icon: string;
  status: "active" | "paused" | "completed" | "archived";
}

export interface TaskListItem {
  task: TaskRecord;
  project: TaskProjectSummary | null;
}

export interface TaskDetail {
  task: TaskRecord;
  checkItems: CheckItemRecord[];
}

export interface TaskListFilter {
  startsOn?: string;
  endsOn?: string;
  projectId?: string;
  category?: TaskCategory;
  completion?: TaskCompletionFilter;
  search?: string;
}

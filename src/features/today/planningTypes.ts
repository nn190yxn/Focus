export type WeeklyGoalCategory = "completedTasks" | "focusMinutes" | "activeDays";

export type DailyNote = {
  id: string;
  body: string;
  noteDate: string;
  createdAt: string;
  updatedAt: string;
};

export type DailyNoteInput = {
  body: string;
  noteDate: string;
};

export type WeeklyGoal = {
  id: string;
  weekStartsOn: string;
  title: string;
  category: WeeklyGoalCategory;
  targetCount: number;
  completedCount: number;
  position: number;
  createdAt: string;
  updatedAt: string;
};

export type WeeklyGoalInput = {
  id: string | null;
  weekStartsOn: string;
  title: string;
  category: WeeklyGoalCategory;
  targetCount: number;
};

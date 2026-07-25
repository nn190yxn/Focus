import { invokeCommand, type CommandResult } from "../../lib/commandClient";
import type { DailyNote, DailyNoteInput, WeeklyGoal, WeeklyGoalInput } from "./planningTypes";

export interface PlanningCommandClient {
  getNote(noteDate: string): Promise<CommandResult<DailyNote | null>>;
  saveNote(input: DailyNoteInput): Promise<CommandResult<DailyNote>>;
  listWeeklyGoals(weekStartsOn: string, timezone: string): Promise<CommandResult<WeeklyGoal[]>>;
  saveWeeklyGoal(input: WeeklyGoalInput, timezone: string): Promise<CommandResult<WeeklyGoal>>;
}

export const planningClient: PlanningCommandClient = {
  getNote: (noteDate) => invokeCommand<DailyNote | null>("note_get", { noteDate }),
  saveNote: (input) => invokeCommand<DailyNote>("note_save", { input }),
  listWeeklyGoals: (weekStartsOn, timezone) => invokeCommand<WeeklyGoal[]>("weekly_goal_list", { weekStartsOn, timezone }),
  saveWeeklyGoal: (input, timezone) => invokeCommand<WeeklyGoal>("weekly_goal_save", { input, timezone }),
};

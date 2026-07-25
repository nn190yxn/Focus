import { invokeCommand } from "../../lib/commandClient";
import type { TaskDetail, TaskInput, TaskListFilter, TaskListItem } from "./types";

export const taskClient = {
  list(filter: TaskListFilter = {}) {
    return invokeCommand<TaskListItem[]>("task_list", { filter });
  },
  get(id: string) {
    return invokeCommand<TaskDetail>("task_get", { id });
  },
  create(input: TaskInput, today: string) {
    return invokeCommand<TaskDetail>("task_create", { input, today });
  },
  update(id: string, input: TaskInput, today: string) {
    return invokeCommand<TaskDetail>("task_update", { id, input, today });
  },
  setCompleted(id: string, completed: boolean) {
    return invokeCommand<TaskDetail>("task_set_completed", { id, completed });
  },
  remove(id: string) {
    return invokeCommand<void>("task_remove", { id });
  },
  setCheckItemCompleted(taskId: string, itemId: string, completed: boolean) {
    return invokeCommand<TaskDetail>("task_set_check_item_completed", { taskId, itemId, completed });
  },
  reorderCheckItems(taskId: string, orderedIds: string[]) {
    return invokeCommand<TaskDetail>("task_reorder_check_items", { taskId, orderedIds });
  },
};

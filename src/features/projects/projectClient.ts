import { invokeCommand, type CommandResult } from "../../lib/commandClient";
import type { ProjectDetail, ProjectInput, ProjectRecord, ProjectStatus, ProjectSummary } from "./types";

export interface ProjectClient {
  list(status: ProjectStatus | null, today: string): Promise<CommandResult<ProjectSummary[]>>;
  get(id: string, today: string): Promise<CommandResult<ProjectDetail>>;
  create(input: ProjectInput): Promise<CommandResult<ProjectRecord>>;
  update(id: string, input: ProjectInput): Promise<CommandResult<ProjectRecord>>;
  setStatus(id: string, status: ProjectStatus): Promise<CommandResult<ProjectRecord>>;
}

export const projectClient: ProjectClient = {
  list(status, today) {
    return invokeCommand<ProjectSummary[]>("project_list", { status, today });
  },
  get(id, today) {
    return invokeCommand<ProjectDetail>("project_get", { id, today });
  },
  create(input) {
    return invokeCommand<ProjectRecord>("project_create", { input });
  },
  update(id, input) {
    return invokeCommand<ProjectRecord>("project_update", { id, input });
  },
  setStatus(id, status) {
    return invokeCommand<ProjectRecord>("project_set_status", { id, status });
  },
};

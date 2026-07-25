import { invokeCommand, type CommandResult } from "../../lib/commandClient";
import type { MemoInput, MemoListQuery, MemoRecord, MemoSummary, MemoTagSummary } from "./types";

export interface MemoClient {
  list(query: MemoListQuery): Promise<CommandResult<MemoSummary[]>>;
  get(id: string): Promise<CommandResult<MemoRecord>>;
  create(input: MemoInput): Promise<CommandResult<MemoRecord>>;
  update(id: string, input: MemoInput): Promise<CommandResult<MemoRecord>>;
  remove(id: string): Promise<CommandResult<null>>;
  listTags(): Promise<CommandResult<MemoTagSummary[]>>;
}

export const memoClient: MemoClient = {
  list(query) {
    return invokeCommand<MemoSummary[]>("memo_list", { query });
  },
  get(id) {
    return invokeCommand<MemoRecord>("memo_get", { id });
  },
  create(input) {
    return invokeCommand<MemoRecord>("memo_create", { input });
  },
  update(id, input) {
    return invokeCommand<MemoRecord>("memo_update", { id, input });
  },
  remove(id) {
    return invokeCommand<null>("memo_remove", { id });
  },
  listTags() {
    return invokeCommand<MemoTagSummary[]>("memo_tag_list");
  },
};

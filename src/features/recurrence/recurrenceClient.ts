import { invokeCommand } from "../../lib/commandClient";
import type { GenerationSummary, RecurrenceChangeScope, RecurrenceRule, RecurrenceStatus, TaskInstanceRecord } from "./types";

export const recurrenceClient = {
  get(ruleId: string) {
    return invokeCommand<RecurrenceRule>("recurrence_get", { ruleId });
  },
  create(rule: RecurrenceRule, rangeStart: string, rangeEnd: string) {
    return invokeCommand<GenerationSummary>("recurrence_create", { rule, rangeStart, rangeEnd });
  },
  update(proposed: RecurrenceRule, scope: RecurrenceChangeScope, rangeEnd: string) {
    return invokeCommand<GenerationSummary>("recurrence_update", { proposed, scope, rangeEnd });
  },
  setStatus(ruleId: string, status: RecurrenceStatus) {
    return invokeCommand<RecurrenceRule>("recurrence_set_status", { ruleId, status });
  },
  complete(instanceId: string) {
    return invokeCommand<TaskInstanceRecord>("instance_complete", { instanceId });
  },
  skip(instanceId: string) {
    return invokeCommand<TaskInstanceRecord>("instance_skip", { instanceId });
  },
  delayToday(instanceId: string, localTime: string) {
    return invokeCommand<TaskInstanceRecord>("instance_delay_today", { instanceId, localTime });
  },
  rescheduleTomorrow(instanceId: string) {
    return invokeCommand<TaskInstanceRecord>("instance_reschedule_tomorrow", { instanceId });
  },
};

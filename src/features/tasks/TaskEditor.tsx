import { useState, type FormEvent } from "react";

import { Button } from "../../components/ui";
import { createI18n, useI18n, type I18nValue } from "../../i18n/I18nContext";
import type { MessageKey } from "../../i18n/messages";
import { RecurrenceEditor, validateRecurrenceInput } from "../recurrence/RecurrenceEditor";
import type { RecurrenceRuleInput } from "../recurrence/types";
import type { CheckItemInput, TaskCategory, TaskInput, TaskProjectSummary } from "./types";

type TaskEditorProps = {
  today: string;
  projects: readonly TaskProjectSummary[];
  initialValue?: TaskInput;
  submitLabel?: string;
  onCancel: () => void;
  onSubmit: (input: TaskInput, recurrence: RecurrenceRuleInput | null) => void | Promise<void>;
};

type TaskEditorErrors = Partial<Record<"title" | "scheduledDate" | "scheduledTime" | "checkItems" | "recurrence", string>>;

const emptyTask: TaskInput = {
  projectId: null,
  title: "",
  category: "work",
  priority: 0,
  scheduledDate: null,
  scheduledTime: null,
  checkItems: [],
};

export function validateTaskInput(input: TaskInput, today: string, t: I18nValue["t"] = createI18n("zh-CN").t): TaskEditorErrors {
  const errors: TaskEditorErrors = {};
  const titleLength = input.title.trim().length;
  if (titleLength < 1 || titleLength > 200) errors.title = t("task.validation.title");
  if (input.scheduledDate && input.scheduledDate < today) errors.scheduledDate = t("task.validation.date", { date: today });
  if (input.scheduledTime && !input.scheduledDate) errors.scheduledTime = t("task.validation.time");
  if (input.checkItems.some((item) => item.title.trim().length < 1 || item.title.trim().length > 200)) {
    errors.checkItems = t("task.validation.checkItems");
  }
  return errors;
}

export function TaskEditor({ today, projects, initialValue, submitLabel, onCancel, onSubmit }: TaskEditorProps) {
  const { t } = useI18n();
  const [value, setValue] = useState<TaskInput>(() => cloneTaskInput(initialValue ?? emptyTask));
  const [errors, setErrors] = useState<TaskEditorErrors>({});
  const [submitting, setSubmitting] = useState(false);
  const [repeatEnabled, setRepeatEnabled] = useState(false);
  const [recurrence, setRecurrence] = useState<RecurrenceRuleInput>(() => defaultRecurrence(initialValue?.scheduledDate ?? today, initialValue?.scheduledTime));

  function updateCheckItem(index: number, patch: Partial<CheckItemInput>) {
    setValue((current) => ({
      ...current,
      checkItems: current.checkItems.map((item, itemIndex) => itemIndex === index ? { ...item, ...patch } : item),
    }));
  }

  function moveCheckItem(index: number, direction: -1 | 1) {
    const target = index + direction;
    if (target < 0 || target >= value.checkItems.length) return;
    setValue((current) => {
      const checkItems = [...current.checkItems];
      [checkItems[index], checkItems[target]] = [checkItems[target], checkItems[index]];
      return { ...current, checkItems };
    });
  }

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const nextValue = {
      ...value,
      title: value.title.trim(),
      checkItems: value.checkItems.map((item) => ({ ...item, title: item.title.trim() })),
    };
    const nextErrors = validateTaskInput(nextValue, today, t);
    const recurrenceError = repeatEnabled ? validateRecurrenceInput(recurrence, t) : null;
    if (recurrenceError) nextErrors.recurrence = recurrenceError;
    setErrors(nextErrors);
    if (Object.keys(nextErrors).length > 0) return;
    setSubmitting(true);
    try {
      await onSubmit(nextValue, repeatEnabled ? recurrence : null);
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <form className="task-editor" onSubmit={submit}>
      <label className="task-editor__field task-editor__field--wide">
        <span>{t("task.editor.title")}</span>
        <input autoFocus aria-label={t("task.editor.title")} value={value.title} maxLength={200} aria-invalid={Boolean(errors.title)} onChange={(event) => setValue({ ...value, title: event.target.value })} />
        {errors.title ? <small className="field__error">{errors.title}</small> : null}
      </label>

      <div className="task-editor__grid">
        <label className="task-editor__field">
          <span>{t("task.editor.category")}</span>
          <select value={value.category} onChange={(event) => setValue({ ...value, category: event.target.value as TaskInput["category"] })}>
            {(["work", "study", "health", "life"] as TaskCategory[]).map((category) => <option key={category} value={category}>{t(`task.category.${category}`)}</option>)}
          </select>
        </label>
        <label className="task-editor__field">
          <span>{t("task.editor.project")}</span>
          <select value={value.projectId ?? ""} onChange={(event) => setValue({ ...value, projectId: event.target.value || null })}>
            <option value="">{t("task.editor.noProject")}</option>
            {projects.map((project) => <option key={project.id} value={project.id}>{project.name}{project.status === "paused" ? ` · ${t("task.state.paused")}` : ""}</option>)}
          </select>
        </label>
        <label className="task-editor__field">
          <span>{t("task.editor.priority")}</span>
          <select value={value.priority} onChange={(event) => setValue({ ...value, priority: Number(event.target.value) })}>
            {[0, 1, 2, 3].map((priority) => <option key={priority} value={priority}>{t(`task.priority.${priority}` as MessageKey)}</option>)}
          </select>
        </label>
        <label className="task-editor__field">
          <span>{t("task.editor.date")}</span>
          <input type="date" min={today} value={value.scheduledDate ?? ""} aria-invalid={Boolean(errors.scheduledDate)} onChange={(event) => setValue({ ...value, scheduledDate: event.target.value || null })} />
          {errors.scheduledDate ? <small className="field__error">{errors.scheduledDate}</small> : null}
        </label>
        <label className="task-editor__field">
          <span>{t("task.editor.time")}</span>
          <input type="time" value={value.scheduledTime ?? ""} aria-invalid={Boolean(errors.scheduledTime)} onChange={(event) => setValue({ ...value, scheduledTime: event.target.value || null })} />
          {errors.scheduledTime ? <small className="field__error">{errors.scheduledTime}</small> : null}
        </label>
      </div>

      <fieldset className="check-item-editor">
        <legend>{t("task.editor.checkItems")}</legend>
        <div className="check-item-editor__list">
          {value.checkItems.map((item, index) => (
            <div className="check-item-editor__row" key={item.id ?? index}>
              <input type="checkbox" aria-label={t("task.editor.checkCompleteLabel", { index: index + 1 })} checked={item.completed} onChange={(event) => updateCheckItem(index, { completed: event.target.checked })} />
              <input aria-label={t("task.editor.checkLabel", { index: index + 1 })} value={item.title} maxLength={200} onChange={(event) => updateCheckItem(index, { title: event.target.value })} />
              <button type="button" aria-label={t("task.editor.checkMoveUpLabel", { index: index + 1 })} disabled={index === 0} onClick={() => moveCheckItem(index, -1)}>{t("task.editor.moveUp")}</button>
              <button type="button" aria-label={t("task.editor.checkMoveDownLabel", { index: index + 1 })} disabled={index === value.checkItems.length - 1} onClick={() => moveCheckItem(index, 1)}>{t("task.editor.moveDown")}</button>
              <button type="button" aria-label={t("task.editor.checkRemoveLabel", { index: index + 1 })} onClick={() => setValue((current) => ({ ...current, checkItems: current.checkItems.filter((_, itemIndex) => itemIndex !== index) }))}>{t("task.editor.remove")}</button>
            </div>
          ))}
        </div>
        {errors.checkItems ? <small className="field__error">{errors.checkItems}</small> : null}
        <Button type="button" tone="ghost" onClick={() => setValue((current) => ({ ...current, checkItems: [...current.checkItems, { title: "", completed: false }] }))}>{t("task.editor.addCheck")}</Button>
      </fieldset>

      <fieldset className="recurrence-toggle">
        <legend>{t("task.editor.recurrence")}</legend>
        <label className="recurrence-toggle__switch"><input type="checkbox" checked={repeatEnabled} onChange={(event) => setRepeatEnabled(event.target.checked)} /><span>{t("task.editor.recurrenceHint")}</span></label>
        {repeatEnabled ? <RecurrenceEditor initialValue={recurrence} showActions={false} onChange={setRecurrence} /> : null}
        {errors.recurrence ? <small className="field__error" role="alert">{errors.recurrence}</small> : null}
      </fieldset>

      <footer className="task-editor__footer">
        <Button type="button" tone="ghost" onClick={onCancel}>{t("common.cancel")}</Button>
        <Button type="submit" tone="primary" disabled={submitting}>{submitting ? t("common.saving") : submitLabel ?? t("task.save")}</Button>
      </footer>
    </form>
  );
}

function defaultRecurrence(startsOn: string, localTime?: string | null): RecurrenceRuleInput {
  return {
    pattern: { kind: "daily", interval: 1 },
    localTime: localTime ?? "09:00",
    timezone: Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC",
    startsOn,
    endsOn: null,
  };
}

function cloneTaskInput(input: TaskInput): TaskInput {
  return { ...input, checkItems: input.checkItems.map((item) => ({ ...item })) };
}

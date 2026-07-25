import { useState } from "react";

import { Button } from "../../components/ui";
import { useI18n } from "../../i18n/I18nContext";
import { RecurrenceEditor, validateRecurrenceInput } from "./RecurrenceEditor";
import type { RecurrenceChangeScope, RecurrenceRule, RecurrenceRuleInput } from "./types";

type RecurrenceScopeEditorProps = {
  instanceId: string;
  effectiveOn: string;
  rule: RecurrenceRule;
  onCancel: () => void;
  onSubmit: (rule: RecurrenceRule, scope: RecurrenceChangeScope) => void | Promise<void>;
  onSetStatus: (status: "paused" | "ended") => void | Promise<void>;
};

export function RecurrenceScopeEditor({ instanceId, effectiveOn, rule, onCancel, onSubmit, onSetStatus }: RecurrenceScopeEditorProps) {
  const { t } = useI18n();
  const [scope, setScope] = useState<"thisInstance" | "future">("thisInstance");
  const [value, setValue] = useState<RecurrenceRuleInput>(rule);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function save() {
    const nextError = validateRecurrenceInput(value, t);
    setError(nextError);
    if (nextError) return;
    const proposed = { ...rule, ...value, version: scope === "future" ? rule.version + 1 : rule.version };
    const changeScope: RecurrenceChangeScope = scope === "future" ? { scope, effectiveOn } : { scope, instanceId };
    setSubmitting(true);
    try {
      await onSubmit(proposed, changeScope);
    } finally {
      setSubmitting(false);
    }
  }

  return <div className="recurrence-scope-editor">
    <fieldset className="scope-options"><legend>{t("recurrence.scope")}</legend><label><input type="radio" name="scope" checked={scope === "thisInstance"} onChange={() => setScope("thisInstance")} /><span><strong>{t("recurrence.scope.this")}</strong><small>{t("recurrence.scope.thisDescription", { date: effectiveOn })}</small></span></label><label><input type="radio" name="scope" checked={scope === "future"} onChange={() => setScope("future")} /><span><strong>{t("recurrence.scope.future")}</strong><small>{t("recurrence.scope.futureDescription", { date: effectiveOn })}</small></span></label></fieldset>
    <RecurrenceEditor initialValue={rule} showActions={false} onChange={setValue} />
    {error ? <small className="field__error" role="alert">{error}</small> : null}
    <div className="recurrence-rule-actions"><span>{t("recurrence.status", { status: t(`recurrence.status.${rule.status}`) })}</span>{rule.status === "active" ? <Button tone="ghost" onClick={() => void onSetStatus("paused")}>{t("recurrence.pause")}</Button> : null}{rule.status !== "ended" ? <Button tone="danger" onClick={() => void onSetStatus("ended")}>{t("recurrence.end")}</Button> : null}</div>
    <footer className="task-editor__footer"><Button tone="ghost" onClick={onCancel}>{t("common.cancel")}</Button><Button tone="primary" disabled={submitting} onClick={() => void save()}>{submitting ? t("common.saving") : t("recurrence.apply")}</Button></footer>
  </div>;
}

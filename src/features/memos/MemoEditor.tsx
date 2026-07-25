import { useEffect, useRef, useState } from "react";

import { Icon } from "../../components/Icon";
import { Button, Dialog } from "../../components/ui";
import { useI18n } from "../../i18n/I18nContext";
import { MemoReminderEditor } from "./MemoReminderEditor";
import { MemoReminderPermissionNotice } from "./MemoReminderPermissionNotice";
import { memoReminderScheduleSummary, memoReminderSummary } from "./MemoListItem";
import type { MemoInput, MemoRecord } from "./types";

export type MemoEditorProps = {
  memo: MemoRecord | null;
  saving: boolean;
  saveError: string | null;
  deleting: boolean;
  deleteError: string | null;
  onSave: (input: MemoInput) => void;
  onDelete: () => void;
};

export function MemoEditor({ memo, saving, saveError, deleting, deleteError, onSave, onDelete }: MemoEditorProps) {
  const i18n = useI18n();
  const { t } = i18n;
  const [draft, setDraft] = useState<MemoInput>(() => inputFromMemo(memo));
  const [tagInput, setTagInput] = useState("");
  const [tagStatus, setTagStatus] = useState<string | null>(null);
  const [confirmingDelete, setConfirmingDelete] = useState(false);
  const [editingReminder, setEditingReminder] = useState(false);
  const lastRequestedRef = useRef(inputFromMemo(memo));
  const previousMemoIdRef = useRef(memo?.id ?? null);
  const onSaveRef = useRef(onSave);
  const displayedTagStatus = tagStatus ?? (draft.tags.length >= 10 ? t("memo.tagLimit") : null);

  useEffect(() => {
    onSaveRef.current = onSave;
  }, [onSave]);

  useEffect(() => {
    const next = inputFromMemo(memo);
    const previousMemoId = previousMemoIdRef.current;
    const nextMemoId = memo?.id ?? null;
    previousMemoIdRef.current = nextMemoId;
    setDraft((current) => {
      const switchedRecord = previousMemoId !== nextMemoId && !(previousMemoId === null && nextMemoId !== null);
      if (!switchedRecord && !sameInput(current, lastRequestedRef.current)) return current;
      lastRequestedRef.current = next;
      return next;
    });
    setTagInput("");
    setTagStatus(null);
  }, [memo]);

  useEffect(() => {
    if (sameInput(draft, lastRequestedRef.current)) return;
    const timer = window.setTimeout(() => save(draft), 500);
    return () => window.clearTimeout(timer);
  }, [draft]);

  useEffect(() => {
    if (!saveError || !memo) return;
    setDraft((current) => ({
      ...current,
      tags: memo.tags.map((tag) => tag.name),
      pinned: memo.pinnedAt !== null,
    }));
    lastRequestedRef.current = {
      ...lastRequestedRef.current,
      tags: memo.tags.map((tag) => tag.name),
      pinned: memo.pinnedAt !== null,
    };
  }, [memo, saveError]);

  function save(next = draft) {
    lastRequestedRef.current = next;
    onSaveRef.current(next);
  }

  function updateMetadata(next: MemoInput) {
    setDraft(next);
    save(next);
  }

  function addTag() {
    const name = tagInput.trim();
    if (!name) {
      setTagStatus(t("memo.tagEmpty"));
      return;
    }
    if (Array.from(name).length > 30) {
      setTagStatus(t("memo.tagTooLong"));
      return;
    }
    if (draft.tags.some((tag) => tag.trim().toLocaleLowerCase() === name.toLocaleLowerCase())) {
      setTagInput("");
      setTagStatus(t("memo.tagAlreadyAdded", { name }));
      return;
    }
    if (draft.tags.length >= 10) {
      setTagStatus(t("memo.tagLimit"));
      return;
    }
    updateMetadata({ ...draft, tags: [...draft.tags, name] });
    setTagInput("");
    setTagStatus(t("memo.tagAdded", { name }));
  }

  function removeTag(index: number, name: string) {
    updateMetadata({ ...draft, tags: draft.tags.filter((_, tagIndex) => tagIndex !== index) });
    setTagStatus(t("memo.tagRemoved", { name }));
  }

  return (
    <div className="memo-editor-content" onKeyDown={(event) => {
      if ((event.ctrlKey || event.metaKey) && event.key === "Enter") {
        event.preventDefault();
        save();
      }
    }}>
      <div className="memo-editor-heading">
        <span className="eyebrow">{memo ? t("memo.editorEyebrow") : t("memo.newDraftEyebrow")}</span>
      </div>
      <label className="memo-editor-field">
        <span>{t("memo.titleLabel")}</span>
        <input aria-label={t("memo.titleLabel")} value={draft.title} placeholder={t("memo.titlePlaceholder")} onChange={(event) => setDraft({ ...draft, title: limitCharacters(event.target.value, 200) })} />
        <small>{t("memo.characterCount", { count: Array.from(draft.title).length, limit: 200 })}</small>
      </label>
      <div className="memo-editor-pin-row">
        <Button tone={draft.pinned ? "primary" : "secondary"} aria-pressed={draft.pinned} onClick={() => updateMetadata({ ...draft, pinned: !draft.pinned })}>
          <Icon name="pin" />{draft.pinned ? t("memo.unpinAction") : t("memo.pinAction")}
        </Button>
      </div>
      <div className="memo-tags-editor">
        <span>{t("memo.editTagsLabel")}</span>
        <div className="memo-tags-editor__items">
          {draft.tags.map((tag, index) => (
            <span key={`${tag}-${index}`}>{tag}<button type="button" aria-label={t("memo.removeTag", { name: tag })} title={t("memo.removeTag", { name: tag })} onClick={() => removeTag(index, tag)}>×</button></span>
          ))}
        </div>
        <div className="memo-tags-editor__input">
          <input
            value={tagInput}
            maxLength={30}
            disabled={draft.tags.length >= 10}
            aria-label={t("memo.tagInputLabel")}
            placeholder={t("memo.tagInputPlaceholder")}
            onChange={(event) => { setTagInput(event.target.value); setTagStatus(null); }}
            onKeyDown={(event) => {
              if (event.key === "Enter") {
                event.preventDefault();
                event.stopPropagation();
                addTag();
              }
            }}
          />
          <Button tone="secondary" disabled={draft.tags.length >= 10} onClick={addTag}>{t("memo.addTag")}</Button>
        </div>
        <span className="memo-tags-editor__count">{t("memo.tagCount", { count: draft.tags.length })}</span>
        {displayedTagStatus ? <span className="memo-editor-status" role="status" aria-live="polite">{displayedTagStatus}</span> : null}
      </div>
      <label className="memo-editor-field memo-editor-field--body">
        <span>{t("memo.bodyLabel")}</span>
        <textarea aria-label={t("memo.bodyLabel")} value={draft.body} placeholder={t("memo.bodyPlaceholder")} onChange={(event) => setDraft({ ...draft, body: limitCharacters(event.target.value, 20_000) })} />
        <small>{t("memo.characterCount", { count: Array.from(draft.body).length, limit: 20_000 })}</small>
      </label>
      {draft.reminder || memo?.reminder ? (
        <section className="memo-reminder-summary" aria-label={t("memo.reminderSummaryLabel")}>
          <div><Icon name="clock" /><span><strong>{t("memo.reminderSummaryTitle")}</strong><small>{draft.reminder ? memoReminderScheduleSummary(draft.reminder, i18n) : memoReminderSummary(memo!.reminder!, new Date(), i18n)}</small></span></div>
          <div className="memo-reminder-summary__actions">
            <Button tone="secondary" disabled={saving || deleting} onClick={() => setEditingReminder(true)}>{t("memo.reminderEdit")}</Button>
            {draft.reminder ? <Button tone="ghost" disabled={saving || deleting} onClick={() => updateMetadata({ ...draft, reminder: null })}>{t("memo.reminderCancel")}</Button> : null}
          </div>
        </section>
      ) : null}
      <MemoReminderPermissionNotice active={Boolean(draft.reminder)} />
      <div className="memo-editor-actions">
        <span className="memo-editor-status" role="status">{saving ? t("memo.saving") : saveError ? t("memo.saveFailedStatus") : t("memo.saveReady")}</span>
        <div className="memo-editor-actions__buttons">
          {!draft.reminder && !memo?.reminder ? <Button tone="secondary" disabled={saving || deleting} onClick={() => setEditingReminder(true)}>{t("memo.reminderSet")}</Button> : null}
          {memo ? <Button tone="danger" disabled={saving || deleting} onClick={() => setConfirmingDelete(true)}>{t("memo.deleteAction")}</Button> : null}
          <Button tone="primary" disabled={saving || deleting} onClick={() => save()}>{saveError ? t("memo.retrySave") : t("memo.saveAction")}</Button>
        </div>
      </div>
      {saveError ? <div className="memo-inline-error" role="alert">{saveError}</div> : null}
      <MemoReminderEditor
        open={editingReminder}
        schedule={draft.reminder}
        saving={saving}
        onClose={() => setEditingReminder(false)}
        onSave={(reminder) => {
          updateMetadata({ ...draft, reminder });
          setEditingReminder(false);
        }}
      />
      <Dialog open={confirmingDelete} title={t("memo.deleteTitle")} onClose={() => !deleting && setConfirmingDelete(false)}>
        {memo ? (
          <div className="memo-delete-confirmation">
            <p>{t("memo.deleteDescription", { title: memo.displayTitle })}</p>
            <p>{t("memo.deleteImpact")}</p>
            {deleteError ? <div className="memo-inline-error" role="alert">{deleteError}</div> : null}
            <div className="memo-delete-confirmation__actions">
              <Button tone="ghost" disabled={deleting} onClick={() => setConfirmingDelete(false)}>{t("common.cancel")}</Button>
              <Button tone="danger" disabled={deleting} onClick={onDelete}>{deleting ? t("memo.deleting") : t("memo.confirmDelete")}</Button>
            </div>
          </div>
        ) : null}
      </Dialog>
    </div>
  );
}

function inputFromMemo(memo: MemoRecord | null): MemoInput {
  return memo ? {
    title: memo.title,
    body: memo.body,
    tags: memo.tags.map((tag) => tag.name),
    pinned: memo.pinnedAt !== null,
    reminder: memo.reminder?.status === "active" ? memo.reminder.schedule : null,
  } : { title: "", body: "", tags: [], pinned: false, reminder: null };
}

function limitCharacters(value: string, limit: number): string {
  const characters = Array.from(value);
  return characters.length <= limit ? value : characters.slice(0, limit).join("");
}

function sameInput(left: MemoInput, right: MemoInput): boolean {
  return left.title === right.title
    && left.body === right.body
    && left.pinned === right.pinned
    && left.tags.length === right.tags.length
    && left.tags.every((tag, index) => tag === right.tags[index])
    && JSON.stringify(left.reminder) === JSON.stringify(right.reminder);
}

import { useEffect, useRef, useState } from "react";

import { Button, Panel } from "../../components/ui";
import { useI18n } from "../../i18n/I18nContext";
import { isTauriRuntime } from "../../lib/commandClient";
import { domainErrorMessage } from "../../lib/domainError";
import { memoClient, type MemoClient } from "./memoClient";
import { MemoEditor } from "./MemoEditor";
import { MemoListItem } from "./MemoListItem";
import { LatestMemoSaveQueue } from "./memoSaveQueue";
import type { MemoInput, MemoListQuery, MemoRecord, MemoSummary, MemoTagSummary } from "./types";

export type MemoOpenRequest = {
  memoId: string;
  sequence: number;
};

export type MemoWorkspaceProps = {
  dataRevision: number;
  openRequest: MemoOpenRequest | null;
  runtime?: boolean;
  client?: MemoClient;
  initialQuery?: MemoListQuery;
  onQueryChange?: (query: MemoListQuery) => void;
};

const emptyQuery: MemoListQuery = { search: "", tagId: null };

export function MemoWorkspace({
  dataRevision,
  openRequest,
  runtime = isTauriRuntime(),
  client = memoClient,
  initialQuery = emptyQuery,
  onQueryChange,
}: MemoWorkspaceProps) {
  const { t } = useI18n();
  const [mobileView, setMobileView] = useState<"list" | "editor">("list");
  const [query, setQuery] = useState(initialQuery);
  const [searchInput, setSearchInput] = useState(initialQuery.search);
  const [memos, setMemos] = useState<MemoSummary[]>([]);
  const [tags, setTags] = useState<MemoTagSummary[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [selectedMemo, setSelectedMemo] = useState<MemoRecord | null>(null);
  const [draftOpen, setDraftOpen] = useState(false);
  const [listLoading, setListLoading] = useState(runtime);
  const [detailLoading, setDetailLoading] = useState(false);
  const [listError, setListError] = useState<string | null>(null);
  const [detailError, setDetailError] = useState<string | null>(null);
  const [metadataSaving, setMetadataSaving] = useState(false);
  const [metadataError, setMetadataError] = useState<string | null>(null);
  const [deleting, setDeleting] = useState(false);
  const [deleteError, setDeleteError] = useState<string | null>(null);
  const [invalidated, setInvalidated] = useState(false);
  const [refreshRevision, setRefreshRevision] = useState(0);
  const selectedMemoRef = useRef<MemoRecord | null>(null);
  const saveQueueRef = useRef(new LatestMemoSaveQueue<MemoInput>());
  const hasFilters = query.search.trim().length > 0 || query.tagId !== null;

  useEffect(() => {
    onQueryChange?.(query);
  }, [onQueryChange, query]);

  useEffect(() => {
    selectedMemoRef.current = selectedMemo;
  }, [selectedMemo]);

  useEffect(() => {
    const timer = window.setTimeout(() => {
      setQuery((current) => current.search === searchInput ? current : { ...current, search: searchInput });
    }, 200);
    return () => window.clearTimeout(timer);
  }, [searchInput]);

  useEffect(() => {
    if (!openRequest) return;
    setSelectedId(openRequest.memoId);
    setDraftOpen(false);
    setMobileView("editor");
  }, [openRequest?.sequence]);

  useEffect(() => {
    if (!runtime) return;
    let active = true;
    setListLoading(true);
    setListError(null);
    void client.list(query).then((result) => {
      if (!active) return;
      if (result.ok) setMemos(result.data);
      else setListError(domainErrorMessage(result.error, t));
      setListLoading(false);
    }).catch(() => {
      if (!active) return;
      setListError(t("error.storage"));
      setListLoading(false);
    });
    return () => { active = false; };
  }, [client, dataRevision, query, refreshRevision, runtime, t]);

  useEffect(() => {
    if (!runtime) return;
    let active = true;
    void client.listTags().then((result) => {
      if (!active || !result.ok) return;
      setTags((current) => sameTags(current, result.data) ? current : result.data);
      setQuery((current) => current.tagId !== null && !result.data.some((tag) => tag.id === current.tagId)
        ? { ...current, tagId: null }
        : current);
    }).catch(() => undefined);
    return () => { active = false; };
  }, [client, dataRevision, refreshRevision, runtime]);

  useEffect(() => {
    if (!runtime || selectedId === null) {
      setSelectedMemo(null);
      setDetailLoading(false);
      return;
    }
    let active = true;
    setDetailLoading(selectedMemoRef.current?.id !== selectedId);
    setDetailError(null);
    void client.get(selectedId).then((result) => {
      if (!active) return;
      if (result.ok) {
        selectedMemoRef.current = result.data;
        setSelectedMemo(result.data);
        setInvalidated(false);
      } else if (result.error.code === "MEMO_NOT_FOUND") {
        setSelectedId(null);
        selectedMemoRef.current = null;
        setSelectedMemo(null);
        setInvalidated(true);
        setMobileView("list");
        setRefreshRevision((value) => value + 1);
      } else {
        setDetailError(domainErrorMessage(result.error, t));
      }
      setDetailLoading(false);
    }).catch(() => {
      if (!active) return;
      setDetailError(t("error.storage"));
      setDetailLoading(false);
    });
    return () => { active = false; };
  }, [client, dataRevision, runtime, selectedId, t]);

  function selectMemo(id: string) {
    setSelectedId(id);
    if (selectedId !== id) {
      selectedMemoRef.current = null;
      setSelectedMemo(null);
    }
    setDraftOpen(false);
    setInvalidated(false);
    setMobileView("editor");
    setMetadataError(null);
    setDeleteError(null);
  }

  function startDraft() {
    setSelectedId(null);
    selectedMemoRef.current = null;
    setSelectedMemo(null);
    setDraftOpen(true);
    setDetailError(null);
    setDeleteError(null);
    setMobileView("editor");
  }

  function clearFilters() {
    setSearchInput("");
    setQuery(emptyQuery);
  }

  function selectTag(tagId: string | null) {
    setQuery((current) => ({ ...current, tagId }));
  }

  function saveMemo(input: MemoInput) {
    const next = saveQueueRef.current.enqueue(input);
    if (next) startSave(next);
  }

  function startSave(input: MemoInput) {
    setMetadataSaving(true);
    setMetadataError(null);
    const authority = selectedMemoRef.current;
    const request = authority ? client.update(authority.id, input) : client.create(input);
    void request.then((result) => {
      if (result.ok) {
        selectedMemoRef.current = result.data;
        setRefreshRevision((value) => value + 1);
      } else {
        setMetadataError(domainErrorMessage(result.error, t));
      }
      finishSave(result.ok ? result.data : authority);
    }).catch(() => {
      setMetadataError(t("error.storage"));
      finishSave(authority);
    });
  }

  function finishSave(authority: MemoRecord | null) {
    const queued = saveQueueRef.current.complete();
    if (queued) {
      startSave(queued);
      return;
    }
    selectedMemoRef.current = authority;
    setSelectedMemo(authority);
    if (authority) {
      setSelectedId(authority.id);
      setDraftOpen(false);
    }
    setMetadataSaving(false);
  }

  function deleteMemo() {
    const authority = selectedMemoRef.current;
    if (!authority || deleting) return;
    setDeleting(true);
    setDeleteError(null);
    void client.remove(authority.id).then((result) => {
      if (result.ok) {
        selectedMemoRef.current = null;
        setSelectedMemo(null);
        setSelectedId(null);
        setDraftOpen(false);
        setMobileView("list");
      } else {
        setSelectedMemo(authority);
        setDeleteError(domainErrorMessage(result.error, t));
      }
      setRefreshRevision((value) => value + 1);
      setDeleting(false);
    }).catch(() => {
      setSelectedMemo(authority);
      setDeleteError(t("error.storage"));
      setRefreshRevision((value) => value + 1);
      setDeleting(false);
    });
  }

  return (
    <section className="memo-workspace" aria-label={t("nav.memos")} data-mobile-view={mobileView}>
      <Panel className="memo-list-pane" aria-label={t("memo.listPaneLabel")}>
        <div className="memo-list-tools">
          <label className="memo-search-field">
            <span>{t("memo.searchLabel")}</span>
            <input type="search" value={searchInput} placeholder={t("memo.searchPlaceholder")} onChange={(event) => setSearchInput(event.target.value)} />
          </label>
          <div className="memo-tag-filters" role="group" aria-label={t("memo.filterLabel")}>
            <button type="button" aria-pressed={query.tagId === null} onClick={() => selectTag(null)}>{t("memo.allTags")}</button>
            {tags.map((tag) => (
              <button key={tag.id} type="button" aria-pressed={query.tagId === tag.id} onClick={() => selectTag(tag.id)}>
                {tag.name}<span>{tag.memoCount}</span>
              </button>
            ))}
          </div>
          {!listLoading && !listError ? <span className="memo-result-count" aria-live="polite">{t("memo.resultCount", { count: memos.length })}</span> : null}
        </div>
        {listLoading ? <MemoSkeleton label={t("memo.listLoading")} /> : null}
        {!listLoading && listError ? <div className="memo-state" role="alert"><p>{listError}</p></div> : null}
        {!listLoading && !listError && invalidated ? <div className="memo-inline-status" role="status">{t("memo.invalidated")}</div> : null}
        {!listLoading && !listError && memos.length === 0 && !hasFilters ? (
          <div className="memo-state">
            <span className="eyebrow">{t("memo.emptyEyebrow")}</span>
            <h2>{t("memo.emptyTitle")}</h2>
            <p>{t("memo.emptyDescription")}</p>
            <Button tone="primary" onClick={startDraft}>{t("memo.createFirst")}</Button>
          </div>
        ) : null}
        {!listLoading && !listError && memos.length === 0 && hasFilters ? (
          <div className="memo-state">
            <span className="eyebrow">{t("memo.zeroEyebrow")}</span>
            <h2>{t("memo.zeroTitle")}</h2>
            <p>{t("memo.zeroDescription")}</p>
            <Button tone="secondary" onClick={clearFilters}>{t("memo.clearFilters")}</Button>
          </div>
        ) : null}
        {!listLoading && !listError && memos.length > 0 ? (
          <div className="memo-list-preview">
            <span className="eyebrow">{t("memo.listEyebrow")}</span>
            <h2>{t("memo.listTitle")}</h2>
            <ul>
              {memos.map((memo) => (
                <li key={memo.id}>
                  <MemoListItem memo={memo} selected={selectedId === memo.id} onSelect={selectMemo} />
                </li>
              ))}
            </ul>
            <Button className="memo-new-button" tone="secondary" onClick={startDraft}>{t("memo.createNew")}</Button>
          </div>
        ) : null}
      </Panel>
      <Panel className="memo-editor" aria-label={t("memo.editorPaneLabel")}>
        <Button className="memo-back-button" tone="ghost" onClick={() => setMobileView("list")}>
          {t("memo.backToList")}
        </Button>
        {detailLoading ? <MemoSkeleton label={t("memo.detailLoading")} /> : null}
        {!detailLoading && detailError ? <div className="memo-state" role="alert"><p>{detailError}</p></div> : null}
        {!detailLoading && !detailError && (selectedMemo || draftOpen) ? (
          <MemoEditor
            memo={selectedMemo}
            saving={metadataSaving}
            saveError={metadataError}
            deleting={deleting}
            deleteError={deleteError}
            onSave={saveMemo}
            onDelete={deleteMemo}
          />
        ) : null}
        {!detailLoading && !detailError && !selectedMemo && !draftOpen ? (
          <div className="memo-pane-placeholder">
            <span className="eyebrow">{t("memo.editorEyebrow")}</span>
            <h2>{t("memo.editorTitle")}</h2>
            <p>{t("memo.editorDescription")}</p>
          </div>
        ) : null}
      </Panel>
    </section>
  );
}

function MemoSkeleton({ label }: { label: string }) {
  return (
    <div className="memo-skeleton" aria-busy="true" aria-label={label}>
      <span /><span /><span />
    </div>
  );
}

function sameTags(current: MemoTagSummary[], next: MemoTagSummary[]): boolean {
  return current.length === next.length && current.every((tag, index) => {
    const candidate = next[index];
    return candidate?.id === tag.id && candidate.name === tag.name && candidate.memoCount === tag.memoCount;
  });
}

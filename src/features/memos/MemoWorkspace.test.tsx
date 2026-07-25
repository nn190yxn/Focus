// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";

import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { I18nProvider } from "../../i18n/I18nContext";
import type { CommandResult } from "../../lib/commandClient";
import type { MemoClient } from "./memoClient";
import { MemoWorkspace } from "./MemoWorkspace";
import type { MemoRecord, MemoSummary } from "./types";

const memoSummary: MemoSummary = {
  id: "memo-1",
  displayTitle: "项目复盘",
  bodyPreview: "整理下一步",
  tags: [],
  pinnedAt: null,
  reminder: null,
  updatedAt: "2026-07-24T08:00:00Z",
};

const memoRecord: MemoRecord = {
  ...memoSummary,
  title: "项目复盘",
  body: "整理下一步",
  createdAt: "2026-07-24T07:00:00Z",
};

function createClient(overrides: Partial<MemoClient> = {}): MemoClient {
  return {
    list: async () => ({ ok: true, data: [], version: 1 }),
    get: async () => ({ ok: false, error: { code: "MEMO_NOT_FOUND", message: "missing" } }),
    create: vi.fn(),
    update: vi.fn(),
    remove: vi.fn(),
    listTags: async () => ({ ok: true, data: [], version: 1 }),
    ...overrides,
  };
}

describe("MemoWorkspace", () => {
  it("renders separate list and editor panels", () => {
    render(
      <I18nProvider locale="zh-CN">
        <MemoWorkspace dataRevision={0} openRequest={null} />
      </I18nProvider>,
    );

    const listPane = screen.getByRole("region", { name: "备忘录列表栏" });
    const editorPane = screen.getByRole("region", { name: "备忘录编辑栏" });
    expect(listPane).toHaveClass("panel", "memo-list-pane");
    expect(editorPane).toHaveClass("panel", "memo-editor");
    expect(screen.getByRole("heading", { name: "写下第一条备忘录" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "备忘录编辑器" })).toBeInTheDocument();
  });

  it("opens the editor for a notification request and returns to the retained list", () => {
    const { container, rerender } = render(
      <I18nProvider locale="zh-CN">
        <MemoWorkspace dataRevision={0} openRequest={null} />
      </I18nProvider>,
    );
    const workspace = container.querySelector(".memo-workspace");
    expect(workspace).toHaveAttribute("data-mobile-view", "list");

    rerender(
      <I18nProvider locale="zh-CN">
        <MemoWorkspace dataRevision={0} openRequest={{ memoId: "memo-1", sequence: 1 }} />
      </I18nProvider>,
    );
    expect(workspace).toHaveAttribute("data-mobile-view", "editor");
    expect(screen.getByRole("region", { name: "备忘录列表栏" })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "返回备忘录列表" }));
    expect(workspace).toHaveAttribute("data-mobile-view", "list");
  });

  it("shows loading skeletons before the authoritative list resolves", () => {
    const list = vi.fn(() => new Promise<CommandResult<MemoSummary[]>>(() => undefined));
    const client = createClient({ list });
    render(
      <I18nProvider locale="zh-CN">
        <MemoWorkspace dataRevision={0} openRequest={null} runtime client={client} />
      </I18nProvider>,
    );

    expect(screen.getByLabelText("正在加载备忘录列表")).toHaveAttribute("aria-busy", "true");
    expect(screen.queryByRole("button", { name: "创建第一条备忘录" })).not.toBeInTheDocument();
  });

  it("renders the first-memo action for an empty authoritative list", async () => {
    const client = createClient();
    const { container } = render(
      <I18nProvider locale="zh-CN">
        <MemoWorkspace dataRevision={0} openRequest={null} runtime client={client} />
      </I18nProvider>,
    );

    const create = await screen.findByRole("button", { name: "创建第一条备忘录" });
    fireEvent.click(create);
    expect(container.querySelector(".memo-workspace")).toHaveAttribute("data-mobile-view", "editor");
  });

  it("creates a new draft on first save and then treats it as authoritative", async () => {
    const createdMemo: MemoRecord = {
      ...memoRecord,
      id: "memo-created",
      title: "新想法",
      body: "保留纯文本内容",
      displayTitle: "新想法",
    };
    const create = vi.fn(async () => ({ ok: true as const, data: createdMemo, version: 1 }));
    const update = vi.fn(async () => ({ ok: true as const, data: { ...createdMemo, pinnedAt: "2026-07-24T09:00:00Z" }, version: 1 }));
    const get = vi.fn(async () => ({ ok: true as const, data: createdMemo, version: 1 }));
    const client = createClient({ create, update, get });
    render(
      <I18nProvider locale="zh-CN">
        <MemoWorkspace dataRevision={0} openRequest={null} runtime client={client} />
      </I18nProvider>,
    );

    fireEvent.click(await screen.findByRole("button", { name: "创建第一条备忘录" }));
    fireEvent.change(screen.getByRole("textbox", { name: "标题" }), { target: { value: "新想法" } });
    fireEvent.change(screen.getByRole("textbox", { name: "正文" }), { target: { value: "保留纯文本内容" } });
    fireEvent.click(screen.getByRole("button", { name: "保存备忘录" }));

    await waitFor(() => expect(create).toHaveBeenCalledWith({
      title: "新想法",
      body: "保留纯文本内容",
      tags: [],
      pinned: false,
      reminder: null,
    }));
    await waitFor(() => expect(screen.getByRole("button", { name: "置顶" })).toBeEnabled());
    fireEvent.click(screen.getByRole("button", { name: "置顶" }));
    await waitFor(() => expect(update).toHaveBeenCalledWith("memo-created", expect.objectContaining({ pinned: true })));
    expect(create).toHaveBeenCalledTimes(1);
  });

  it("clears a zero-result query and reloads the complete list", async () => {
    const list = vi.fn(async () => ({ ok: true as const, data: [], version: 1 }));
    const client = createClient({ list });
    render(
      <I18nProvider locale="zh-CN">
        <MemoWorkspace dataRevision={0} openRequest={null} runtime client={client} initialQuery={{ search: "missing", tagId: "tag-1" }} />
      </I18nProvider>,
    );

    fireEvent.click(await screen.findByRole("button", { name: "清除搜索与筛选" }));
    await waitFor(() => expect(list).toHaveBeenLastCalledWith({ search: "", tagId: null }));
    expect(await screen.findByRole("button", { name: "创建第一条备忘录" })).toBeInTheDocument();
  });

  it("starts a new draft when authoritative records already exist", async () => {
    const list = vi.fn(async () => ({ ok: true as const, data: [memoSummary], version: 1 }));
    const client = createClient({ list });
    render(<I18nProvider locale="zh-CN"><MemoWorkspace dataRevision={0} openRequest={null} runtime client={client} /></I18nProvider>);

    fireEvent.click(await screen.findByRole("button", { name: "新建备忘录" }));
    expect(screen.getByRole("textbox", { name: "标题" })).toHaveValue("");
    expect(screen.getByRole("textbox", { name: "正文" })).toHaveValue("");
  });

  it("debounces search and combines it with the selected tag", async () => {
    const list = vi.fn(async () => ({ ok: true as const, data: [memoSummary], version: 1 }));
    const listTags = vi.fn(async () => ({ ok: true as const, data: [{ id: "tag-work", name: "工作", memoCount: 4 }], version: 1 }));
    const client = createClient({ list, listTags });
    render(
      <I18nProvider locale="zh-CN">
        <MemoWorkspace dataRevision={0} openRequest={null} runtime client={client} />
      </I18nProvider>,
    );

    const search = screen.getByRole("searchbox", { name: "搜索备忘录" });
    const tag = await screen.findByRole("button", { name: /^工作\s*4$/ });
    await waitFor(() => expect(list).toHaveBeenCalledWith({ search: "", tagId: null }));
    list.mockClear();

    fireEvent.change(search, { target: { value: "复盘" } });
    expect(list).not.toHaveBeenCalled();
    await waitFor(() => expect(list).toHaveBeenCalledWith({ search: "复盘", tagId: null }));

    fireEvent.click(tag);
    await waitFor(() => expect(list).toHaveBeenLastCalledWith({ search: "复盘", tagId: "tag-work" }));
    expect(tag).toHaveAttribute("aria-pressed", "true");
  });

  it("closes an invalidated detail and refreshes the authoritative list", async () => {
    const list = vi.fn(async () => ({ ok: true as const, data: [memoSummary], version: 1 }));
    const get = vi.fn(async () => ({ ok: false as const, error: { code: "MEMO_NOT_FOUND", message: "internal detail" } }));
    const client = createClient({ list, get });
    const { container } = render(
      <I18nProvider locale="zh-CN">
        <MemoWorkspace dataRevision={0} openRequest={{ memoId: "memo-1", sequence: 1 }} runtime client={client} />
      </I18nProvider>,
    );

    expect(await screen.findByRole("status")).toHaveTextContent("这条备忘录已不存在，列表已刷新。");
    expect(container.querySelector(".memo-workspace")).toHaveAttribute("data-mobile-view", "list");
    expect(get).toHaveBeenCalledWith("memo-1");
    await waitFor(() => expect(list).toHaveBeenCalledTimes(2));
    expect(screen.queryByText("internal detail")).not.toBeInTheDocument();
  });

  it("rolls back an optimistic pin when the authoritative save fails", async () => {
    const list = vi.fn(async () => ({ ok: true as const, data: [memoSummary], version: 1 }));
    const get = vi.fn(async () => ({ ok: true as const, data: memoRecord, version: 1 }));
    const update = vi.fn(async () => ({ ok: false as const, error: { code: "MEMO_SAVE_FAILED", message: "internal" } }));
    const client = createClient({ list, get, update });
    render(
      <I18nProvider locale="zh-CN">
        <MemoWorkspace dataRevision={0} openRequest={{ memoId: "memo-1", sequence: 1 }} runtime client={client} />
      </I18nProvider>,
    );

    fireEvent.click(await screen.findByRole("button", { name: "置顶" }));
    expect(update).toHaveBeenCalledWith("memo-1", expect.objectContaining({ pinned: true }));
    expect(await screen.findByRole("alert")).toHaveTextContent("备忘录保存失败，草稿已保留，请重新保存。");
    expect(await screen.findByRole("button", { name: "置顶" })).toHaveAttribute("aria-pressed", "false");
    expect(screen.getByRole("button", { name: "重新保存" })).toBeInTheDocument();
    expect(screen.queryByText("internal")).not.toBeInTheDocument();
  });

  it("retains an edited title after failure and retries the latest draft", async () => {
    const list = vi.fn(async () => ({ ok: true as const, data: [memoSummary], version: 1 }));
    const get = vi.fn(async () => ({ ok: true as const, data: memoRecord, version: 1 }));
    const savedMemo = { ...memoRecord, title: "保留的草稿", displayTitle: "保留的草稿" };
    const update = vi.fn()
      .mockResolvedValueOnce({ ok: false as const, error: { code: "MEMO_SAVE_FAILED", message: "internal" } })
      .mockResolvedValueOnce({ ok: true as const, data: savedMemo, version: 1 });
    const client = createClient({ list, get, update });
    render(
      <I18nProvider locale="zh-CN">
        <MemoWorkspace dataRevision={0} openRequest={{ memoId: "memo-1", sequence: 1 }} runtime client={client} />
      </I18nProvider>,
    );

    const title = await screen.findByRole("textbox", { name: "标题" });
    fireEvent.change(title, { target: { value: "保留的草稿" } });
    fireEvent.click(screen.getByRole("button", { name: "保存备忘录" }));
    expect(await screen.findByRole("button", { name: "重新保存" })).toBeInTheDocument();
    expect(title).toHaveValue("保留的草稿");

    fireEvent.click(screen.getByRole("button", { name: "重新保存" }));
    await waitFor(() => expect(update).toHaveBeenCalledTimes(2));
    expect(update).toHaveBeenLastCalledWith("memo-1", expect.objectContaining({ title: "保留的草稿" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "保存备忘录" })).toBeEnabled());
    expect(title).toHaveValue("保留的草稿");
  });

  it("serializes saves and commits the latest draft after an earlier request completes", async () => {
    let resolveFirst: (result: CommandResult<MemoRecord>) => void = () => undefined;
    let resolveSecond: (result: CommandResult<MemoRecord>) => void = () => undefined;
    const first = new Promise<CommandResult<MemoRecord>>((resolve) => { resolveFirst = resolve; });
    const second = new Promise<CommandResult<MemoRecord>>((resolve) => { resolveSecond = resolve; });
    const list = vi.fn(async () => ({ ok: true as const, data: [memoSummary], version: 1 }));
    const get = vi.fn(async () => ({ ok: true as const, data: memoRecord, version: 1 }));
    const update = vi.fn().mockReturnValueOnce(first).mockReturnValueOnce(second);
    const client = createClient({ list, get, update });
    render(
      <I18nProvider locale="zh-CN">
        <MemoWorkspace dataRevision={0} openRequest={{ memoId: "memo-1", sequence: 1 }} runtime client={client} />
      </I18nProvider>,
    );

    const title = await screen.findByRole("textbox", { name: "标题" });
    fireEvent.change(title, { target: { value: "第一版" } });
    fireEvent.click(screen.getByRole("button", { name: "保存备忘录" }));
    fireEvent.change(title, { target: { value: "最终版" } });
    await waitFor(() => expect(update).toHaveBeenCalledTimes(1), { timeout: 700 });

    await act(async () => resolveFirst({ ok: true, data: { ...memoRecord, title: "第一版" }, version: 1 }));
    await waitFor(() => expect(update).toHaveBeenCalledWith("memo-1", expect.objectContaining({ title: "最终版" })), { timeout: 700 });
    expect(title).toHaveValue("最终版");

    await act(async () => resolveSecond({ ok: true, data: { ...memoRecord, title: "最终版" }, version: 1 }));
    await waitFor(() => expect(screen.getByRole("button", { name: "保存备忘录" })).toBeEnabled());
    expect(title).toHaveValue("最终版");
  });

  it("confirms deletion with the display title and preserves the memo when cancelled", async () => {
    const list = vi.fn(async () => ({ ok: true as const, data: [memoSummary], version: 1 }));
    const get = vi.fn(async () => ({ ok: true as const, data: memoRecord, version: 1 }));
    const remove = vi.fn(async () => ({ ok: true as const, data: null, version: 1 }));
    const client = createClient({ list, get, remove });
    render(
      <I18nProvider locale="zh-CN">
        <MemoWorkspace dataRevision={0} openRequest={{ memoId: "memo-1", sequence: 1 }} runtime client={client} />
      </I18nProvider>,
    );

    const deleteAction = await screen.findByRole("button", { name: "删除备忘录" });
    fireEvent.click(deleteAction);
    expect(screen.getByRole("dialog", { name: "确认删除备忘录" })).toHaveTextContent("确定删除“项目复盘”吗？");
    expect(screen.getByRole("dialog", { name: "确认删除备忘录" })).toHaveTextContent("标签关联和未发送提醒");
    fireEvent.click(screen.getByRole("button", { name: "取消" }));
    expect(remove).not.toHaveBeenCalled();
    expect(screen.getByRole("textbox", { name: "标题" })).toHaveValue("项目复盘");
  });

  it("removes a confirmed memo and retains it with an error when deletion fails", async () => {
    const list = vi.fn(async () => ({ ok: true as const, data: [memoSummary], version: 1 }));
    const get = vi.fn(async () => ({ ok: true as const, data: memoRecord, version: 1 }));
    const remove = vi.fn()
      .mockResolvedValueOnce({ ok: false as const, error: { code: "MEMO_DELETE_FAILED", message: "internal" } })
      .mockResolvedValueOnce({ ok: true as const, data: null, version: 1 });
    const client = createClient({ list, get, remove });
    const { container } = render(
      <I18nProvider locale="zh-CN">
        <MemoWorkspace dataRevision={0} openRequest={{ memoId: "memo-1", sequence: 1 }} runtime client={client} />
      </I18nProvider>,
    );

    fireEvent.click(await screen.findByRole("button", { name: "删除备忘录" }));
    fireEvent.click(screen.getByRole("button", { name: "确认删除" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("备忘录删除失败，记录已保留，请重试。");
    expect(screen.getByRole("textbox", { name: "标题" })).toHaveValue("项目复盘");
    expect(screen.queryByText("internal")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "确认删除" }));
    await waitFor(() => expect(remove).toHaveBeenCalledTimes(2));
    await waitFor(() => expect(screen.queryByRole("dialog", { name: "确认删除备忘录" })).not.toBeInTheDocument());
    expect(container.querySelector(".memo-workspace")).toHaveAttribute("data-mobile-view", "list");
    expect(screen.getByRole("heading", { name: "备忘录编辑器" })).toBeInTheDocument();
    await waitFor(() => expect(list).toHaveBeenCalledTimes(3));
  });
});

import { beforeEach, describe, expect, it, vi } from "vitest";

import type { MemoInput, MemoListQuery } from "./types";

const invokeCommand = vi.hoisted(() => vi.fn());

vi.mock("../../lib/commandClient", () => ({ invokeCommand }));

import { memoClient } from "./memoClient";

describe("memoClient", () => {
  beforeEach(() => invokeCommand.mockResolvedValue({ ok: true, data: null, version: 1 }));

  it("maps memo reads and writes to typed Tauri commands", async () => {
    const query: MemoListQuery = { search: "launch", tagId: "tag-1" };
    const input: MemoInput = {
      title: "Launch",
      body: "Review checklist",
      tags: ["Work"],
      pinned: true,
      reminder: {
        kind: "recurring",
        frequency: "weekdays",
        interval: 1,
        weekdays: [],
        monthlyDay: null,
        localTime: "09:00",
        startsOn: "2026-07-23",
        endsOn: null,
        timezone: "Asia/Shanghai",
      },
    };

    await memoClient.list(query);
    await memoClient.get("memo-1");
    await memoClient.create(input);
    await memoClient.update("memo-1", input);
    await memoClient.remove("memo-1");
    await memoClient.listTags();

    expect(invokeCommand.mock.calls).toEqual([
      ["memo_list", { query }],
      ["memo_get", { id: "memo-1" }],
      ["memo_create", { input }],
      ["memo_update", { id: "memo-1", input }],
      ["memo_remove", { id: "memo-1" }],
      ["memo_tag_list"],
    ]);
  });
});

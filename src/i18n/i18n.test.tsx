// @vitest-environment jsdom
import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { createI18n } from "./I18nContext";
import { resolveLocale, useResolvedLocale } from "./locale";
import { enUSMessages, zhCNMessages } from "./messages";

afterEach(() => {
  vi.restoreAllMocks();
});

describe("i18n", () => {
  it("keeps both message catalogs structurally complete", () => {
    expect(Object.keys(enUSMessages).sort()).toEqual(Object.keys(zhCNMessages).sort());
  });

  it("resolves explicit and system language preferences", () => {
    expect(resolveLocale("zhCn", ["en-US"])).toBe("zh-CN");
    expect(resolveLocale("en", ["zh-CN"])).toBe("en-US");
    expect(resolveLocale("system", ["zh-Hans-CN", "en-US"])).toBe("zh-CN");
    expect(resolveLocale("system", ["fr-FR"])).toBe("en-US");
  });

  it("reacts to operating-system language changes", () => {
    const languages = vi.spyOn(window.navigator, "languages", "get").mockReturnValue(["zh-CN"]);
    const { result } = renderHook(() => useResolvedLocale("system"));
    expect(result.current).toBe("zh-CN");

    languages.mockReturnValue(["en-US"]);
    act(() => window.dispatchEvent(new Event("languagechange")));
    expect(result.current).toBe("en-US");
  });

  it("formats translated parameters, dates, times, and relative values", () => {
    const zh = createI18n("zh-CN");
    const en = createI18n("en-US");

    expect(zh.t("page.focusTaskDescription", { title: "季度复盘" })).toContain("季度复盘");
    expect(en.t("page.focusTaskDescription", { title: "Quarterly review" })).toContain("Quarterly review");
    expect(zh.formatDate("2026-07-20", { month: "long", day: "numeric" })).toContain("7月");
    expect(en.formatDate("2026-07-20", { month: "long", day: "numeric" })).toContain("July");
    expect(en.formatTime("2026-07-20T13:05:00", { hour: "2-digit", minute: "2-digit", hour12: false })).toContain("13:05");
    expect(zh.formatRelativeTime(-1, "day")).toContain("昨天");
    expect(en.formatRelativeTime(-1, "day")).toContain("yesterday");
  });
});

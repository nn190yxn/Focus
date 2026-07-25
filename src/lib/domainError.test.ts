import { describe, expect, it } from "vitest";

import { createI18n } from "../i18n/I18nContext";
import { domainErrorMessage } from "./domainError";

describe("domainErrorMessage", () => {
  it("maps actionable errors in both supported locales", () => {
    const error = { code: "TASK_DATE_IN_PAST", message: "internal detail", field: "scheduledDate" };

    expect(domainErrorMessage(error, createI18n("zh-CN").t)).toBe("任务日期不能早于今天。");
    expect(domainErrorMessage(error, createI18n("en-US").t)).toBe("Task dates cannot be earlier than today.");

    const pausedProject = { code: "FOCUS_PROJECT_PAUSED", message: "internal detail" };
    expect(domainErrorMessage(pausedProject, createI18n("zh-CN").t)).toBe("该项目已暂停，请恢复项目后开始专注。");
    expect(domainErrorMessage(pausedProject, createI18n("en-US").t)).toBe("This project is paused. Resume it before starting focus.");
  });

  it("maps storage failures to a safe message", () => {
    const error = {
      code: "DATABASE_ERROR",
      message: "C:\\Users\\private\\arrive-focus.sqlite3: secret task title",
    };
    const message = domainErrorMessage(error, createI18n("zh-CN").t);

    expect(message).toBe("本地数据暂时不可用，请稍后重试。");
    expect(message).not.toContain("private");
    expect(message).not.toContain("secret task title");
  });

  it.each([
    ["MEMO_TITLE_INVALID", "备忘录标题过长，请精简后重试。", "The memo title is too long. Shorten it and try again."],
    ["MEMO_BODY_INVALID", "备忘录正文过长，请精简后重试。", "The memo body is too long. Shorten it and try again."],
    ["MEMO_TAG_LIMIT_EXCEEDED", "请检查标签内容，每条备忘录最多添加 10 个标签。", "Check the tags. Each memo can have up to 10 tags."],
    ["MEMO_NOT_FOUND", "该备忘录已不存在，请刷新列表。", "This memo no longer exists. Refresh the list."],
    ["MEMO_SAVE_FAILED", "备忘录保存失败，草稿已保留，请重新保存。", "The memo could not be saved. Your draft was preserved. Save it again."],
    ["MEMO_DELETE_FAILED", "备忘录删除失败，记录已保留，请重试。", "The memo could not be deleted. The record was preserved. Try again."],
    ["MEMO_REMINDER_TIMEZONE_INVALID", "提醒设置无效，请检查日期、时间、频率和时区。", "Check the reminder date, time, frequency, and time zone."],
  ])("maps %s to safe memo guidance in both locales", (code, zhCN, enUS) => {
    const error = { code, message: "private memo title, body, tags, and search text" };

    expect(domainErrorMessage(error, createI18n("zh-CN").t)).toBe(zhCN);
    expect(domainErrorMessage(error, createI18n("en-US").t)).toBe(enUS);
  });

  it("uses a safe fallback for unknown codes", () => {
    const error = { code: "FUTURE_INTERNAL_ERROR", message: "private note body" };

    expect(domainErrorMessage(error, createI18n("en-US").t)).toBe("The operation failed. Please try again.");
  });
});

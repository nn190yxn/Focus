// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { UpdateSettingsPanel } from "./UpdateSettingsPanel";

describe("UpdateSettingsPanel", () => {
  it("points users to GitHub Releases instead of running an unconfigured updater", () => {
    render(<UpdateSettingsPanel />);

    expect(screen.getByText("手动更新")).toBeInTheDocument();
    expect(screen.getByText("当前版本通过 GitHub Release 手动下载更新。下载后请核对 SHA-256 校验值，再运行安装包。")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "检查更新" })).not.toBeInTheDocument();
    expect(screen.getByRole("link", { name: "打开 GitHub Release" })).toHaveAttribute("href", "https://github.com/nn190yxn/Focus/releases/latest");
  });
});

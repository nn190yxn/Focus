// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { GeneralSettingsPanel } from "./GeneralSettingsPanel";
import { defaultGeneralPreferences } from "./types";

describe("GeneralSettingsPanel", () => {
  it("saves each general preference as a narrow patch", async () => {
    const onSave = vi.fn(async (patch) => ({ ...defaultGeneralPreferences, ...patch }));
    render(<GeneralSettingsPanel preferences={defaultGeneralPreferences} onSave={onSave} />);

    fireEvent.change(screen.getByLabelText("界面语言"), { target: { value: "en" } });
    await waitFor(() => expect(onSave).toHaveBeenCalledWith({ language: "en" }));

    fireEvent.change(screen.getByLabelText("界面外观"), { target: { value: "dark" } });
    await waitFor(() => expect(onSave).toHaveBeenCalledWith({ appearance: "dark" }));

    fireEvent.change(screen.getByLabelText("设置主题"), { target: { value: "office" } });
    await waitFor(() => expect(onSave).toHaveBeenCalledWith({ theme: "office" }));

    fireEvent.click(screen.getByLabelText("启用后台运行"));
    await waitFor(() => expect(onSave).toHaveBeenCalledWith({ backgroundRunning: false }));
  });

  it("keeps the current preference and exposes save failures", async () => {
    const onSave = vi.fn(async () => {
      throw new Error("C:\\Private\\settings.json 包含机密任务标题");
    });
    render(<GeneralSettingsPanel preferences={defaultGeneralPreferences} onSave={onSave} />);

    fireEvent.change(screen.getByLabelText("设置主题"), { target: { value: "noir" } });

    expect(await screen.findByRole("alert")).toHaveTextContent("设置保存失败");
    expect(screen.queryByText(/机密任务标题/)).not.toBeInTheDocument();
    expect(screen.getByLabelText("设置主题")).toHaveValue("mint");
  });
});

// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { I18nProvider } from "../../i18n/I18nContext";
import type { ProjectClient } from "./projectClient";
import { ProjectWorkspace } from "./ProjectWorkspace";
import type { ProjectDetail, ProjectInput, ProjectRecord, ProjectSummary } from "./types";

const today = "2026-07-21";
const alpha = project("project-alpha", "Alpha");
const alphaSummary = summary(alpha);
const pendingTask = {
  id: "task-1",
  projectId: alpha.id,
  title: "Ship persistence",
  category: "work" as const,
  priority: 2,
  scheduledDate: today,
  scheduledTime: "10:00",
  status: "pending" as const,
  completedAt: null,
  createdAt: "2026-07-21T08:00:00.000Z",
  updatedAt: "2026-07-21T08:00:00.000Z",
};
const alphaDetail: ProjectDetail = { summary: { ...alphaSummary, nextTaskTitle: pendingTask.title }, tasks: [pendingTask] };

describe("ProjectWorkspace", () => {
  beforeEach(() => vi.clearAllMocks());

  it("loads authoritative project summaries and details", async () => {
    const client = mockClient([alphaSummary], alphaDetail);
    const onProjectsChange = vi.fn();

    renderWorkspace(client, { onProjectsChange });

    expect(await screen.findByRole("heading", { name: "Alpha" })).toBeInTheDocument();
    expect(client.list).toHaveBeenCalledWith(null, today);
    expect(await screen.findByRole("heading", { name: "Ship persistence" })).toBeInTheDocument();
    expect(client.get).toHaveBeenCalledWith(alpha.id, today);
    expect(onProjectsChange).toHaveBeenCalledWith([alphaSummary]);
  });

  it("creates a project with the complete persisted input", async () => {
    const created = project("project-new", "New project");
    const createdSummary = summary(created);
    const createdDetail: ProjectDetail = { summary: createdSummary, tasks: [] };
    const client = mockClient([alphaSummary], alphaDetail);
    vi.mocked(client.list).mockResolvedValueOnce(success([alphaSummary])).mockResolvedValueOnce(success([alphaSummary, createdSummary]));
    vi.mocked(client.get).mockResolvedValueOnce(success(alphaDetail)).mockResolvedValueOnce(success(createdDetail));
    vi.mocked(client.create).mockResolvedValue(success(created));

    renderWorkspace(client);
    await screen.findByRole("heading", { name: "Alpha" });
    fireEvent.click(screen.getByRole("button", { name: "新建项目" }));
    fireEvent.change(screen.getByLabelText("项目名称"), { target: { value: "New project" } });
    fireEvent.change(screen.getByLabelText("项目说明"), { target: { value: "Persistent plan" } });
    fireEvent.change(screen.getByLabelText("项目颜色"), { target: { value: "#123456" } });
    fireEvent.change(screen.getByLabelText("项目图标"), { target: { value: "NP" } });
    fireEvent.change(screen.getByLabelText("目标日期"), { target: { value: "2026-08-01" } });
    fireEvent.click(screen.getByRole("button", { name: "保存项目" }));

    const expected: ProjectInput = { name: "New project", description: "Persistent plan", color: "#123456", icon: "NP", startedOn: today, targetOn: "2026-08-01" };
    await waitFor(() => expect(client.create).toHaveBeenCalledWith(expected));
    expect(await screen.findByRole("heading", { name: "New project" })).toBeInTheDocument();
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("keeps edits open after a failed write and supports task actions", async () => {
    const client = mockClient([alphaSummary], alphaDetail);
    vi.mocked(client.update).mockResolvedValue({ ok: false, error: { code: "PROJECT_NAME_INVALID", message: "sensitive detail" } });
    const taskActions = { setCompleted: vi.fn(async () => success({ task: { ...pendingTask, status: "completed" as const }, checkItems: [] })) };
    const onAddTask = vi.fn();
    const onStartFocus = vi.fn();

    renderWorkspace(client, { taskActions, onAddTask, onStartFocus });
    await screen.findByRole("heading", { name: "Alpha" });
    fireEvent.click(screen.getByRole("button", { name: "编辑" }));
    fireEvent.change(screen.getByLabelText("项目名称"), { target: { value: "" } });
    fireEvent.change(screen.getByLabelText("项目名称"), { target: { value: "Rejected name" } });
    fireEvent.click(screen.getByRole("button", { name: "保存项目" }));

    expect(await screen.findByRole("dialog", { name: "编辑项目" })).toBeInTheDocument();
    expect(screen.getByDisplayValue("Rejected name")).toBeInTheDocument();
    expect(screen.queryByText("sensitive detail")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "取消" }));
    fireEvent.click(screen.getByRole("button", { name: "开始专注" }));
    expect(onStartFocus).toHaveBeenCalledWith(pendingTask, alpha);
    fireEvent.click(screen.getByRole("radio", { name: "任务" }));
    fireEvent.click(screen.getByRole("button", { name: "添加任务" }));
    fireEvent.click(screen.getByRole("button", { name: "完成任务：Ship persistence" }));
    expect(onAddTask).toHaveBeenCalledWith(alpha);
    await waitFor(() => expect(taskActions.setCompleted).toHaveBeenCalledWith(pendingTask.id, true));
  });
});

function renderWorkspace(client: ProjectClient, props: Partial<React.ComponentProps<typeof ProjectWorkspace>> = {}) {
  return render(<I18nProvider locale="zh-CN"><ProjectWorkspace today={today} runtime client={client} {...props} /></I18nProvider>);
}

function mockClient(projects: ProjectSummary[], detail: ProjectDetail): ProjectClient {
  return {
    list: vi.fn(async () => success(projects)),
    get: vi.fn(async () => success(detail)),
    create: vi.fn(),
    update: vi.fn(),
    setStatus: vi.fn(),
  };
}

function success<T>(data: T) {
  return { ok: true as const, data, version: 1 };
}

function project(id: string, name: string): ProjectRecord {
  return { id, name, description: "Project plan", color: "#4eaa98", icon: "AL", status: "active", startedOn: today, targetOn: "2026-08-21", createdAt: "2026-07-21T08:00:00.000Z", updatedAt: "2026-07-21T08:00:00.000Z" };
}

function summary(value: ProjectRecord): ProjectSummary {
  return { project: value, aggregation: { activeTaskCount: 1, completedTaskCount: 0, totalTaskCount: 1, completionPercent: 0, focusSeconds: 0 }, nextTaskTitle: null, nextTaskDate: today, deadlineState: "onTrack", deadlineDays: 31 };
}

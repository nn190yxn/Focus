export type WidgetSize = "compact" | "standard" | "expanded";
export type WidgetMode = "desktop" | "floating";
export type WidgetModule = "clock" | "currentFocus" | "todayProgress" | "tasks" | "quickActions" | "projectProgress" | "weeklyGoals" | "noteEntry";
export type ShellFallbackReason = "UNSUPPORTED_PLATFORM" | "HOST_NOT_FOUND" | "ATTACHMENT_FAILED" | "DETACHMENT_FAILED";

export type WidgetModeFallbackEvent = {
  fromMode: "desktop";
  toMode: "floating";
  reason: ShellFallbackReason;
};

export type WidgetConfigInput = {
  size: WidgetSize;
  mode: WidgetMode;
  locked: boolean;
  opacity: number;
  modules: WidgetModule[];
  x: number;
  y: number;
  width: number;
  height: number;
  monitorId: string | null;
  scaleFactor: number;
};

export type WidgetConfig = WidgetConfigInput & {
  lastVisibleAt: string | null;
  updatedAt: string;
};

export const defaultWidgetConfig: WidgetConfig = {
  size: "standard",
  mode: "desktop",
  locked: false,
  opacity: 1,
  modules: ["clock", "currentFocus", "todayProgress", "tasks", "quickActions"],
  x: 40,
  y: 40,
  width: 360,
  height: 420,
  monitorId: null,
  scaleFactor: 1,
  lastVisibleAt: null,
  updatedAt: new Date(0).toISOString(),
};

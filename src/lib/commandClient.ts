import { invoke } from "@tauri-apps/api/core";

export type DomainVersion = number;

export interface DomainError {
  code: string;
  message: string;
  field?: string;
}

export type CommandResult<T> =
  | { ok: true; data: T; version: DomainVersion }
  | { ok: false; error: DomainError };

export async function invokeCommand<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<CommandResult<T>> {
  try {
    return await invoke<CommandResult<T>>(command, args);
  } catch (error) {
    try {
      await invoke("diagnostic_command_failure", {
        command,
        error: invocationErrorMessage(error),
      });
    } catch {
      // The original stable error remains usable when the IPC channel itself is unavailable.
    }
    return {
      ok: false,
      error: {
        code: "COMMAND_INVOCATION_FAILED",
        message: `command invocation failed: ${command}`,
        field: command,
      },
    };
  }
}

function invocationErrorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  return "unknown invocation error";
}

export function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

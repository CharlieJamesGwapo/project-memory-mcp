import { invoke, isTauri } from "@tauri-apps/api/core";
import type { ConnectionInfo, MemoryAdapter, SetupReport } from "./types";
import { createSampleAdapter } from "./sample";
type Invoke = (
  command: string,
  args?: Record<string, unknown>,
) => Promise<unknown>;
export function createNativeAdapter(call: Invoke = invoke): MemoryAdapter {
  return {
    sample: false,
    bootstrap: () => call("bootstrap") as Promise<ConnectionInfo>,
    chooseDatabase: () =>
      call("choose_database") as Promise<ConnectionInfo | null>,
    setupPreview: (args) =>
      call("preview_setup", { ...args }) as Promise<SetupReport>,
    query: <T>(
      session: string,
      operation: string,
      args: Record<string, unknown>,
    ) =>
      call("memory_query", {
        session_id: session,
        operation,
        args,
      }) as Promise<T>,
    copy: (markdown) => call("copy_context", { markdown }) as Promise<void>,
    save: (markdown) =>
      call("export_context", { markdown }) as Promise<{
        saved: boolean;
        file_name: string | null;
      }>,
  };
}
export function defaultAdapter(): MemoryAdapter {
  return isTauri() ? createNativeAdapter() : createSampleAdapter();
}
export function isStaleSession(error: unknown): boolean {
  return (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    error.code === "STALE_SESSION"
  );
}

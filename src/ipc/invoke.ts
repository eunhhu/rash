import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { mockInvoke } from "./mock";

export class IpcError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "IpcError";
  }
}

function hasTauri(): boolean {
  return typeof window !== "undefined" && !!(window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
}

export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    if (hasTauri()) {
      return await tauriInvoke<T>(cmd, args);
    }
    return await mockInvoke<T>(cmd, args);
  } catch (err) {
    throw new IpcError(typeof err === "string" ? err : String(err));
  }
}

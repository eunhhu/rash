import { invoke as tauriInvoke } from "@tauri-apps/api/core";

export class IpcError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "IpcError";
  }
}

export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await tauriInvoke<T>(cmd, args);
  } catch (err) {
    throw new IpcError(typeof err === "string" ? err : String(err));
  }
}

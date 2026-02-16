import { createSignal } from "solid-js";
import type { Language, Framework } from "../ipc/types";
import { onEvent } from "../ipc/events";
import * as cmd from "../ipc/commands";
import type { ServerStatus, LogEntry, PreflightReport, HmuResultPayload } from "../ipc/commands";

function createRuntimeStore() {
  const [serverStatus, setServerStatus] = createSignal<ServerStatus>("stopped");
  const [logs, setLogs] = createSignal<LogEntry[]>([]);
  const [port, setPort] = createSignal<number | null>(null);
  const [preflight, setPreflight] = createSignal<PreflightReport | null>(null);
  const [building, setBuilding] = createSignal(false);

  // Event subscriptions
  let unlistenLog: (() => void) | undefined;
  let unlistenStatus: (() => void) | undefined;
  let unlistenHmu: (() => void) | undefined;

  function subscribe() {
    onEvent<LogEntry>("server:log", (entry) => {
      setLogs((prev) => [...prev.slice(-499), entry]);
    }).then((fn) => { unlistenLog = fn; });

    onEvent<ServerStatus>("server:status", (status) => {
      setServerStatus(status);
    }).then((fn) => { unlistenStatus = fn; });

    onEvent<HmuResultPayload>("hmu:result", (_payload) => {
      // HMU result tracking — future UI can display this
    }).then((fn) => { unlistenHmu = fn; });
  }

  function unsubscribe() {
    unlistenLog?.();
    unlistenStatus?.();
    unlistenHmu?.();
  }

  // Auto-subscribe on creation
  subscribe();

  async function build(): Promise<void> {
    setBuilding(true);
    try {
      // 1. Preflight check
      const report = await cmd.runPreflight();
      setPreflight(report);
      if (!report.ok) {
        setBuilding(false);
        return;
      }

      // 2. Generate code
      const projectTree = await cmd.getProjectTree();
      const config = projectTree.config as Record<string, unknown>;
      const target = config?.target as Record<string, string> | undefined;
      if (target) {
        await cmd.generateProject(
          "./dist",
          target.language as Language,
          target.framework as Framework,
        );
      }

      // 3. Start server
      const p = await cmd.startServer();
      setPort(p);
      setServerStatus("running");
    } catch (err) {
      console.error("Build failed:", err);
      setServerStatus("errored");
    } finally {
      setBuilding(false);
    }
  }

  async function stop(): Promise<void> {
    try {
      await cmd.stopServer();
      setServerStatus("stopped");
      setPort(null);
    } catch (err) {
      console.error("Stop failed:", err);
    }
  }

  function clearLogs(): void {
    setLogs([]);
  }

  return {
    serverStatus,
    logs,
    port,
    preflight,
    building,
    build,
    stop,
    clearLogs,
    unsubscribe,
  };
}

// Singleton instance
let store: ReturnType<typeof createRuntimeStore> | undefined;

export function useRuntimeStore() {
  if (!store) {
    store = createRuntimeStore();
  }
  return store;
}

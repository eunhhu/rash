import { createSignal } from "solid-js";
import { createStore, produce } from "solid-js/store";

export interface EditorTab {
  id: string;
  label: string;
  kind: string;
  filePath: string;
  dirty: boolean;
}

function createEditorStore() {
  const [tabs, setTabs] = createStore<EditorTab[]>([]);
  const [activeTabId, setActiveTabId] = createSignal<string | null>(null);

  function openTab(id: string, label: string, kind: string, filePath: string): void {
    const existing = tabs.find((t) => t.id === id);
    if (existing) {
      setActiveTabId(id);
      return;
    }
    setTabs(produce((draft) => {
      draft.push({ id, label, kind, filePath, dirty: false });
    }));
    setActiveTabId(id);
  }

  function closeTab(id: string): void {
    const idx = tabs.findIndex((t) => t.id === id);
    if (idx === -1) return;

    setTabs(produce((draft) => {
      draft.splice(idx, 1);
    }));

    // If the closed tab was active, activate an adjacent tab
    if (activeTabId() === id) {
      if (tabs.length > 1) {
        const nextIdx = idx >= tabs.length - 1 ? idx - 1 : idx;
        const nextTab = tabs[nextIdx === idx ? nextIdx + 1 : nextIdx];
        setActiveTabId(nextTab ? nextTab.id : null);
      } else {
        setActiveTabId(null);
      }
    }
  }

  function setActiveTab(id: string): void {
    if (tabs.find((t) => t.id === id)) {
      setActiveTabId(id);
    }
  }

  function markDirty(id: string): void {
    const idx = tabs.findIndex((t) => t.id === id);
    if (idx !== -1) {
      setTabs(idx, "dirty", true);
    }
  }

  function markClean(id: string): void {
    const idx = tabs.findIndex((t) => t.id === id);
    if (idx !== -1) {
      setTabs(idx, "dirty", false);
    }
  }

  return {
    tabs,
    activeTabId,
    openTab,
    closeTab,
    setActiveTab,
    markDirty,
    markClean,
  };
}

// Singleton instance
let store: ReturnType<typeof createEditorStore> | undefined;

export function useEditorStore() {
  if (!store) {
    store = createEditorStore();
  }
  return store;
}

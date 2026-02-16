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

    // Calculate next tab BEFORE splice modifies the array
    let nextId: string | null = null;
    if (activeTabId() === id) {
      const next = tabs[idx + 1] ?? tabs[idx - 1];
      nextId = next?.id ?? null;
    }

    setTabs(produce((draft) => {
      draft.splice(idx, 1);
    }));

    if (activeTabId() === id) {
      setActiveTabId(nextId);
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

  function updateTabPath(oldId: string, newId: string, newLabel: string): void {
    const idx = tabs.findIndex((t) => t.id === oldId);
    if (idx === -1) return;
    const wasActive = activeTabId() === oldId;
    setTabs(produce((draft) => {
      draft[idx].id = newId;
      draft[idx].filePath = newId;
      draft[idx].label = newLabel;
    }));
    if (wasActive) {
      setActiveTabId(newId);
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
    updateTabPath,
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

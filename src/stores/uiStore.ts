import { createSignal } from "solid-js";

export type Theme = "dark" | "light";

function createUiStore() {
  const [sidebarWidth, setSidebarWidth] = createSignal(260);
  const [bottomPanelHeight, setBottomPanelHeight] = createSignal(200);
  const [bottomPanelOpen, setBottomPanelOpen] = createSignal(false);
  const [theme, setTheme] = createSignal<Theme>("dark");

  return {
    sidebarWidth,
    setSidebarWidth,
    bottomPanelHeight,
    setBottomPanelHeight,
    bottomPanelOpen,
    setBottomPanelOpen,
    theme,
    setTheme,
  };
}

// Singleton instance
let store: ReturnType<typeof createUiStore> | undefined;

export function useUiStore() {
  if (!store) {
    store = createUiStore();
  }
  return store;
}

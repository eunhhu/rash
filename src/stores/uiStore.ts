import { createSignal } from "solid-js";

export type Theme = "dark" | "light";

export interface ContextMenuItem {
  label: string;
  icon?: string;
  shortcut?: string;
  danger?: boolean;
  action: () => void;
  separator?: boolean;
}

export interface ContextMenuState {
  x: number;
  y: number;
  items: ContextMenuItem[];
}

export interface ConfirmDialogState {
  title: string;
  message: string;
  confirmLabel?: string;
  danger?: boolean;
  onConfirm: () => void;
}

export interface CreateDialogState {
  kind: string;
  onSubmit: (name: string) => void;
}

export interface ClipboardState {
  ids: string[];
  paths: string[];
  kind: string;
  op: "cut" | "copy";
}

export interface DragState {
  sourceId: string;
  sourcePath: string;
  sourceKind: string;
  overId: string | null;
  position: "before" | "after" | null;
}

function createUiStore() {
  const [sidebarWidth, setSidebarWidth] = createSignal(260);
  const [bottomPanelHeight, setBottomPanelHeight] = createSignal(200);
  const [bottomPanelOpen, setBottomPanelOpen] = createSignal(false);
  const [theme, setTheme] = createSignal<Theme>("dark");
  const [activeSidebarTab, setActiveSidebarTab] = createSignal<string>("route");
  const [contextMenu, setContextMenu] = createSignal<ContextMenuState | null>(null);
  const [confirmDialog, setConfirmDialog] = createSignal<ConfirmDialogState | null>(null);
  const [createDialog, setCreateDialog] = createSignal<CreateDialogState | null>(null);
  const [searchQuery, setSearchQuery] = createSignal("");

  // Explorer: multi-select
  const [selectedNodeIds, setSelectedNodeIds] = createSignal<Set<string>>(new Set());
  // Explorer: keyboard focus
  const [focusedNodeId, setFocusedNodeId] = createSignal<string | null>(null);
  // Explorer: inline editing
  const [editingNodeId, setEditingNodeId] = createSignal<string | null>(null);
  const [editingNodeValue, setEditingNodeValue] = createSignal("");
  const [editingMode, setEditingMode] = createSignal<"rename" | "create" | null>(null);
  // Explorer: inline creation
  const [creatingInSection, setCreatingInSection] = createSignal<string | null>(null);
  // Explorer: clipboard
  const [clipboard, setClipboard] = createSignal<ClipboardState | null>(null);
  // Explorer: drag & drop
  const [dragState, setDragState] = createSignal<DragState | null>(null);

  function clearEditing() {
    setEditingNodeId(null);
    setEditingNodeValue("");
    setEditingMode(null);
    setCreatingInSection(null);
  }

  function startRename(nodeId: string, currentLabel: string) {
    setEditingNodeId(nodeId);
    setEditingNodeValue(currentLabel);
    setEditingMode("rename");
  }

  function startCreate(sectionKind: string) {
    setCreatingInSection(sectionKind);
    setEditingMode("create");
    setEditingNodeValue("");
  }

  return {
    sidebarWidth,
    setSidebarWidth,
    bottomPanelHeight,
    setBottomPanelHeight,
    bottomPanelOpen,
    setBottomPanelOpen,
    theme,
    setTheme,
    contextMenu,
    setContextMenu,
    confirmDialog,
    setConfirmDialog,
    createDialog,
    setCreateDialog,
    searchQuery,
    setSearchQuery,
    // Explorer state
    selectedNodeIds,
    setSelectedNodeIds,
    focusedNodeId,
    setFocusedNodeId,
    editingNodeId,
    setEditingNodeId,
    editingNodeValue,
    setEditingNodeValue,
    editingMode,
    setEditingMode,
    creatingInSection,
    setCreatingInSection,
    clipboard,
    setClipboard,
    dragState,
    setDragState,
    // Sidebar tab
    activeSidebarTab,
    setActiveSidebarTab,
    // Convenience
    clearEditing,
    startRename,
    startCreate,
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

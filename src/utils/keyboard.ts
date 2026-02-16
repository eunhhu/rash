interface KeyboardCallbacks {
  newSpec?: () => void;
  save?: () => void;
  search?: () => void;
  closeTab?: () => void;
}

export function setupKeyboardShortcuts(callbacks: KeyboardCallbacks): () => void {
  const handler = (e: KeyboardEvent) => {
    const mod = e.metaKey || e.ctrlKey;
    if (!mod) return;

    switch (e.key.toLowerCase()) {
      case "n":
        e.preventDefault();
        callbacks.newSpec?.();
        break;
      case "s":
        e.preventDefault();
        callbacks.save?.();
        break;
      case "p":
        e.preventDefault();
        callbacks.search?.();
        break;
      case "w":
        e.preventDefault();
        callbacks.closeTab?.();
        break;
    }
  };

  window.addEventListener("keydown", handler);
  return () => window.removeEventListener("keydown", handler);
}

// ── Tree keyboard handler for VSCode-style explorer ──

export interface TreeNode {
  id: string;
  kind: string;
  path?: string;
  children: TreeNode[];
}

export interface TreeKeyboardOpts {
  getFocusedId: () => string | null;
  getVisibleNodes: () => TreeNode[];
  getCollapsed: () => Set<string>;
  onNavigate: (id: string) => void;
  onOpen: (id: string) => void;
  onDelete: (ids: string[]) => void;
  onRename: (id: string) => void;
  onToggleCollapse: (sectionId: string) => void;
  onCut: () => void;
  onCopy: () => void;
  onPaste: () => void;
  getSelectedIds: () => Set<string>;
}

/** Flatten a tree of nodes into a depth-first ordered list of leaf/section ids */
function flattenVisible(nodes: TreeNode[], collapsed: Set<string>): string[] {
  const result: string[] = [];
  for (const node of nodes) {
    result.push(node.id);
    if (node.children.length > 0 && !collapsed.has(node.id)) {
      result.push(...flattenVisible(node.children, collapsed));
    }
  }
  return result;
}

export function setupTreeKeyboard(opts: TreeKeyboardOpts): (e: KeyboardEvent) => void {
  return (e: KeyboardEvent) => {
    const mod = e.metaKey || e.ctrlKey;
    const focusedId = opts.getFocusedId();

    // Cmd+X / Cmd+C / Cmd+V
    if (mod) {
      switch (e.key.toLowerCase()) {
        case "x":
          e.preventDefault();
          opts.onCut();
          return;
        case "c":
          e.preventDefault();
          opts.onCopy();
          return;
        case "v":
          e.preventDefault();
          opts.onPaste();
          return;
      }
    }

    // Non-modifier keys
    const flat = flattenVisible(opts.getVisibleNodes(), opts.getCollapsed());

    switch (e.key) {
      case "ArrowUp": {
        e.preventDefault();
        if (!focusedId) {
          if (flat.length > 0) opts.onNavigate(flat[0]);
          return;
        }
        const idx = flat.indexOf(focusedId);
        if (idx > 0) opts.onNavigate(flat[idx - 1]);
        return;
      }
      case "ArrowDown": {
        e.preventDefault();
        if (!focusedId) {
          if (flat.length > 0) opts.onNavigate(flat[0]);
          return;
        }
        const idx = flat.indexOf(focusedId);
        if (idx < flat.length - 1) opts.onNavigate(flat[idx + 1]);
        return;
      }
      case "ArrowLeft": {
        e.preventDefault();
        if (focusedId) opts.onToggleCollapse(focusedId);
        return;
      }
      case "ArrowRight": {
        e.preventDefault();
        if (focusedId) opts.onToggleCollapse(focusedId);
        return;
      }
      case "Enter": {
        e.preventDefault();
        if (focusedId) opts.onOpen(focusedId);
        return;
      }
      case "Delete":
      case "Backspace": {
        if (mod || e.target instanceof HTMLInputElement) return;
        e.preventDefault();
        const selected = opts.getSelectedIds();
        if (selected.size > 0) {
          opts.onDelete([...selected]);
        } else if (focusedId) {
          opts.onDelete([focusedId]);
        }
        return;
      }
      case "F2": {
        e.preventDefault();
        if (focusedId) opts.onRename(focusedId);
        return;
      }
    }
  };
}

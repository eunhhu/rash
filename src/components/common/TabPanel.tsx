import { Component, For, JSX } from "solid-js";

export interface TabItem {
  id: string;
  label: string;
  dirty?: boolean;
  closable?: boolean;
}

interface TabPanelProps {
  tabs: TabItem[];
  activeId: string;
  onSelect: (id: string) => void;
  onClose?: (id: string) => void;
}

export const TabPanel: Component<TabPanelProps> = (props) => {
  const handleClose = (id: string, e: MouseEvent) => {
    e.stopPropagation();
    props.onClose?.(id);
  };

  return (
    <div class="tab-panel">
      <div class="tab-panel-bar">
        <For each={props.tabs}>
          {(tab) => (
            <div
              class="tab-panel-tab"
              classList={{ "tab-panel-tab-active": props.activeId === tab.id }}
              onClick={() => props.onSelect(tab.id)}
            >
              <span class="tab-panel-label">{tab.label}</span>
              {tab.dirty && <span class="tab-panel-dirty" />}
              {(tab.closable !== false) && props.onClose && (
                <button
                  class="tab-panel-close"
                  onClick={(e: MouseEvent) => handleClose(tab.id, e)}
                >
                  {"\u00D7"}
                </button>
              )}
            </div>
          )}
        </For>
      </div>

      <style>{`
        .tab-panel {
          flex-shrink: 0;
        }
        .tab-panel-bar {
          display: flex;
          height: var(--rash-tab-height);
          background: var(--rash-bg-secondary);
          border-bottom: 1px solid var(--rash-border);
          overflow-x: auto;
          scrollbar-width: none;
        }
        .tab-panel-bar::-webkit-scrollbar {
          display: none;
        }
        .tab-panel-tab {
          display: flex;
          align-items: center;
          gap: 6px;
          padding: 0 14px;
          cursor: pointer;
          white-space: nowrap;
          font-size: 12px;
          color: var(--rash-text-muted);
          border-right: 1px solid var(--rash-border);
          transition: color 0.1s ease, background 0.1s ease;
        }
        .tab-panel-tab:hover {
          color: var(--rash-text);
          background: var(--rash-surface);
        }
        .tab-panel-tab-active {
          color: var(--rash-text) !important;
          background: var(--rash-bg) !important;
          border-bottom: 2px solid var(--rash-accent);
        }
        .tab-panel-label {
          overflow: hidden;
          text-overflow: ellipsis;
        }
        .tab-panel-dirty {
          width: 6px;
          height: 6px;
          border-radius: 50%;
          background: var(--rash-accent);
          flex-shrink: 0;
        }
        .tab-panel-close {
          display: flex;
          align-items: center;
          justify-content: center;
          width: 16px;
          height: 16px;
          border: none;
          background: transparent;
          color: var(--rash-text-muted);
          font-size: 14px;
          cursor: pointer;
          border-radius: 2px;
          padding: 0;
          line-height: 1;
        }
        .tab-panel-close:hover {
          background: var(--rash-surface-hover);
          color: var(--rash-text);
        }
      `}</style>
    </div>
  );
};

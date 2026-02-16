import { Component, For, Show } from "solid-js";
import type { ErrorEntry, Severity } from "../../ipc/types";

interface ErrorListProps {
  errors: ErrorEntry[];
  onClickFile?: (file: string, path: string) => void;
}

const SEVERITY_ICONS: Record<Severity, string> = {
  error: "\u2716",
  warning: "\u26A0",
  info: "\u2139",
};

const SEVERITY_COLORS: Record<Severity, string> = {
  error: "var(--rash-error)",
  warning: "var(--rash-warning)",
  info: "var(--rash-info)",
};

export const ErrorList: Component<ErrorListProps> = (props) => {
  return (
    <div class="error-list">
      <Show when={props.errors.length > 0} fallback={
        <div class="error-list-empty">No issues found</div>
      }>
        <table class="error-list-table">
          <thead>
            <tr>
              <th class="error-list-th-sev" />
              <th>Code</th>
              <th>Message</th>
              <th>File</th>
              <th>Path</th>
            </tr>
          </thead>
          <tbody>
            <For each={props.errors}>
              {(entry) => (
                <tr class="error-list-row">
                  <td>
                    <span
                      class="error-list-severity"
                      style={{ color: SEVERITY_COLORS[entry.severity] }}
                      title={entry.severity}
                    >
                      {SEVERITY_ICONS[entry.severity]}
                    </span>
                  </td>
                  <td>
                    <span class="error-list-code">{entry.code}</span>
                  </td>
                  <td>
                    <span class="error-list-message">{entry.message}</span>
                    <Show when={entry.suggestion}>
                      <span class="error-list-suggestion">{entry.suggestion}</span>
                    </Show>
                  </td>
                  <td>
                    <span
                      class="error-list-file"
                      classList={{ "error-list-file-link": !!props.onClickFile }}
                      onClick={() => props.onClickFile?.(entry.file, entry.path)}
                    >
                      {entry.file}
                    </span>
                  </td>
                  <td>
                    <span class="error-list-path">{entry.path}</span>
                  </td>
                </tr>
              )}
            </For>
          </tbody>
        </table>
      </Show>

      <style>{`
        .error-list {
          overflow: auto;
          height: 100%;
        }
        .error-list-empty {
          display: flex;
          align-items: center;
          justify-content: center;
          height: 100%;
          color: var(--rash-text-muted);
          font-size: 13px;
        }
        .error-list-table {
          width: 100%;
          border-collapse: collapse;
          font-size: 12px;
        }
        .error-list-table th {
          text-align: left;
          padding: 6px 10px;
          color: var(--rash-text-muted);
          font-weight: 500;
          font-size: 11px;
          border-bottom: 1px solid var(--rash-border);
          position: sticky;
          top: 0;
          background: var(--rash-bg-secondary);
          z-index: 1;
        }
        .error-list-th-sev {
          width: 28px;
        }
        .error-list-row td {
          padding: 4px 10px;
          border-bottom: 1px solid var(--rash-border);
          vertical-align: top;
        }
        .error-list-row:hover td {
          background: var(--rash-surface);
        }
        .error-list-severity {
          font-size: 13px;
        }
        .error-list-code {
          font-family: var(--rash-font);
          color: var(--rash-text-secondary);
          white-space: nowrap;
        }
        .error-list-message {
          color: var(--rash-text);
        }
        .error-list-suggestion {
          display: block;
          font-size: 11px;
          color: var(--rash-text-muted);
          margin-top: 2px;
        }
        .error-list-file {
          font-family: var(--rash-font);
          color: var(--rash-text-secondary);
          white-space: nowrap;
        }
        .error-list-file-link {
          cursor: pointer;
          color: var(--rash-accent);
          text-decoration: underline;
          text-decoration-style: dotted;
        }
        .error-list-file-link:hover {
          color: var(--rash-accent-hover);
        }
        .error-list-path {
          font-family: var(--rash-font);
          color: var(--rash-text-muted);
          white-space: nowrap;
        }
      `}</style>
    </div>
  );
};

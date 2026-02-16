import { Component, createEffect, onCleanup, onMount } from "solid-js";
import { EditorView, keymap } from "@codemirror/view";
import { EditorState } from "@codemirror/state";
import { oneDark } from "@codemirror/theme-one-dark";
import { javascript } from "@codemirror/lang-javascript";
import { python } from "@codemirror/lang-python";
import { rust } from "@codemirror/lang-rust";

interface CodeViewerProps {
  code: string;
  language?: "typescript" | "python" | "rust" | "go";
  editable?: boolean;
  onChange?: (code: string) => void;
}

function getLanguageExtension(lang?: string) {
  switch (lang) {
    case "typescript":
      return javascript({ typescript: true });
    case "python":
      return python();
    case "rust":
      return rust();
    case "go":
      // No official go extension in codemirror; fall back to plain
      return [];
    default:
      return javascript({ typescript: true });
  }
}

export const CodeViewer: Component<CodeViewerProps> = (props) => {
  let containerRef!: HTMLDivElement;
  let view: EditorView | undefined;

  onMount(() => {
    const editable = props.editable ?? false;
    const extensions = [
      oneDark,
      getLanguageExtension(props.language),
      EditorView.lineWrapping,
      EditorState.readOnly.of(!editable),
      EditorView.theme({
        "&": {
          background: "var(--rash-bg-secondary)",
          fontSize: "13px",
          fontFamily: "var(--rash-font)",
        },
        ".cm-gutters": {
          background: "var(--rash-bg-tertiary)",
          border: "none",
        },
        ".cm-content": {
          padding: "8px 0",
        },
      }),
    ];

    if (editable && props.onChange) {
      extensions.push(
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            props.onChange!(update.state.doc.toString());
          }
        }),
      );
    }

    view = new EditorView({
      state: EditorState.create({
        doc: props.code,
        extensions,
      }),
      parent: containerRef,
    });
  });

  createEffect(() => {
    const newCode = props.code;
    if (view && view.state.doc.toString() !== newCode) {
      view.dispatch({
        changes: {
          from: 0,
          to: view.state.doc.length,
          insert: newCode,
        },
      });
    }
  });

  onCleanup(() => {
    view?.destroy();
  });

  return (
    <div
      ref={containerRef}
      class="code-viewer"
    >
      <style>{`
        .code-viewer {
          flex: 1;
          overflow: hidden;
          border: 1px solid var(--rash-border);
          border-radius: var(--rash-radius-sm);
        }
        .code-viewer .cm-editor {
          height: 100%;
        }
        .code-viewer .cm-scroller {
          overflow: auto;
        }
      `}</style>
    </div>
  );
};

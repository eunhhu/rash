import { Component, JSX, Show, onCleanup, onMount } from "solid-js";
import "../layout/layout.css";

interface ModalProps {
  open: boolean;
  onClose: () => void;
  title: string;
  children: JSX.Element;
}

export const Modal: Component<ModalProps> = (props) => {
  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      props.onClose();
    }
  };

  const handleOverlayClick = (e: MouseEvent) => {
    if (e.target === e.currentTarget) {
      props.onClose();
    }
  };

  onMount(() => {
    document.addEventListener("keydown", handleKeyDown);
  });

  onCleanup(() => {
    document.removeEventListener("keydown", handleKeyDown);
  });

  return (
    <Show when={props.open}>
      <div class="modal-overlay" onClick={handleOverlayClick}>
        <div class="modal-content" onClick={(e) => e.stopPropagation()}>
          <div class="modal-header">
            <h2>{props.title}</h2>
            <button class="btn-icon" onClick={props.onClose} title="Close">
              &#x2715;
            </button>
          </div>
          <div class="modal-body">{props.children}</div>
        </div>
      </div>
    </Show>
  );
};

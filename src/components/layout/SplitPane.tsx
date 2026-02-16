import { Component, JSX, createSignal, onCleanup, children as resolveChildren } from "solid-js";
import "./layout.css";

interface SplitPaneProps {
  direction: "horizontal" | "vertical";
  initialSize: number;
  minSize?: number;
  children: JSX.Element;
}

export const SplitPane: Component<SplitPaneProps> = (props) => {
  const minSize = () => props.minSize ?? 100;
  const [size, setSize] = createSignal(props.initialSize);
  const [dragging, setDragging] = createSignal(false);

  let containerRef: HTMLDivElement | undefined;

  const resolved = resolveChildren(() => props.children);
  const panels = () => {
    const c = resolved();
    return Array.isArray(c) ? c : [c];
  };

  const onPointerDown = (e: PointerEvent) => {
    e.preventDefault();
    setDragging(true);

    const onPointerMove = (ev: PointerEvent) => {
      if (!containerRef) return;
      const rect = containerRef.getBoundingClientRect();

      let newSize: number;
      if (props.direction === "horizontal") {
        newSize = ev.clientX - rect.left;
      } else {
        newSize = ev.clientY - rect.top;
      }

      const maxSize =
        props.direction === "horizontal"
          ? rect.width - minSize()
          : rect.height - minSize();

      newSize = Math.max(minSize(), Math.min(newSize, maxSize));
      setSize(newSize);
    };

    const onPointerUp = () => {
      setDragging(false);
      document.removeEventListener("pointermove", onPointerMove);
      document.removeEventListener("pointerup", onPointerUp);
    };

    document.addEventListener("pointermove", onPointerMove);
    document.addEventListener("pointerup", onPointerUp);
  };

  onCleanup(() => {
    // Defensive cleanup (listeners are already removed on pointerup)
  });

  const firstPanelStyle = (): JSX.CSSProperties => {
    if (props.direction === "horizontal") {
      return { width: `${size()}px`, "min-width": `${minSize()}px` };
    }
    return { height: `${size()}px`, "min-height": `${minSize()}px` };
  };

  return (
    <div
      ref={containerRef}
      class={`split-pane ${props.direction}`}
      style={{ cursor: dragging() ? (props.direction === "horizontal" ? "col-resize" : "row-resize") : undefined }}
    >
      <div class="split-pane-panel" style={firstPanelStyle()}>
        {panels()[0]}
      </div>
      <div
        class={`split-divider ${dragging() ? "dragging" : ""}`}
        onPointerDown={onPointerDown}
      />
      <div class="split-pane-panel" style={{ flex: "1" }}>
        {panels()[1]}
      </div>
    </div>
  );
};

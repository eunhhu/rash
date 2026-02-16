import { Component, Show, createSignal, onCleanup } from "solid-js";
import { FiSearch, FiX } from "solid-icons/fi";

interface SearchInputProps {
  value?: string;
  onSearch: (query: string) => void;
  placeholder?: string;
  debounceMs?: number;
  ref?: (el: HTMLInputElement) => void;
}

export const SearchInput: Component<SearchInputProps> = (props) => {
  const [local, setLocal] = createSignal(props.value ?? "");
  let timer: ReturnType<typeof setTimeout> | undefined;

  const handleInput = (value: string) => {
    setLocal(value);
    clearTimeout(timer);
    timer = setTimeout(() => {
      props.onSearch(value);
    }, props.debounceMs ?? 200);
  };

  const handleClear = () => {
    setLocal("");
    clearTimeout(timer);
    props.onSearch("");
  };

  onCleanup(() => clearTimeout(timer));

  return (
    <div class="flex items-center gap-2 px-3 py-1.5 glass-surface">
      <FiSearch size={14} class="text-rash-text-muted flex-shrink-0" />
      <input
        ref={props.ref}
        type="text"
        value={local()}
        onInput={(e) => handleInput(e.currentTarget.value)}
        placeholder={props.placeholder ?? "Search specs..."}
        class="flex-1 bg-transparent border-none outline-none text-xs text-rash-text placeholder:text-rash-text-muted p-0"
      />
      <Show when={local().length > 0}>
        <button
          class="carbon-btn-icon p-0.5"
          onClick={handleClear}
        >
          <FiX size={12} />
        </button>
      </Show>
    </div>
  );
};

import { Component, For, JSX } from "solid-js";
import "../layout/layout.css";

interface SelectOption {
  value: string;
  label: string;
}

interface SelectProps {
  label?: string;
  value: string;
  onChange: (value: string) => void;
  options: SelectOption[];
  disabled?: boolean;
  id?: string;
}

export const Select: Component<SelectProps> = (props) => {
  const id = () => props.id ?? props.label?.toLowerCase().replace(/\s+/g, "-");

  const handleChange: JSX.EventHandler<HTMLSelectElement, Event> = (e) => {
    props.onChange(e.currentTarget.value);
  };

  return (
    <div class="form-field">
      {props.label && <label for={id()}>{props.label}</label>}
      <select
        id={id()}
        value={props.value}
        onChange={handleChange}
        disabled={props.disabled}
      >
        <For each={props.options}>
          {(opt) => (
            <option value={opt.value} selected={opt.value === props.value}>
              {opt.label}
            </option>
          )}
        </For>
      </select>
    </div>
  );
};

import { Component, JSX } from "solid-js";
import "../layout/layout.css";

interface InputProps {
  label?: string;
  value: string;
  onInput: (value: string) => void;
  type?: string;
  placeholder?: string;
  disabled?: boolean;
  id?: string;
}

export const Input: Component<InputProps> = (props) => {
  const id = () => props.id ?? props.label?.toLowerCase().replace(/\s+/g, "-");

  const handleInput: JSX.EventHandler<HTMLInputElement, InputEvent> = (e) => {
    props.onInput(e.currentTarget.value);
  };

  return (
    <div class="form-field">
      {props.label && <label for={id()}>{props.label}</label>}
      <input
        id={id()}
        type={props.type ?? "text"}
        value={props.value}
        onInput={handleInput}
        placeholder={props.placeholder}
        disabled={props.disabled}
      />
    </div>
  );
};

import { Component, JSX } from "solid-js";

interface ButtonProps {
  variant?: "primary" | "secondary" | "icon";
  size?: "sm" | "md";
  onClick?: (e: MouseEvent) => void;
  disabled?: boolean;
  title?: string;
  children: JSX.Element;
}

export const Button: Component<ButtonProps> = (props) => {
  const classes = () => {
    const parts = ["btn"];
    if (props.variant) {
      parts.push(`btn-${props.variant}`);
    }
    if (props.size === "sm") {
      parts.push("btn-sm");
    }
    return parts.join(" ");
  };

  return (
    <button
      class={classes()}
      onClick={props.onClick}
      disabled={props.disabled}
      title={props.title}
    >
      {props.children}
    </button>
  );
};

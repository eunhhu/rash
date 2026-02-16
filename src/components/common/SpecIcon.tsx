import { Component, Show } from "solid-js";
import { TbOutlineRoute2, TbOutlineBraces, TbOutlineDatabase, TbOutlineFunction } from "solid-icons/tb";
import { FiLayers } from "solid-icons/fi";

type IconComp = Component<{ size?: number; class?: string }>;

const ICON_MAP: Record<string, IconComp> = {
  route: TbOutlineRoute2,
  schema: TbOutlineBraces,
  model: TbOutlineDatabase,
  middleware: FiLayers,
  handler: TbOutlineFunction,
};

interface SpecIconProps {
  kind: string;
  class?: string;
  size?: number;
}

export const SpecIcon: Component<SpecIconProps> = (props) => {
  const Icon = () => ICON_MAP[props.kind];

  return (
    <Show when={Icon()} fallback={<span class={props.class}>#</span>}>
      {(IconComp) => {
        const I = IconComp();
        return <I size={props.size ?? 16} class={props.class} />;
      }}
    </Show>
  );
};

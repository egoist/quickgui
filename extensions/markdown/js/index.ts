import type { NativeNode, ColorValue } from "@quickgui/native";
import { ExtensionComponent, flattenStyle, mergeProps, type NativeProps, type NativeStyle, type ExtensionComponentProps } from "@quickgui/solid";
import { omit } from "solid-js";

export interface CodeBlockComponent {
  package: string;
  name: string;
  props?: Record<string, unknown>;
}
export interface MarkdownStyle extends NativeStyle {
  linkColor?: ColorValue;
  mutedColor?: ColorValue;
  codeColor?: ColorValue;
  codeBackground?: ColorValue;
  codeBorderColor?: ColorValue;
  blockGap?: number;
  codeFontSize?: number;
}
export type MarkdownStyleProp = MarkdownStyle | false | null | undefined | readonly MarkdownStyleProp[];
export interface MarkdownProps extends Omit<NativeProps, "children" | "style"> {
  content?: string;
  source?: string;
  streaming?: boolean;
  codeBlockComponent?: CodeBlockComponent;
  codeBlockMaxHeight?: number;
  style?: MarkdownStyleProp;
}

export function Markdown(props: MarkdownProps): NativeNode {
  const styles = () => flattenStyle(props.style) as MarkdownStyle;
  const keys = ["linkColor", "mutedColor", "codeColor", "codeBackground", "codeBorderColor", "blockGap", "codeFontSize"] as const;
  return ExtensionComponent(mergeProps(omit(props, "content", "source", "streaming", "codeBlockComponent", "codeBlockMaxHeight", "style"), {
    package: "markdown",
    component: "markdown",
    get style() { return omit(styles(), ...keys); },
    get properties() {
      return {
        value: props.content ?? props.source ?? "", streaming: props.streaming,
        codeBlockComponent: props.codeBlockComponent && { ...props.codeBlockComponent, props: props.codeBlockComponent.props ?? {} },
        codeBlockMaxHeight: props.codeBlockMaxHeight, style: styles(),
        theme: Object.fromEntries(keys.map(key => [key, styles()[key]])),
      };
    },
  }) as ExtensionComponentProps);
}

export default Markdown;

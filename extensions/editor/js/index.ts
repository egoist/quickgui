import { QuickGuiEvent, invokeExtension, type ColorValue, type NativeNode } from "@quickgui/native";
import { ExtensionComponent, mergeProps, type NativeProps, type ExtensionComponentProps } from "@quickgui/solid";
import { omit } from "solid-js";

/** Use this extension's CodeBlock to render parsed Markdown fences. */
export const HighlightedCodeBlock = { package: "editor", name: "code-block", props: {} } as const;

/** Load a portable Tree-sitter Wasm pack and return its canonical language names.
 * Await this before selecting the language. Registration is shared by editor, code blocks,
 * diffs and injected languages. path must be an absolute packaged resource path.
 * One file contains all selected grammars and queries; no native libraries are loaded.
 */
export async function loadLanguagePack(path: string): Promise<string[]> {
  if (typeof path !== "string" || path.length === 0 || path.includes("\0"))
    throw new TypeError("A language pack path is required");
  const names = await invokeExtension<unknown>("editor", "load-language-pack", { path });
  if (!Array.isArray(names) || !names.length || names.some(name => typeof name !== "string" || !name)) throw new Error("Invalid language registration reply");
  return names as string[];
}

export type EditorLanguage =
  | "text"
  | "rust"
  | "typescript"
  | "tsx"
  | "javascript"
  | "jsx"
  | "python"
  | "go"
  | "cpp"
  | "java"
  | "ruby"
  | "swift"
  | "json"
  | "yaml"
  | "toml"
  | "shell"
  | "css"
  | "html"
  | "sql"
  | "markdown"
  | (string & {});

export interface SyntaxTheme {
  keyword?: ColorValue;
  literal?: ColorValue;
  string?: ColorValue;
  comment?: ColorValue;
  number?: ColorValue;
  type?: ColorValue;
  function?: ColorValue;
  metadata?: ColorValue;
}

/** Document insets scroll with the code; they do not inset the viewport. */
export interface ContentPadding { left?: number; right?: number; vertical?: number }
export interface GutterPadding { left?: number; right?: number }
export interface DiffPresentation { headerHeight?: number; headerPadding?: number; gutterPadding?: number }

export interface CodeBlockProps extends Omit<NativeProps, "children"> {
  /** Controlled UTF-8 source displayed without an editor caret. */
  value: string;
  language?: EditorLanguage;
  lineNumbers?: boolean;
  wrap?: boolean;
  contentPadding?: ContentPadding;
  gutterPadding?: GutterPadding;
  gutterBackground?: ColorValue;
  gutterColor?: ColorValue;
  syntaxTheme?: SyntaxTheme;
}


export interface EditorProps extends Omit<NativeProps, "children"> {
  /** Controlled UTF-8 document. */
  value: string;
  language?: EditorLanguage;
  lineNumbers?: boolean;
  tabSize?: number;
  insertSpaces?: boolean;
  autoIndent?: boolean;
  readOnly?: boolean;
  contentPadding?: ContentPadding;
  gutterPadding?: GutterPadding;
  gutterBackground?: ColorValue;
  gutterColor?: ColorValue;
  activeLineBackground?: ColorValue;
  activeLineNumberColor?: ColorValue;
  syntaxTheme?: SyntaxTheme;
}


export type DiffLayout = "split" | "unified";
export type DiffIndicators = "bars" | "classic" | "none";

export interface DiffViewOptions {
  layout?: DiffLayout;
  indicators?: DiffIndicators;
  backgrounds?: boolean;
  lineNumbers?: boolean;
  wrap?: boolean;
  fileHeader?: boolean;
  language?: EditorLanguage;
}

export interface DiffTheme {
  mutedColor?: ColorValue;
  addedBackground?: ColorValue;
  removedBackground?: ColorValue;
  addedGutterBackground?: ColorValue;
  removedGutterBackground?: ColorValue;
  addedColor?: ColorValue;
  removedColor?: ColorValue;
  lineNumberColor?: ColorValue;
  hunkBackground?: ColorValue;
  hunkColor?: ColorValue;
  headerBackground?: ColorValue;
  inlineAddedBackground?: ColorValue;
  inlineRemovedBackground?: ColorValue;
}

export type DiffSource =
  | {
      /** Captured Git or ordinary unified patch. */
      patch: string;
      oldText?: never;
      newText?: never;
      oldPath?: never;
      newPath?: never;
    }
  | {
      patch?: never;
      oldText: string;
      newText: string;
      oldPath?: string;
      newPath?: string;
    };

export type DiffViewProps = Omit<NativeProps, "children"> &
  DiffSource & {
    options?: DiffViewOptions;
    presentation?: DiffPresentation;
    theme?: DiffTheme;
    syntaxTheme?: SyntaxTheme;
  };


function surface(name: string, props: NativeProps, keys: readonly string[]): NativeNode {
  const values = props as unknown as Record<string, unknown>;
  return ExtensionComponent(mergeProps(omit(props, ...keys as (keyof NativeProps)[], "onInput"), {
    package: "editor",
    component: name,
    get properties() {
      const properties: Record<string, unknown> = { style: props.style };
      for (const key of keys) properties[key] = values[key];
      return properties;
    },
    onEvent(kind: string, value: unknown, event: QuickGuiEvent) {
      if (kind === "input" && typeof props.onInput === "function")
        props.onInput(new QuickGuiEvent("input", event.target, String(value)));
    },
  }) as ExtensionComponentProps);
}

export function Editor(props: EditorProps): NativeNode {
  return surface("editor", props, ["value", "language", "lineNumbers", "tabSize", "insertSpaces", "autoIndent", "readOnly", "contentPadding", "gutterPadding", "gutterBackground", "gutterColor", "activeLineBackground", "activeLineNumberColor", "syntaxTheme"]);
}
export function CodeBlock(props: CodeBlockProps): NativeNode {
  return surface("code-block", props, ["value", "language", "lineNumbers", "wrap", "contentPadding", "gutterPadding", "gutterBackground", "gutterColor", "syntaxTheme"]);
}
export function DiffView(props: DiffViewProps): NativeNode {
  return surface("diff-view", props, ["patch", "oldText", "newText", "oldPath", "newPath", "options", "presentation", "theme", "syntaxTheme"]);
}

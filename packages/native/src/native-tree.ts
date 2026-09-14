import { NativeNodeTag, PropertyCode, type NativePropertyValue } from "./protocol.ts";

export type ColorValue = number | string;
export type NativeElementName =
  | "view"
  | "div"
  | "text"
  | "button"
  | "input"
  | "textarea"
  | "virtual-list"
  | "extension"
  | "svg"
  | "image"
  | "shader"
  | "swift-ui-host"
  | "swift-ui-button"
  | "swift-ui-quickgui-host"
  | "swift-ui-popover"
  | "swift-ui-popover-trigger"
  | "swift-ui-popover-content"
  | "swift-ui-slider"
  | "swift-ui-toggle"
  | "swift-ui-progress-view"
  | "swift-ui-stepper"
  | "swift-ui-text-field"
  | "swift-ui-picker"
  | "swift-ui-date-picker"
  | "swift-ui-color-picker"
  | "swift-ui-gauge";
export type NativeEventType =
  | "click"
  | "mouseenter"
  | "mouseleave"
  | "input"
  | "submit"
  | "dismiss"
  | "pointer"
  | "presentationchange"
  | "menuselect"
  | "keydown"
  | "keyup"
  | "mousedown"
  | "mouseup"
  | "mousemove"
  | "dblclick"
  | "wheel"
  | "contextmenu"
  | "pinch"
  | "rotate"
  | "smartmagnify"
  | "pressure"
  | "focus"
  | "blur"
  | "action"
  | "dragstart"
  | "dragend"
  | "drop"
  | "filesdropped"
  | "componentchange"
  | "commit";

/** Declared input listeners whose presence is one boolean property. */
const inputListenerProperties: ReadonlyMap<NativeEventType, PropertyCode> = new Map([
  ["keydown", PropertyCode.KeyDownListener],
  ["keyup", PropertyCode.KeyUpListener],
  ["mousedown", PropertyCode.MouseDownListener],
  ["mouseup", PropertyCode.MouseUpListener],
  ["mousemove", PropertyCode.MouseMoveListener],
  ["dblclick", PropertyCode.DoubleClickListener],
  ["wheel", PropertyCode.ScrollListener],
  ["contextmenu", PropertyCode.ContextMenuListener],
  ["pinch", PropertyCode.PinchListener],
  ["rotate", PropertyCode.RotationListener],
  ["smartmagnify", PropertyCode.SmartMagnifyListener],
  ["pressure", PropertyCode.PressureListener],
  ["action", PropertyCode.ActionListener],
  ["drop", PropertyCode.DropListener],
  ["filesdropped", PropertyCode.DropListener],
]);
export type NativeEventListener = (event: QuickGuiEvent) => void;

export interface NativeNodeHost {
  readonly nativeId: number;
  readonly app: { readonly nativeId: number };
  readonly nodes: Map<number, NativeNode>;
  readonly closed: boolean;
  flush(): number | undefined;
  _trackMount(dispose: () => void): () => void;
  _focusNode(node: NativeNode): boolean;
  _enqueueCreate(node: NativeNode): void;
  _enqueueProperty(
    node: NativeNode,
    property: PropertyCode,
    value: NativePropertyValue,
    color: boolean,
  ): void;
  _enqueueText(node: NativeNode): void;
  _enqueueInsert(parent: NativeNode, child: NativeNode, before?: NativeNode): void;
  _enqueueRemove(parent: NativeNode, child: NativeNode): void;
  _enqueueCleanup(parent: NativeNode, children: readonly NativeNode[]): void;
}

let nextNodeId = 1;

export class NativeNode {
  readonly id: number;
  readonly tag: NativeNodeTag;
  text: string;
  parent: NativeNode | undefined;
  readonly children: NativeNode[] = [];
  readonly properties = new Map<PropertyCode, NativePropertyValue>();
  readonly colorProperties = new Set<PropertyCode>();
  readonly listeners = new Map<NativeEventType, NativeEventListener>();
  host: NativeNodeHost | undefined;
  materialized = false;

  constructor(tag: NativeNodeTag, text = "", id = allocateNodeId()) {
    this.id = id;
    this.tag = tag;
    this.text = text;
  }

  /** Focus this mounted node, matching the web `HTMLElement.focus()` shape. */
  focus(): boolean {
    return this.host?._focusNode(this) ?? false;
  }
}

export class QuickGuiEvent {
  readonly type: NativeEventType;
  readonly target: NativeNode;
  currentTarget: NativeNode;
  defaultPrevented = false;
  propagationStopped = false;
  readonly value: string | undefined;

  constructor(type: NativeEventType, target: NativeNode, value?: string) {
    this.type = type;
    this.target = target;
    this.currentTarget = target;
    this.value = value;
  }

  preventDefault(): void {
    this.defaultPrevented = true;
  }

  stopPropagation(): void {
    this.propagationStopped = true;
  }
}

function nativeNodeTag(name: NativeElementName): NativeNodeTag {
  switch (name) {
    case "button":
      return NativeNodeTag.Button;
    case "input":
    case "textarea":
      return NativeNodeTag.Input;
    case "virtual-list":
      return NativeNodeTag.VirtualList;
    case "extension":
      return NativeNodeTag.Extension;
    case "svg":
      return NativeNodeTag.Svg;
    case "image":
      return NativeNodeTag.Image;
    case "shader":
      return NativeNodeTag.Shader;
    case "swift-ui-host":
      return NativeNodeTag.SwiftUIHost;
    case "swift-ui-button":
      return NativeNodeTag.SwiftUIButton;
    case "swift-ui-quickgui-host":
      return NativeNodeTag.SwiftUIQuickGUIHost;
    case "swift-ui-popover":
      return NativeNodeTag.SwiftUIPopover;
    case "swift-ui-popover-trigger":
      return NativeNodeTag.SwiftUIPopoverTrigger;
    case "swift-ui-popover-content":
      return NativeNodeTag.SwiftUIPopoverContent;
    case "swift-ui-slider":
      return NativeNodeTag.SwiftUISlider;
    case "swift-ui-toggle":
      return NativeNodeTag.SwiftUIToggle;
    case "swift-ui-progress-view":
      return NativeNodeTag.SwiftUIProgressView;
    case "swift-ui-stepper":
      return NativeNodeTag.SwiftUIStepper;
    case "swift-ui-text-field":
      return NativeNodeTag.SwiftUITextField;
    case "swift-ui-picker":
      return NativeNodeTag.SwiftUIPicker;
    case "swift-ui-date-picker":
      return NativeNodeTag.SwiftUIDatePicker;
    case "swift-ui-color-picker":
      return NativeNodeTag.SwiftUIColorPicker;
    case "swift-ui-gauge":
      return NativeNodeTag.SwiftUIGauge;
    case "view":
    case "div":
    case "text":
      return NativeNodeTag.View;
  }
}

export function createNativeElement(name: NativeElementName): NativeNode {
  const node = new NativeNode(nativeNodeTag(name));
  if (name === "textarea") setNativeProperty(node, PropertyCode.Multiline, true);
  return node;
}

export function createNativeText(value: string): NativeNode {
  return new NativeNode(NativeNodeTag.Text, value);
}

export function createNativeSentinel(): NativeNode {
  return new NativeNode(NativeNodeTag.Sentinel);
}

export function replaceNativeText(node: NativeNode, value: string): void {
  if (node.tag !== NativeNodeTag.Text) throw new TypeError("replaceText expects a text node");
  if (node.text === value) return;
  node.text = value;
  if (node.materialized) node.host?._enqueueText(node);
}

export function setNativeProperty(
  node: NativeNode,
  property: PropertyCode,
  value: NativePropertyValue,
  options: { color?: boolean } = {},
): void {
  const normalized = value ?? null;
  if (normalized === null) {
    if (!node.properties.delete(property)) return;
    node.colorProperties.delete(property);
  } else {
    const previous = node.properties.get(property);
    if (Object.is(previous, normalized) && node.colorProperties.has(property) === !!options.color) {
      return;
    }
    node.properties.set(property, normalized);
    if (options.color) node.colorProperties.add(property);
    else node.colorProperties.delete(property);
  }
  if (node.materialized) {
    node.host?._enqueueProperty(node, property, normalized, !!options.color);
  }
}

export function setNativeEventListener(
  node: NativeNode,
  type: NativeEventType,
  listener: NativeEventListener | undefined,
): void {
  if (listener) node.listeners.set(type, listener);
  else node.listeners.delete(type);
  if (type === "click") {
    setNativeProperty(node, PropertyCode.ClickListener, node.listeners.has("click"));
  } else if (type === "input") {
    setNativeProperty(node, PropertyCode.InputListener, node.listeners.has("input"));
  } else if (type === "submit") {
    setNativeProperty(node, PropertyCode.SubmitListener, node.listeners.has("submit"));
  } else if (type === "dismiss") {
    setNativeProperty(node, PropertyCode.DismissListener, node.listeners.has("dismiss"));

  } else if (type === "pointer") {
    setNativeProperty(node, PropertyCode.PointerListener, node.listeners.has("pointer"));
  } else if (inputListenerProperties.has(type)) {
    const code = inputListenerProperties.get(type)!;
    // `drop` and `filesdropped` share one declared listener property; the accepted payload kinds
    // are declared separately through `dropKinds`.
    const declared =
      code === PropertyCode.DropListener
        ? node.listeners.has("drop") || node.listeners.has("filesdropped")
        : node.listeners.has(type);
    setNativeProperty(node, code, declared);
  } else if (type === "focus" || type === "blur") {
    setNativeProperty(
      node,
      PropertyCode.FocusListener,
      node.listeners.has("focus") || node.listeners.has("blur"),
    );
  } else if (type === "dragstart" || type === "dragend") {
    setNativeProperty(
      node,
      PropertyCode.DragListener,
      node.listeners.has("dragstart") || node.listeners.has("dragend"),
    );
  } else if (type === "menuselect") {
    setNativeProperty(node, PropertyCode.SelectListener, node.listeners.has("menuselect"));
  } else if (type === "commit") {
    // Committing an option, a number, or a collection row is an edge, not a value: the same
    // commit can repeat while the controlled value never moves, so it travels on its own channel.
    setNativeProperty(node, PropertyCode.CommitListener, node.listeners.has("commit"));
  } else if (type === "componentchange") {
    // A declared range, ordering, or roving-focus component reports whatever the Rust core
    // decided as one asynchronous payload; the declaration only says whether anyone listens.
    setNativeProperty(
      node,
      PropertyCode.ComponentChangeListener,
      node.listeners.has("componentchange"),
    );
  } else if (type === "presentationchange") {
    // Presentation changes are emitted by the Rust lifecycle without a listener flag.
    return;
  } else {
    const listensForHover = node.listeners.has("mouseenter") || node.listeners.has("mouseleave");
    setNativeProperty(node, PropertyCode.HoverListener, listensForHover);
  }
}

export function insertNativeNode(parent: NativeNode, node: NativeNode, anchor?: NativeNode): void {
  if (anchor && anchor.parent !== parent) throw new Error("anchor is not a child of parent");
  if (node === parent) throw new Error("a native node cannot contain itself");
  if (anchor === node && node.parent === parent) return;

  if (node.parent) {
    const previousIndex = node.parent.children.indexOf(node);
    if (previousIndex >= 0) node.parent.children.splice(previousIndex, 1);
  }
  const index = anchor ? parent.children.indexOf(anchor) : parent.children.length;
  parent.children.splice(index, 0, node);
  node.parent = parent;

  if (parent.host) {
    materialize(node, parent.host);
    parent.host._enqueueInsert(parent, node, anchor);
  }
}

export function removeNativeNode(parent: NativeNode, node: NativeNode): void {
  if (node.parent !== parent) return;
  const index = parent.children.indexOf(node);
  if (index >= 0) parent.children.splice(index, 1);
  node.parent = undefined;
  if (node.materialized && parent.host) parent.host._enqueueRemove(parent, node);
  dematerialize(node);
}

export function cleanupNativeNodes(parent: NativeNode, nodes: readonly NativeNode[]): void {
  const attached = nodes.filter((node) => node.parent === parent);
  if (attached.length === 0) return;
  for (const node of attached) {
    const index = parent.children.indexOf(node);
    if (index >= 0) parent.children.splice(index, 1);
    node.parent = undefined;
  }
  if (parent.host) parent.host._enqueueCleanup(parent, attached);
  for (const node of attached) dematerialize(node);
}

export function getNativeParent(node: NativeNode): NativeNode | undefined {
  return node.parent;
}

export function getNativeFirstChild(node: NativeNode): NativeNode | undefined {
  return node.children[0];
}

export function getNativeNextSibling(node: NativeNode): NativeNode | undefined {
  if (!node.parent) return undefined;
  const index = node.parent.children.indexOf(node);
  return index < 0 ? undefined : node.parent.children[index + 1];
}

export function isNativeText(node: NativeNode): boolean {
  return node.tag === NativeNodeTag.Text;
}

export function parseColor(value: ColorValue): number {
  if (typeof value === "number") return value >>> 0;
  const color = value.trim().toLowerCase();
  if (color === "transparent") return 0;
  if (color === "black") return packColor(0, 0, 0, 255);
  if (color === "white") return packColor(255, 255, 255, 255);
  if (color.startsWith("#")) {
    const hex = color.slice(1);
    if (hex.length === 3 || hex.length === 4) {
      const [r = "0", g = "0", b = "0", a = "f"] = hex;
      return packColor(
        Number.parseInt(r + r, 16),
        Number.parseInt(g + g, 16),
        Number.parseInt(b + b, 16),
        Number.parseInt(a + a, 16),
      );
    }
    if (hex.length === 6 || hex.length === 8) {
      return packColor(
        Number.parseInt(hex.slice(0, 2), 16),
        Number.parseInt(hex.slice(2, 4), 16),
        Number.parseInt(hex.slice(4, 6), 16),
        hex.length === 8 ? Number.parseInt(hex.slice(6, 8), 16) : 255,
      );
    }
  }
  const rgb = color.match(/^rgba?\(([^)]+)\)$/);
  if (rgb) {
    const parts = rgb[1]?.split(",").map((part) => part.trim()) ?? [];
    if (parts.length === 3 || parts.length === 4) {
      return packColor(
        Number(parts[0]),
        Number(parts[1]),
        Number(parts[2]),
        parts[3] === undefined ? 255 : Math.round(Number(parts[3]) * 255),
      );
    }
  }
  throw new TypeError(`unsupported QuickGUI color \`${value}\``);
}

function packColor(r: number, g: number, b: number, a: number): number {
  const component = (value: number) => Math.max(0, Math.min(255, Math.round(value)));
  return (component(r) | (component(g) << 8) | (component(b) << 16) | (component(a) << 24)) >>> 0;
}

function allocateNodeId(): number {
  if (nextNodeId >= 0xffff_ffff) throw new Error("QuickGUI native node id space exhausted");
  return nextNodeId++;
}

function materialize(node: NativeNode, host: NativeNodeHost): void {
  if (node.materialized) {
    if (node.host !== host) throw new Error("a native node cannot move between QuickGUI windows");
    return;
  }
  node.host = host;
  node.materialized = true;
  host.nodes.set(node.id, node);
  host._enqueueCreate(node);
  for (const [property, value] of node.properties) {
    host._enqueueProperty(node, property, value, node.colorProperties.has(property));
  }
  for (const child of node.children) {
    materialize(child, host);
    host._enqueueInsert(node, child);
  }
}

function dematerialize(node: NativeNode): void {
  const host = node.host;
  if (host) host.nodes.delete(node.id);
  node.materialized = false;
  node.host = undefined;
  for (const child of node.children) dematerialize(child);
}

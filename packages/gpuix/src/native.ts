import {
  Window,
  NativeNode,
  createNativeElement,
  createNativeText,
  insertNativeNode,
  removeNativeNode,
  replaceNativeText,
  setNativeEventListener,
  setNativeProperty,
  PropertyCode,
  color,
  type NativeElementName,
  type NativeEventType,
  type QuickGuiEvent,
} from "@quickgui/native";
import { setProp } from "@quickgui/solid";

type Payload = { elementId: number; eventType: string; [key: string]: unknown };
type Callback = (error: Error | null, event: Payload) => void;
type Operation = [string, number, ...unknown[]];
type RecordNode = { node: NativeNode; type: string; props: Record<string, unknown> };
const tags: Record<string, NativeElementName> = {
  div: "div",
  anchored: "div",
  code: "div",
  diff: "div",
  text: "div",
  input: "input",
  textarea: "textarea",
  svg: "svg",
  img: "image",
  markdown: "markdown",
  "virtual-list": "virtual-list",
};
const styles = new Set(
  `display flexDirection flexWrap flexGrow flexShrink flexBasis alignItems alignSelf justifyContent alignContent gap columnGap rowGap width height minWidth minHeight maxWidth maxHeight padding paddingTop paddingRight paddingBottom paddingLeft margin marginTop marginRight marginBottom marginLeft color opacity borderWidth borderTopWidth borderRightWidth borderBottomWidth borderLeftWidth borderColor borderRadius boxShadow fontSize fontFamily fontWeight lineHeight textAlign whiteSpace lineClamp textOverflow overflow overflowX overflowY cursor position top right bottom left userSelect visibility aspectRatio`.split(
    " ",
  ),
);
const events: Record<string, NativeEventType> = {
  click: "click",
  showMore: "click",
  toggleFile: "click",
  lineClick: "click",
  mouseDown: "mousedown",
  mouseUp: "mouseup",
  mouseMove: "mousemove",
  mouseEnter: "mouseenter",
  mouseLeave: "mouseleave",
  keyDown: "keydown",
  keyUp: "keyup",
  focus: "focus",
  blur: "blur",
  change: "input",
  submit: "submit",
  mouseDownOutside: "dismiss",
  scroll: "wheel",
};
export class UnsupportedGpuixFeature extends Error {
  constructor(feature: string) {
    super(`GPUIX parity gap: ${feature}`);
  }
}
export function translateStyle(value: Record<string, unknown>): Record<string, unknown> {
  const output: Record<string, unknown> = {};
  for (const [key, item] of Object.entries(value)) {
    if (item === undefined) continue;
    if (key === "boxShadow" && typeof item === "object" && item !== null) {
      const shadows = Array.isArray(item) ? item : [item];
      output.boxShadow = shadows
        .map(
          (shadow) =>
            `${shadow.offsetX ?? 0}px ${shadow.offsetY ?? 0}px ${(shadow.blurRadius ?? 0) * 2}px ${shadow.spreadRadius ?? 0}px ${shadow.color ?? "#000000"}`,
        )
        .join(", ");
    } else if (key === "textAlign" && (item === "center" || item === "right"))
      output.textAlign = `${item}-including-whitespace`;
    else if (key === "background" && typeof item === "object" && item !== null) {
      const gradient = item as Record<string, unknown>;
      if (gradient.type !== "linear-gradient")
        throw new UnsupportedGpuixFeature(`background.${gradient.type}`);
      output.bg = {
        ...gradient,
        type: "linear",
        interpolation: gradient.colorSpace ?? "srgb",
        dither: true,
        boxProjection: true,
      };
    } else if (key === "whiteSpace" && item === "normal")
      output.whiteSpace = "normal-with-trailing-space";
    else if (key === "background") output.bg = item;
    else if (key === "pointerEvents") {
      if (item !== "none" && item !== "auto")
        throw new UnsupportedGpuixFeature(`pointerEvents ${item}`);
    } else if (key === "backgroundColor") output.bg = item;
    else if (key === "hover" || key === "active" || key === "focus")
      output[key] = translateStyle(item as Record<string, unknown>);
    else if (styles.has(key)) output[key] = item;
    else throw new UnsupportedGpuixFeature(`style.${key}`);
  }
  return output;
}

/** GPUIX's React reconciler stays intact; QuickGUI owns native nodes and the OS loop. */
export class GpuixRenderer {
  readonly records = new Map<number, RecordNode>();
  private nativeIds = new WeakMap<NativeNode, number>();
  readonly resizeListeners = new Set<() => void>();
  window?: Window;
  private size = { width: 800, height: 600 };
  private scrollRevision = 0;
  private rootId: number | null = null;
  private closed = false;
  constructor(private callback?: Callback) {}
  init(options: Record<string, any> = {}): void {
    if (this.window) throw new Error("GPUIX renderer already initialized");
    this.size = { width: options.width ?? 800, height: options.height ?? 600 };
    this.window = new Window({
      ...this.size,
      title: options.title ?? "QuickGUI",
      background: "#000000",
      minimumWidth: options.minWidth,
      minimumHeight: options.minHeight,
      titleBarStyle: options.titlebarTransparent ? "hiddenInset" : "default",
      trafficLightPosition: { x: options.trafficLightX ?? 18, y: options.trafficLightY ?? 24 },
      focus: options.focus ?? true,
      renderer: () => () => {
        this.closed = true;
        this.records.clear();
        this.resizeListeners.clear();
      },
    });
    setNativeProperty(this.window.root, PropertyCode.LayoutRounding, false);
    if (options.titlebarTransparent) {
      const dragBand = createNativeElement("div");
      setProp(dragBand, "style", {
        position: "absolute",
        top: 0,
        left: 0,
        width: "100%",
        height: options.titlebarHeight ?? 48,
        appRegion: "drag",
      });
      insertNativeNode(this.window.root, dragBand);
    }
    this.window.onStateChange((state) => {
      const next = state.viewportSize;
      if (next.width === this.size.width && next.height === this.size.height) return;
      this.size = { ...next };
      for (const listener of this.resizeListeners) listener();
    });
  }
  requiresTick(): boolean {
    return false;
  }
  isInitialized(): boolean {
    return !!this.window && !this.closed;
  }
  getWindowSize() {
    return this.size;
  }
  subscribeWindowSize(listener: () => void) {
    this.resizeListeners.add(listener);
    return () => {
      this.resizeListeners.delete(listener);
    };
  }
  getWindowInsets() {
    const edges = { top: 0, right: 0, bottom: 0, left: 0 };
    return { safeArea: edges, ime: edges, effective: edges };
  }
  setWindowKeyEvents(down: boolean, up: boolean, id: number) {
    if (!this.window) throw new Error("Initialize the renderer first");
    for (const [enabled, type, eventType] of [
      [down, "keydown", "windowKeyDown"],
      [up, "keyup", "windowKeyUp"],
    ] as const) {
      setNativeEventListener(
        this.window.root,
        type,
        enabled ? (event) => this.emit(id, eventType, event) : undefined,
      );
    }
  }
  private emit(id: number, eventType: string, event: QuickGuiEvent) {
    let data: Record<string, unknown> = {};
    if (event.type === "input") data = { value: event.value ?? "" };
    else if (event.value) {
      try {
        const parsed = JSON.parse(event.value);
        if (parsed && typeof parsed === "object") data = parsed;
      } catch {}
    }
    if (typeof data.key === "string") data.key = data.key.replace(/^Arrow/, "").toLowerCase();
    if (data.text !== undefined) data.keyChar = data.text;
    data.modifiers = {
      shift: !!data.shift,
      ctrl: !!data.control,
      alt: !!data.alt,
      cmd: !!data.meta,
    };
    if (typeof data.button === "string")
      data.button = ({ left: 0, middle: 1, right: 2 } as Record<string, number>)[data.button];
    this.callback?.(null, {
      ...data,
      elementId: id,
      eventType: typeof data.gpuixEvent === "string" ? data.gpuixEvent : eventType,
    });
  }
  private refreshPresentation(item: RecordNode) {
    const props = item.props;
    if (item.type === "code" || item.type === "diff") {
      setNativeProperty(
        item.node,
        PropertyCode.RichDocument,
        JSON.stringify({
          kind: item.type,
          scroll: props.scroll,
          source: props.code ?? props.patch ?? "",
          language: props.language,
          path: props.path,
          showLineNumbers: props.showLineNumbers,
          wordDiff: props.wordDiff,
          maxLines: props.maxLines,
          collapsedFiles: props.collapsedFiles,
          theme: props.theme,
          events: props.documentEvents ?? [],
        }),
      );
    }
    if (item.type === "anchored") {
      if (!item.node.parent) return;
      const side = String(props.side ?? "bottom");
      const align = String(props.align ?? "start");
      const pointPlacement: Record<string, string> = {
        topLeft: "bottom-start",
        topCenter: "bottom",
        topRight: "bottom-end",
        bottomLeft: "top-start",
        bottomCenter: "top",
        bottomRight: "top-end",
        leftCenter: "right",
        rightCenter: "left",
      };
      const position = props.position as { x: number; y: number } | undefined;
      const offset = (props.offset ?? {}) as { x?: number; y?: number };
      setNativeProperty(
        item.node,
        PropertyCode.AnchoredLayer,
        JSON.stringify({
          ...(position
            ? { position: [position.x + (offset.x ?? 0), position.y + (offset.y ?? 0)] }
            : { target: item.node.parent.id }),
          placement: props.anchor
            ? pointPlacement[String(props.anchor)]
            : side + (align === "center" ? "" : `-${align}`),
          gap: props.gap ?? 0,
          alignOffset: position
            ? 0
            : ((side === "top" || side === "bottom" ? offset.x : offset.y) ?? 0),
          margin: props.snapMargin ?? 8,
          flip: props.fit === "switch",
          roundOffset: true,
          occlude: props.occlude ?? true,
          priority: props.priority ?? 1,
        }),
      );
    }
    if (item.type === "virtual-list") {
      setProp(item.node, "estimatedItemHeight", props.estimatedItemHeight ?? 48);
      setProp(item.node, "listAlignment", props.alignment ?? "top");
      setProp(item.node, "followMode", props.followTail ? "tail" : "normal");
      setProp(item.node, "overscan", 0);
      setNativeProperty(item.node, PropertyCode.OverscanPixels, Number(props.overdraw ?? 600));
    }
    if (item.type === "input" || item.type === "textarea") {
      const theme = (props.theme ?? {}) as Record<string, unknown>;
      setNativeProperty(
        item.node,
        PropertyCode.InputPresentation,
        JSON.stringify({
          insets: [0, 0, 0, 0],
          caretWidth: 2,
          caretHeightEm: 0.75,
          caretColor: color(String(theme.caret ?? "#ffffff")),
          placeholderColor: color("#8f8f8f"),
        }),
      );
    }
  }
  private applyStyle(item: RecordNode) {
    const theme = (item.props.theme ?? {}) as Record<string, unknown>;
    const input = item.type === "input" || item.type === "textarea";
    const document = ["markdown", "code", "diff", "virtual-list"].includes(item.type);
    const sourceStyle = (item.props.style ?? {}) as Record<string, unknown>;
    setNativeProperty(
      item.node,
      PropertyCode.BlockPointer,
      sourceStyle.pointerEvents === "none"
        ? false
        : sourceStyle.pointerEvents === "auto"
          ? true
          : null,
    );
    const lineHeight = Math.round(
      Number(
        sourceStyle.lineHeight ?? Number(sourceStyle.fontSize ?? 16) * ((1 + Math.sqrt(5)) / 2),
      ),
    );
    setProp(item.node, "style", {
      display: input || document ? "flex" : "block",
      flexDirection: document ? "column" : "row",
      flexShrink: 1,
      ...(input ||
      item.type === "anchored" ||
      (item.props.dragBlockingHandlers as string[] | undefined)?.length
        ? { appRegion: "no-drag" }
        : {}),
      ...(this.rootId !== null && this.get(this.rootId) === item
        ? { whiteSpace: "normal-with-trailing-space" }
        : {}),
      ...(item.type === "anchored" ? { bg: "#1a1a1a" } : {}),
      ...(input
        ? {
            width: "100%",
            height: "auto",
            minWidth: 0,
            color: theme.text ?? "#ffffff",
            fontFamily: theme.fontSans,
            fontSize: 16,
            lineHeight,
            minHeight: Number(item.props.minRows ?? 1) * lineHeight,
            maxHeight: Number(item.props.maxRows ?? item.props.minRows ?? 1) * lineHeight,
            focus: { borderWidth: 0 },
          }
        : {}),
      ...translateStyle((item.props.style ?? {}) as Record<string, unknown>),
      // GPUIX 0.7.0 measures the editor with its captured style but paints its
      // lines using the window default. Preserve the pinned release's behavior.
      ...(input ? { lineHeight: Math.round(16 * ((1 + Math.sqrt(5)) / 2)) } : {}),
    });
    if (sourceStyle.boxShadow && typeof sourceStyle.boxShadow === "object") {
      const shadows = Array.isArray(sourceStyle.boxShadow)
        ? sourceStyle.boxShadow
        : [sourceStyle.boxShadow];
      setNativeProperty(
        item.node,
        PropertyCode.BoxShadow,
        JSON.stringify(
          shadows.map((shadow) => ({
            offsetX: shadow.offsetX ?? 0,
            offsetY: shadow.offsetY ?? 0,
            blurRadius: (shadow.blurRadius ?? 0) * 2,
            spreadRadius: shadow.spreadRadius ?? 0,
            color: color(shadow.color ?? "#000000"),
            orderBySubject: true,
          })),
        ),
      );
    }
  }
  private get(id: number): RecordNode {
    const item = this.records.get(id);
    if (!item) throw new Error(`Unknown GPUIX node ${id}`);
    return item;
  }
  private validateStructure(operations: Operation[]): void {
    const created = new Set<number>(),
      removed = new Set<number>();
    const parents = new Map<number, number | undefined>();
    const children = new Map<number, number[]>();
    const exists = (id: number) => !removed.has(id) && (created.has(id) || this.records.has(id));
    const requireNode = (id: number) => {
      if (!Number.isSafeInteger(id) || id < 1 || !exists(id))
        throw new Error(`Unknown GPUIX node ${id}`);
    };
    const parentOf = (id: number) =>
      parents.has(id) ? parents.get(id) : this.nativeIds.get(this.records.get(id)?.node.parent!);
    const childrenOf = (id: number): number[] => {
      if (!children.has(id))
        children.set(
          id,
          this.records.get(id)?.node.children.flatMap((node) => {
            const key = this.nativeIds.get(node);
            return key === undefined ? [] : [key];
          }) ?? [],
        );
      return children.get(id)!;
    };
    for (const [op, id, first, second] of operations) {
      if (!Number.isSafeInteger(id) || id < 1) throw new Error("Invalid GPUIX ID");
      if (op === "createElement") {
        if (exists(id)) throw new Error(`Duplicate GPUIX node ${id}`);
        created.add(id);
        removed.delete(id);
        parents.set(id, undefined);
        children.set(id, []);
      } else if (op === "destroyElement") {
        if (!exists(id)) continue;
        const parent = parentOf(id);
        if (parent !== undefined)
          children.set(
            parent,
            childrenOf(parent).filter((child) => child !== id),
          );
        const pending = [id];
        while (pending.length) {
          const key = pending.pop()!;
          pending.push(...childrenOf(key));
          removed.add(key);
        }
      } else {
        requireNode(id);
        if (op === "appendChild" || op === "insertBefore") {
          const child = Number(first);
          requireNode(child);
          if (second !== undefined) {
            requireNode(Number(second));
            if (parentOf(Number(second)) !== id) throw new Error("Invalid GPUIX insertion anchor");
          }
          let ancestor: number | undefined = id;
          let depth = 0;
          while (ancestor !== undefined) {
            if (ancestor === child) throw new Error("GPUIX insertion creates a cycle");
            if (++depth > 256) throw new Error("GPUIX tree depth exceeds 256");
            ancestor = parentOf(ancestor);
          }
          const previous = parentOf(child);
          if (previous !== undefined)
            children.set(
              previous,
              childrenOf(previous).filter((key) => key !== child),
            );
          const siblings = childrenOf(id);
          const index = second === undefined ? siblings.length : siblings.indexOf(Number(second));
          siblings.splice(index, 0, child);
          parents.set(child, id);
        }
      }
    }
    if (this.records.size + created.size - removed.size > 65536)
      throw new Error("GPUIX node limit exceeded");
  }
  applyBatch(json: string): number[] {
    if (this.closed) throw new Error("GPUIX renderer is closed");
    if (Buffer.byteLength(json) > 16 * 1024 * 1024) throw new Error("GPUIX batch exceeds 16 MiB");
    const operations = JSON.parse(json) as Operation[];
    if (!Array.isArray(operations) || operations.length > 65536)
      throw new Error("Invalid GPUIX batch");
    if (operations.some((op) => !Array.isArray(op) || op.length < 2))
      throw new Error("Invalid GPUIX operation");
    this.validateStructure(operations);
    // Validate capability coverage before touching the retained native tree.
    const gaps = new Set<string>();
    for (const [op, id, first, second] of operations) {
      try {
        if (op === "createElement" && !(String(first) in tags))
          throw new UnsupportedGpuixFeature(`element ${first}`);
        if (op === "setStyle") translateStyle(first as Record<string, unknown>);
        if (op === "setCustomProp" && first === "deferred" && second === false)
          throw new UnsupportedGpuixFeature("non-deferred anchored layers");
        if (
          op === "setCustomProp" &&
          first === "priority" &&
          second != null &&
          (!Number.isInteger(second) || Number(second) < 0 || Number(second) > 32767)
        )
          throw new UnsupportedGpuixFeature("anchor priority outside 0..32767");
        if (op === "setEventListener" && second && !(String(first) in events))
          throw new UnsupportedGpuixFeature(`event ${first}`);
        if (
          op === "setCustomProp" &&
          ![
            "scroll",
            "code",
            "patch",
            "language",
            "path",
            "showLineNumbers",
            "wordDiff",
            "maxLines",
            "collapsedFiles",
            "testId",
            "source",
            "src",
            "value",
            "placeholder",
            "autoFocus",
            "tabIndex",
            "disabled",
            "motion",
            "position",
            "side",
            "align",
            "anchor",
            "gap",
            "offset",
            "fit",
            "snapMargin",
            "deferred",
            "priority",
            "occlude",
            "theme",
            "readOnly",
            "minRows",
            "maxRows",
            "alignment",
            "followTail",
            "estimatedItemHeight",
            "overdraw",
          ].includes(String(first))
        )
          throw new UnsupportedGpuixFeature(`property ${first}`);
        if (
          ![
            "createElement",
            "destroyElement",
            "appendChild",
            "insertBefore",
            "setStyle",
            "setText",
            "setEventListener",
            "setRoot",
            "setCustomProp",
          ].includes(op)
        )
          throw new UnsupportedGpuixFeature(`operation ${op}`);
      } catch (error) {
        gaps.add(String(error));
      }
    }
    if (gaps.size) throw new Error([...gaps].join("\n"));
    const destroyed: number[] = [];
    const presentations = new Set<RecordNode>();
    for (const [op, id, first, second] of operations) {
      if (op === "createElement") {
        if (this.records.has(id)) throw new Error(`Duplicate GPUIX node ${id}`);
        const type = String(first);
        const node = createNativeElement(tags[type]!);
        setProp(node, "style", { display: "block", flexDirection: "row", flexShrink: 1 });
        this.records.set(id, { node, type, props: {} });
        this.nativeIds.set(node, id);
        this.applyStyle(this.get(id));
        presentations.add(this.get(id));
      } else if (op === "destroyElement") {
        const item = this.records.get(id);
        if (!item) continue;
        const subtree = new Set<NativeNode>();
        const visit = (node: NativeNode) => {
          subtree.add(node);
          node.children.forEach(visit);
        };
        visit(item.node);
        if (item.node.parent) removeNativeNode(item.node.parent, item.node);
        for (const node of subtree) {
          const key = this.nativeIds.get(node);
          if (key !== undefined) {
            this.records.delete(key);
            this.nativeIds.delete(node);
            destroyed.push(key);
          }
        }
        if (this.rootId === id) this.rootId = null;
      } else if (op === "appendChild" || op === "insertBefore") {
        insertNativeNode(
          this.get(id).node,
          this.get(Number(first)).node,
          second === undefined ? undefined : this.get(Number(second)).node,
        );
        presentations.add(this.get(Number(first)));
      } else if (op === "setRoot") {
        if (!this.window) throw new Error("Initialize the renderer first");
        if (this.rootId !== null && this.rootId !== id)
          removeNativeNode(this.window.root, this.get(this.rootId).node);
        this.rootId = id;
        this.applyStyle(this.get(id));
        insertNativeNode(this.window.root, this.get(id).node);
      } else if (op === "setStyle") {
        this.get(id).props.style = first;
        this.applyStyle(this.get(id));
      } else if (op === "setText") {
        const node = this.get(id).node;
        const child = node.children[0];
        if (child) replaceNativeText(child, String(first));
        else insertNativeNode(node, createNativeText(String(first)));
      } else if (op === "setEventListener") {
        if (first === "click" || first === "mouseDown") {
          const item = this.get(id);
          const handlers = new Set((item.props.dragBlockingHandlers as string[]) ?? []);
          if (second) handlers.add(String(first));
          else handlers.delete(String(first));
          item.props.dragBlockingHandlers = [...handlers];
          setNativeProperty(
            item.node,
            PropertyCode.AppRegion,
            handlers.size ||
              item.type === "input" ||
              item.type === "textarea" ||
              item.type === "anchored"
              ? "no-drag"
              : null,
          );
        }
        if (["showMore", "toggleFile", "lineClick"].includes(String(first))) {
          const item = this.get(id);
          const listeners = new Set((item.props.documentEvents as string[]) ?? []);
          if (second) listeners.add(String(first));
          else listeners.delete(String(first));
          item.props.documentEvents = [...listeners];
          presentations.add(item);
          setNativeEventListener(item.node, "click", (event) => this.emit(id, "click", event));
          continue;
        }
        if (first === "submit" && this.get(id).type === "textarea")
          setNativeProperty(this.get(id).node, PropertyCode.InputSubmitOnEnter, !!second);
        if (first === "mouseDownOutside")
          setNativeProperty(this.get(id).node, PropertyCode.DismissOnPointerOutside, !!second);
        setNativeEventListener(
          this.get(id).node,
          events[String(first)]!,
          second ? (event) => this.emit(id, String(first), event) : undefined,
        );
      } else if (op === "setCustomProp") {
        const item = this.get(id);
        if (first === "motion") {
          item.props.motion = second;
          setNativeProperty(
            item.node,
            PropertyCode.Motion,
            second == null ? null : JSON.stringify(second),
          );
          continue;
        }
        item.props[String(first)] = second;
        if (first === "testId") continue;
        if (item.type === "code" || item.type === "diff") {
          presentations.add(item);
          continue;
        }
        if (item.type === "virtual-list") {
          presentations.add(item);
          continue;
        }
        if (item.type === "anchored") {
          presentations.add(item);
          continue;
        }
        if (first === "minRows" || first === "maxRows") {
          this.applyStyle(item);
          continue;
        }
        if (first === "theme") {
          if (item.type === "markdown")
            setNativeProperty(
              item.node,
              PropertyCode.DocumentTheme,
              second == null ? null : JSON.stringify(second),
            );
          this.applyStyle(item);
          presentations.add(item);
          continue;
        }
        if (first === "autoFocus") {
          setNativeProperty(item.node, PropertyCode.AutoFocus, !!second);
          continue;
        }
        if (first === "source" || first === "src") setProp(item.node, "value", second);
        else setProp(item.node, String(first), second);
      }
    }
    for (const item of presentations)
      if (this.nativeIds.has(item.node)) this.refreshPresentation(item);
    this.window?.flush();
    return destroyed;
  }
  scrollTo(id: number, x: number, y: number) {
    if (![x, y].every(Number.isFinite)) throw new TypeError("scroll offsets must be finite");
    setNativeProperty(
      this.get(id).node,
      PropertyCode.ScrollRequest,
      JSON.stringify({ revision: ++this.scrollRevision, x: -x, y: -y }),
    );
    this.window?.flush();
  }
  scrollToItem(id: number, index: number, offset = 0) {
    if (!Number.isSafeInteger(index) || index < 0 || !Number.isFinite(offset))
      throw new TypeError("invalid scroll item");
    setNativeProperty(
      this.get(id).node,
      PropertyCode.ScrollRequest,
      JSON.stringify({ revision: ++this.scrollRevision, index, x: 0, y: offset }),
    );
    this.window?.flush();
  }
  focusElement(id: number) {
    this.get(id).node.focus();
  }
  setWindowTitle(title: string) {
    this.window?.setTitle(title);
  }
  activateWindow() {
    this.window?.show();
  }
  getRootId() {
    return this.rootId;
  }
  getTreeJson() {
    return JSON.stringify(
      [...this.records].map(([id, item]) => ({ id, type: item.type, props: item.props })),
    );
  }
}

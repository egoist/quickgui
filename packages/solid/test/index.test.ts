import { describe, expect, test } from "bun:test";
import {
  app,
  parseColor,
  MAX_COLLECTION_JSON_BYTES,
  MAX_COMPONENT_VALUE_BYTES,
  MAX_OPTIONS_JSON_BYTES,
  MAX_KEYMAP_JSON_BYTES,
  MAX_MENU_JSON_BYTES,
  MAX_GROUP_STYLES_PER_ELEMENT,
  MAX_HOVER_GROUP_NAME_BYTES,
  MAX_STATE_STYLE_JSON_BYTES,
  MAX_STYLE_DECLARATION_BYTES,
  MAX_TOOLTIP_TEXT_BYTES,
  NativeNodeTag,
  PropertyCode,
  QuickGuiEvent,
  Window,
} from "@quickgui/native";
import { createSignal, flush, onCleanup } from "solid-js";
import * as solid from "../src/index.ts";
import {
  Accordion,
  AlertDialog,
  Autocomplete,
  Avatar,
  Button,
  Calendar,
  Checkbox,
  CheckboxGroup,
  Collapsible,
  Combobox,
  ContextMenu,
  DateField,
  Dialog,
  Drawer,
  Image,
  Meter,
  Field,
  Fieldset,
  Input,
  Markdown,
  Menu,
  Menubar,
  NavigationMenu,
  NumberField,
  OtpField,
  Popover,
  PopoverMenu,
  PreviewCard,
  Progress,
  Radio,
  RadioGroup,
  ScrollArea,
  Select,
  Separator,
  Switch,
  Shader,
  Slider,
  Splitter,
  Table,
  Tabs,
  TimeField,
  Toast,
  Toggle,
  ToggleGroup,
  Toolbar,
  Tree,
  SystemPopover,
  Svg,
  Terminal,
  Text,
  TextArea,
  View,
  VirtualList,
  createComponent,
  createElement,
  createRenderer,
  createTextNode,
  actionFromEvent,
  capturedPointerFromEvent,
  dropEventFromEvent,
  encodeMenu,
  gestureEventFromEvent,
  keyEventFromEvent,
  insertNode,
  menuSelectionFromEvent,
  mouseEventFromEvent,
  wheelEventFromEvent,
  setProp,
  terminalStatusFromEvent,
  type MenuSelectDetails,
} from "../src/index.ts";

describe("Solid universal host", () => {
  test("keeps native application APIs in @quickgui/native", () => {
    expect("app" in solid).toBe(false);
    expect("Window" in solid).toBe(false);
    expect(solid.Router).toBeTypeOf("function");
    expect(solid.Route).toBeTypeOf("function");
    expect(solid.Link).toBeTypeOf("function");
    expect(solid.Outlet).toBeTypeOf("function");
    expect("A" in solid).toBe(false);
    // `Dialog` in this package is the caller-styled in-window composition, never the native
    // alert/file dialog namespace that stays in @quickgui/native.
    expect("showAlertDialog" in solid.Dialog).toBe(false);
    expect("showOpenDialog" in solid.Dialog).toBe(false);
    expect(solid.Dialog.Popup).toBe(solid.DialogPopup);
  });

  test("exports the Base UI popover parts and only Content for the system host", () => {
    expect(Popover.Content).toBe(solid.PopoverContent);
    expect(SystemPopover.Content).toBe(solid.SystemPopoverContent);
    expect(Object.keys(Popover)).toEqual([
      "Root",
      "Trigger",
      "Content",
      "Portal",
      "Backdrop",
      "Positioner",
      "Popup",
      "Arrow",
      "Viewport",
      "Title",
      "Description",
      "Close",
    ]);
    // A native child window is placed by the platform, so it has no positioner of its own.
    expect(Object.keys(SystemPopover)).toEqual(["Root", "Trigger", "Content"]);
  });

  test("retains an unattached native tree without crossing N-API", () => {
    const parent = createElement("view");
    const child = createTextNode("hello");
    insertNode(parent, child);
    setProp(parent, "style", { display: "flex", padding: 12 });

    expect(parent.children).toEqual([child]);
    expect(child.parent).toBe(parent);
    expect(parent.properties.size).toBe(2);
  });

  test("projects retained hover and pressed styles into the native core", () => {
    const button = createComponent(Button, {
      style: {
        hoverBg: "#222233",
        hoverColor: "#ffffff",
        activeBg: "#111122",
        activeColor: "#ddddff",
        transition: "bg 90ms, border-color 90ms, color 90ms",
      },
      children: "New agent",
    });

    expect(button.properties.get(PropertyCode.HoverBackgroundColor)).toBeTypeOf("number");
    expect(button.properties.get(PropertyCode.HoverColor)).toBeTypeOf("number");
    expect(button.properties.get(PropertyCode.ActiveBackgroundColor)).toBeTypeOf("number");
    expect(button.properties.get(PropertyCode.ActiveColor)).toBeTypeOf("number");
    expect(button.properties.get(PropertyCode.Transition)).toBe(90);
  });

  test("projects independent border edges and CSS-like box shadows", () => {
    const panel = createComponent(View, {
      style: {
        color: "#445566",
        borderWidth: 1,
        borderTopWidth: 0,
        borderRightWidth: "2px",
        borderBottomWidth: 3,
        borderLeftWidth: "4px",
        borderColor: "#11223380",
        boxShadow: "0 8px 24px -8px rgba(15, 23, 42, 0.35), inset 0 1px 0 currentColor",
      },
    });

    expect(panel.properties.get(PropertyCode.BorderWidth)).toBe(1);
    expect(panel.properties.get(PropertyCode.BorderTopWidth)).toBe(0);
    expect(panel.properties.get(PropertyCode.BorderRightWidth)).toBe(2);
    expect(panel.properties.get(PropertyCode.BorderBottomWidth)).toBe(3);
    expect(panel.properties.get(PropertyCode.BorderLeftWidth)).toBe(4);
    expect(panel.properties.get(PropertyCode.BorderColor)).toBe(0x80332211);
    expect(JSON.parse(String(panel.properties.get(PropertyCode.BoxShadow)))).toEqual([
      {
        offsetX: 0,
        offsetY: 8,
        blurRadius: 24,
        spreadRadius: -8,
        color: 0x592a170f,
        inset: false,
      },
      {
        offsetX: 0,
        offsetY: 1,
        blurRadius: 0,
        spreadRadius: 0,
        color: null,
        inset: true,
      },
    ]);
  });

  test("rejects invalid CSS-like box shadows", () => {
    const negativeBlur = createElement("view");
    expect(() => setProp(negativeBlur, "style", { boxShadow: "0 2px -1px black" })).toThrow(
      "blur radius cannot be negative",
    );
    const tooMany = createElement("view");
    expect(() =>
      setProp(tooMany, "style", { boxShadow: Array(9).fill("0 1px black").join(", ") }),
    ).toThrow("at most 8 shadows");
  });

  test("projects modal input, focus, dismissal, and accessibility to the native core", () => {
    const prompt = createComponent(TextArea, {
      autoFocus: true,
      value: "",
    });
    const surface = createComponent(View, {
      overlay: true,
      focusTrap: true,
      restorePreviousFocus: true,
      "aria-modal": true,
      dismissOnEscape: true,
      dismissOnPointerOutside: true,
      onDismiss() {},
      children: prompt,
    });

    expect(surface.properties.get(PropertyCode.Overlay)).toBe(true);
    expect(surface.properties.get(PropertyCode.FocusTrap)).toBe(true);
    expect(surface.properties.get(PropertyCode.RestorePreviousFocus)).toBe(true);
    expect(surface.properties.get(PropertyCode.AccessibilityModal)).toBe(true);
    expect(surface.properties.get(PropertyCode.DismissOnEscape)).toBe(true);
    expect(surface.properties.get(PropertyCode.DismissOnPointerOutside)).toBe(true);
    expect(surface.properties.get(PropertyCode.DismissListener)).toBe(true);
    expect(prompt.properties.get(PropertyCode.AutoFocus)).toBe(true);
  });

  test("flushes Solid 2 signal writes at the native event boundary", async () => {
    await app.whenReady();
    let renderedWindow: Window | undefined;
    let eventWindow: Window | undefined;
    const window = new Window({
      renderer: createRenderer(() => {
        renderedWindow = Window.getCurrentWindow();
        const [count, setCount] = createSignal(0);
        return createComponent(View, {
          get children() {
            return [
              createComponent(Text, {
                get children() {
                  return `Count: ${count()}`;
                },
              }),
              createComponent(Button, {
                onClick: () => {
                  eventWindow = Window.getCurrentWindow();
                  setCount((value) => value + 1);
                },
                children: "Increment",
              }),
            ];
          },
        });
      }),
    });

    const container = window.root.children[0]!;
    const count = container.children[0]!;
    const button = container.children[1]!;
    expect(renderedWindow).toBe(window);
    expect(count.children[0]?.text).toBe("Count: 0");

    window._dispatchEvent("click", button.id);

    expect(count.children[0]?.text).toBe("Count: 1");
    expect(eventWindow).toBe(window);
    expect(() => Window.getCurrentWindow()).toThrow("while rendering or handling a window event");
    window.close();
  });

  test("disposes a mounted Solid root when its Window closes", async () => {
    await app.whenReady();
    let cleaned = false;
    const window = new Window({
      title: "Dispose test",
      renderer: createRenderer(() => {
        onCleanup(() => {
          cleaned = true;
        });
        return createComponent(View, { children: "Mounted" });
      }),
    });

    expect(window.root.children).toHaveLength(1);
    window.close();
    await Promise.resolve(); // The native close notification owns renderer disposal.

    expect(window.closed).toBe(true);
    expect(cleaned).toBe(true);
    expect(window.root.children).toHaveLength(0);
  });

  test("commits shared signal writes before a secondary window root closes", async () => {
    await app.whenReady();
    const [value, setValue] = createSignal("before");
    const mainWindow = new Window({
      title: "Main",
      renderer: createRenderer(() =>
        createComponent(Text, {
          get children() {
            return value();
          },
        }),
      ),
    });
    const secondaryWindow = new Window({
      title: "Secondary",
      renderer: createRenderer(() => {
        const window = Window.getCurrentWindow();
        return createComponent(Button, {
          onClick: () => {
            setValue("after");
            window.close();
          },
          children: "Save",
        });
      }),
    });

    const text = mainWindow.root.children[0]!;
    const button = secondaryWindow.root.children[0]!;
    expect(text.children[0]?.text).toBe("before");
    secondaryWindow._dispatchEvent("click", button.id);
    await Promise.resolve();

    expect(secondaryWindow.closed).toBe(true);
    expect(text.children[0]?.text).toBe("after");
    mainWindow.close();
  });

  test("bridges controlled input values and exact native input and submit payloads", () => {
    let value = "";
    let submitted = "";
    const input = createComponent(Input, {
      value: "hello",
      onInput: (event: QuickGuiEvent) => {
        value = event.value ?? "";
      },
      onSubmit: (event: QuickGuiEvent) => {
        submitted = event.value ?? "";
      },
    });

    expect(input.properties.get(PropertyCode.Value)).toBe("hello");
    expect(input.properties.get(PropertyCode.InputListener)).toBe(true);
    expect(input.properties.get(PropertyCode.SubmitListener)).toBe(true);
    input.listeners.get("input")!(new QuickGuiEvent("input", input, "hello world"));
    expect(value).toBe("hello world");
    input.listeners.get("submit")!(new QuickGuiEvent("submit", input, "hello world"));
    expect(submitted).toBe("hello world");
  });

  test("maps web-style password input types without replacing the controlled value", () => {
    const input = createComponent(Input, {
      type: "password",
      value: "sk-secret",
    });

    expect(input.properties.get(PropertyCode.Password)).toBe(true);
    expect(input.properties.get(PropertyCode.Value)).toBe("sk-secret");

    setProp(input, "type", "text", "password");
    expect(input.properties.get(PropertyCode.Password)).toBe(false);
    expect(input.properties.get(PropertyCode.Value)).toBe("sk-secret");
  });

  test("can preserve keyboard focus when a button is pressed with a pointer", () => {
    const button = createComponent(Button, { focusOnPointer: false });

    expect(button.properties.get(PropertyCode.FocusOnPointer)).toBe(false);
  });

  test("maps paint-free hit slop for thin native interaction targets", () => {
    const divider = createComponent(View, {
      style: {
        width: 1,
        hitSlop: 2,
        hitSlopTop: 3,
        hitSlopRight: 4,
        hitSlopBottom: 5,
        hitSlopLeft: 6,
      },
    });

    expect(divider.properties.get(PropertyCode.Width)).toBe(1);
    expect(divider.properties.get(PropertyCode.HitSlop)).toBe(2);
    expect(divider.properties.get(PropertyCode.HitSlopTop)).toBe(3);
    expect(divider.properties.get(PropertyCode.HitSlopRight)).toBe(4);
    expect(divider.properties.get(PropertyCode.HitSlopBottom)).toBe(5);
    expect(divider.properties.get(PropertyCode.HitSlopLeft)).toBe(6);
  });

  test("creates retained SVG nodes whose source is parsed by the Rust core", () => {
    const source = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" />';
    const icon = createComponent(Svg, {
      source,
      style: { width: 16, height: 16, color: "#ffffff" },
    });

    expect(icon.tag).toBe(NativeNodeTag.Svg);
    expect(icon.properties.get(PropertyCode.Value)).toBe(source);
    expect(icon.properties.get(PropertyCode.Width)).toBe(16);
    expect(icon.properties.get(PropertyCode.Height)).toBe(16);
    expect(icon.properties.get(PropertyCode.Color)).toBeTypeOf("number");
  });

  test("declaratively configures a core-owned PTY terminal and decodes status events", () => {
    let status = "";
    const terminal = createComponent(Terminal, {
      program: "/bin/zsh",
      args: ["-l"],
      cwd: "/tmp",
      env: { QUICKGUI_TERMINAL_TEST: "1" },
      scrollback: 20_000,
      terminalCursorColor: "#0969da",
      terminalPaddingColor: "extend",
      fontThicken: true,
      terminalPalette: [
        "#24292f",
        "#cf222e",
        "#116329",
        "#4d2d00",
        "#0969da",
        "#8250df",
        "#1b7c83",
        "#6e7781",
        "#57606a",
        "#a40e26",
        "#1a7f37",
        "#633c01",
        "#218bff",
        "#a475f9",
        "#3192aa",
        "#8c959f",
      ],
      style: { fontFamily: "JetBrainsMono Nerd Font Mono" },
      onStatus: (event) => {
        status = terminalStatusFromEvent(event).status;
      },
    });

    expect(terminal.tag).toBe(NativeNodeTag.Terminal);
    expect(terminal.properties.get(PropertyCode.TerminalProgram)).toBe("/bin/zsh");
    expect(terminal.properties.get(PropertyCode.TerminalArguments)).toBe('["-l"]');
    expect(terminal.properties.get(PropertyCode.TerminalWorkingDirectory)).toBe("/tmp");
    expect(terminal.properties.get(PropertyCode.TerminalEnvironment)).toBe(
      '{"QUICKGUI_TERMINAL_TEST":"1"}',
    );
    expect(terminal.properties.get(PropertyCode.TerminalScrollback)).toBe(20_000);
    expect(terminal.properties.get(PropertyCode.TerminalStatusListener)).toBe(true);
    expect(terminal.properties.get(PropertyCode.FontFamily)).toBe("JetBrainsMono Nerd Font Mono");
    expect(terminal.properties.get(PropertyCode.TerminalCursorColor)).toBeTypeOf("number");
    expect(terminal.properties.get(PropertyCode.TerminalPaddingColor)).toBe("extend");
    expect(terminal.properties.get(PropertyCode.TerminalFontThicken)).toBe(true);
    expect(
      JSON.parse(terminal.properties.get(PropertyCode.TerminalPalette) as string),
    ).toHaveLength(16);

    terminal.listeners.get("terminal")!(
      new QuickGuiEvent(
        "terminal",
        terminal,
        '{"status":"running","title":"zsh","workingDirectory":"/tmp","processId":42}',
      ),
    );
    expect(status).toBe("running");
  });

  test("bridges the Rust-core captured pointer stream", () => {
    let delta = 0;
    const divider = createComponent(View, {
      onPointer: (event) => {
        delta =
          capturedPointerFromEvent(event).position.x - capturedPointerFromEvent(event).origin.x;
      },
    });

    expect(divider.properties.get(PropertyCode.PointerListener)).toBe(true);
    divider.listeners.get("pointer")!(
      new QuickGuiEvent(
        "pointer",
        divider,
        '{"phase":"move","position":{"x":310,"y":40},"origin":{"x":250,"y":40},"localPosition":{"x":60,"y":20},"localOrigin":{"x":0,"y":20},"delta":{"x":4,"y":0},"button":"left"}',
      ),
    );
    expect(delta).toBe(60);
  });

  test("retains Markdown source and streaming presentation properties", () => {
    const markdown = createComponent(Markdown, {
      content: "# Hello",
      streaming: true,
      style: { markdownLinkColor: "#60a5fa" },
    });

    expect(markdown.properties.get(PropertyCode.Value)).toBe("# Hello");
    expect(markdown.properties.get(PropertyCode.Streaming)).toBe(true);
    expect(markdown.properties.get(PropertyCode.MarkdownLinkColor)).toBeTypeOf("number");
  });

  test("creates unstyled variable lists with native windowing properties", () => {
    const list = createComponent(VirtualList, {
      estimatedItemHeight: 180,
      overscan: 3,
      overscanPixels: 240,
      itemHeights: [2_000_000],
      listAlignment: "bottom",
      followMode: "tail",
      children: createComponent(Text, { children: "Visible row" }),
    });

    expect(list.properties.get(PropertyCode.EstimatedItemHeight)).toBe(180);
    expect(list.properties.get(PropertyCode.Overscan)).toBe(3);
    expect(list.properties.get(PropertyCode.OverscanPixels)).toBe(240);
    expect(list.properties.get(PropertyCode.ItemHeights)).toBe("[2000000]");
    expect(list.properties.get(PropertyCode.ListAlignment)).toBe("bottom");
    expect(list.properties.get(PropertyCode.FollowMode)).toBe("tail");
    expect(list.children).toHaveLength(1);
  });

  test("coordinates Popover compound parts without exposing the native anchor", async () => {
    await app.whenReady();
    const [open, setOpen] = createSignal(false);
    const changes: Array<{ open: boolean; reason: string }> = [];
    const window = new Window({
      title: "Popover compound parts",
      renderer: createRenderer(() =>
        createComponent(Popover.Root, {
          get open() {
            return open();
          },
          dismissOnEscape: false,
          onOpenChange(nextOpen, details) {
            changes.push({ open: nextOpen, reason: details.reason });
            setOpen(nextOpen);
          },
          get children() {
            return [
              createComponent(Popover.Trigger, { children: "Open" }),
              createComponent(Popover.Content, {
                width: 240,
                height: 120,
                placement: "bottom-end",
                gap: 8,
                viewportMargin: 12,
                children: createComponent(Text, { children: "Popover" }),
              }),
            ];
          },
        }),
      ),
    });

    const trigger = window.root.children[0]!;
    window._focusNode = () => {
      throw new Error("Popover focus restoration must stay in the Rust core");
    };
    expect(window.root.children).toEqual([trigger]);

    window._dispatchEvent("click", trigger.id);

    const popover = window.root.children[1]!;
    expect(open()).toBe(true);
    expect(popover.properties.get(PropertyCode.AnchorTarget)).toBe(String(trigger.id));
    expect(popover.properties.get(PropertyCode.AnchorPlacement)).toBe("bottom-end");
    expect(popover.properties.get(PropertyCode.AnchorGap)).toBe(8);
    expect(popover.properties.get(PropertyCode.ViewportMargin)).toBe(12);
    expect(popover.properties.get(PropertyCode.DismissOnEscape)).toBe(false);
    expect(popover.properties.get(PropertyCode.DismissOnPointerOutside)).toBe(true);
    expect(popover.properties.get(PropertyCode.DismissListener)).toBe(true);
    expect(popover.children[0]?.children[0]?.text).toBe("Popover");
    expect(changes).toEqual([{ open: true, reason: "trigger-press" }]);

    popover.listeners.get("dismiss")!(new QuickGuiEvent("dismiss", popover));
    await Promise.resolve();
    await Promise.resolve();
    expect(open()).toBe(false);
    expect(window.root.children).toEqual([trigger]);
    expect(changes).toEqual([
      { open: true, reason: "trigger-press" },
      { open: false, reason: "dismiss" },
    ]);

    window._dispatchEvent("click", trigger.id);
    expect(open()).toBe(true);
    setOpen(false);
    await Promise.resolve();
    await Promise.resolve();
    expect(window.root.children).toEqual([trigger]);
    window.close();
  });

  test("mounts SystemPopover.Content into a separately disposed renderer", async () => {
    await app.whenReady();
    const [open, setOpen] = createSignal(false);
    const changes: Array<{ open: boolean; reason: string }> = [];
    const owner = new Window({
      title: "System popover owner",
      renderer: createRenderer(() =>
        createComponent(SystemPopover.Root, {
          get open() {
            return open();
          },
          onOpenChange(nextOpen, details) {
            changes.push({ open: nextOpen, reason: details.reason });
            setOpen(nextOpen);
          },
          get children() {
            return [
              createComponent(SystemPopover.Trigger, { children: "Open" }),
              createComponent(SystemPopover.Content, {
                width: 260,
                height: 140,
                placement: "bottom-start",
                gap: 8,
                viewportMargin: 12,
                children: createComponent(Text, { children: "Separate root" }),
              }),
            ];
          },
        }),
      ),
    });

    const trigger = owner.root.children[0]!;
    owner._focusNode = () => {
      throw new Error("SystemPopover focus restoration must stay in the Rust core");
    };
    expect(trigger.materialized).toBe(true);
    expect(trigger.host).toBe(owner);
    expect(owner.nodes.has(trigger.id)).toBe(true);
    expect(owner.root.children).toEqual([trigger]);

    const beforeOpen = new Set(app.windows.keys());
    owner._dispatchEvent("click", trigger.id);
    await Promise.resolve();

    const systemWindow = [...app.windows.values()].find(
      (window) => window !== owner && !beforeOpen.has(window.nativeId),
    );
    expect(open()).toBe(true);
    expect(systemWindow).toBeDefined();
    expect(systemWindow!.root.children).toHaveLength(1);
    const surface = systemWindow!.root.children[0]!;
    expect(surface.properties.get(PropertyCode.Width)).toBe(260);
    expect(surface.properties.get(PropertyCode.Height)).toBe(140);
    expect(surface.properties.has(PropertyCode.AnchorTarget)).toBe(false);
    expect(surface.children[0]?.children[0]?.text).toBe("Separate root");
    expect(changes).toEqual([{ open: true, reason: "trigger-press" }]);

    systemWindow!.close();
    await Promise.resolve();
    await Promise.resolve();
    expect(open()).toBe(false);
    expect(systemWindow!.closed).toBe(true);
    expect(systemWindow!.root.children).toHaveLength(0);
    expect(changes).toEqual([
      { open: true, reason: "trigger-press" },
      { open: false, reason: "dismiss" },
    ]);

    const beforeReopen = new Set(app.windows.keys());
    owner._dispatchEvent("click", trigger.id);
    await Promise.resolve();
    const reopened = [...app.windows.values()].find(
      (window) => window !== owner && !beforeReopen.has(window.nativeId),
    );
    expect(open()).toBe(true);
    expect(reopened).toBeDefined();

    // AppKit consumes a press on the active SystemPopover anchor before JavaScript dispatch. A
    // native close still settles the controlled JSX lifecycle normally.
    reopened!.close();
    await Promise.resolve();
    await Promise.resolve();
    expect(open()).toBe(false);
    expect(changes.at(-1)).toEqual({ open: false, reason: "dismiss" });
    expect([...app.windows.values()]).toEqual([owner]);

    const beforeOwned = new Set(app.windows.keys());
    owner._dispatchEvent("click", trigger.id);
    await Promise.resolve();
    const ownedPopover = [...app.windows.values()].find(
      (window) => window !== owner && !beforeOwned.has(window.nativeId),
    );
    expect(ownedPopover).toBeDefined();
    owner.close();
    await Promise.resolve();
    expect(ownedPopover!.closed).toBe(true);
    expect([...app.windows.values()]).toEqual([]);
    await Promise.resolve();
    await Promise.resolve();
  });

  test("exports Base UI-shaped compound parts for every bound core component", () => {
    expect(Object.keys(Checkbox)).toEqual(["Root", "Indicator"]);
    expect(Object.keys(Radio)).toEqual(["Root", "Indicator"]);
    expect(Object.keys(Switch)).toEqual(["Root", "Thumb"]);
    expect(Object.keys(Tabs)).toEqual(["Root", "List", "Tab", "Indicator", "Panel"]);
    expect(Object.keys(Collapsible)).toEqual(["Root", "Trigger", "Panel"]);
    expect(Object.keys(Accordion)).toEqual(["Root", "Item", "Header", "Trigger", "Panel"]);
    expect(Object.keys(Field)).toEqual([
      "Root",
      "Item",
      "Label",
      "Control",
      "Validity",
      "Description",
      "Error",
    ]);
    expect(Object.keys(Fieldset)).toEqual(["Root", "Legend", "Description", "Control"]);
    expect(Object.keys(Dialog)).toEqual([
      "Viewport",
      "Root",
      "Trigger",
      "Portal",
      "Backdrop",
      "Popup",
      "Title",
      "Description",
      "Close",
    ]);
    expect(Object.keys(AlertDialog)).toEqual(Object.keys(Dialog));
    expect(Object.keys(Slider)).toEqual([
      "Root",
      "Label",
      "Value",
      "Control",
      "Track",
      "Range",
      "Indicator",
      "Thumb",
    ]);
    expect(Object.keys(NumberField)).toEqual([
      "Root",
      "Group",
      "Input",
      "Increment",
      "Decrement",
      "ScrubArea",
      "ScrubAreaCursor",
    ]);
    expect(Object.keys(Progress)).toEqual(["Root", "Track", "Indicator", "Label", "Value"]);
    expect(Object.keys(Meter)).toEqual(["Root", "Track", "Indicator", "Label", "Value"]);
    expect(Object.keys(Toolbar)).toEqual([
      "Root",
      "Item",
      "Button",
      "Link",
      "Input",
      "Group",
      "Separator",
    ]);
    expect(Object.keys(Toast)).toEqual([
      "Provider",
      "Portal",
      "Viewport",
      "Positioner",
      "Root",
      "Content",
      "Title",
      "Description",
      "Action",
      "Close",
    ]);
    expect(Object.keys(solid.Tooltip)).toEqual([
      "Provider",
      "Root",
      "Trigger",
      "Portal",
      "Positioner",
      "Popup",
      "Arrow",
    ]);
  });

  test("declares controlled checkbox and switch toggle state ahead of the core click", () => {
    let checked: boolean | undefined;
    const checkbox = createComponent(Checkbox.Root, {
      checked: "indeterminate",
      onCheckedChange: (next: boolean) => {
        checked = next;
      },
      children: createComponent(Checkbox.Indicator, { children: "–" }),
    });

    expect(checkbox.properties.get(PropertyCode.Part)).toBe("checkbox");
    expect(checkbox.properties.get(PropertyCode.Checked)).toBe(false);
    expect(checkbox.properties.get(PropertyCode.Indeterminate)).toBe(true);
    expect(checkbox.properties.get(PropertyCode.ClickListener)).toBe(true);
    checkbox.listeners.get("click")!(new QuickGuiEvent("click", checkbox));
    expect(checked).toBe(true);

    const toggled = createComponent(Switch.Root, { defaultChecked: true });
    expect(toggled.properties.get(PropertyCode.Part)).toBe("switch");
    expect(toggled.properties.get(PropertyCode.Checked)).toBe(true);
    toggled.listeners.get("click")!(new QuickGuiEvent("click", toggled));
    expect(toggled.properties.get(PropertyCode.Checked)).toBe(false);

    const thumb = createComponent(Switch.Thumb, {});
    expect(thumb.properties.get(PropertyCode.Part)).toBe("switch-thumb");
  });

  test("bounds compound scopes and tooltip text before they cross N-API", () => {
    const hinted = createComponent(View, {
      tooltip: "Rename this workspace",
      tooltipPlacement: "top",
      tooltipDelay: 250,
      tooltipGap: 7,
      tooltipViewportMargin: 8,
    });

    expect(hinted.properties.get(PropertyCode.Tooltip)).toBe("Rename this workspace");
    expect(hinted.properties.get(PropertyCode.TooltipPlacement)).toBe("top");
    expect(hinted.properties.get(PropertyCode.TooltipDelay)).toBe(250);

    const overlong = createElement("view");
    expect(() => setProp(overlong, "tooltip", "x".repeat(MAX_TOOLTIP_TEXT_BYTES + 1))).toThrow(
      "tooltip text is limited",
    );
    expect(() => setProp(overlong, "scope", "s".repeat(MAX_COMPONENT_VALUE_BYTES + 1))).toThrow(
      "scopes and values are limited",
    );
  });

  test("shares one scope across controlled tab parts and reports activation", async () => {
    await app.whenReady();
    const [value, setValue] = createSignal("overview");
    let changed: string | undefined;
    const window = new Window({
      title: "Tabs",
      renderer: createRenderer(() =>
        createComponent(Tabs.Root, {
          get value() {
            return value();
          },
          onValueChange: (next: string) => {
            changed = next;
            setValue(next);
          },
          orientation: "vertical",
          activation: "automatic",
          loop: false,
          keepMounted: true,
          get children() {
            return [
              createComponent(Tabs.List, {
                get children() {
                  return [
                    createComponent(Tabs.Tab, {
                      value: "overview",
                      children: "Overview",
                    }),
                    createComponent(Tabs.Tab, {
                      value: "usage",
                      children: "Usage",
                    }),
                  ];
                },
              }),
              createComponent(Tabs.Panel, {
                value: "overview",
                children: "Overview panel",
              }),
              createComponent(Tabs.Panel, {
                value: "usage",
                children: "Usage panel",
              }),
            ];
          },
        }),
      ),
    });

    const root = window.root.children[0]!;
    const list = root.children[0]!;
    const first = list.children[0]!;
    const second = list.children[1]!;
    const usagePanel = root.children[2]!;
    const scope = String(root.properties.get(PropertyCode.Scope));

    expect(root.properties.get(PropertyCode.Part)).toBe("tabs");
    expect(scope.startsWith("qg-tabs-")).toBe(true);
    expect(list.properties.get(PropertyCode.Part)).toBe("tabs-list");
    expect(list.properties.get(PropertyCode.Scope)).toBe(scope);
    expect(list.properties.get(PropertyCode.Orientation)).toBe("vertical");
    expect(list.properties.get(PropertyCode.ActivateOnFocus)).toBe(true);
    expect(list.properties.get(PropertyCode.LoopFocus)).toBe(false);
    expect(first.properties.get(PropertyCode.Part)).toBe("tab");
    expect(first.properties.get(PropertyCode.PartValue)).toBe("overview");
    expect(first.properties.get(PropertyCode.ActiveValue)).toBe("overview");
    expect(usagePanel.properties.get(PropertyCode.Part)).toBe("tab-panel");
    expect(usagePanel.properties.get(PropertyCode.PartValue)).toBe("usage");
    expect(usagePanel.properties.get(PropertyCode.KeepMounted)).toBe(true);

    window._dispatchEvent("click", second.id);
    expect(changed).toBe("usage");
    expect(first.properties.get(PropertyCode.ActiveValue)).toBe("usage");
    expect(usagePanel.properties.get(PropertyCode.ActiveValue)).toBe("usage");
    window.close();
  });

  test("controls collapsible and accordion disclosure state through core part properties", async () => {
    await app.whenReady();
    const window = new Window({
      title: "Disclosures",
      renderer: createRenderer(() => [
        createComponent(Collapsible.Root, {
          defaultOpen: false,
          keepMounted: true,
          get children() {
            return [
              createComponent(Collapsible.Trigger, { children: "Details" }),
              createComponent(Collapsible.Panel, { children: "Body" }),
            ];
          },
        }),
        createComponent(Accordion.Root, {
          defaultValue: "first",
          headingLevel: 4,
          get children() {
            return createComponent(Accordion.Item, {
              value: "first",
              index: 0,
              get children() {
                return [
                  createComponent(Accordion.Header, {
                    get children() {
                      return createComponent(Accordion.Trigger, {
                        children: "First",
                      });
                    },
                  }),
                  createComponent(Accordion.Panel, { children: "Body" }),
                ];
              },
            });
          },
        }),
      ]),
    });

    const collapsible = window.root.children[0]!;
    const trigger = collapsible.children[0]!;
    const panel = collapsible.children[1]!;
    expect(collapsible.properties.get(PropertyCode.Part)).toBe("collapsible");
    expect(trigger.properties.get(PropertyCode.Part)).toBe("collapsible-trigger");
    expect(panel.properties.get(PropertyCode.Open)).toBe(false);
    expect(panel.properties.get(PropertyCode.KeepMounted)).toBe(true);
    window._dispatchEvent("click", trigger.id);
    expect(panel.properties.get(PropertyCode.Open)).toBe(true);

    const accordion = window.root.children[1]!;
    const item = accordion.children[0]!;
    const header = item.children[0]!;
    const accordionTrigger = header.children[0]!;
    const accordionPanel = item.children[1]!;
    expect(item.properties.get(PropertyCode.Part)).toBe("accordion-item");
    expect(item.properties.get(PropertyCode.PartValue)).toBe("first");
    expect(item.properties.get(PropertyCode.ItemIndex)).toBe(0);
    expect(header.properties.get(PropertyCode.HeadingLevel)).toBe(4);
    expect(accordionPanel.properties.get(PropertyCode.Open)).toBe(true);
    window._dispatchEvent("click", accordionTrigger.id);
    expect(accordionPanel.properties.get(PropertyCode.Open)).toBe(false);
    window.close();
  });

  test("projects field and fieldset relationships and inherited disabled state", async () => {
    await app.whenReady();
    const window = new Window({
      title: "Field",
      renderer: createRenderer(() =>
        createComponent(Fieldset.Root, {
          disabled: true,
          get children() {
            return [
              createComponent(Fieldset.Legend, { children: "Account" }),
              createComponent(Field.Root, {
                invalid: true,
                required: true,
                validationMessage: "Enter an address",
                get children() {
                  return [
                    createComponent(Field.Label, { children: "Email" }),
                    createComponent(Field.Label, {
                      passive: true,
                      children: "Email",
                    }),
                    createComponent(Field.Control, {
                      value: "",
                      placeholder: "you@example.com",
                    }),
                    createComponent(Field.Description, {
                      children: "We never share it",
                    }),
                    createComponent(Field.Error, {
                      children: "Enter an address",
                    }),
                  ];
                },
              }),
            ];
          },
        }),
      ),
    });

    const fieldset = window.root.children[0]!;
    const field = fieldset.children[1]!;
    const label = field.children[0]!;
    const passiveLabel = field.children[1]!;
    const control = field.children[2]!;
    const description = field.children[3]!;
    const error = field.children[4]!;

    expect(fieldset.properties.get(PropertyCode.Part)).toBe("fieldset");
    expect(field.properties.get(PropertyCode.Part)).toBe("field");
    expect(field.properties.get(PropertyCode.Disabled)).toBe(true);
    expect(label.properties.get(PropertyCode.Part)).toBe("field-label");
    expect(passiveLabel.properties.get(PropertyCode.Part)).toBe("field-passive-label");
    expect(control.tag).toBe(NativeNodeTag.Input);
    expect(control.properties.get(PropertyCode.Part)).toBe("field-control");
    expect(control.properties.get(PropertyCode.Required)).toBe(true);
    expect(control.properties.get(PropertyCode.Invalid)).toBe(true);
    expect(control.properties.get(PropertyCode.ValidationMessage)).toBe("Enter an address");
    expect(description.properties.get(PropertyCode.Part)).toBe("field-description");
    expect(error.properties.get(PropertyCode.Part)).toBe("field-error");
    expect(
      new Set(
        [field, label, control, description, error].map((node) =>
          node.properties.get(PropertyCode.Scope),
        ),
      ).size,
    ).toBe(1);
    window.close();
  });

  test("selects one radio through its group without a native round trip", async () => {
    await app.whenReady();
    const selected: string[] = [];
    const window = new Window({
      title: "Radios",
      renderer: createRenderer(() =>
        createComponent(RadioGroup.Root, {
          defaultValue: "light",
          onValueChange: (next: string) => selected.push(next),
          get children() {
            return [
              createComponent(Radio.Root, { value: "light", children: "Light" }),
              createComponent(Radio.Root, { value: "dark", children: "Dark" }),
            ];
          },
        }),
      ),
    });

    const group = window.root.children[0]!;
    const light = group.children[0]!;
    const dark = group.children[1]!;
    expect(group.properties.get(PropertyCode.Part)).toBe("radio-group");
    expect(light.properties.get(PropertyCode.Part)).toBe("radio");
    expect(light.properties.get(PropertyCode.Checked)).toBe(true);
    expect(dark.properties.get(PropertyCode.Checked)).toBe(false);

    window._dispatchEvent("click", dark.id);
    expect(selected).toEqual(["dark"]);
    expect(light.properties.get(PropertyCode.Checked)).toBe(false);
    expect(dark.properties.get(PropertyCode.Checked)).toBe(true);
    window.close();
  });

  test("declares in-window dialog dismissal policy ahead of every core decision", async () => {
    await app.whenReady();
    const changes: Array<{ open: boolean; reason: string }> = [];
    const window = new Window({
      title: "Dialogs",
      renderer: createRenderer(() =>
        createComponent(AlertDialog.Root, {
          defaultOpen: false,
          onOpenChange: (open: boolean, details: { reason: string }) =>
            changes.push({ open, reason: details.reason }),
          get children() {
            return [
              createComponent(AlertDialog.Trigger, { children: "Delete" }),
              createComponent(AlertDialog.Portal, {
                get children() {
                  return [
                    createComponent(AlertDialog.Backdrop, {}),
                    createComponent(AlertDialog.Popup, {
                      get children() {
                        return [
                          createComponent(AlertDialog.Title, {
                            children: "Delete project?",
                          }),
                          createComponent(AlertDialog.Description, {
                            children: "This cannot be undone.",
                          }),
                          createComponent(AlertDialog.Close, {
                            "aria-label": "Cancel",
                            children: "Cancel",
                          }),
                        ];
                      },
                    }),
                  ];
                },
              }),
            ];
          },
        }),
      ),
    });

    const trigger = window.root.children[0]!;
    const portal = window.root.children[1]!;
    const popup = portal.children[1]!;
    const close = popup.children[2]!;

    expect(trigger.properties.get(PropertyCode.Part)).toBe("dialog-trigger");
    expect(trigger.properties.get(PropertyCode.Variant)).toBe("alertdialog");
    expect(portal.properties.get(PropertyCode.Part)).toBe("dialog");
    expect(portal.properties.get(PropertyCode.Open)).toBe(false);
    expect(popup.properties.get(PropertyCode.Part)).toBe("dialog-popup");
    // An alert dialog keeps Escape and blocks backdrop dismissal by default.
    expect(popup.properties.get(PropertyCode.DismissOnEscape)).toBe(true);
    expect(popup.properties.get(PropertyCode.DismissOnPointerOutside)).toBe(false);
    expect(popup.properties.get(PropertyCode.DismissListener)).toBe(true);
    expect(close.properties.get(PropertyCode.AccessibilityLabel)).toBe("Cancel");

    window._dispatchEvent("click", trigger.id);
    expect(portal.properties.get(PropertyCode.Open)).toBe(true);
    window._dispatchEvent("dismiss", popup.id);
    expect(portal.properties.get(PropertyCode.Open)).toBe(false);
    window._dispatchEvent("click", trigger.id);
    window._dispatchEvent("click", close.id);
    expect(changes).toEqual([
      { open: true, reason: "trigger-press" },
      { open: false, reason: "dismiss" },
      { open: true, reason: "trigger-press" },
      { open: false, reason: "close-press" },
    ]);
    window.close();
  });

  test("declares a bounded popover-menu model and adopts core selection", async () => {
    await app.whenReady();
    const [open, setOpen] = createSignal(false);
    const selections: MenuSelectDetails[] = [];
    const items = [
      { type: "group" as const, label: "File" },
      { id: "open", label: "Open…", shortcut: "⌘O" },
      { type: "separator" as const },
      { type: "checkbox" as const, id: "sidebar", label: "Sidebar", checked: true },
      { id: "recent", label: "Recent", items: [{ id: "one", label: "One" }] },
    ];
    const window = new Window({
      title: "Popover menu",
      renderer: createRenderer(() =>
        createComponent(PopoverMenu.Root, {
          items,
          appearance: { width: 240, background: "#101014", highlightColor: "#ffffff" },
          get open() {
            return open();
          },
          onOpenChange: (next: boolean) => setOpen(next),
          onSelect: (details: MenuSelectDetails) => selections.push(details),
          placement: "bottom-end",
          gap: 6,
          get children() {
            return [
              createComponent(PopoverMenu.Trigger, { children: "Actions" }),
              createComponent(PopoverMenu.Popup, {}),
            ];
          },
        }),
      ),
    });

    const trigger = window.root.children[0]!;
    expect(trigger.properties.get(PropertyCode.Part)).toBe("popover-menu-trigger");
    expect(trigger.properties.get(PropertyCode.Open)).toBe(false);
    expect(window.root.children.length).toBe(1);

    window._dispatchEvent("click", trigger.id);
    expect(open()).toBe(true);

    const popup = window.root.children[1]!;
    expect(popup.properties.get(PropertyCode.Part)).toBe("popover-menu-popup");
    expect(popup.properties.get(PropertyCode.AnchorTarget)).toBe(String(trigger.id));
    expect(popup.properties.get(PropertyCode.AnchorPlacement)).toBe("bottom-end");
    expect(popup.properties.get(PropertyCode.AnchorGap)).toBe(6);
    expect(popup.properties.get(PropertyCode.SelectListener)).toBe(true);
    expect(popup.properties.get(PropertyCode.DismissListener)).toBe(true);
    // The trigger relates to the mounted surface through the validated controls property.
    expect(trigger.properties.get(PropertyCode.Controls)).toBe(String(popup.id));
    expect(trigger.properties.get(PropertyCode.Open)).toBe(true);

    const declaration = JSON.parse(String(popup.properties.get(PropertyCode.Menu)));
    expect(declaration.width).toBe(240);
    expect(declaration.background).toBeTypeOf("number");
    expect(declaration.items.length).toBe(5);
    expect(declaration.items[4].items[0].id).toBe("one");

    // The core decides what an activation means; JavaScript only receives the declared id.
    window._dispatchEvent(
      "menuselect",
      popup.id,
      JSON.stringify({ id: "sidebar", checked: false }),
    );
    await Promise.resolve();
    await Promise.resolve();
    expect(selections).toEqual([{ id: "sidebar", checked: false }]);
    expect(open()).toBe(false);
    expect(window.root.children.length).toBe(1);
    window.close();
  });

  test("declares a bounded context-menu model on its secondary-click target", async () => {
    await app.whenReady();
    const selections: MenuSelectDetails[] = [];
    const window = new Window({
      title: "Context menu",
      renderer: createRenderer(() =>
        createComponent(ContextMenu.Root, {
          items: [
            { id: "cut", label: "Cut", shortcut: "⌘X" },
            { type: "separator" as const },
            { type: "radio" as const, id: "list", group: "view", label: "List", checked: true },
          ],
          appearance: { itemHeight: 26, mutedColor: "#8a8a8a", loop: false },
          onSelect: (details: MenuSelectDetails) => selections.push(details),
          get children() {
            return createComponent(ContextMenu.Trigger, { children: "Canvas" });
          },
        }),
      ),
    });

    const target = window.root.children[0]!;
    expect(target.properties.get(PropertyCode.Part)).toBe("context-menu-trigger");
    expect(target.properties.get(PropertyCode.SelectListener)).toBe(true);
    const declaration = JSON.parse(String(target.properties.get(PropertyCode.Menu)));
    expect(declaration.itemHeight).toBe(26);
    expect(declaration.loopFocus).toBe(false);
    expect(declaration.mutedColor).toBeTypeOf("number");
    expect(declaration.items[2]).toEqual({
      type: "radio",
      id: "list",
      group: "view",
      label: "List",
      checked: true,
    });

    window._dispatchEvent("menuselect", target.id, JSON.stringify({ id: "cut" }));
    expect(selections).toEqual([{ id: "cut" }]);
    window.close();
  });

  test("bounds declared menu models before they cross N-API", () => {
    expect(() => encodeMenu([{ id: "big", label: "x".repeat(MAX_MENU_JSON_BYTES + 1) }])).toThrow(
      "bounded",
    );
    expect(JSON.parse(encodeMenu(undefined)).items).toEqual([]);
    expect(menuSelectionFromEvent(new QuickGuiEvent("menuselect", createElement("view")))).toBe(
      undefined,
    );
    expect(
      menuSelectionFromEvent(new QuickGuiEvent("menuselect", createElement("view"), "not json")),
    ).toBe(undefined);
    expect(Object.keys(PopoverMenu)).toEqual(["Root", "Trigger", "Popup"]);
    expect(Object.keys(ContextMenu)).toEqual(["Root", "Trigger"]);
  });

  test("projects CSS grid templates, flow, and item placement", () => {
    const grid = createComponent(View, {
      style: {
        display: "grid",
        gridTemplateColumns: ["200px", "1fr", "minmax(120px, 2fr)"],
        gridTemplateRows: 3,
        gridAutoFlow: "column dense",
      },
    });
    expect(grid.properties.get(PropertyCode.GridTemplateColumns)).toBe(
      "200px 1fr minmax(120px, 2fr)",
    );
    expect(grid.properties.get(PropertyCode.GridTemplateRows)).toBe(3);
    expect(grid.properties.get(PropertyCode.GridAutoFlow)).toBe("column dense");

    const spanned = createComponent(View, {
      style: { gridColumn: "2 / span 3", gridRow: "span 2" },
    });
    // `2 / span 3` is lines 2 through 5, which is the placement the core exposes.
    expect(spanned.properties.get(PropertyCode.GridColumnStart)).toBe(2);
    expect(spanned.properties.get(PropertyCode.GridColumnEnd)).toBe(5);
    expect(spanned.properties.get(PropertyCode.GridColumnSpan)).toBe(undefined);
    expect(spanned.properties.get(PropertyCode.GridRowSpan)).toBe(2);

    const lines = createComponent(View, { style: { gridColumn: "1 / 4" } });
    expect(lines.properties.get(PropertyCode.GridColumnStart)).toBe(1);
    expect(lines.properties.get(PropertyCode.GridColumnEnd)).toBe(4);
  });

  test("declares the complete paint transition the Rust core supports", () => {
    const shorthand = createComponent(View, {
      style: { transition: "opacity 180ms ease-out, box-shadow 180ms ease-out" },
    });
    expect(shorthand.properties.get(PropertyCode.Transition)).toBe(180);
    expect(shorthand.properties.get(PropertyCode.TransitionProperties)).toBe("opacity,box-shadow");
    expect(shorthand.properties.get(PropertyCode.TransitionEasing)).toBe("ease-out");

    const declared = createComponent(View, {
      style: {
        transition: {
          properties: ["bg", "color"],
          duration: "0.2s",
          easing: "ease",
          maxFps: 30,
        },
      },
    });
    expect(declared.properties.get(PropertyCode.TransitionDuration)).toBe(200);
    expect(declared.properties.get(PropertyCode.TransitionProperties)).toBe(
      "background-color,color",
    );
    // `ease` is the core's own ease-in-out curve.
    expect(declared.properties.get(PropertyCode.TransitionEasing)).toBe("ease-in-out");
    expect(declared.properties.get(PropertyCode.TransitionMaxFps)).toBe(30);

    const rejected = createElement("view");
    expect(() => setProp(rejected, "style", { transition: "left 100ms" })).toThrow(
      "cannot transition",
    );
    expect(() => setProp(rejected, "style", { transition: "opacity 100ms 40ms" })).toThrow("delay");
  });

  test("creates retained image and shader nodes with bounded declarations", () => {
    const image = createComponent(Image, {
      source: "/assets/logo.png",
      fit: "cover",
      style: { width: 64, height: 64 },
    });
    expect(image.tag).toBe(NativeNodeTag.Image);
    expect(image.properties.get(PropertyCode.Value)).toBe("/assets/logo.png");
    expect(image.properties.get(PropertyCode.ObjectFit)).toBe("cover");

    const shader = createComponent(Shader, {
      source:
        "fn quickgui_fragment(input: QuickGuiShaderInput) -> vec4<f32> { return vec4<f32>(1.0); }",
      shaderParameters: [
        [0.5, 0.25, 0, 1],
        [1, 0, 0, 1],
      ],
    });
    expect(shader.tag).toBe(NativeNodeTag.Shader);
    expect(JSON.parse(String(shader.properties.get(PropertyCode.ShaderParameters)))).toEqual([
      0.5, 0.25, 0, 1, 1, 0, 0, 1,
    ]);

    const bounded = createElement("shader");
    expect(() => setProp(bounded, "shaderParameters", new Array(20).fill(0))).toThrow(
      "16 shader parameter floats",
    );
  });

  test("declares progress, meter, and toggle state ahead of the core", () => {
    const progress = createComponent(Progress.Root, {
      value: 3,
      max: 12,
      valueText: "3 of 12 files",
      children: createComponent(Progress.Indicator, {}),
    });
    expect(progress.properties.get(PropertyCode.Part)).toBe("progress");
    expect(progress.properties.get(PropertyCode.Value)).toBe(3);
    expect(progress.properties.get(PropertyCode.Maximum)).toBe(12);
    expect(progress.properties.get(PropertyCode.ValueText)).toBe("3 of 12 files");

    const meter = createComponent(Meter.Root, {
      value: 20,
      min: 0,
      max: 100,
      low: 25,
      high: 75,
      optimum: 90,
    });
    expect(meter.properties.get(PropertyCode.Part)).toBe("meter");
    expect(meter.properties.get(PropertyCode.Low)).toBe(25);
    expect(meter.properties.get(PropertyCode.High)).toBe(75);
    expect(meter.properties.get(PropertyCode.Optimum)).toBe(90);

    let pressedChanges = 0;
    const toggle = createComponent(Toggle.Root, {
      defaultPressed: true,
      onPressedChange: () => {
        pressedChanges += 1;
      },
    });
    expect(toggle.properties.get(PropertyCode.Part)).toBe("toggle");
    expect(toggle.properties.get(PropertyCode.Pressed)).toBe(true);
    toggle.listeners.get("click")!(new QuickGuiEvent("click", toggle));
    expect(pressedChanges).toBe(1);
    expect(toggle.properties.get(PropertyCode.Pressed)).toBe(false);
    expect(Object.keys(Toggle)).toEqual(["Root", "Indicator"]);
  });

  test("declares every input listener ahead of the core decision", () => {
    const seen: string[] = [];
    const record = (name: string) => (event: QuickGuiEvent) => {
      seen.push(`${name}:${event.value ?? ""}`);
    };
    const node = createComponent(View, {
      tabIndex: 0,
      onKeyDown: record("keydown"),
      onKeyUp: record("keyup"),
      onMouseDown: record("mousedown"),
      onMouseUp: record("mouseup"),
      onMouseMove: record("mousemove"),
      onDoubleClick: record("dblclick"),
      onWheel: record("wheel"),
      onContextMenu: record("contextmenu"),
      onPinch: record("pinch"),
      onRotate: record("rotate"),
      onSmartMagnify: record("smartmagnify"),
      onPressure: record("pressure"),
      onFocus: record("focus"),
      onBlur: record("blur"),
    });

    for (const code of [
      PropertyCode.KeyDownListener,
      PropertyCode.KeyUpListener,
      PropertyCode.MouseDownListener,
      PropertyCode.MouseUpListener,
      PropertyCode.MouseMoveListener,
      PropertyCode.DoubleClickListener,
      PropertyCode.ScrollListener,
      PropertyCode.ContextMenuListener,
      PropertyCode.PinchListener,
      PropertyCode.RotationListener,
      PropertyCode.SmartMagnifyListener,
      PropertyCode.PressureListener,
      PropertyCode.FocusListener,
    ]) {
      expect(node.properties.get(code)).toBe(true);
    }

    const keyPayload = JSON.stringify({
      key: "s",
      repeat: false,
      shift: false,
      control: false,
      alt: false,
      meta: true,
    });
    node.listeners.get("keydown")!(new QuickGuiEvent("keydown", node, keyPayload));
    expect(keyEventFromEvent(new QuickGuiEvent("keydown", node, keyPayload))).toEqual({
      key: "s",
      repeat: false,
      shift: false,
      control: false,
      alt: false,
      meta: true,
    });
    expect(seen).toEqual([`keydown:${keyPayload}`]);

    const wheel = new QuickGuiEvent(
      "wheel",
      node,
      JSON.stringify({
        x: 4,
        y: 8,
        deltaX: 0,
        deltaY: -24,
        precise: true,
        phase: "moved",
        shift: false,
        control: false,
        alt: false,
        meta: false,
      }),
    );
    expect(wheelEventFromEvent(wheel)?.deltaY).toBe(-24);
    expect(wheelEventFromEvent(wheel)?.precise).toBe(true);
    expect(mouseEventFromEvent(new QuickGuiEvent("mousedown", node, "not json"))).toBe(undefined);
    expect(
      gestureEventFromEvent(new QuickGuiEvent("pressure", node, JSON.stringify({ stage: "force" })))
        ?.stage,
    ).toBe("force");
  });

  test("declares bounded accelerator keymaps and typed action ids", () => {
    let dispatched: string | undefined;
    const node = createComponent(View, {
      tabIndex: 0,
      keymap: { "CmdOrCtrl+S": "save", "CmdOrCtrl+Shift+P": "palette" },
      onAction: (event: QuickGuiEvent) => {
        dispatched = actionFromEvent(event);
      },
    });
    expect(node.properties.get(PropertyCode.ActionListener)).toBe(true);
    expect(JSON.parse(String(node.properties.get(PropertyCode.Keymap)))).toEqual({
      "CmdOrCtrl+S": "save",
      "CmdOrCtrl+Shift+P": "palette",
    });
    node.listeners.get("action")!(new QuickGuiEvent("action", node, "save"));
    expect(dispatched).toBe("save");

    const invalid = createElement("view");
    expect(() => setProp(invalid, "keymap", { "Cmd+S": 3 })).toThrow("binding ids must be strings");
    expect(() =>
      setProp(invalid, "keymap", { "Cmd+S": "x".repeat(MAX_KEYMAP_JSON_BYTES) }),
    ).toThrow("bounded");
  });

  test("declares drag payloads and accepted drop kinds ahead of the native drag", () => {
    const source = createComponent(View, {
      draggable: { id: "row-7", text: "quickgui", files: [{ path: "/tmp/a.txt" }] },
      onDragStart: () => {},
      onDragEnd: () => {},
    });
    expect(source.properties.get(PropertyCode.DragListener)).toBe(true);
    expect(JSON.parse(String(source.properties.get(PropertyCode.Draggable)))).toEqual({
      id: "row-7",
      text: "quickgui",
      files: [{ path: "/tmp/a.txt" }],
    });

    const target = createComponent(View, {
      dropKinds: ["local", "files"],
      onDrop: () => {},
      onFilesDropped: () => {},
    });
    expect(target.properties.get(PropertyCode.DropListener)).toBe(true);
    expect(JSON.parse(String(target.properties.get(PropertyCode.DropKinds)))).toEqual([
      "local",
      "files",
    ]);
    expect(
      dropEventFromEvent(
        new QuickGuiEvent(
          "filesdropped",
          target,
          JSON.stringify({ x: 0, y: 0, paths: ["/tmp/a.txt"], origin: "external" }),
        ),
      )?.paths,
    ).toEqual(["/tmp/a.txt"]);

    const invalid = createElement("view");
    expect(() => setProp(invalid, "dropKinds", ["text"])).toThrow("drop kinds");
    expect(() => setProp(invalid, "draggable", 7)).toThrow("declare its payload");
  });
});

describe("declared range, ordering, and roving-focus components", () => {
  test("declares slider bounds and adopts the value the core reports", async () => {
    await app.whenReady();
    let reported: readonly number[] | undefined;
    const window = new Window({
      title: "Slider",
      renderer: createRenderer(() => [
        createComponent(Slider.Root, {
          scope: "volume",
          defaultValue: [10, 60],
          min: 0,
          max: 100,
          step: 5,
          largeStep: 25,
          orientation: "horizontal",
          onValueChange: (values: readonly number[]) => {
            reported = values;
          },
          get children() {
            return [
              createComponent(Slider.Track, {
                scope: "volume",
                get children() {
                  return createComponent(Slider.Range, { scope: "volume" });
                },
              }),
              createComponent(Slider.Thumb, { scope: "volume", itemIndex: 0 }),
              createComponent(Slider.Thumb, { scope: "volume", itemIndex: 1 }),
            ];
          },
        }),
      ]),
    });

    const root = window.root.children[0]!;
    const track = root.children[0]!;
    const upper = root.children[2]!;
    expect(root.properties.get(PropertyCode.Part)).toBe("slider");
    expect(root.properties.get(PropertyCode.Scope)).toBe("volume");
    expect(root.properties.get(PropertyCode.Values)).toBe("[10,60]");
    expect(root.properties.get(PropertyCode.Minimum)).toBe(0);
    expect(root.properties.get(PropertyCode.Maximum)).toBe(100);
    expect(root.properties.get(PropertyCode.Step)).toBe(5);
    expect(root.properties.get(PropertyCode.LargeStep)).toBe(25);
    expect(root.properties.get(PropertyCode.ComponentChangeListener)).toBe(true);
    expect(track.properties.get(PropertyCode.Part)).toBe("slider-track");
    expect(track.children[0]!.properties.get(PropertyCode.Part)).toBe("slider-range");
    expect(upper.properties.get(PropertyCode.Part)).toBe("slider-thumb");
    expect(upper.properties.get(PropertyCode.ItemIndex)).toBe(1);

    // The core decides the snapped, ordered result; the renderer only adopts it.
    window._dispatchEvent("componentchange", root.id, JSON.stringify({ values: [10, 90] }));
    expect(reported).toEqual([10, 90]);
    expect(root.properties.get(PropertyCode.Values)).toBe("[10,90]");
    window.close();
  });

  test("declares splitter panes and toolbar and toggle-group navigation models", async () => {
    await app.whenReady();
    let sizes: readonly number[] | undefined;
    let active: string | undefined;
    let pressed: readonly string[] | undefined;
    const window = new Window({
      title: "Components",
      renderer: createRenderer(() => [
        createComponent(Splitter.Root, {
          scope: "panes",
          defaultValue: [200, 200],
          panes: [{ min: 80 }, { min: 80, collapsible: true }],
          step: 16,
          onSizesChange: (next: readonly number[]) => {
            sizes = next;
          },
          get children() {
            return [
              createComponent(Splitter.Pane, { scope: "panes", itemIndex: 0 }),
              createComponent(Splitter.Handle, { scope: "panes", itemIndex: 0 }),
              createComponent(Splitter.Pane, { scope: "panes", itemIndex: 1 }),
            ];
          },
        }),
        createComponent(Toolbar.Root, {
          scope: "actions",
          items: [{ value: "cut" }, { value: "copy", disabled: true }],
          defaultActive: "cut",
          loopFocus: true,
          onActiveChange: (next: string | undefined) => {
            active = next;
          },
          get children() {
            return [
              createComponent(Toolbar.Item, { scope: "actions", partValue: "cut" }),
              createComponent(Toolbar.Item, { scope: "actions", partValue: "copy" }),
            ];
          },
        }),
        createComponent(ToggleGroup.Root, {
          scope: "align",
          items: [{ value: "left" }, { value: "right" }],
          defaultValue: ["left"],
          variant: "multiple",
          onValueChange: (next: readonly string[]) => {
            pressed = next;
          },
          get children() {
            return [
              createComponent(ToggleGroup.Item, { scope: "align", partValue: "left" }),
              createComponent(ToggleGroup.Item, { scope: "align", partValue: "right" }),
            ];
          },
        }),
      ]),
    });

    const splitter = window.root.children[0]!;
    const toolbar = window.root.children[1]!;
    const group = window.root.children[2]!;
    expect(splitter.properties.get(PropertyCode.Part)).toBe("splitter");
    expect(splitter.properties.get(PropertyCode.Values)).toBe("[200,200]");
    expect(splitter.properties.get(PropertyCode.Items)).toBe(
      '[{"min":80},{"min":80,"collapsible":true}]',
    );
    expect(splitter.children[1]!.properties.get(PropertyCode.Part)).toBe("splitter-handle");
    expect(toolbar.properties.get(PropertyCode.Part)).toBe("toolbar");
    expect(toolbar.properties.get(PropertyCode.Items)).toBe(
      '[{"value":"cut"},{"value":"copy","disabled":true}]',
    );
    expect(toolbar.properties.get(PropertyCode.ActiveValue)).toBe("cut");
    expect(toolbar.properties.get(PropertyCode.LoopFocus)).toBe(true);
    expect(toolbar.children[0]!.properties.get(PropertyCode.PartValue)).toBe("cut");
    expect(group.properties.get(PropertyCode.Part)).toBe("toggle-group");
    expect(group.properties.get(PropertyCode.Variant)).toBe("multiple");
    expect(group.properties.get(PropertyCode.Values)).toBe('["left"]');

    window._dispatchEvent("componentchange", splitter.id, JSON.stringify({ sizes: [216, 184] }));
    window._dispatchEvent("componentchange", toolbar.id, JSON.stringify({ active: "copy" }));
    window._dispatchEvent(
      "componentchange",
      group.id,
      JSON.stringify({ pressed: ["left", "right"] }),
    );
    expect(sizes).toEqual([216, 184]);
    expect(active).toBe("copy");
    expect(pressed).toEqual(["left", "right"]);
    expect(splitter.properties.get(PropertyCode.Values)).toBe("[216,184]");
    expect(toolbar.properties.get(PropertyCode.ActiveValue)).toBe("copy");
    expect(group.properties.get(PropertyCode.Values)).toBe('["left","right"]');
    window.close();
  });

  test("keeps a controlled component declaration authoritative and bounds the payload", async () => {
    await app.whenReady();
    const [values, setValues] = createSignal<readonly number[]>([25]);
    let reported = 0;
    const window = new Window({
      title: "Controlled slider",
      renderer: createRenderer(() => [
        createComponent(Slider.Root, {
          scope: "controlled",
          get value() {
            return values();
          },
          onValueChange: () => {
            reported += 1;
          },
        }),
      ]),
    });

    const root = window.root.children[0]!;
    expect(root.properties.get(PropertyCode.Values)).toBe("[25]");
    // A reported value never overwrites a controlled declaration on its own.
    window._dispatchEvent("componentchange", root.id, JSON.stringify({ values: [30] }));
    expect(reported).toBe(1);
    expect(root.properties.get(PropertyCode.Values)).toBe("[25]");
    setValues([30]);
    await Promise.resolve();
    expect(root.properties.get(PropertyCode.Values)).toBe("[30]");

    // A malformed payload is ignored rather than throwing into the renderer.
    window._dispatchEvent("componentchange", root.id, "{not json");
    expect(reported).toBe(1);

    expect(() =>
      solid.componentChangeFromEvent(new QuickGuiEvent("componentchange", root, "{oops")),
    ).not.toThrow();
    expect(
      solid.componentChangeFromEvent(new QuickGuiEvent("componentchange", root, '{"values":[1]}')),
    ).toEqual({ values: [1] });
    window.close();
  });
});

describe("declared option sources, virtual collections, and stateful fields", () => {
  test("declares a bounded select source and adopts the value the core commits", async () => {
    await app.whenReady();
    let value: string | undefined;
    let open: boolean | undefined;
    let committed: solid.CommitDetails | undefined;
    const window = new Window({
      title: "Select",
      renderer: createRenderer(() => [
        createComponent(Select.Root, {
          scope: "theme",
          ariaLabel: "Theme",
          defaultValue: "light",
          filterMode: "fuzzy",
          items: [
            { value: "light", label: "Light" },
            { value: "dark", label: "Dark", detail: "⌘D" },
          ],
          appearance: { width: 240, rowHeight: 28, background: "#101014" },
          onValueChange: (next: string | undefined) => {
            value = next;
          },
          onOpenChange: (next: boolean) => {
            open = next;
          },
          onCommit: (details: solid.CommitDetails) => {
            committed = details;
          },
        }),
      ]),
    });

    const root = window.root.children[0]!;
    expect(root.tag).toBe(NativeNodeTag.Button);
    expect(root.properties.get(PropertyCode.Part)).toBe("select");
    expect(root.properties.get(PropertyCode.Scope)).toBe("theme");
    expect(root.properties.get(PropertyCode.ActiveValue)).toBe("light");
    expect(root.properties.get(PropertyCode.FilterMode)).toBe("fuzzy");
    expect(root.properties.get(PropertyCode.Options)).toBe(
      JSON.stringify([
        { value: "light", label: "Light" },
        { value: "dark", label: "Dark", detail: "⌘D" },
      ]),
    );
    // Declared colors travel packed, exactly like every other declared paint value.
    expect(root.properties.get(PropertyCode.Appearance)).toBe(
      JSON.stringify({
        width: 240,
        rowHeight: 28,
        background: parseColor("#101014"),
      }),
    );
    expect(root.properties.get(PropertyCode.ComponentChangeListener)).toBe(true);
    expect(root.properties.get(PropertyCode.CommitListener)).toBe(true);

    window._dispatchEvent(
      "componentchange",
      root.id,
      JSON.stringify({ value: "dark", open: false }),
    );
    expect(value).toBe("dark");
    expect(open).toBe(false);
    expect(root.properties.get(PropertyCode.ActiveValue)).toBe("dark");

    window._dispatchEvent("commit", root.id, JSON.stringify({ value: "dark" }));
    expect(committed).toEqual({ value: "dark" });
    window.close();
  });

  test("declares combobox and autocomplete inputs with child option nodes", async () => {
    await app.whenReady();
    let inputValue: string | undefined;
    const window = new Window({
      title: "Pickers",
      renderer: createRenderer(() => [
        createComponent(Combobox.Root, {
          scope: "fruit",
          inputValue: "ap",
          placeholder: "Fruit",
          get children() {
            return [
              createComponent(Combobox.Option, {
                partValue: "apple",
                label: "Apple",
                group: "recent",
              }),
              createComponent(Combobox.Option, {
                partValue: "banana",
                label: "Banana",
                disabled: true,
              }),
            ];
          },
        }),
        createComponent(Autocomplete.Root, {
          scope: "search",
          inputValue: "al",
          filterMode: "none",
          items: [{ value: "alpha" }],
          onInputValueChange: (next: string) => {
            inputValue = next;
          },
        }),
      ]),
    });

    // A text input paints its own content, so the compound's other declarations are siblings of
    // it; the Rust binding gathers every declared option from the whole compound in order.
    const combobox = window.root.children[0]!;
    const apple = window.root.children[1]!;
    const banana = window.root.children[2]!;
    const autocomplete = window.root.children[3]!;
    expect(combobox.tag).toBe(NativeNodeTag.Input);
    expect(combobox.properties.get(PropertyCode.Part)).toBe("combobox");
    expect(combobox.properties.get(PropertyCode.InputValue)).toBe("ap");
    expect(combobox.properties.get(PropertyCode.Options)).toBeUndefined();
    expect(combobox.properties.get(PropertyCode.Scope)).toBe("fruit");
    expect(apple.properties.get(PropertyCode.Part)).toBe("option");
    expect(apple.properties.get(PropertyCode.Scope)).toBe("fruit");
    expect(apple.properties.get(PropertyCode.PartValue)).toBe("apple");
    expect(apple.properties.get(PropertyCode.Value)).toBe("Apple");
    expect(apple.properties.get(PropertyCode.Group)).toBe("recent");
    expect(banana.properties.get(PropertyCode.Disabled)).toBe(true);
    expect(autocomplete.properties.get(PropertyCode.Part)).toBe("autocomplete");
    expect(autocomplete.properties.get(PropertyCode.FilterMode)).toBe("none");

    window._dispatchEvent(
      "componentchange",
      autocomplete.id,
      JSON.stringify({ inputValue: "alp", open: true }),
    );
    expect(inputValue).toBe("alp");
    window.close();
  });

  test("declares a virtual table and adopts everything the core decides", async () => {
    await app.whenReady();
    let range: solid.VisibleRange | undefined;
    let selection: readonly (readonly number[])[] | undefined;
    let sort: solid.TableSortState | undefined;
    let widths: Readonly<Record<string, number>> | undefined;
    let order: readonly string[] | undefined;
    let edit: solid.TableEditEndDetails | undefined;
    let activated: solid.TableCell | undefined;
    const window = new Window({
      title: "Table",
      renderer: createRenderer(() => [
        createComponent(Table.Root, {
          scope: "files",
          rowCount: 1000,
          rowHeight: 24,
          headerHeight: 32,
          selectionMode: "multiple",
          selection: [[0, 2]],
          sort: { column: "name", direction: "descending" },
          editing: { row: 1, column: 0 },
          columns: [
            { id: "name", label: "Name", width: 160, sortable: true },
            { id: "size", label: "Size", track: "1fr", align: "end" },
          ],
          onVisibleRangeChange: (next: solid.VisibleRange) => {
            range = next;
          },
          onSelectionChange: (next: readonly (readonly number[])[]) => {
            selection = next;
          },
          onSortChange: (next: solid.TableSortState | undefined) => {
            sort = next;
          },
          onColumnResize: (next: Readonly<Record<string, number>>) => {
            widths = next;
          },
          onColumnReorder: (next: readonly string[]) => {
            order = next;
          },
          onEditEnd: (next: solid.TableEditEndDetails) => {
            edit = next;
          },
          onActivate: (cell: solid.TableCell) => {
            activated = cell;
          },
          get children() {
            return [
              createComponent(Table.Header, { column: "name" }),
              createComponent(Table.Row, {
                index: 0,
                get children() {
                  return createComponent(Table.Cell, { column: "name" });
                },
              }),
            ];
          },
        }),
      ]),
    });

    const root = window.root.children[0]!;
    expect(root.properties.get(PropertyCode.Part)).toBe("table");
    expect(root.properties.get(PropertyCode.RowCount)).toBe(1000);
    expect(root.properties.get(PropertyCode.RowHeight)).toBe(24);
    expect(root.properties.get(PropertyCode.HeaderHeight)).toBe(32);
    expect(root.properties.get(PropertyCode.SelectionMode)).toBe("multiple");
    expect(root.properties.get(PropertyCode.Selection)).toBe("[[0,2]]");
    expect(root.properties.get(PropertyCode.SortColumn)).toBe("name");
    expect(root.properties.get(PropertyCode.SortDirection)).toBe("descending");
    expect(root.properties.get(PropertyCode.Editing)).toBe('{"row":1,"column":0}');
    expect(root.properties.get(PropertyCode.Columns)).toBe(
      JSON.stringify([
        { id: "name", label: "Name", width: 160, sortable: true },
        { id: "size", label: "Size", track: "1fr", align: "end" },
      ]),
    );
    expect(root.children[0]!.properties.get(PropertyCode.Part)).toBe("table-header");
    expect(root.children[0]!.properties.get(PropertyCode.PartValue)).toBe("name");
    expect(root.children[1]!.properties.get(PropertyCode.Part)).toBe("table-row");
    expect(root.children[1]!.properties.get(PropertyCode.RowIndex)).toBe(0);
    expect(root.children[1]!.children[0]!.properties.get(PropertyCode.Part)).toBe("table-cell");

    window._dispatchEvent(
      "componentchange",
      root.id,
      JSON.stringify({
        visibleRange: { start: 12, end: 40 },
        selectedRanges: [[3, 5]],
        sort: { column: "size", direction: "ascending" },
        columnWidths: { name: 168 },
        columnOrder: ["size", "name"],
        editEnded: { row: 1, column: 0, committed: true },
      }),
    );
    expect(range).toEqual({ start: 12, end: 40 });
    expect(selection).toEqual([[3, 5]]);
    expect(sort).toEqual({ column: "size", direction: "ascending" });
    expect(widths).toEqual({ name: 168 });
    expect(order).toEqual(["size", "name"]);
    expect(edit).toEqual({ row: 1, column: 0, committed: true });

    window._dispatchEvent("commit", root.id, JSON.stringify({ row: 4, column: 1 }));
    expect(activated).toEqual({ row: 4, column: 1 });
    window.close();
  });

  test("declares a lazy virtual tree and adopts the core's expansion and requests", async () => {
    await app.whenReady();
    let expanded: readonly string[] | undefined;
    let requested: string | undefined;
    let activated: string | undefined;
    const window = new Window({
      title: "Tree",
      renderer: createRenderer(() => [
        createComponent(Tree.Root, {
          scope: "explorer",
          rowHeight: 22,
          loadingLabel: "Fetching…",
          disclosure: "trailing",
          nodes: [
            { id: "src", label: "src", pending: true },
            { id: "readme", label: "README" },
          ],
          defaultExpanded: [],
          value: "readme",
          setChildren: { id: "src", children: [{ id: "main", label: "main.rs" }] },
          onExpandedChange: (next: readonly string[]) => {
            expanded = next;
          },
          onLoadChildren: (id: string) => {
            requested = id;
          },
          onActivate: (id: string) => {
            activated = id;
          },
          get children() {
            return createComponent(Tree.Row, { nodeId: "src" });
          },
        }),
      ]),
    });

    const root = window.root.children[0]!;
    expect(root.properties.get(PropertyCode.Part)).toBe("tree");
    expect(root.properties.get(PropertyCode.RowHeight)).toBe(22);
    expect(root.properties.get(PropertyCode.LoadingLabel)).toBe("Fetching…");
    expect(root.properties.get(PropertyCode.Disclosure)).toBe("trailing");
    expect(root.properties.get(PropertyCode.SelectedValue)).toBe("readme");
    expect(root.properties.get(PropertyCode.Expanded)).toBe("[]");
    expect(root.properties.get(PropertyCode.SetChildren)).toBe(
      '{"id":"src","children":[{"id":"main","label":"main.rs"}]}',
    );
    expect(root.children[0]!.properties.get(PropertyCode.Part)).toBe("tree-row");
    expect(root.children[0]!.properties.get(PropertyCode.PartValue)).toBe("src");

    window._dispatchEvent(
      "componentchange",
      root.id,
      JSON.stringify({ expanded: ["src"], loadChildren: "src" }),
    );
    expect(expanded).toEqual(["src"]);
    expect(requested).toBe("src");
    expect(root.properties.get(PropertyCode.Expanded)).toBe('["src"]');

    window._dispatchEvent("commit", root.id, JSON.stringify({ value: "readme" }));
    expect(activated).toBe("readme");
    window.close();
  });

  test("declares number, date, time, calendar, menubar, and toast declarations", async () => {
    await app.whenReady();
    let numeric: number | undefined;
    let valid = true;
    let day: string | undefined;
    let dismissed: readonly string[] | undefined;
    let openMenu: number | undefined;
    const window = new Window({
      title: "Fields",
      renderer: createRenderer(() => [
        createComponent(NumberField.Root, {
          scope: "quantity",
          defaultValue: 2,
          min: 0,
          max: 10,
          step: 2,
          precision: 1,
          onValueChange: (next: number | undefined, isValid: boolean) => {
            numeric = next;
            valid = isValid;
          },
          get children() {
            return [
              createComponent(NumberField.Input, { scope: "quantity" }),
              createComponent(NumberField.Increment, { scope: "quantity" }),
              createComponent(NumberField.Decrement, { scope: "quantity" }),
            ];
          },
        }),
        createComponent(DateField.Root, {
          scope: "due",
          defaultValue: "2026-09-03",
          min: "2000-01-01",
          format: "mdy",
          get children() {
            return createComponent(DateField.Segment, {
              scope: "due",
              segment: "year",
            });
          },
        }),
        createComponent(TimeField.Root, {
          scope: "alarm",
          defaultValue: "07:30",
          hour12: true,
          showSeconds: true,
          get children() {
            return createComponent(TimeField.Segment, {
              scope: "alarm",
              segment: "period",
            });
          },
        }),
        createComponent(Calendar.Root, {
          scope: "month",
          defaultValue: "2026-09-03",
          firstWeekday: 0,
          onFocusChange: (next: string) => {
            day = next;
          },
          get children() {
            return createComponent(Calendar.Week, {
              scope: "month",
              itemIndex: 0,
              get children() {
                return createComponent(Calendar.Day, {
                  scope: "month",
                  day: "2026-09-03",
                });
              },
            });
          },
        }),
        createComponent(Menubar.Root, {
          scope: "bar",
          count: 2,
          onOpenChange: (next: number | undefined) => {
            openMenu = next;
          },
          get children() {
            return createComponent(Menubar.Item, { scope: "bar", itemIndex: 0 });
          },
        }),
        createComponent(Toast.Viewport, {
          scope: "toasts",
          toasts: [{ id: "saved", title: "Saved", kind: "success", duration: 4000 }],
          onDismiss: (ids: readonly string[]) => {
            dismissed = ids;
          },
          get children() {
            return createComponent(Toast.Root, {
              scope: "toasts",
              toastId: "saved",
              get children() {
                return [
                  createComponent(Toast.Title, { scope: "toasts", toastId: "saved" }),
                  createComponent(Toast.Close, { scope: "toasts", toastId: "saved" }),
                ];
              },
            });
          },
        }),
      ]),
    });

    const [field, date, time, calendar, menubar, toasts] = window.root.children;
    expect(field!.properties.get(PropertyCode.Part)).toBe("number-field");
    expect(field!.properties.get(PropertyCode.Values)).toBe("[2]");
    expect(field!.properties.get(PropertyCode.Precision)).toBe(1);
    expect(field!.children[0]!.properties.get(PropertyCode.Part)).toBe("number-field-input");
    expect(field!.children[1]!.properties.get(PropertyCode.Part)).toBe("number-field-increment");

    expect(date!.properties.get(PropertyCode.CivilValue)).toBe("2026-09-03");
    expect(date!.properties.get(PropertyCode.CivilMinimum)).toBe("2000-01-01");
    expect(date!.properties.get(PropertyCode.SegmentOrder)).toBe("mdy");
    expect(date!.children[0]!.properties.get(PropertyCode.Segment)).toBe("year");

    expect(time!.properties.get(PropertyCode.CivilValue)).toBe("07:30");
    expect(time!.properties.get(PropertyCode.SegmentOrder)).toBe("h12");
    expect(time!.properties.get(PropertyCode.Variant)).toBe("seconds");
    expect(time!.children[0]!.properties.get(PropertyCode.Segment)).toBe("period");

    expect(calendar!.properties.get(PropertyCode.CivilValue)).toBe("2026-09-03");
    expect(calendar!.properties.get(PropertyCode.FirstWeekday)).toBe(0);
    expect(calendar!.children[0]!.properties.get(PropertyCode.Part)).toBe("calendar-week");
    expect(calendar!.children[0]!.children[0]!.properties.get(PropertyCode.CivilValue)).toBe(
      "2026-09-03",
    );

    expect(menubar!.properties.get(PropertyCode.MenuCount)).toBe(2);
    expect(menubar!.properties.get(PropertyCode.Open)).toBe(false);
    expect(menubar!.children[0]!.properties.get(PropertyCode.Part)).toBe("menubar-item");

    expect(toasts!.properties.get(PropertyCode.Toasts)).toBe(
      JSON.stringify([{ id: "saved", title: "Saved", kind: "success", duration: 4000 }]),
    );
    expect(toasts!.children[0]!.properties.get(PropertyCode.PartValue)).toBe("saved");
    expect(toasts!.children[0]!.children[1]!.properties.get(PropertyCode.Part)).toBe("toast-close");

    window._dispatchEvent(
      "componentchange",
      field!.id,
      JSON.stringify({ value: 42, text: "42", valid: false }),
    );
    expect(numeric).toBe(42);
    expect(valid).toBe(false);

    window._dispatchEvent(
      "componentchange",
      calendar!.id,
      JSON.stringify({ focused: "2026-09-04", month: "2026-09" }),
    );
    expect(day).toBe("2026-09-04");

    window._dispatchEvent("componentchange", menubar!.id, JSON.stringify({ open: 1, focused: 1 }));
    expect(openMenu).toBe(1);
    expect(menubar!.properties.get(PropertyCode.ItemIndex)).toBe(1);
    expect(menubar!.properties.get(PropertyCode.Open)).toBe(true);

    window._dispatchEvent("componentchange", toasts!.id, JSON.stringify({ dismissed: ["saved"] }));
    expect(dismissed).toEqual(["saved"]);
    window.close();
  });

  test("refuses an option or collection declaration past the boundary's own bounds", async () => {
    await app.whenReady();
    const window = new Window({ title: "Bounds", renderer: createRenderer(() => []) });
    const node = createElement("view");
    insertNode(window.root, node);
    expect(() =>
      setProp(node, "options", [{ value: "a", label: "x".repeat(MAX_OPTIONS_JSON_BYTES) }]),
    ).toThrow(/option source/);
    expect(() =>
      setProp(node, "columns", [{ id: "a", label: "x".repeat(MAX_COLLECTION_JSON_BYTES) }]),
    ).toThrow(/collection/);
    window.close();
  });

  test("projects extended text styling onto the core's own typography declarations", () => {
    const label = createComponent(Text, {
      style: {
        textAlign: "center",
        letterSpacing: "0.4px",
        wordSpacing: 2,
        textTransform: "uppercase",
        textShadow: "0 2px 4px #00000080",
        textDecoration: "underline line-through",
        textDecorationColor: "#2266dd",
        textDecorationStyle: "wavy",
        textDecorationThickness: "2px",
        wordBreak: "break-all",
        overflowWrap: "anywhere",
        hyphens: "manual",
        textDirection: "rtl",
      },
      children: "Extended text",
    });

    expect(label.properties.get(PropertyCode.TextAlign)).toBe("center");
    expect(label.properties.get(PropertyCode.LetterSpacing)).toBe(0.4);
    expect(label.properties.get(PropertyCode.WordSpacing)).toBe(2);
    expect(label.properties.get(PropertyCode.TextTransform)).toBe("uppercase");
    expect(label.properties.get(PropertyCode.TextShadow)).toBe("0 2px 4px #00000080");
    expect(label.properties.get(PropertyCode.TextDecorationLine)).toBe("underline line-through");
    expect(label.properties.get(PropertyCode.TextDecorationColor)).toBe(parseColor("#2266dd"));
    expect(label.properties.get(PropertyCode.TextDecorationStyle)).toBe("wavy");
    expect(label.properties.get(PropertyCode.TextDecorationThickness)).toBe(2);
    expect(label.properties.get(PropertyCode.WordBreak)).toBe("break-all");
    expect(label.properties.get(PropertyCode.OverflowWrap)).toBe("anywhere");
    expect(label.properties.get(PropertyCode.Hyphens)).toBe("manual");
    expect(label.properties.get(PropertyCode.TextDirection)).toBe("rtl");

    // `start` and `end` stay logical so an RTL subtree can resolve them in the core.
    const logical = createComponent(Text, {
      style: { textAlign: "end" },
      children: "logical",
    });
    expect(logical.properties.get(PropertyCode.TextAlign)).toBe("end");

    // A text shadow may also be declared as the object form the Rust binding deserializes.
    const declared = createComponent(Text, {
      style: { textShadow: { offsetX: 0, offsetY: 1, blur: 3 } },
      children: "object",
    });
    expect(JSON.parse(String(declared.properties.get(PropertyCode.TextShadow)))).toEqual({
      offsetX: 0,
      offsetY: 1,
      blur: 3,
    });
  });

  test("projects direction-relative layout and logical spacing", () => {
    const panel = createComponent(View, {
      style: {
        direction: "rtl",
        paddingStart: 12,
        paddingEnd: "4px",
        marginStart: 6,
        marginEnd: 2,
        borderStartWidth: "3px",
        borderEndWidth: 1,
      },
    });

    expect(panel.properties.get(PropertyCode.Direction)).toBe("rtl");
    expect(panel.properties.get(PropertyCode.PaddingStart)).toBe(12);
    expect(panel.properties.get(PropertyCode.PaddingEnd)).toBe(4);
    expect(panel.properties.get(PropertyCode.MarginStart)).toBe(6);
    expect(panel.properties.get(PropertyCode.MarginEnd)).toBe(2);
    expect(panel.properties.get(PropertyCode.BorderStartWidth)).toBe(3);
    expect(panel.properties.get(PropertyCode.BorderEndWidth)).toBe(1);
  });

  test("routes one background declaration to either a color or a bounded gradient", () => {
    const solid = createComponent(View, { style: { bg: "#101828" } });
    expect(solid.properties.get(PropertyCode.BackgroundColor)).toBe(parseColor("#101828"));
    expect(solid.properties.has(PropertyCode.BackgroundGradient)).toBe(false);

    const gradient = createComponent(View, {
      style: { bg: "linear-gradient(135deg, #0f172a, #38bdf8)" },
    });
    expect(gradient.properties.get(PropertyCode.BackgroundGradient)).toBe(
      "linear-gradient(135deg, #0f172a, #38bdf8)",
    );
    expect(gradient.properties.has(PropertyCode.BackgroundColor)).toBe(false);

    const declared = createComponent(View, {
      style: {
        bgGradient: {
          type: "radial",
          shape: "circle",
          center: { x: 0.3, y: 0.2 },
          stops: [{ color: "#1d4ed8", position: 0 }, "#0f172a"],
        },
      },
    });
    expect(JSON.parse(String(declared.properties.get(PropertyCode.BackgroundGradient)))).toEqual({
      type: "radial",
      shape: "circle",
      center: { x: 0.3, y: 0.2 },
      stops: [{ color: "#1d4ed8", position: 0 }, "#0f172a"],
    });

    // Every state that the core's `ElementStateStyle` can swap a gradient in follows the same rule.
    const stateful = createComponent(Button, {
      style: {
        hoverBg: "linear-gradient(90deg, #111827, #334155)",
        activeBg: "#0b1220",
        focusBg: "conic-gradient(from 90deg, #1d4ed8, #0f172a)",
      },
      children: "Run",
    });
    expect(stateful.properties.get(PropertyCode.HoverBackgroundGradient)).toBe(
      "linear-gradient(90deg, #111827, #334155)",
    );
    expect(stateful.properties.get(PropertyCode.ActiveBackgroundColor)).toBe(parseColor("#0b1220"));
    expect(stateful.properties.get(PropertyCode.FocusBackgroundGradient)).toBe(
      "conic-gradient(from 90deg, #1d4ed8, #0f172a)",
    );
  });

  test("projects per-corner radii, border and outline rings, filters, transforms, and blending", () => {
    const card = createComponent(View, {
      style: {
        borderRadius: "16px 4px 16px 4px",
        borderTopLeftRadius: 20,
        borderStyle: "dashed",
        outline: "2px dotted #38bdf8",
        outlineOffset: 3,
        bgImage: "/assets/paper.png",
        bgSize: "cover",
        bgRepeat: "no-repeat",
        bgPosition: "center",
        filter: ["saturate(1.4)", "blur(2px)"],
        backdropFilter: "blur(18px) brightness(1.1)",
        transform: "rotate(3deg) scale(1.02)",
        transformOrigin: "left top",
        mixBlendMode: "multiply",
      },
    });

    expect(card.properties.get(PropertyCode.BorderRadius)).toBe("16px 4px 16px 4px");
    expect(card.properties.get(PropertyCode.BorderTopLeftRadius)).toBe(20);
    expect(card.properties.get(PropertyCode.BorderStyle)).toBe("dashed");
    expect(card.properties.get(PropertyCode.OutlineWidth)).toBe(2);
    expect(card.properties.get(PropertyCode.OutlineStyle)).toBe("dotted");
    expect(card.properties.get(PropertyCode.OutlineColor)).toBe(parseColor("#38bdf8"));
    expect(card.properties.get(PropertyCode.OutlineOffset)).toBe(3);
    expect(card.properties.get(PropertyCode.BackgroundImage)).toBe("/assets/paper.png");
    expect(card.properties.get(PropertyCode.BackgroundSize)).toBe("cover");
    expect(card.properties.get(PropertyCode.BackgroundRepeat)).toBe("no-repeat");
    expect(card.properties.get(PropertyCode.BackgroundPosition)).toBe("center");
    expect(card.properties.get(PropertyCode.Filter)).toBe("saturate(1.4) blur(2px)");
    expect(card.properties.get(PropertyCode.BackdropFilter)).toBe("blur(18px) brightness(1.1)");
    expect(card.properties.get(PropertyCode.Transform)).toBe("rotate(3deg) scale(1.02)");
    expect(card.properties.get(PropertyCode.TransformOrigin)).toBe("left top");
    expect(card.properties.get(PropertyCode.MixBlendMode)).toBe("multiply");

    const uniform = createComponent(View, { style: { borderRadius: "12px" } });
    expect(uniform.properties.get(PropertyCode.BorderRadius)).toBe(12);

    const cleared = createComponent(View, { style: { outline: "none" } });
    expect(cleared.properties.get(PropertyCode.OutlineStyle)).toBe("none");
  });

  test("projects hover, active, and focus state styling the core supports", () => {
    const tile = createComponent(Button, {
      style: {
        hoverTransform: "translate(0, -2px)",
        hoverOutline: "2px solid #f8fafc",
        activeTransform: { a: 0.98, b: 0, c: 0, d: 0.98, tx: 0, ty: 0 },
        activeOutline: "1px solid #94a3b8",
        focusBg: "#1f2937",
        focusColor: "#f8fafc",
        focusOutline: "2px solid #60a5fa",
        focusTransform: "scale(1.01)",
      },
      children: "Apply",
    });

    expect(tile.properties.get(PropertyCode.HoverTransform)).toBe("translate(0, -2px)");
    expect(tile.properties.get(PropertyCode.HoverOutline)).toBe("2px solid #f8fafc");
    expect(JSON.parse(String(tile.properties.get(PropertyCode.ActiveTransform)))).toEqual({
      a: 0.98,
      b: 0,
      c: 0,
      d: 0.98,
      tx: 0,
      ty: 0,
    });
    expect(tile.properties.get(PropertyCode.ActiveOutline)).toBe("1px solid #94a3b8");
    expect(tile.properties.get(PropertyCode.FocusBackgroundColor)).toBe(parseColor("#1f2937"));
    expect(tile.properties.get(PropertyCode.FocusColor)).toBe(parseColor("#f8fafc"));
    expect(tile.properties.get(PropertyCode.FocusOutline)).toBe("2px solid #60a5fa");
    expect(tile.properties.get(PropertyCode.FocusTransform)).toBe("scale(1.01)");
  });

  test("projects nested interaction states as one bounded declaration per state", () => {
    const decode = (node: { properties: Map<PropertyCode, unknown> }, code: PropertyCode) =>
      JSON.parse(String(node.properties.get(code)));
    const tile = createComponent(Button, {
      style: {
        bg: "#ffffff",
        hover: {
          bgGradient: "linear-gradient(90deg, #1d4ed8, #38bdf8)",
          color: "#f8fafc",
          borderColor: "#93c5fd",
          borderWidth: "2px",
          borderRadius: 12,
          outline: "2px dashed #93c5fd",
          boxShadow: "0 4px 12px -2px #0000003d",
          opacity: 0.9,
          cursor: "pointer",
          transform: "scale(1.03) translate(0, -2px)",
          transformOrigin: "left top",
        },
        active: {
          bg: "#0b1220",
          transform: { a: 0.98, b: 0, c: 0, d: 0.98, tx: 0, ty: 0 },
        },
        focus: { outline: "2px solid #60a5fa" },
        disabled: { opacity: 0.5, cursor: "not-allowed" },
        invalid: { borderColor: "#ef4444" },
        dragging: { opacity: 0.6 },
        dragOver: { bg: "#eff6ff", outline: "none", boxShadow: "none" },
        groupHover: { group: "sidebar", opacity: 1 },
        groupActive: [{ opacity: 0.8 }, { group: "list", transform: "scale(0.98)" }],
        focusWithin: { outline: "1px solid #93c5fd" },
        selected: { bg: "#1d4ed8", color: "#ffffff" },
      },
      children: "Apply",
    });

    expect(decode(tile, PropertyCode.HoverStyle)).toEqual({
      background: "linear-gradient(90deg, #1d4ed8, #38bdf8)",
      color: parseColor("#f8fafc"),
      borderColor: parseColor("#93c5fd"),
      borderWidth: 2,
      borderRadius: 12,
      outline: "2px dashed #93c5fd",
      boxShadow: [
        {
          offsetX: 0,
          offsetY: 4,
          blurRadius: 12,
          spreadRadius: -2,
          color: parseColor("#0000003d"),
          inset: false,
        },
      ],
      opacity: 0.9,
      cursor: "pointer",
      transform: "scale(1.03) translate(0, -2px)",
      transformOrigin: "left top",
    });
    expect(decode(tile, PropertyCode.ActiveStyle)).toEqual({
      backgroundColor: parseColor("#0b1220"),
      transform: JSON.stringify({ a: 0.98, b: 0, c: 0, d: 0.98, tx: 0, ty: 0 }),
    });
    expect(decode(tile, PropertyCode.FocusStyle)).toEqual({
      outline: "2px solid #60a5fa",
    });
    expect(decode(tile, PropertyCode.DisabledStyle)).toEqual({
      opacity: 0.5,
      cursor: "not-allowed",
    });
    expect(decode(tile, PropertyCode.InvalidStyle)).toEqual({
      borderColor: parseColor("#ef4444"),
    });
    expect(decode(tile, PropertyCode.DraggingStyle)).toEqual({ opacity: 0.6 });
    // `none` travels as an explicit removal, not as an absent declaration.
    expect(decode(tile, PropertyCode.DragOverStyle)).toEqual({
      backgroundColor: parseColor("#eff6ff"),
      outline: "none",
      boxShadow: [],
    });
    expect(decode(tile, PropertyCode.GroupHoverStyle)).toEqual({
      group: "sidebar",
      opacity: 1,
    });
    // A group state with several entries travels as a list, one entry per group it follows.
    expect(decode(tile, PropertyCode.GroupActiveStyle)).toEqual([
      { opacity: 0.8 },
      { group: "list", transform: "scale(0.98)" },
    ]);
    expect(decode(tile, PropertyCode.FocusWithinStyle)).toEqual({
      outline: "1px solid #93c5fd",
    });
    expect(decode(tile, PropertyCode.SelectedStyle)).toEqual({
      backgroundColor: parseColor("#1d4ed8"),
      color: parseColor("#ffffff"),
    });
    // The base style and the flat legacy codes are untouched by nested states.
    expect(tile.properties.get(PropertyCode.BackgroundColor)).toBe(parseColor("#ffffff"));
    expect(tile.properties.has(PropertyCode.HoverBackgroundColor)).toBe(false);
  });

  test("tells a nested state style apart from the flag that shares its name", () => {
    const [style, setStyle] = createSignal<Record<string, unknown>>({
      disabled: { opacity: 0.5 },
      hover: { opacity: 0.9 },
    });
    const button = createComponent(Button, {
      disabled: true,
      get style() {
        return style();
      },
      children: "Save",
    });
    expect(button.properties.get(PropertyCode.Disabled)).toBe(true);
    expect(JSON.parse(String(button.properties.get(PropertyCode.DisabledStyle)))).toEqual({
      opacity: 0.5,
    });

    // Withdrawing the state styles clears their declarations without touching the flag.
    setStyle({});
    flush();
    expect(button.properties.has(PropertyCode.DisabledStyle)).toBe(false);
    expect(button.properties.has(PropertyCode.HoverStyle)).toBe(false);
    expect(button.properties.get(PropertyCode.Disabled)).toBe(true);

    // `selected` is a flag and a state style in the same way: a custom list row declares the
    // flag, and the nested style paints while it is set.
    const row = createComponent(View, {
      selected: true,
      style: { selected: { bg: "#1d4ed8" } },
      children: "README.md",
    });
    expect(row.properties.get(PropertyCode.Selected)).toBe(true);
    expect(JSON.parse(String(row.properties.get(PropertyCode.SelectedStyle)))).toEqual({
      backgroundColor: parseColor("#1d4ed8"),
    });
    setProp(row, "selected", false);
    expect(row.properties.has(PropertyCode.Selected)).toBe(false);
    expect(row.properties.has(PropertyCode.SelectedStyle)).toBe(true);
  });

  test("marks hover groups with `group`, named or not, apart from option group labels", () => {
    const row = createComponent(View, { group: true });
    expect(row.properties.get(PropertyCode.HoverGroup)).toBe(true);
    expect(row.properties.has(PropertyCode.Group)).toBe(false);

    const sidebar = createComponent(View, { group: "sidebar" });
    expect(sidebar.properties.get(PropertyCode.HoverGroup)).toBe("sidebar");
    expect(sidebar.properties.has(PropertyCode.Group)).toBe(false);
    setProp(sidebar, "group", false);
    expect(sidebar.properties.has(PropertyCode.HoverGroup)).toBe(false);
    expect(() => setProp(sidebar, "group", "x".repeat(MAX_HOVER_GROUP_NAME_BYTES + 1))).toThrow(
      "hover group names",
    );
    expect(() => setProp(sidebar, "group", " ")).toThrow("cannot be empty");

    // An option-like part's `group` is its option group label, never a hover group.
    const option = createComponent(Select.Option, {
      partValue: "apple",
      label: "Apple",
      group: "Fruit",
    });
    expect(option.properties.get(PropertyCode.Group)).toBe("Fruit");
    expect(option.properties.has(PropertyCode.HoverGroup)).toBe(false);
  });

  test("refuses layout, unpaintable, and unbounded declarations inside a state", () => {
    const node = createElement("view");
    expect(() => setProp(node, "style", { hover: { padding: 12 } })).toThrow("paint-only");
    expect(() => setProp(node, "style", { hover: { borderRadius: "50%" } })).toThrow(
      "logical pixels",
    );
    expect(() => setProp(node, "style", { hover: { outline: "2px solid chartreuse" } })).toThrow(
      "unsupported QuickGUI color",
    );
    expect(() => setProp(node, "style", { groupHover: { cursor: "pointer" } })).toThrow(
      "cannot declare a cursor",
    );
    expect(() => setProp(node, "style", { hover: { group: "sidebar" } })).toThrow(
      "only `groupHover` and `groupActive`",
    );
    expect(() => setProp(node, "style", { focusWithin: { cursor: "pointer" } })).toThrow(
      "cannot declare a cursor",
    );
    expect(() =>
      setProp(node, "style", {
        groupHover: Array.from({ length: MAX_GROUP_STYLES_PER_ELEMENT + 1 }, (_, index) => ({
          group: `g${index}`,
          opacity: 1,
        })),
      }),
    ).toThrow("follows at most");
    expect(() =>
      setProp(node, "style", {
        hover: { transform: `a${"b".repeat(MAX_STYLE_DECLARATION_BYTES)}` },
      }),
    ).toThrow("style declarations");
    expect(() =>
      setProp(node, "style", {
        hover: { cursor: "x".repeat(MAX_STATE_STYLE_JSON_BYTES) },
      }),
    ).toThrow("bounded to");
    // An empty state declares nothing at all.
    setProp(node, "style", { hover: {} });
    expect(node.properties.has(PropertyCode.HoverStyle)).toBe(false);
  });

  test("accepts style arrays that merge left to right and skip falsy entries", () => {
    const [selected, setSelected] = createSignal(false);
    const card = {
      padding: 12,
      bg: "#ffffff",
      hover: { opacity: 0.9, transform: "scale(1.01)" },
    };
    const node = createComponent(View, {
      get style() {
        return [
          card,
          selected() && { bg: "#eff6ff", hover: { opacity: 1 } },
          null,
          [undefined, { borderRadius: 8 }],
        ];
      },
    });
    const hover = () => JSON.parse(String(node.properties.get(PropertyCode.HoverStyle)));
    expect(node.properties.get(PropertyCode.Padding)).toBe(12);
    expect(node.properties.get(PropertyCode.BackgroundColor)).toBe(parseColor("#ffffff"));
    expect(node.properties.get(PropertyCode.BorderRadius)).toBe(8);
    expect(hover()).toEqual({ opacity: 0.9, transform: "scale(1.01)" });

    setSelected(true);
    flush();
    expect(node.properties.get(PropertyCode.BackgroundColor)).toBe(parseColor("#eff6ff"));
    // A later state object adds to and overrides keys of an earlier one instead of replacing it.
    expect(hover()).toEqual({ opacity: 1, transform: "scale(1.01)" });

    setSelected(false);
    flush();
    expect(node.properties.get(PropertyCode.BackgroundColor)).toBe(parseColor("#ffffff"));
    expect(hover()).toEqual({ opacity: 0.9, transform: "scale(1.01)" });
    expect(node.properties.get(PropertyCode.BorderRadius)).toBe(8);
  });

  test("flattenStyle exposes the array merge and a later null removes a state", () => {
    expect(
      solid.flattenStyle([
        { padding: 4, hover: { opacity: 1 } },
        false,
        [{ padding: 8 }, { hover: null }],
      ]),
    ).toEqual({ padding: 8, hover: null });
    // Group entries for the same group merge; entries for different groups accumulate.
    expect(
      solid.flattenStyle([
        { groupHover: { opacity: 1 } },
        { groupHover: { group: "sidebar", opacity: 0.6 } },
        { groupHover: [{ opacity: 0.9 }, { group: "sidebar", transform: "scale(1.02)" }] },
      ]),
    ).toEqual({
      groupHover: [{ opacity: 0.9 }, { group: "sidebar", opacity: 0.6, transform: "scale(1.02)" }],
    });
    expect(solid.flattenStyle([{ groupHover: { opacity: 1 } }, { groupHover: null }])).toEqual({
      groupHover: null,
    });
    expect(solid.flattenStyle(undefined)).toEqual({});
    expect(solid.flattenStyle({ color: "#ffffff" })).toEqual({ color: "#ffffff" });
  });

  test("projects sticky positioning, horizontal scrolling, and scroll snapping", () => {
    const header = createComponent(View, {
      style: { position: "sticky", top: 0 },
    });
    expect(header.properties.get(PropertyCode.Position)).toBe("sticky");
    expect(header.properties.get(PropertyCode.Top)).toBe(0);

    const rail = createComponent(View, {
      style: {
        overflowX: "scroll",
        scrollSnapType: "x mandatory",
      },
    });
    expect(rail.properties.get(PropertyCode.OverflowX)).toBe("scroll");
    expect(rail.properties.get(PropertyCode.ScrollSnapType)).toBe("x mandatory");

    const page = createComponent(View, {
      style: { scrollSnapAlign: "start", scrollSnapStop: "always" },
    });
    expect(page.properties.get(PropertyCode.ScrollSnapAlign)).toBe("start");
    expect(page.properties.get(PropertyCode.ScrollSnapStop)).toBe("always");
  });

  test("bounds every declared styling string at the JavaScript boundary", () => {
    const overlong = createElement("view");
    for (const name of [
      "filter",
      "backdropFilter",
      "transform",
      "textShadow",
      "hoverOutline",
      "focusTransform",
    ]) {
      expect(() =>
        setProp(overlong, "style", {
          [name]: `a${"b".repeat(MAX_STYLE_DECLARATION_BYTES)}`,
        }),
      ).toThrow("style declarations");
    }
    expect(() =>
      setProp(overlong, "style", {
        bgGradient: `linear-gradient(90deg, ${"#000000, ".repeat(MAX_STYLE_DECLARATION_BYTES)}#ffffff)`,
      }),
    ).toThrow("style declarations");
    expect(() => setProp(overlong, "style", { borderRadius: "1 2 3 4 5" })).toThrow(
      "one to four corner radii",
    );
    expect(() => setProp(overlong, "style", { outline: "2px solid chartreuse" })).toThrow(
      "unsupported QuickGUI color",
    );
  });

  test("clears extended styling when a declaration is withdrawn", () => {
    const node = createElement("view");
    setProp(node, "style", { bg: "linear-gradient(90deg, #000000, #ffffff)" });
    expect(node.properties.get(PropertyCode.BackgroundGradient)).toBeTypeOf("string");
    setProp(node, "style", { bg: "#101828" });
    expect(node.properties.has(PropertyCode.BackgroundGradient)).toBe(false);
    setProp(node, "style", undefined);
    expect(node.properties.has(PropertyCode.BackgroundColor)).toBe(false);

    setProp(node, "style", { outline: "2px solid #38bdf8" });
    expect(node.properties.get(PropertyCode.OutlineWidth)).toBe(2);
    setProp(node, "style", undefined);
    expect(node.properties.has(PropertyCode.OutlineWidth)).toBe(false);
    expect(node.properties.has(PropertyCode.OutlineColor)).toBe(false);
  });
  test("declares separators, avatars, and checkbox groups through one shared scope", async () => {
    await app.whenReady();
    let status: string | undefined;
    let checked: readonly string[] | undefined;
    const window = new Window({
      title: "Base UI identity",
      renderer: createRenderer(() => [
        createComponent(Separator.Root, { orientation: "vertical" }),
        createComponent(Avatar.Root, {
          ariaLabel: "Ada Lovelace",
          onLoadingStatusChange: (next: string) => {
            status = next;
          },
          get children() {
            return [
              createComponent(Avatar.Image, { src: "/tmp/ada.png" }),
              createComponent(Avatar.Fallback, { delay: 200, children: "AL" }),
            ];
          },
        }),
        createComponent(CheckboxGroup.Root, {
          allValues: ["red", "green", "blue"],
          defaultValue: ["green"],
          onValueChange: (next: readonly string[]) => {
            checked = next;
          },
          get children() {
            return [
              createComponent(Checkbox.Root, {
                value: "red",
                get children() {
                  return createComponent(Checkbox.Indicator, { children: "x" });
                },
              }),
              createComponent(Checkbox.Root, { parent: true }),
            ];
          },
        }),
      ]),
    });

    const [separator, avatar, group] = window.root.children;
    expect(separator!.properties.get(PropertyCode.Part)).toBe("separator");
    expect(separator!.properties.get(PropertyCode.Orientation)).toBe("vertical");
    // A separator retains nothing, so it declares no scope at all.
    expect(separator!.properties.has(PropertyCode.Scope)).toBe(false);

    const avatarScope = String(avatar!.properties.get(PropertyCode.Scope));
    expect(avatar!.properties.get(PropertyCode.Part)).toBe("avatar");
    expect(avatarScope.startsWith("qg-avatar-")).toBe(true);
    expect(avatar!.properties.get(PropertyCode.AccessibilityLabel)).toBe("Ada Lovelace");
    const [image, fallback] = avatar!.children;
    expect(image!.tag).toBe(NativeNodeTag.Image);
    expect(image!.properties.get(PropertyCode.Part)).toBe("avatar-image");
    expect(image!.properties.get(PropertyCode.Scope)).toBe(avatarScope);
    expect(image!.properties.get(PropertyCode.Value)).toBe("/tmp/ada.png");
    // Base UI declares the hold-back on the fallback, and the core owns the deadline.
    expect(fallback!.properties.get(PropertyCode.Part)).toBe("avatar-fallback");
    expect(fallback!.properties.get(PropertyCode.Delay)).toBe(200);

    window._dispatchEvent(
      "componentchange",
      avatar!.id,
      JSON.stringify({ loadingStatus: "loaded" }),
    );
    expect(status).toBe("loaded");

    const groupScope = String(group!.properties.get(PropertyCode.Scope));
    expect(group!.properties.get(PropertyCode.Part)).toBe("checkbox-group");
    expect(group!.properties.get(PropertyCode.Items)).toBe('["red","green","blue"]');
    expect(group!.properties.get(PropertyCode.Values)).toBe('["green"]');
    const [item, parent] = group!.children;
    // A checkbox inside a group becomes a member of it, with the core owning the click.
    expect(item!.properties.get(PropertyCode.Part)).toBe("checkbox-group-item");
    expect(item!.properties.get(PropertyCode.Scope)).toBe(groupScope);
    expect(item!.properties.get(PropertyCode.PartValue)).toBe("red");
    expect(item!.properties.has(PropertyCode.ClickListener)).toBe(false);
    expect(item!.children[0]!.properties.get(PropertyCode.Part)).toBe("checkbox-group-indicator");
    expect(parent!.properties.get(PropertyCode.Part)).toBe("checkbox-group-parent");
    expect(parent!.properties.has(PropertyCode.PartValue)).toBe(false);

    window._dispatchEvent(
      "componentchange",
      group!.id,
      JSON.stringify({ checkedValues: ["red", "green"] }),
    );
    expect(checked).toEqual(["red", "green"]);
    expect(group!.properties.get(PropertyCode.Values)).toBe('["red","green"]');
    window.close();
  });

  test("declares preview-card deadlines on the trigger and adopts the core's open value", async () => {
    await app.whenReady();
    let open: boolean | undefined;
    const window = new Window({
      title: "Preview card",
      renderer: createRenderer(() =>
        createComponent(PreviewCard.Root, {
          placement: "top",
          gap: 6,
          viewportMargin: 8,
          onOpenChange: (next: boolean) => {
            open = next;
          },
          get children() {
            return [
              createComponent(PreviewCard.Trigger, {
                delay: 600,
                closeDelay: 300,
                children: "@ada",
              }),
              createComponent(PreviewCard.Positioner, {
                get children() {
                  return createComponent(PreviewCard.Popup, {
                    get children() {
                      return createComponent(PreviewCard.Arrow, {});
                    },
                  });
                },
              }),
            ];
          },
        }),
      ),
    });

    const root = window.root.children[0]!;
    const scope = String(root.properties.get(PropertyCode.Scope));
    expect(root.properties.get(PropertyCode.Part)).toBe("preview-card");
    expect(root.properties.get(PropertyCode.Open)).toBe(false);
    expect(root.properties.get(PropertyCode.AnchorPlacement)).toBe("top");
    expect(root.properties.get(PropertyCode.AnchorGap)).toBe(6);
    expect(root.properties.get(PropertyCode.ViewportMargin)).toBe(8);

    const [trigger, positioner] = root.children;
    expect(trigger!.properties.get(PropertyCode.Part)).toBe("preview-card-trigger");
    expect(trigger!.properties.get(PropertyCode.Scope)).toBe(scope);
    expect(trigger!.properties.get(PropertyCode.Delay)).toBe(600);
    expect(trigger!.properties.get(PropertyCode.CloseDelay)).toBe(300);
    expect(positioner!.properties.get(PropertyCode.Part)).toBe("preview-card-positioner");
    const popup = positioner!.children[0]!;
    expect(popup.properties.get(PropertyCode.Part)).toBe("preview-card-popup");
    expect(popup.children[0]!.properties.get(PropertyCode.Part)).toBe("preview-card-arrow");

    // The core decides when the hover deadline elapses; JavaScript only commits the result.
    window._dispatchEvent("componentchange", root.id, JSON.stringify({ open: true }));
    expect(open).toBe(true);
    expect(root.properties.get(PropertyCode.Open)).toBe(true);
    window.close();
  });

  test("declares scroll-area geometry and reports the core's derived style state", async () => {
    await app.whenReady();
    let reported: solid.ScrollAreaState | undefined;
    let live: (() => solid.ScrollAreaState) | undefined;
    const window = new Window({
      title: "Scroll area",
      renderer: createRenderer(() =>
        createComponent(ScrollArea.Root, {
          viewportSize: { width: 260, height: 160 },
          contentSize: { width: 260, height: 900 },
          overflowEdgeThreshold: 2,
          onScrollStateChange: (next: solid.ScrollAreaState) => {
            reported = next;
          },
          get children() {
            live = solid.useScrollAreaState();
            return [
              createComponent(ScrollArea.Viewport, {
                get children() {
                  return createComponent(ScrollArea.Content, {});
                },
              }),
              createComponent(ScrollArea.Scrollbar, {
                orientation: "vertical",
                keepMounted: true,
                get children() {
                  return createComponent(ScrollArea.Thumb, {});
                },
              }),
              createComponent(ScrollArea.Corner, {}),
            ];
          },
        }),
      ),
    });

    const root = window.root.children[0]!;
    const scope = String(root.properties.get(PropertyCode.Scope));
    expect(root.properties.get(PropertyCode.Part)).toBe("scroll-area");
    expect(root.properties.get(PropertyCode.ViewportSize)).toBe("[260,160]");
    expect(root.properties.get(PropertyCode.ContentSize)).toBe("[260,900]");
    expect(root.properties.get(PropertyCode.OverflowEdgeThreshold)).toBe(2);

    const [viewport, scrollbar, corner] = root.children;
    expect(viewport!.properties.get(PropertyCode.Part)).toBe("scroll-area-viewport");
    expect(viewport!.properties.get(PropertyCode.Scope)).toBe(scope);
    expect(viewport!.children[0]!.properties.get(PropertyCode.Part)).toBe("scroll-area-content");
    expect(scrollbar!.properties.get(PropertyCode.Part)).toBe("scroll-area-scrollbar");
    expect(scrollbar!.properties.get(PropertyCode.KeepMounted)).toBe(true);
    // A thumb inherits its axis from the scrollbar it is drawn inside.
    expect(scrollbar!.children[0]!.properties.get(PropertyCode.Orientation)).toBe("vertical");
    expect(corner!.properties.get(PropertyCode.Part)).toBe("scroll-area-corner");

    window._dispatchEvent(
      "componentchange",
      root.id,
      JSON.stringify({
        offset: { x: 0, y: 40 },
        scrolling: true,
        hovering: true,
        hasOverflowX: false,
        hasOverflowY: true,
        overflowXStart: false,
        overflowXEnd: false,
        overflowYStart: true,
        overflowYEnd: true,
      }),
    );
    expect(reported?.offset).toEqual({ x: 0, y: 40 });
    expect(reported?.hasOverflowY).toBe(true);
    expect(live?.().overflowYStart).toBe(true);
    window.close();
  });

  test("declares OTP slots and reports the core's value and completion edge", async () => {
    await app.whenReady();
    let value: string | undefined;
    let completed: string | undefined;
    const window = new Window({
      title: "OTP field",
      renderer: createRenderer(() =>
        createComponent(OtpField.Root, {
          length: 4,
          validationType: "alphanumeric",
          mask: true,
          required: true,
          autoSubmit: "verify",
          defaultValue: "12",
          onValueChange: (next: string) => {
            value = next;
          },
          onComplete: (next: string) => {
            completed = next;
          },
          get children() {
            return [
              createComponent(OtpField.Input, { index: 0 }),
              createComponent(OtpField.Separator, { index: 0 }),
              createComponent(OtpField.Input, { index: 1 }),
            ];
          },
        }),
      ),
    });

    const root = window.root.children[0]!;
    const scope = String(root.properties.get(PropertyCode.Scope));
    expect(root.properties.get(PropertyCode.Part)).toBe("otp-field");
    expect(root.properties.get(PropertyCode.Length)).toBe(4);
    expect(root.properties.get(PropertyCode.Variant)).toBe("alphanumeric");
    expect(root.properties.get(PropertyCode.Mask)).toBe(true);
    expect(root.properties.get(PropertyCode.Required)).toBe(true);
    expect(root.properties.get(PropertyCode.AutoSubmit)).toBe("verify");
    expect(root.properties.get(PropertyCode.Value)).toBe("12");

    const [first, separator] = root.children;
    expect(first!.tag).toBe(NativeNodeTag.Input);
    expect(first!.properties.get(PropertyCode.Part)).toBe("otp-field-input");
    expect(first!.properties.get(PropertyCode.Scope)).toBe(scope);
    expect(first!.properties.get(PropertyCode.ItemIndex)).toBe(0);
    // The core owns every slot's editing, so no slot declares an input listener.
    expect(first!.properties.has(PropertyCode.InputListener)).toBe(false);
    expect(separator!.properties.get(PropertyCode.Part)).toBe("otp-field-separator");

    window._dispatchEvent(
      "componentchange",
      root.id,
      JSON.stringify({ value: "1234", complete: "1234" }),
    );
    expect(value).toBe("1234");
    expect(completed).toBe("1234");
    expect(root.properties.get(PropertyCode.Value)).toBe("1234");
    window.close();
  });

  test("declares drawer modality, snap points, and the swipe the core reported", async () => {
    await app.whenReady();
    let open: boolean | undefined;
    let snap: number | undefined;
    let swipe: solid.DrawerSwipeState | undefined;
    const window = new Window({
      title: "Drawer",
      renderer: createRenderer(() =>
        createComponent(Drawer.Root, {
          modal: "trap-focus",
          swipeDirection: "down",
          snapPoints: [0.45, 1],
          snapPoint: 0,
          disablePointerDismissal: true,
          onOpenChange: (next: boolean) => {
            open = next;
          },
          onSnapPointChange: (next: number) => {
            snap = next;
          },
          onSwipeChange: (next: solid.DrawerSwipeState) => {
            swipe = next;
          },
          get children() {
            return [
              createComponent(Drawer.Trigger, { children: "Filters" }),
              createComponent(Drawer.Portal, {
                get children() {
                  return [
                    createComponent(Drawer.Backdrop, {}),
                    createComponent(Drawer.Viewport, {
                      get children() {
                        return createComponent(Drawer.Popup, {
                          get children() {
                            return [
                              createComponent(Drawer.SwipeArea, {}),
                              createComponent(Drawer.Title, {
                                children: "Filters",
                              }),
                              createComponent(Drawer.Description, {
                                children: "Narrow the results",
                              }),
                              createComponent(Drawer.Content, {}),
                              createComponent(Drawer.Close, {
                                children: "Close",
                              }),
                            ];
                          },
                        });
                      },
                    }),
                  ];
                },
              }),
            ];
          },
        }),
      ),
    });

    const root = window.root.children[0]!;
    const scope = String(root.properties.get(PropertyCode.Scope));
    expect(root.properties.get(PropertyCode.Part)).toBe("drawer");
    expect(root.properties.get(PropertyCode.Variant)).toBe("trap-focus");
    expect(root.properties.get(PropertyCode.SwipeDirection)).toBe("down");
    expect(root.properties.get(PropertyCode.Values)).toBe("[0.45,1]");
    expect(root.properties.get(PropertyCode.ItemIndex)).toBe(0);
    expect(root.properties.get(PropertyCode.DisablePointerDismissal)).toBe(true);
    expect(root.properties.get(PropertyCode.Open)).toBe(false);

    const [trigger, portal] = root.children;
    expect(trigger!.properties.get(PropertyCode.Part)).toBe("drawer-trigger");
    expect(portal!.properties.get(PropertyCode.Part)).toBe("drawer-portal");
    expect(portal!.properties.get(PropertyCode.Scope)).toBe(scope);
    const popup = portal!.children[1]!.children[0]!;
    expect(popup.properties.get(PropertyCode.Part)).toBe("drawer-popup");
    expect(popup.children.map((child) => child.properties.get(PropertyCode.Part))).toEqual([
      "drawer-swipe-area",
      "drawer-title",
      "drawer-description",
      "drawer-content",
      "drawer-close",
    ]);
    // The swipe area's gesture belongs to the core, so it declares no pointer listener.
    expect(popup.children[0]!.properties.has(PropertyCode.PointerListener)).toBe(false);

    window._dispatchEvent("click", trigger!.id);
    expect(open).toBe(true);
    expect(root.properties.get(PropertyCode.Open)).toBe(true);

    window._dispatchEvent(
      "componentchange",
      root.id,
      JSON.stringify({
        open: true,
        snapPoint: 1,
        swiping: true,
        swipeOffset: 24,
      }),
    );
    expect(snap).toBe(1);
    expect(swipe).toEqual({ swiping: true, swipeOffset: 24 });

    window._dispatchEvent(
      "componentchange",
      root.id,
      JSON.stringify({
        open: false,
        snapPoint: 1,
        swiping: false,
        swipeOffset: 0,
      }),
    );
    expect(open).toBe(false);
    expect(root.properties.get(PropertyCode.Open)).toBe(false);
    window.close();
  });

  test("shares one navigation-menu scope and reports the core's activation direction", async () => {
    await app.whenReady();
    let value: string | undefined;
    let direction: string | null | undefined;
    const window = new Window({
      title: "Navigation menu",
      renderer: createRenderer(() =>
        createComponent(NavigationMenu.Root, {
          orientation: "horizontal",
          delay: 50,
          closeDelay: 80,
          onValueChange: (next: string | undefined) => {
            value = next;
          },
          onActivationDirectionChange: (next: string | null) => {
            direction = next;
          },
          get children() {
            return [
              createComponent(NavigationMenu.List, {
                get children() {
                  return ["products", "solutions"].map((item) =>
                    createComponent(NavigationMenu.Item, {
                      value: item,
                      get children() {
                        return [
                          createComponent(NavigationMenu.Trigger, {
                            children: item,
                          }),
                          createComponent(NavigationMenu.Icon, {}),
                          createComponent(NavigationMenu.Positioner, {
                            get children() {
                              return createComponent(NavigationMenu.Popup, {
                                get children() {
                                  return createComponent(NavigationMenu.Content, {
                                    get children() {
                                      return createComponent(NavigationMenu.Link, {
                                        value: `${item}-home`,
                                        active: true,
                                      });
                                    },
                                  });
                                },
                              });
                            },
                          }),
                        ];
                      },
                    }),
                  );
                },
              }),
            ];
          },
        }),
      ),
    });

    const root = window.root.children[0]!;
    const scope = String(root.properties.get(PropertyCode.Scope));
    expect(root.properties.get(PropertyCode.Part)).toBe("navigation-menu");
    expect(root.properties.get(PropertyCode.Delay)).toBe(50);
    expect(root.properties.get(PropertyCode.CloseDelay)).toBe(80);

    const list = root.children[0]!;
    expect(list.properties.get(PropertyCode.Part)).toBe("navigation-menu-list");
    const item = list.children[1]!;
    expect(item.properties.get(PropertyCode.Part)).toBe("navigation-menu-item");
    expect(item.properties.get(PropertyCode.PartValue)).toBe("solutions");
    const [trigger, icon, positioner] = item.children;
    // Every part inherits the item's value, so no composition repeats it.
    expect(trigger!.properties.get(PropertyCode.PartValue)).toBe("solutions");
    expect(trigger!.properties.get(PropertyCode.Scope)).toBe(scope);
    expect(trigger!.properties.has(PropertyCode.ClickListener)).toBe(false);
    expect(icon!.properties.get(PropertyCode.Part)).toBe("navigation-menu-icon");
    expect(positioner!.properties.get(PropertyCode.PartValue)).toBe("solutions");
    const link = positioner!.children[0]!.children[0]!.children[0]!;
    expect(link.properties.get(PropertyCode.Part)).toBe("navigation-menu-link");
    expect(link.properties.get(PropertyCode.PartValue)).toBe("solutions-home");
    expect(link.properties.get(PropertyCode.Checked)).toBe(true);

    window._dispatchEvent(
      "componentchange",
      root.id,
      JSON.stringify({
        value: "solutions",
        focused: "solutions",
        activationDirection: "right",
      }),
    );
    expect(value).toBe("solutions");
    expect(direction).toBe("right");
    expect(root.properties.get(PropertyCode.ActiveValue)).toBe("solutions");

    window._dispatchEvent(
      "componentchange",
      root.id,
      JSON.stringify({ value: null, focused: "solutions", activationDirection: null }),
    );
    expect(value).toBeUndefined();
    expect(root.properties.has(PropertyCode.ActiveValue)).toBe(false);
    window.close();
  });

  test("names every Base UI compound part and bounds its declarations", () => {
    expect(Object.keys(Separator)).toEqual(["Root"]);
    expect(Object.keys(Avatar)).toEqual(["Root", "Image", "Fallback"]);
    expect(Object.keys(CheckboxGroup)).toEqual(["Root"]);
    expect(Object.keys(PreviewCard)).toEqual([
      "Root",
      "Trigger",
      "Portal",
      "Backdrop",
      "Positioner",
      "Popup",
      "Arrow",
    ]);
    expect(Object.keys(ScrollArea)).toEqual([
      "Root",
      "Viewport",
      "Content",
      "Scrollbar",
      "Thumb",
      "Corner",
    ]);
    expect(Object.keys(OtpField)).toEqual(["Root", "Input", "Separator"]);
    expect(Object.keys(Drawer)).toEqual([
      "Root",
      "Trigger",
      "Portal",
      "Backdrop",
      "Viewport",
      "Popup",
      "Content",
      "Title",
      "Description",
      "Close",
      "SwipeArea",
    ]);
    expect(Object.keys(NavigationMenu)).toEqual([
      "Root",
      "List",
      "Item",
      "Trigger",
      "Icon",
      "Content",
      "Link",
      "Portal",
      "Positioner",
      "Popup",
      "Viewport",
      "Arrow",
      "Backdrop",
    ]);

    // Malformed and oversized declarations are refused before they can cross N-API.
    const node = createElement("view");
    expect(() => setProp(node, "viewportSize", { width: Number.NaN, height: 1 })).toThrow(
      "extents must be finite",
    );
    setProp(node, "contentSize", [4, 8]);
    expect(node.properties.get(PropertyCode.ContentSize)).toBe("[4,8]");
    expect(() => setProp(node, "autoSubmit", "f".repeat(MAX_COMPONENT_VALUE_BYTES + 1))).toThrow(
      "scopes and values are limited",
    );
  });
});

describe("Base UI-aligned popovers, tooltips, range parts, toasts, tabs, and fields", () => {
  test("declares popover positioning on the trigger and adopts the placement the core resolved", async () => {
    await app.whenReady();
    const [open, setOpen] = createSignal(true);
    let placement: solid.AnchorPlacementDetails | undefined;
    let live: (() => solid.AnchorPlacementDetails) | undefined;
    const window = new Window({
      title: "Popover parts",
      renderer: createRenderer(() =>
        createComponent(Popover.Root, {
          get open() {
            return open();
          },
          modal: true,
          onOpenChange: (next: boolean) => setOpen(next),
          onPlacementChange: (next: solid.AnchorPlacementDetails) => {
            placement = next;
          },
          get children() {
            live = solid.usePopoverPlacement();
            return [
              createComponent(Popover.Trigger, {
                openOnHover: true,
                delay: 120,
                closeDelay: 90,
                children: "Account",
              }),
              createComponent(Popover.Positioner, {
                side: "top",
                align: "end",
                sideOffset: 10,
                alignOffset: 4,
                collisionPadding: 12,
                sticky: false,
                get children() {
                  return createComponent(Popover.Popup, {
                    get children() {
                      return [
                        createComponent(Popover.Arrow, {}),
                        createComponent(Popover.Title, { children: "Account" }),
                        createComponent(Popover.Viewport, {}),
                        createComponent(Popover.Close, { children: "×" }),
                      ];
                    },
                  });
                },
              }),
            ];
          },
        }),
      ),
    });

    const trigger = window.root.children[0]!;
    const scope = String(trigger.properties.get(PropertyCode.Scope));
    // The trigger is mounted whether the popup is open or closed, so it carries the declaration.
    expect(trigger.properties.get(PropertyCode.Part)).toBe("popover-trigger");
    expect(trigger.properties.get(PropertyCode.Open)).toBe(true);
    expect(trigger.properties.get(PropertyCode.Modal)).toBe(true);
    expect(trigger.properties.get(PropertyCode.OpenOnHover)).toBe(true);
    expect(trigger.properties.get(PropertyCode.Delay)).toBe(120);
    expect(trigger.properties.get(PropertyCode.CloseDelay)).toBe(90);
    expect(trigger.properties.get(PropertyCode.Side)).toBe("top");
    expect(trigger.properties.get(PropertyCode.Align)).toBe("end");
    expect(trigger.properties.get(PropertyCode.SideOffset)).toBe(10);
    expect(trigger.properties.get(PropertyCode.AlignOffset)).toBe(4);
    expect(trigger.properties.get(PropertyCode.CollisionPadding)).toBe(12);
    expect(trigger.properties.get(PropertyCode.Sticky)).toBe(false);

    const positioner = window.root.children[1]!;
    expect(positioner.properties.get(PropertyCode.Part)).toBe("popover-positioner");
    expect(positioner.properties.get(PropertyCode.Scope)).toBe(scope);
    const popup = positioner.children[0]!;
    expect(popup.properties.get(PropertyCode.Part)).toBe("popover-popup");
    expect(popup.children.map((child) => child.properties.get(PropertyCode.Part))).toEqual([
      "popover-arrow",
      "popover-title",
      "popover-viewport",
      "popover-close",
    ]);

    // The declared side is only a preference; the core reports the one it really used.
    window._dispatchEvent(
      "componentchange",
      trigger.id,
      JSON.stringify({
        open: true,
        placement: {
          side: "bottom",
          align: "start",
          anchorHidden: false,
          anchorWidth: 120,
          anchorHeight: 32,
          availableWidth: 900,
          availableHeight: 500,
        },
      }),
    );
    expect(placement?.side).toBe("bottom");
    expect(placement?.anchorWidth).toBe(120);
    expect(live?.().align).toBe("start");

    // A hover the core decided reaches the controlled declaration.
    window._dispatchEvent(
      "componentchange",
      trigger.id,
      JSON.stringify({ open: false, placement: { side: "bottom" } }),
    );
    expect(open()).toBe(false);
    window.close();
  });

  test("anchors a popover to a declared logical point", async () => {
    await app.whenReady();
    const window = new Window({
      title: "Popover point anchor",
      renderer: createRenderer(() =>
        createComponent(Popover.Root, {
          open: true,
          anchor: { x: 120, y: 48 },
          get children() {
            return createComponent(Popover.Trigger, { children: "Anchor" });
          },
        }),
      ),
    });
    const trigger = window.root.children[0]!;
    expect(trigger.properties.get(PropertyCode.AnchorPoint)).toBe("120,48");
    expect(trigger.properties.has(PropertyCode.AnchorTarget)).toBe(false);
    expect(() => setProp(createElement("view"), "anchor", { x: 1 })).toThrow(
      "must be a NativeNode or an { x, y } point",
    );
    window.close();
  });

  test("shares one warm tooltip provider and reports the resolved side", async () => {
    await app.whenReady();
    let open: boolean | undefined;
    const window = new Window({
      title: "Tooltip",
      renderer: createRenderer(() =>
        createComponent(solid.Tooltip.Provider, {
          delay: 600,
          closeDelay: 200,
          timeout: 400,
          get children() {
            return createComponent(solid.Tooltip.Root, {
              hoverable: true,
              trackCursorAxis: "x",
              side: "top",
              sideOffset: 9,
              collisionPadding: 8,
              onOpenChange: (next: boolean) => {
                open = next;
              },
              get children() {
                return [
                  createComponent(solid.Tooltip.Trigger, {
                    delay: 120,
                    closeOnClick: false,
                    children: "Save",
                  }),
                  createComponent(solid.Tooltip.Positioner, {
                    get children() {
                      return createComponent(solid.Tooltip.Popup, {
                        get children() {
                          return createComponent(solid.Tooltip.Arrow, {});
                        },
                      });
                    },
                  }),
                ];
              },
            });
          },
        }),
      ),
    });

    const provider = window.root.children[0]!;
    expect(provider.properties.get(PropertyCode.Part)).toBe("tooltip-provider");
    expect(provider.properties.get(PropertyCode.Timeout)).toBe(400);
    const providerScope = String(provider.properties.get(PropertyCode.Scope));

    const trigger = provider.children[0]!;
    expect(trigger.properties.get(PropertyCode.Part)).toBe("tooltip-trigger");
    expect(trigger.properties.get(PropertyCode.Provider)).toBe(providerScope);
    expect(trigger.properties.get(PropertyCode.Delay)).toBe(120);
    expect(trigger.properties.get(PropertyCode.CloseOnClick)).toBe(false);
    expect(trigger.properties.get(PropertyCode.Hoverable)).toBe(true);
    expect(trigger.properties.get(PropertyCode.TrackCursorAxis)).toBe("x");
    expect(trigger.properties.get(PropertyCode.Side)).toBe("top");
    expect(trigger.properties.get(PropertyCode.SideOffset)).toBe(9);

    const positioner = provider.children[1]!;
    expect(positioner.properties.get(PropertyCode.Part)).toBe("tooltip-positioner");
    expect(positioner.children[0]!.properties.get(PropertyCode.Part)).toBe("tooltip-popup");
    expect(positioner.children[0]!.children[0]!.properties.get(PropertyCode.Part)).toBe(
      "tooltip-arrow",
    );

    window._dispatchEvent(
      "componentchange",
      trigger.id,
      JSON.stringify({ open: true, placement: { side: "bottom", align: "center" } }),
    );
    expect(open).toBe(true);
    window.close();
  });

  test("declares slider format and reports the core's commit boundary", async () => {
    await app.whenReady();
    let committed: readonly number[] | undefined;
    let live: (() => solid.SliderState) | undefined;
    const window = new Window({
      title: "Slider parts",
      renderer: createRenderer(() =>
        createComponent(Slider.Root, {
          scope: "volume",
          defaultValue: [20],
          min: 0,
          max: 100,
          step: 10,
          minStepsBetweenValues: 1,
          thumbAlignment: "edge",
          format: "percent",
          onValueCommitted: (values: readonly number[]) => {
            committed = values;
          },
          get children() {
            live = solid.useSliderState();
            return [
              createComponent(Slider.Label, { scope: "volume" }),
              createComponent(Slider.Value, { scope: "volume" }),
              createComponent(Slider.Control, {
                scope: "volume",
                get children() {
                  return createComponent(Slider.Track, {
                    scope: "volume",
                    get children() {
                      return createComponent(Slider.Indicator, {
                        scope: "volume",
                      });
                    },
                  });
                },
              }),
              createComponent(Slider.Thumb, { scope: "volume", index: 0 }),
            ];
          },
        }),
      ),
    });

    const root = window.root.children[0]!;
    expect(root.properties.get(PropertyCode.MinStepsBetweenValues)).toBe(1);
    expect(root.properties.get(PropertyCode.ThumbAlignment)).toBe("edge");
    expect(root.properties.get(PropertyCode.Format)).toBe("percent");
    const [label, value, control, thumb] = root.children;
    expect(label!.properties.get(PropertyCode.Part)).toBe("slider-label");
    expect(value!.properties.get(PropertyCode.Part)).toBe("slider-value");
    expect(control!.properties.get(PropertyCode.Part)).toBe("slider-control");
    expect(control!.children[0]!.children[0]!.properties.get(PropertyCode.Part)).toBe(
      "slider-indicator",
    );
    expect(thumb!.properties.get(PropertyCode.ItemIndex)).toBe(0);

    window._dispatchEvent(
      "componentchange",
      root.id,
      JSON.stringify({
        values: [80],
        dragging: true,
        committed: false,
        displayValue: "80%",
      }),
    );
    expect(live?.().dragging).toBe(true);
    expect(live?.().displayValue).toBe("80%");
    expect(committed).toBeUndefined();

    window._dispatchEvent(
      "componentchange",
      root.id,
      JSON.stringify({
        values: [80],
        dragging: false,
        committed: true,
        displayValue: "80%",
      }),
    );
    expect(committed).toEqual([80]);
    window.close();
  });

  test("declares number-field scrub props and reports the core's scrub state", async () => {
    await app.whenReady();
    let committed: number | undefined;
    let live: (() => solid.NumberFieldState) | undefined;
    const window = new Window({
      title: "Number field parts",
      renderer: createRenderer(() =>
        createComponent(NumberField.Root, {
          scope: "quantity",
          defaultValue: 10,
          step: 1,
          smallStep: 0.5,
          largeStep: 5,
          snapOnStep: true,
          allowWheelScrub: false,
          readOnly: false,
          required: true,
          scrubDirection: "vertical",
          scrubSensitivity: 2,
          onValueCommitted: (value: number | undefined) => {
            committed = value;
          },
          get children() {
            live = solid.useNumberFieldState();
            return createComponent(NumberField.Group, {
              scope: "quantity",
              get children() {
                return createComponent(NumberField.ScrubArea, {
                  scope: "quantity",
                  get children() {
                    return createComponent(NumberField.ScrubAreaCursor, {
                      scope: "quantity",
                    });
                  },
                });
              },
            });
          },
        }),
      ),
    });

    const root = window.root.children[0]!;
    expect(root.properties.get(PropertyCode.SmallStep)).toBe(0.5);
    expect(root.properties.get(PropertyCode.LargeStep)).toBe(5);
    expect(root.properties.get(PropertyCode.SnapOnStep)).toBe(true);
    expect(root.properties.get(PropertyCode.AllowWheelScrub)).toBe(false);
    expect(root.properties.get(PropertyCode.Required)).toBe(true);
    expect(root.properties.get(PropertyCode.Orientation)).toBe("vertical");
    expect(root.properties.get(PropertyCode.Pitch)).toBe(2);

    const group = root.children[0]!;
    expect(group.properties.get(PropertyCode.Part)).toBe("number-field-group");
    const scrub = group.children[0]!;
    expect(scrub.properties.get(PropertyCode.Part)).toBe("number-field-scrub-area");
    expect(scrub.children[0]!.properties.get(PropertyCode.Part)).toBe(
      "number-field-scrub-area-cursor",
    );

    window._dispatchEvent(
      "componentchange",
      root.id,
      JSON.stringify({
        value: 12,
        valid: true,
        scrubbing: true,
        required: true,
        committed: true,
      }),
    );
    expect(live?.().scrubbing).toBe(true);
    expect(committed).toBe(12);
    window.close();
  });

  test("declares progress and meter parts and adopts the core's status", async () => {
    await app.whenReady();
    let live: (() => solid.GaugeState) | undefined;
    const window = new Window({
      title: "Progress parts",
      renderer: createRenderer(() => [
        createComponent(Progress.Root, {
          scope: "upload",
          value: 3,
          max: 4,
          format: "fraction",
          get children() {
            live = solid.useGaugeState();
            return [
              createComponent(Progress.Label, { scope: "upload" }),
              createComponent(Progress.Value, { scope: "upload" }),
              createComponent(Progress.Track, {
                scope: "upload",
                get children() {
                  return createComponent(Progress.Indicator, {
                    scope: "upload",
                  });
                },
              }),
            ];
          },
        }),
        createComponent(Meter.Root, {
          scope: "storage",
          value: 50,
          min: 0,
          max: 100,
          format: "percent",
          get children() {
            return createComponent(Meter.Track, {
              scope: "storage",
              get children() {
                return createComponent(Meter.Indicator, { scope: "storage" });
              },
            });
          },
        }),
      ]),
    });

    const progress = window.root.children[0]!;
    expect(progress.properties.get(PropertyCode.Format)).toBe("fraction");
    expect(progress.children.map((child) => child.properties.get(PropertyCode.Part))).toEqual([
      "progress-label",
      "progress-value",
      "progress-track",
    ]);
    const meter = window.root.children[1]!;
    expect(meter.properties.get(PropertyCode.Format)).toBe("percent");
    expect(meter.children[0]!.properties.get(PropertyCode.Part)).toBe("meter-track");

    window._dispatchEvent(
      "componentchange",
      progress.id,
      JSON.stringify({
        status: "progressing",
        displayValue: "3 of 4",
        completion: 0.75,
      }),
    );
    expect(live?.().status).toBe("progressing");
    expect(live?.().displayValue).toBe("3 of 4");
    window.close();
  });

  test("drives a toast queue through the manager and adopts the core's stack", async () => {
    await app.whenReady();
    let manager: solid.ToastManager | undefined;
    const window = new Window({
      title: "Toast provider",
      renderer: createRenderer(() =>
        createComponent(Toast.Provider, {
          timeout: 4000,
          limit: 2,
          expanded: true,
          swipeDirection: "left",
          pitch: 12,
          get children() {
            manager = solid.useToastManager();
            return createComponent(Toast.Viewport, {
              get children() {
                return manager!.toasts().map((toast) =>
                  createComponent(Toast.Root, {
                    toastId: toast.id,
                    get children() {
                      return createComponent(Toast.Content, {
                        toastId: toast.id,
                        get children() {
                          return createComponent(Toast.Title, {
                            toastId: toast.id,
                            children: toast.title,
                          });
                        },
                      });
                    },
                  }),
                );
              },
            });
          },
        }),
      ),
    });

    const viewport = window.root.children[0]!;
    expect(viewport.properties.get(PropertyCode.Part)).toBe("toast-viewport");
    expect(viewport.properties.get(PropertyCode.Timeout)).toBe(4000);
    expect(viewport.properties.get(PropertyCode.Limit)).toBe(2);
    expect(viewport.properties.get(PropertyCode.StackExpanded)).toBe(true);
    expect(viewport.properties.get(PropertyCode.SwipeDirection)).toBe("left");
    expect(viewport.properties.get(PropertyCode.Pitch)).toBe(12);
    const scope = String(viewport.properties.get(PropertyCode.Scope));

    const id = manager!.add({ title: "Saved", type: "success" });
    flush();
    expect(JSON.parse(String(viewport.properties.get(PropertyCode.Toasts)))).toEqual([
      { title: "Saved", type: "success", id },
    ]);
    const toast = viewport.children[0]!;
    expect(toast.properties.get(PropertyCode.Part)).toBe("toast");
    expect(toast.properties.get(PropertyCode.Scope)).toBe(scope);
    expect(toast.children[0]!.properties.get(PropertyCode.Part)).toBe("toast-content");

    // The core owns the stack index, the limited flag, and each toast's own offset.
    window._dispatchEvent(
      "componentchange",
      viewport.id,
      JSON.stringify({
        toasts: [
          {
            id,
            index: 0,
            type: "success",
            limited: false,
            expanded: true,
            swiping: false,
            swipeMovement: 0,
            offset: 0,
          },
        ],
      }),
    );
    expect(manager!.stack()[0]?.index).toBe(0);

    // A dismissal the core decided drops the toast from the declaration.
    window._dispatchEvent(
      "componentchange",
      viewport.id,
      JSON.stringify({ dismissed: [id], toasts: [] }),
    );
    flush();
    expect(manager!.toasts()).toEqual([]);
    window.close();
  });

  test("declares tab positions and adopts the core's activation direction", async () => {
    await app.whenReady();
    let live: (() => solid.TabsState) | undefined;
    const window = new Window({
      title: "Tabs geometry",
      renderer: createRenderer(() =>
        createComponent(Tabs.Root, {
          defaultValue: "list",
          get children() {
            live = solid.useTabsState();
            return createComponent(Tabs.List, {
              get children() {
                return [
                  createComponent(Tabs.Tab, {
                    value: "list",
                    index: 0,
                    children: "List",
                  }),
                  createComponent(Tabs.Tab, {
                    value: "grid",
                    index: 1,
                    children: "Grid",
                  }),
                  createComponent(Tabs.Indicator, {
                    value: "list",
                    placement: "bottom",
                  }),
                ];
              },
            });
          },
        }),
      ),
    });

    const root = window.root.children[0]!;
    const list = root.children[0]!;
    expect(list.children[1]!.properties.get(PropertyCode.ItemIndex)).toBe(1);
    expect(list.children[2]!.properties.get(PropertyCode.AnchorPlacement)).toBe("bottom");

    window._dispatchEvent(
      "componentchange",
      root.id,
      JSON.stringify({
        activationDirection: "right",
        indicator: { left: 60, top: 30, width: 48, height: 2 },
      }),
    );
    expect(live?.().activationDirection).toBe("right");
    expect(live?.().indicator?.width).toBe(48);
    window.close();
  });

  test("declares toolbar roles and a disabled item that keeps its Tab stop", async () => {
    await app.whenReady();
    const window = new Window({
      title: "Toolbar parts",
      renderer: createRenderer(() =>
        createComponent(Toolbar.Root, {
          scope: "actions",
          items: [
            { value: "cut" },
            { value: "docs" },
            { value: "search" },
            { value: "paste", disabled: true, focusableWhenDisabled: true },
          ],
          get children() {
            return [
              createComponent(Toolbar.Group, {
                scope: "actions",
                get children() {
                  return [
                    createComponent(Toolbar.Button, {
                      scope: "actions",
                      partValue: "cut",
                      children: "Cut",
                    }),
                    createComponent(Toolbar.Link, {
                      scope: "actions",
                      partValue: "docs",
                      children: "Docs",
                    }),
                    createComponent(Toolbar.Input, {
                      scope: "actions",
                      partValue: "search",
                    }),
                  ];
                },
              }),
              createComponent(Toolbar.Separator, { scope: "actions" }),
              createComponent(Toolbar.Item, {
                scope: "actions",
                partValue: "paste",
                children: "Paste",
              }),
            ];
          },
        }),
      ),
    });

    const root = window.root.children[0]!;
    expect(JSON.parse(String(root.properties.get(PropertyCode.Items)))).toEqual([
      { value: "cut" },
      { value: "docs" },
      { value: "search" },
      { value: "paste", disabled: true, focusableWhenDisabled: true },
    ]);
    const group = root.children[0]!;
    expect(group.properties.get(PropertyCode.Part)).toBe("toolbar-group");
    expect(group.children.map((child) => child.properties.get(PropertyCode.Part))).toEqual([
      "toolbar-button",
      "toolbar-link",
      "toolbar-input",
    ]);
    expect(root.children[1]!.properties.get(PropertyCode.Part)).toBe("toolbar-separator");
    window.close();
  });

  test("declares a field's validation mode and mounts its item and validity parts", async () => {
    await app.whenReady();
    const window = new Window({
      title: "Field parts",
      renderer: createRenderer(() =>
        createComponent(Field.Root, {
          validationMode: "onChange",
          validationDebounceTime: 250,
          get children() {
            return [
              createComponent(Field.Item, {
                get children() {
                  return createComponent(Field.Control, {});
                },
              }),
              createComponent(Field.Validity, { visible: true }),
            ];
          },
        }),
      ),
    });

    const root = window.root.children[0]!;
    expect(root.properties.get(PropertyCode.ValidationMode)).toBe("onChange");
    expect(root.properties.get(PropertyCode.ValidationDebounceTime)).toBe(250);
    expect(root.children[0]!.properties.get(PropertyCode.Part)).toBe("field-item");
    expect(root.children[1]!.properties.get(PropertyCode.Part)).toBe("field-validity");
    expect(root.children[1]!.properties.get(PropertyCode.Open)).toBe(true);
    window.close();
  });

  test("declares read-only selection controls and a derived parent checkbox", async () => {
    await app.whenReady();
    const window = new Window({
      title: "Read-only controls",
      renderer: createRenderer(() => [
        createComponent(Checkbox.Root, {
          parent: true,
          childrenChecked: [true, false, true],
        }),
        createComponent(Switch.Root, { checked: true, readOnly: true }),
        createComponent(Radio.Root, { readOnly: true }),
      ]),
    });

    const [parent, locked, radio] = window.root.children;
    // The core folds the declared children into on, mixed, or off with no registry at all.
    expect(parent!.properties.get(PropertyCode.Parent)).toBe(true);
    expect(parent!.properties.get(PropertyCode.Values)).toBe("[true,false,true]");
    // `readOnly` is not `disabled`: the control keeps its place in the Tab sequence.
    expect(locked!.properties.get(PropertyCode.ReadOnly)).toBe(true);
    expect(locked!.properties.has(PropertyCode.Disabled)).toBe(false);
    expect(radio!.properties.get(PropertyCode.ReadOnly)).toBe(true);
    window.close();
  });

  test("declares a dialog's exit transition and adopts its completion", async () => {
    await app.whenReady();
    let completed: boolean | undefined;
    const window = new Window({
      title: "Dialog viewport",
      renderer: createRenderer(() =>
        createComponent(Dialog.Root, {
          open: true,
          enterDuration: 0,
          exitDuration: 160,
          onOpenChangeComplete: (next: boolean) => {
            completed = next;
          },
          get children() {
            return createComponent(Dialog.Portal, {
              get children() {
                return createComponent(Dialog.Popup, {
                  get children() {
                    return createComponent(Dialog.Viewport, {});
                  },
                });
              },
            });
          },
        }),
      ),
    });

    const portal = window.root.children[0]!;
    expect(portal.properties.get(PropertyCode.EnterDuration)).toBe(0);
    expect(portal.properties.get(PropertyCode.ExitDuration)).toBe(160);
    expect(portal.children[0]!.children[0]!.properties.get(PropertyCode.Part)).toBe(
      "dialog-viewport",
    );

    window._dispatchEvent(
      "componentchange",
      portal.id,
      JSON.stringify({ openChangeComplete: false }),
    );
    expect(completed).toBe(false);
    window.close();
  });
});

describe("Base UI-aligned menus, selects, and comboboxes", () => {
  test("declares the whole menu compound on the trigger and adopts what the core decides", async () => {
    await app.whenReady();
    let open: boolean | undefined;
    let checked: boolean | undefined;
    let density: string | undefined;
    let navigated: string | undefined;
    let state: solid.MenuSurfaceState | undefined;
    const window = new Window({
      title: "Menu",
      renderer: createRenderer(() =>
        createComponent(Menu.Root, {
          scope: "edit",
          open: true,
          modal: true,
          orientation: "vertical",
          loopFocus: false,
          closeParentOnEsc: true,
          side: "top",
          align: "end",
          sideOffset: 8,
          onOpenChange: (next: boolean) => {
            open = next;
          },
          get children() {
            return [
              createComponent(Menu.Trigger, {
                openOnHover: true,
                delay: 120,
                closeDelay: 40,
                children: "Edit",
              }),
              createComponent(Menu.Positioner, {
                collisionPadding: 12,
                get children() {
                  return createComponent(Menu.Popup, {
                    get children() {
                      return [
                        createComponent(Menu.GroupLabel, {
                          value: "clipboard",
                          label: "Clipboard",
                        }),
                        createComponent(Menu.Item, {
                          value: "copy",
                          label: "Copy",
                          closeOnClick: false,
                        }),
                        createComponent(Menu.LinkItem, {
                          value: "docs",
                          label: "Documentation",
                          href: "https://example.invalid/docs",
                          onNavigate: (href: string) => {
                            navigated = href;
                          },
                        }),
                        createComponent(Menu.Separator, {}),
                        createComponent(Menu.CheckboxItem, {
                          value: "wrap",
                          label: "Wrap lines",
                          checked: false,
                          onCheckedChange: (next: boolean) => {
                            checked = next;
                          },
                          get children() {
                            return createComponent(Menu.CheckboxItemIndicator, {});
                          },
                        }),
                        createComponent(Menu.RadioGroup, {
                          name: "density",
                          value: "cozy",
                          onValueChange: (next: string) => {
                            density = next;
                          },
                          get children() {
                            return [
                              createComponent(Menu.RadioItem, {
                                value: "compact",
                                label: "Compact",
                              }),
                              createComponent(Menu.RadioItem, {
                                value: "cozy",
                                label: "Cozy",
                                get children() {
                                  return createComponent(Menu.RadioItemIndicator, {});
                                },
                              }),
                            ];
                          },
                        }),
                      ];
                    },
                  });
                },
              }),
            ];
          },
        }),
      ),
    });

    const trigger = window.root.children[0]!;
    expect(trigger.tag).toBe(NativeNodeTag.Button);
    expect(trigger.properties.get(PropertyCode.Part)).toBe("menu-trigger");
    expect(trigger.properties.get(PropertyCode.Scope)).toBe("edit");
    // The trigger is the one part the core keeps mounted either way, so the whole `Menu.Root`
    // declaration travels on it.
    expect(trigger.properties.get(PropertyCode.Open)).toBe(true);
    expect(trigger.properties.get(PropertyCode.Modal)).toBe(true);
    expect(trigger.properties.get(PropertyCode.LoopFocus)).toBe(false);
    expect(trigger.properties.get(PropertyCode.CloseParentOnEsc)).toBe(true);
    expect(trigger.properties.get(PropertyCode.OpenOnHover)).toBe(true);
    expect(trigger.properties.get(PropertyCode.Delay)).toBe(120);
    expect(trigger.properties.get(PropertyCode.CloseDelay)).toBe(40);
    expect(trigger.properties.get(PropertyCode.Side)).toBe("top");
    expect(trigger.properties.get(PropertyCode.Align)).toBe("end");
    expect(trigger.properties.get(PropertyCode.SideOffset)).toBe(8);
    // The positioner is unmounted while the menu is closed, so its own declaration is routed to
    // the trigger the core always holds.
    expect(trigger.properties.get(PropertyCode.CollisionPadding)).toBe(12);

    const positioner = window.root.children[1]!;
    const popup = positioner.children[0]!;
    expect(positioner.properties.get(PropertyCode.Part)).toBe("menu-positioner");
    expect(popup.properties.get(PropertyCode.Part)).toBe("menu-popup");
    const rows = popup.children;
    expect(rows.map((row) => row.properties.get(PropertyCode.Part))).toEqual([
      "menu-group-label",
      "menu-item",
      "menu-link-item",
      "menu-separator",
      "menu-checkbox-item",
      "menu-radio-group",
    ]);
    const copy = rows[1]!;
    const link = rows[2]!;
    const wrap = rows[4]!;
    const group = rows[5]!;
    expect(copy.properties.get(PropertyCode.PartValue)).toBe("copy");
    expect(copy.properties.get(PropertyCode.AccessibilityLabel)).toBe("Copy");
    expect(copy.properties.get(PropertyCode.CloseOnClick)).toBe(false);
    expect(link.properties.get(PropertyCode.Href)).toBe("https://example.invalid/docs");
    expect(wrap.properties.get(PropertyCode.Checked)).toBe(false);
    expect(group.properties.get(PropertyCode.ActiveValue)).toBe("cozy");
    // Membership stays in the core's model: the group's value decides which row is checked.
    expect(group.children[1]!.properties.get(PropertyCode.Checked)).toBe(true);
    expect(group.children[0]!.properties.get(PropertyCode.Checked)).toBe(false);

    // Everything the core decided arrives as one asynchronous change on the part that owns it.
    window._dispatchEvent(
      "componentchange",
      wrap.id,
      JSON.stringify({ activated: "wrap", checked: true }),
    );
    expect(checked).toBe(true);
    window._dispatchEvent(
      "componentchange",
      link.id,
      JSON.stringify({ activated: "docs", href: "https://example.invalid/docs" }),
    );
    expect(navigated).toBe("https://example.invalid/docs");
    window._dispatchEvent("componentchange", group.id, JSON.stringify({ value: "compact" }));
    expect(density).toBe("compact");
    window._dispatchEvent(
      "componentchange",
      trigger.id,
      JSON.stringify({ open: false, side: "bottom", align: "start", anchorHidden: true }),
    );
    expect(open).toBe(false);
    expect(state).toBeUndefined();
    window.close();
  });

  test("declares a submenu level and the same item parts inside a context menu", async () => {
    await app.whenReady();
    const window = new Window({
      title: "Submenu",
      renderer: createRenderer(() => [
        createComponent(Menu.Root, {
          scope: "file",
          open: true,
          get children() {
            return [
              createComponent(Menu.Trigger, { children: "File" }),
              createComponent(Menu.Portal, {
                get children() {
                  return createComponent(Menu.Popup, {
                    get children() {
                      return createComponent(Menu.SubmenuRoot, {
                        scope: "recent",
                        open: true,
                        get children() {
                          return [
                            createComponent(Menu.SubmenuTrigger, {
                              value: "recent",
                              label: "Open recent",
                              openOnHover: true,
                            }),
                            createComponent(Menu.Positioner, {
                              get children() {
                                return createComponent(Menu.Popup, {
                                  get children() {
                                    return createComponent(Menu.Item, {
                                      value: "notes",
                                      label: "notes.md",
                                    });
                                  },
                                });
                              },
                            }),
                          ];
                        },
                      });
                    },
                  });
                },
              }),
            ];
          },
        }),
        createComponent(ContextMenu.Root, {
          scope: "canvas",
          get children() {
            return createComponent(ContextMenu.Trigger, {
              get children() {
                return [
                  createComponent(Menu.Item, { value: "copy", label: "Copy" }),
                  createComponent(Menu.Item, { value: "paste", label: "Paste" }),
                ];
              },
            });
          },
        }),
      ]),
    });

    // `Menu.SubmenuRoot` is a logical coordinator like `Menu.Root`, so the nested level's trigger
    // and positioner are ordinary rows of the parent popup carrying the child scope.
    const portal = window.root.children[1]!;
    const popup = portal.children[0]!;
    const submenuTrigger = popup.children[0]!;
    expect(submenuTrigger.properties.get(PropertyCode.Part)).toBe("menu-submenu-trigger");
    expect(submenuTrigger.properties.get(PropertyCode.Scope)).toBe("recent");
    expect(submenuTrigger.properties.get(PropertyCode.PartValue)).toBe("recent");
    expect(submenuTrigger.properties.get(PropertyCode.OpenOnHover)).toBe(true);
    expect(submenuTrigger.properties.get(PropertyCode.Open)).toBe(true);
    const nested = popup.children[1]!.children[0]!.children[0]!;
    expect(nested.properties.get(PropertyCode.Part)).toBe("menu-item");
    expect(nested.properties.get(PropertyCode.Scope)).toBe("recent");

    // The same item parts declare a cursor-point context menu's rows.
    const contextTrigger = window.root.children[2]!;
    expect(contextTrigger.properties.get(PropertyCode.Part)).toBe("context-menu-trigger");
    expect(
      contextTrigger.children.map((row) => row.properties.get(PropertyCode.PartValue)),
    ).toEqual(["copy", "paste"]);
    window.close();
  });

  test("declares Base UI's select parts, multiple values, and the map item form", async () => {
    await app.whenReady();
    let values: readonly string[] | undefined;
    const window = new Window({
      title: "Select parts",
      renderer: createRenderer(() =>
        createComponent(Select.Root, {
          scope: "fruit",
          ariaLabel: "Fruit",
          multiple: true,
          values: ["alpha"],
          required: true,
          readOnly: true,
          modal: true,
          alignItemWithTrigger: true,
          filterMode: "startsWith",
          items: { alpha: "Alpha", bravo: "Bravo" },
          onValuesChange: (next: readonly string[]) => {
            values = next;
          },
          get children() {
            return [
              createComponent(Select.Label, { children: "Fruit" }),
              createComponent(Select.Value, {}),
              createComponent(Select.Icon, {}),
              createComponent(Select.Backdrop, {}),
              createComponent(Select.Positioner, {
                side: "top",
                align: "end",
                sideOffset: 12,
                get children() {
                  return [
                    createComponent(Select.ScrollUpArrow, {}),
                    createComponent(Select.List, {
                      get children() {
                        return createComponent(Select.Group, {
                          get children() {
                            return [
                              createComponent(Select.GroupLabel, {
                                children: "Common",
                              }),
                              createComponent(Select.Item, {
                                partValue: "alpha",
                                get children() {
                                  return [
                                    createComponent(Select.ItemText, {
                                      children: "Alpha",
                                    }),
                                    createComponent(Select.ItemIndicator, {}),
                                  ];
                                },
                              }),
                            ];
                          },
                        });
                      },
                    }),
                    createComponent(Select.ScrollDownArrow, {}),
                  ];
                },
              }),
            ];
          },
        }),
      ),
    });

    const trigger = window.root.children[0]!;
    expect(trigger.properties.get(PropertyCode.Part)).toBe("select");
    expect(trigger.properties.get(PropertyCode.Multiple)).toBe(true);
    expect(trigger.properties.get(PropertyCode.Required)).toBe(true);
    expect(trigger.properties.get(PropertyCode.ReadOnly)).toBe(true);
    expect(trigger.properties.get(PropertyCode.Modal)).toBe(true);
    expect(trigger.properties.get(PropertyCode.AlignItemWithTrigger)).toBe(true);
    expect(trigger.properties.get(PropertyCode.FilterMode)).toBe("startsWith");
    expect(trigger.properties.get(PropertyCode.Values)).toBe('["alpha"]');
    // Base UI's map `items` form travels unchanged; the Rust binding decodes both shapes.
    expect(trigger.properties.get(PropertyCode.Options)).toBe(
      JSON.stringify({ alpha: "Alpha", bravo: "Bravo" }),
    );
    // A multiple select declares no single `activeValue`.
    expect(trigger.properties.get(PropertyCode.ActiveValue)).toBeUndefined();

    const parts = trigger.children.map((child) => child.properties.get(PropertyCode.Part));
    expect(parts).toEqual([
      "select-label",
      "select-value",
      "select-icon",
      "select-backdrop",
      "select-positioner",
    ]);
    const positioner = trigger.children[4]!;
    expect(positioner.properties.get(PropertyCode.Side)).toBe("top");
    expect(positioner.properties.get(PropertyCode.SideOffset)).toBe(12);
    expect(positioner.children.map((child) => child.properties.get(PropertyCode.Part))).toEqual([
      "select-scroll-up-arrow",
      "select-list",
      "select-scroll-down-arrow",
    ]);
    for (const child of trigger.children) {
      expect(child.properties.get(PropertyCode.Scope)).toBe("fruit");
    }

    window._dispatchEvent(
      "componentchange",
      trigger.id,
      JSON.stringify({
        selectedValues: ["alpha", "bravo"],
        valueText: "Alpha, Bravo",
        open: false,
        state: { ...{ popupOpen: false, required: true, filled: true } },
      }),
    );
    expect(values).toEqual(["alpha", "bravo"]);
    window.close();
  });

  test("declares Base UI's combobox parts, chips, and filter policy", async () => {
    await app.whenReady();
    let chips: readonly string[] | undefined;
    const window = new Window({
      title: "Combobox parts",
      renderer: createRenderer(() =>
        createComponent(Combobox.Root, {
          scope: "tags",
          ariaLabel: "Tags",
          multiple: true,
          values: ["rust"],
          filterMode: "contains",
          autoHighlight: true,
          openOnInputClick: false,
          highlightItemOnHover: false,
          loopFocus: false,
          readOnly: true,
          required: true,
          items: [{ value: "rust", label: "Rust" }],
          onValuesChange: (next: readonly string[]) => {
            chips = next;
          },
          get children() {
            return [
              createComponent(Combobox.Label, { children: "Tags" }),
              createComponent(Combobox.InputGroup, {
                get children() {
                  return [
                    createComponent(Combobox.Chips, {
                      get children() {
                        return createComponent(Combobox.Chip, {
                          index: 0,
                          get children() {
                            return createComponent(Combobox.ChipRemove, {
                              index: 0,
                              ariaLabel: "Remove Rust",
                            });
                          },
                        });
                      },
                    }),
                    createComponent(Combobox.Clear, {}),
                    createComponent(Combobox.Trigger, {}),
                  ];
                },
              }),
              createComponent(Combobox.Status, {}),
              createComponent(Combobox.Empty, { children: "No results" }),
            ];
          },
        }),
      ),
    });

    const input = window.root.children[0]!;
    expect(input.tag).toBe(NativeNodeTag.Input);
    expect(input.properties.get(PropertyCode.Part)).toBe("combobox");
    expect(input.properties.get(PropertyCode.Multiple)).toBe(true);
    expect(input.properties.get(PropertyCode.FilterMode)).toBe("contains");
    expect(input.properties.get(PropertyCode.AutoHighlight)).toBe(true);
    expect(input.properties.get(PropertyCode.OpenOnInputClick)).toBe(false);
    expect(input.properties.get(PropertyCode.HighlightItemOnHover)).toBe(false);
    expect(input.properties.get(PropertyCode.LoopFocus)).toBe(false);
    expect(input.properties.get(PropertyCode.ReadOnly)).toBe(true);
    expect(input.properties.get(PropertyCode.Required)).toBe(true);
    expect(input.properties.get(PropertyCode.Values)).toBe('["rust"]');

    // A text input paints its own content, so every other part is a sibling that repeats the
    // scope the Rust binding resolves the instance from.
    const label = window.root.children[1]!;
    const inputGroup = window.root.children[2]!;
    expect(label.properties.get(PropertyCode.Part)).toBe("combobox-label");
    expect(label.properties.get(PropertyCode.Scope)).toBe("tags");
    expect(inputGroup.properties.get(PropertyCode.Part)).toBe("combobox-input-group");
    const chipsNode = inputGroup.children[0]!;
    const chip = chipsNode.children[0]!;
    expect(chip.properties.get(PropertyCode.Part)).toBe("combobox-chip");
    expect(chip.properties.get(PropertyCode.ItemIndex)).toBe(0);
    expect(chip.children[0]!.properties.get(PropertyCode.Part)).toBe("combobox-chip-remove");
    expect(window.root.children[3]!.properties.get(PropertyCode.Part)).toBe("combobox-status");
    expect(window.root.children[4]!.properties.get(PropertyCode.Part)).toBe("combobox-empty");

    window._dispatchEvent(
      "componentchange",
      input.id,
      JSON.stringify({ chipValues: ["rust", "zig"], chipLabels: ["Rust", "Zig"] }),
    );
    expect(chips).toEqual(["rust", "zig"]);
    window.close();
  });
});

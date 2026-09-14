/** Extract public Rust constructors and impl methods from the QuickGUI crate. */
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import type { ComponentDoc } from "../src/lib/component-docs";
import type { ApiEntry, ApiSection } from "../src/lib/component-api";

const root = resolve(import.meta.dir, "../..");
const compact = (text: string) => text.replace(/\s+/g, " ").trim();

type Target = {
  file: string;
  fn?: string;
  type?: string;
  ctor?: string;
  extraTypes?: { file: string; type: string }[];
  extraFns?: { file: string; fn: string }[];
  element?: string[];
};

const ELEMENT_METHODS: Record<string, string> = {
  on_click: "src/element/interaction.rs",
  on_input: "src/element/interaction.rs",
  on_pointer: "src/element/interaction.rs",
  disabled: "src/element/state.rs",
  hover: "src/element/state.rs",
  placeholder: "src/element/state.rs",
  transition: "src/element/state.rs",
  child: "src/element/construction_layout.rs",
  children: "src/element/construction_layout.rs",
  id: "src/element/construction_layout.rs",
  flex_col: "src/element/construction_layout.rs",
  flex: "src/element/construction_layout.rs",
  px: "src/element/construction_layout.rs",
  py: "src/element/construction_layout.rs",
  p: "src/element/construction_layout.rs",
  h: "src/element/construction_layout.rs",
  w: "src/element/construction_layout.rs",
  gap: "src/element/construction_layout.rs",
  bg: "src/element/style.rs",
  rounded: "src/element/style.rs",
  text_color: "src/element/style.rs",
  virtual_scroll: "src/element/style.rs",
};

const STYLE = ["bg", "px", "hover"] as const;
const CLICK = ["on_click", "disabled", "child"] as const;
const INPUT = ["on_input", "placeholder", "child"] as const;

const UI: Record<string, Target> = {
  view: { file: "src/element.rs", fn: "div", element: [...STYLE, "child", "on_click", "id"] },
  text: { file: "src/element.rs", fn: "text", element: ["text_color", "child"] },
  button: { file: "src/element.rs", fn: "button", element: [...CLICK, "h", "px", "bg", "rounded", "hover"] },
  input: { file: "src/element.rs", fn: "text_input", element: [...INPUT] },
  "text-area": { file: "src/element.rs", fn: "text_area", element: [...INPUT] },
  markdown: { file: "src/markdown/mod.rs", type: "Markdown" },
  image: { file: "src/image.rs", type: "Image", extraFns: [{ file: "src/element.rs", fn: "img" }] },
  svg: { file: "src/svg.rs", type: "Svg", extraFns: [{ file: "src/element.rs", fn: "svg" }] },
  shader: {
    file: "src/custom_shader.rs",
    type: "CustomShader",
    extraFns: [{ file: "src/element.rs", fn: "custom_shader" }],
  },
  "virtual-list": {
    file: "src/virtual_list.rs",
    type: "VirtualList",
    element: ["virtual_scroll", "children"],
  },
  terminal: { file: "src/terminal/api.rs", type: "Terminal" },
  checkbox: { file: "src/selection_control.rs", type: "Checkbox", extraFns: [{ file: "src/selection_control.rs", fn: "checkbox" }] },
  "checkbox-group": { file: "src/checkbox_group.rs", type: "CheckboxGroup" },
  radio: { file: "src/selection_control.rs", type: "Radio", extraFns: [{ file: "src/selection_control.rs", fn: "radio" }] },
  "radio-group": { file: "src/selection_control.rs", type: "RadioGroup", extraFns: [{ file: "src/selection_control.rs", fn: "radio_group" }] },
  switch: { file: "src/selection_control.rs", type: "Switch", extraFns: [{ file: "src/selection_control.rs", fn: "switch" }] },
  toggle: { file: "src/toggle.rs", type: "Toggle", extraFns: [{ file: "src/toggle.rs", fn: "toggle" }] },
  "toggle-group": { file: "src/toggle.rs", type: "ToggleGroup" },
  slider: { file: "src/slider.rs", type: "Slider", extraTypes: [{ file: "src/slider.rs", type: "SliderThumb" }] },
  "number-field": { file: "src/number_field.rs", type: "NumberField" },
  select: { file: "src/select.rs", type: "SelectState" },
  combobox: { file: "src/constrained_combobox.rs", type: "ComboboxState" },
  autocomplete: { file: "src/autocomplete.rs", type: "AutocompleteState" },
  field: { file: "src/field.rs", type: "Field" },
  fieldset: { file: "src/field.rs", type: "Fieldset" },
  "date-field": { file: "src/date_field.rs", type: "DateField" },
  "time-field": { file: "src/date_field.rs", type: "TimeField" },
  calendar: { file: "src/calendar.rs", type: "Calendar" },
  "otp-field": { file: "src/otp_field.rs", type: "OtpField" },
  tabs: { file: "src/tabs.rs", type: "Tabs" },
  accordion: { file: "src/disclosure.rs", type: "Accordion", extraTypes: [{ file: "src/disclosure.rs", type: "AccordionItem" }] },
  collapsible: { file: "src/disclosure.rs", type: "Collapsible" },
  splitter: { file: "src/splitter.rs", type: "Splitter" },
  "scroll-area": { file: "src/scroll_area.rs", type: "ScrollArea" },
  table: { file: "src/table.rs", type: "TableState" },
  tree: { file: "src/tree.rs", type: "TreeState" },
  separator: { file: "src/separator.rs", fn: "separator", type: "Separator" },
  avatar: { file: "src/avatar.rs", type: "Avatar", extraFns: [{ file: "src/avatar.rs", fn: "avatar" }] },
  progress: { file: "src/progress.rs", type: "Progress", extraFns: [{ file: "src/progress.rs", fn: "progress" }] },
  meter: { file: "src/progress.rs", type: "Meter", extraFns: [{ file: "src/progress.rs", fn: "meter" }] },
  toolbar: { file: "src/toolbar.rs", type: "Toolbar" },
  popover: { file: "src/popover_component.rs", type: "Popover" },
  "system-popover": { file: "src/popover_component.rs", type: "SystemPopover" },
  dialog: { file: "src/dialog.rs", type: "Dialog" },
  "alert-dialog": { file: "src/dialog.rs", type: "Dialog", ctor: "alert" },
  drawer: {
    file: "src/drawer.rs",
    type: "Drawer",
    extraTypes: [{ file: "src/drawer.rs", type: "DrawerState" }],
  },
  tooltip: { file: "src/tooltip.rs", type: "Tooltip" },
  "preview-card": { file: "src/preview_card.rs", type: "PreviewCard" },
  toast: { file: "src/toast.rs", type: "Toast", extraFns: [{ file: "src/toast.rs", fn: "toast_viewport" }] },
  menu: { file: "src/popover_menu.rs", type: "PopoverMenu" },
  "popover-menu": { file: "src/popover_menu.rs", type: "PopoverMenu" },
  "context-menu": { file: "src/context_menu.rs", type: "ContextMenuState" },
  menubar: { file: "src/menubar.rs", type: "Menubar" },
  "navigation-menu": { file: "src/navigation_menu.rs", type: "NavigationMenu" },
  router: { file: "src/router.rs", type: "Router", extraTypes: [{ file: "src/router.rs", type: "RouteDefinition" }] },
};

const SWIFT: Record<string, Target> = {
  host: {
    file: "src/swift_ui.rs",
    type: "MacSwiftUiHost",
    extraFns: [{ file: "src/element.rs", fn: "native_view" }],
  },
  button: { file: "src/swift_ui.rs", type: "SwiftUiButton" },
  slider: { file: "src/swift_ui.rs", type: "SwiftUiSlider" },
  toggle: { file: "src/swift_ui.rs", type: "SwiftUiToggle" },
  "progress-view": { file: "src/swift_ui.rs", type: "SwiftUiProgressView" },
  stepper: { file: "src/swift_ui.rs", type: "SwiftUiStepper" },
  "text-field": { file: "src/swift_ui.rs", type: "SwiftUiTextField" },
  "secure-field": { file: "src/swift_ui.rs", type: "SwiftUiTextField" },
  picker: { file: "src/swift_ui.rs", type: "SwiftUiPicker" },
  "segmented-control": { file: "src/swift_ui.rs", type: "SwiftUiPicker", ctor: "segmented" },
  "date-picker": { file: "src/swift_ui.rs", type: "SwiftUiDatePicker" },
  "color-picker": { file: "src/swift_ui.rs", type: "SwiftUiColorPicker" },
  gauge: { file: "src/swift_ui.rs", type: "SwiftUiGauge" },
  "quickgui-host-view": { file: "src/swift_ui.rs", type: "SwiftUiQuickGuiHost" },
  popover: { file: "src/swift_ui.rs", type: "SwiftUiPopover" },
};

const cache = new Map<string, string>();
function source(file: string): string {
  const existing = cache.get(file);
  if (existing) return existing;
  const text = readFileSync(resolve(root, file), "utf8");
  cache.set(file, text);
  return text;
}

function lineNumber(text: string, index: number): number {
  return text.slice(0, index).split("\n").length;
}

function leadingDocs(text: string, index: number): string {
  const before = text.slice(0, index);
  const lines = before.split("\n");
  const docs: string[] = [];
  for (let i = lines.length - 2; i >= 0; i--) {
    const line = lines[i]!;
    if (/^\s*\/\/\//.test(line)) docs.push(line.replace(/^\s*\/\/\/\s?/, ""));
    else if (/^\s*#\[/.test(line) || /^\s*$/.test(line)) continue;
    else break;
  }
  return compact(docs.reverse().join(" "));
}

function sliceSignature(text: string, start: number): string {
  const brace = text.indexOf("{", start);
  const semi = text.indexOf(";", start);
  const end = brace === -1 ? semi : semi === -1 ? brace : Math.min(brace, semi);
  if (end === -1) throw new Error("Unterminated Rust signature");
  return compact(text.slice(start, end));
}

type Fn = { name: string; signature: string; description: string; source: string; index: number };

function extractFn(file: string, name: string): Fn {
  const text = source(file);
  const re = new RegExp(`pub(?:\\(([^)]+)\\))?\\s+(?:const\\s+)?fn\\s+${name}\\b`, "g");
  let match: RegExpExecArray | null;
  while ((match = re.exec(text))) {
    if (match[1]) continue;
    const start = match.index;
    return {
      name,
      signature: sliceSignature(text, start),
      description: leadingDocs(text, start),
      source: `${file}#L${lineNumber(text, start)}`,
      index: start,
    };
  }
  throw new Error(`Missing Rust fn ${file}::${name}`);
}

function skipWhitespace(text: string, index: number): number {
  while (index < text.length && /\s/.test(text[index]!)) index++;
  return index;
}

function skipGenerics(text: string, index: number): number {
  if (text[index] !== "<") return index;
  let depth = 0;
  for (; index < text.length; index++) {
    if (text[index] === "<") depth++;
    else if (text[index] === ">") {
      depth--;
      if (depth === 0) return index + 1;
    }
  }
  return index;
}

function isInherentImpl(header: string, typeName: string): boolean {
  if (!header.startsWith("impl") || /\bfor\b/.test(header)) return false;
  let index = skipWhitespace(header, 4);
  index = skipGenerics(header, index);
  index = skipWhitespace(header, index);
  return (
    header.startsWith(typeName, index) &&
    !/[A-Za-z0-9_]/.test(header[index + typeName.length] ?? "")
  );
}

function closeBlock(text: string, open: number): number {
  let depth = 1;
  let index = open + 1;
  while (index < text.length && depth > 0) {
    const ch = text[index];
    if (ch === "{") depth++;
    else if (ch === "}") depth--;
    index++;
  }
  return index;
}

function findImplBlocks(text: string, typeName: string): { body: string; bodyStart: number }[] {
  const blocks: { body: string; bodyStart: number }[] = [];
  const re = /^impl\b/gm;
  let match: RegExpExecArray | null;
  while ((match = re.exec(text))) {
    const start = match.index;
    const open = text.indexOf("{", start);
    if (open === -1) break;
    const header = text.slice(start, open);
    if (/\nimpl\b/.test(header.slice(4)) || !isInherentImpl(header, typeName)) {
      re.lastIndex = start + 4;
      continue;
    }
    const end = closeBlock(text, open);
    blocks.push({ body: text.slice(open + 1, end - 1), bodyStart: open + 1 });
    re.lastIndex = end;
  }
  return blocks;
}

function extractImplFns(file: string, typeName: string): Fn[] {
  const text = source(file);
  const blocks = findImplBlocks(text, typeName);
  if (blocks.length === 0) throw new Error(`Missing Rust impl ${file}::${typeName}`);
  const fns: Fn[] = [];
  const seen = new Set<string>();
  const re = /^[ \t]*pub(?:\(([^)]+)\))?\s+(?:const\s+)?fn\s+([A-Za-z0-9_]+)\b/gm;
  for (const block of blocks) {
    re.lastIndex = 0;
    let match: RegExpExecArray | null;
    while ((match = re.exec(block.body))) {
      if (match[1]) continue;
      const name = match[2]!;
      if (seen.has(name)) continue;
      seen.add(name);
      const start = block.bodyStart + match.index + match[0].indexOf("pub");
      fns.push({
        name,
        signature: sliceSignature(text, start),
        description: leadingDocs(text, start),
        source: `${file}#L${lineNumber(text, start)}`,
        index: start,
      });
    }
  }
  if (fns.length === 0) throw new Error(`No public methods on ${file}::${typeName}`);
  return fns;
}

function toEntry(fn: Fn): ApiEntry {
  return {
    name: fn.name,
    type: fn.signature,
    description: fn.description || `Configures ${fn.name.replaceAll("_", " ")} for this part.`,
    source: fn.source,
  };
}

function toSection(name: string, fn: Fn, entries: ApiEntry[]): ApiSection {
  return {
    name,
    signature: fn.signature,
    description: fn.description,
    source: fn.source,
    entries,
  };
}

function uniqueEntries(entries: ApiEntry[]): ApiEntry[] {
  return [...new Map(entries.map((entry) => [entry.name, entry])).values()];
}

function elementEntries(names: string[] = []): ApiEntry[] {
  return names.map((name) => toEntry(extractFn(ELEMENT_METHODS[name]!, name)));
}

function constructorFn(fns: Fn[], preferred?: string): Fn {
  if (preferred) {
    const match = fns.find((fn) => fn.name === preferred);
    if (!match) throw new Error(`Missing preferred constructor ${preferred}`);
    return match;
  }
  return (
    fns.find((fn) => fn.name === "new") ??
    fns.find((fn) => fn.name === "spawn") ??
    fns.find((fn) => fn.name === "from_bytes") ??
    fns.find((fn) => fn.name === "from_rgba") ??
    fns.find((fn) => fn.name === "with_text") ??
    fns[0]!
  );
}

function typeSections(
  file: string,
  typeName: string,
  extra: ApiEntry[] = [],
  preferred?: string,
): ApiSection[] {
  const fns = extractImplFns(file, typeName);
  const ctor = constructorFn(fns, preferred);
  const parts = fns.filter((fn) => fn !== ctor && /-> (?:Element|Option<Element>)(?:\s|$)/.test(fn.signature));
  const rest = fns.filter((fn) => fn !== ctor && !parts.includes(fn));
  const primary = toSection(typeName, ctor, uniqueEntries([...rest.map(toEntry), ...extra]));
  const partSections = parts.map((fn) => toSection(`${typeName}::${fn.name}`, fn, []));
  return [primary, ...partSections];
}

export function rustApi(component: ComponentDoc): ApiSection[] {
  const target = (component.kind === "swift-ui" ? SWIFT : UI)[component.slug];
  if (!target) throw new Error(`Missing Rust API mapping: ${component.kind}/${component.slug}`);
  const sections: ApiSection[] = [];
  if (target.fn) {
    const fn = extractFn(target.file, target.fn);
    sections.push(toSection(target.fn, fn, elementEntries(target.element)));
  }
  if (target.type) {
    const extra = target.fn ? [] : elementEntries(target.element);
    sections.push(...typeSections(target.file, target.type, extra, target.ctor));
  }
  for (const extra of target.extraTypes ?? []) {
    sections.push(...typeSections(extra.file, extra.type));
  }
  for (const extra of target.extraFns ?? []) {
    const fn = extractFn(extra.file, extra.fn);
    sections.push(toSection(extra.fn, fn, []));
  }
  if (sections.length === 0) throw new Error(`No Rust API sections: ${component.kind}/${component.slug}`);
  return sections;
}

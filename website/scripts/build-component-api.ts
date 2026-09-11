import { existsSync, readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { ALL_COMPONENT_DOCS } from "../src/lib/component-docs";
import { propertyNotes } from "./component-property-notes";
import type { ApiEntry, ApiSection, ComponentApi } from "../src/lib/component-api";
import { typescriptApi } from "./typescript-api";
import { rustApi } from "./rust-api";

const root = resolve(import.meta.dir, "../..");
const output = resolve(root, "website/src/lib/generated/component-api.json");
const go = Bun.spawnSync(["go", "run", "./website/scripts/api-extract/main.go"], { cwd: root });
if (go.exitCode) throw new Error(go.stderr.toString());
type Decl = {
  name: string;
  receiver?: string;
  signature: string;
  description: string;
  source: string;
  fields?: ApiEntry[];
};
const declarations: Decl[] = JSON.parse(go.stdout.toString());
const types = new Map(declarations.filter((d) => !d.receiver && d.fields).map((d) => [d.name, d]));
const compact = (s: string) => s.replace(/\s+/g, " ").trim();
const snake = (s: string) =>
  s
    .replace(/([a-z0-9])([A-Z])/g, "$1_$2")
    .replaceAll("-", "_")
    .toLowerCase();
const notes: Record<string, string> = {
  ...propertyNotes,
  children:
    "Content mounted inside the component. Compound child callbacks run after the parent context exists.",
  style:
    "Reusable layout, typography, color, and interaction-state styles. Build with ui.Style() and compose with Merge.",
  ref: "Receives the retained native node after construction. Keep imperative work in event or lifecycle callbacks.",
  disabled:
    "Prevents activation and exposes the disabled state to accessibility. Compound parts inherit a disabled root.",
  read_only:
    "Preserves the current value while keeping the control available for focus and inspection.",
  required: "Marks the control as required for validation and accessibility.",
  value:
    "The current controlled value. Update application state from the change callback to accept a new value.",
  default_value:
    "Initial value for uncontrolled state. Use Value when the application should own subsequent changes.",
  checked:
    "Controlled selection state. A checkbox also accepts the indeterminate state where supported.",
  default_checked: "Initial checked state when no controlled Checked accessor is supplied.",
  open: "Controlled visibility of the panel or overlay. Pair with the open-change callback.",
  default_open: "Initial visibility when the component owns its open state.",
  pressed: "Controlled pressed state of a toggle.",
  default_pressed: "Initial pressed state for an uncontrolled toggle.",
  on_click:
    "Called after QuickGUI resolves an activation, including supported keyboard activation.",
  on_input: "Receives edits from the text control. Write the new text into the controlled value.",
  on_submit: "Called when the control submits its current value.",
  on_value_change:
    "Reports an accepted value change. Use the callback payload to update controlled state.",
  on_checked_change: "Reports the next checked state after user interaction.",
  on_open_change: "Reports a requested visibility change, including dismissal.",
  on_pressed_change: "Reports the next pressed state after activation.",
  on_component_change:
    "Receives the component change payload, including values for coordinated controls.",
  aria_label:
    "Accessible name, especially for icon-only controls or controls without a visible label.",
  role: "Accessibility role override. Prefer the component’s built-in semantics.",
  tab_index:
    "Controls sequential keyboard focus order. Omit to retain the component’s default focus behavior.",
  focus_on_pointer: "Whether a pointer press moves keyboard focus to the control.",
  focusable_when_disabled:
    "Allows a disabled button to remain keyboard focusable while activation stays disabled.",
  group: "Names this element as a group for descendant interaction-state styling.",
  keep_mounted:
    "Keeps hidden content mounted so its retained state can survive visibility changes.",
  orientation: "Direction used for layout and keyboard navigation.",
  min: "Lower bound of the accepted value.",
  minimum: "Lower bound of the accepted value.",
  max: "Upper bound of the accepted value.",
  maximum: "Upper bound of the accepted value.",
  step: "Increment applied by pointer, keyboard, or step controls.",
  placeholder: "Hint displayed while the text value is empty.",
  multiline: "Allows multiple lines in the core text editor.",
  multiple: "Allows more than one selected value.",
  items: "Items made available to the control. Use stable item values to preserve selection.",
  values:
    "Declared values used by this coordinated control. The callback payload follows the component contract.",
  part: "Creates a named component part under its root context. Supply a Slot and the part’s children.",
  part_value:
    "Stable value associated with an item or panel. Matching values connect related parts.",
  part_index: "Index of a repeated part, such as a slider thumb or a segmented input.",
};
// Reuse existing component-specific explanations, while the parser supplies exact types.
for (const component of ALL_COMPONENT_DOCS) {
  const path = resolve(
    root,
    `website/src/content/docs/go/components/${component.kind}/${component.slug}.mdx`,
  );
  const content = readFileSync(path, "utf8");
  for (const match of content.matchAll(/^\| `([A-Za-z]+)` \| (.+?) \|$/gm)) {
    notes[snake(match[1])] ??= match[2];
  }
}
function describe(name: string, description = "") {
  if (description.trim()) return compact(description);
  const key = snake(name)
    .replace(/^bind_/, "")
    .replace(/^swift_ui_/, "");
  if (notes[key]) return notes[key];
  if (key.startsWith("on_"))
    return `Callback for ${key.slice(3).replaceAll("_", " ")}. The signature below lists the event payload.`;
  return `Configures ${key.replaceAll("_", " ")} for this part. The declared type and source define the accepted value.`;
}
function fields(name: string, seen = new Set<string>()): ApiEntry[] {
  if (seen.has(name)) throw new Error(`Recursive props: ${name}`);
  seen.add(name);
  const declared = types.get(name.replace(/^ui\./, ""))?.fields ?? [];
  const inherited = declared.filter((f) => !f.name).flatMap((f) => fields(f.type, new Set(seen)));
  const own = declared
    .filter((f) => f.name)
    .map((f) => ({ ...f, description: describe(f.name, f.description) }));
  return [...new Map([...inherited, ...own].map((f) => [f.name, f])).values()];
}
function goApi(component: (typeof ALL_COMPONENT_DOCS)[number]): ApiSection[] {
  if (component.kind === "swift-ui" && component.slug !== "popover") {
    const d = declarations.find(
      (d) => d.receiver?.toLowerCase() === "swiftuiapi" && d.name === component.name,
    );
    if (!d) throw new Error(`Missing SwiftUI part: ${component.name}`);
    const props = d.signature.match(/\(props (\w+)/)?.[1];
    return [
      {
        name: `SwiftUI.${component.name}`,
        signature: `ui.SwiftUI.${d.signature}`,
        description: d.description,
        source: d.source,
        entries: props ? fields(props) : [],
      },
    ];
  }
  const name =
    component.slug === "terminal"
      ? "terminal.View"
      : component.kind === "swift-ui"
        ? `SwiftUI${component.name}`
        : component.name;
  // Compound API receivers consistently use the component identity followed by API.
  const receiver = name.toLowerCase() + "api";
  const parts = declarations.filter((d) => d.receiver?.toLowerCase() === receiver);
  if (parts.length)
    return parts.map((d) => {
      const props = d.signature.match(/\(props (\w+)/)?.[1];
      return {
        name: `${name}.${d.name}`,
        signature: `ui.${name}.${d.signature}`,
        description: d.description,
        source: d.source,
        entries: props ? fields(props) : [],
      };
    });
  const constructor = declarations.find((d) => !d.receiver && d.name === name && !d.fields);
  if (!constructor) throw new Error(`Missing Go constructor: ${name}`);
  const props = constructor.signature.match(/\((?:props|options) (\w+)/)?.[1];
  const common = [
    "Style",
    "Disabled",
    "AriaLabel",
    "TabIndex",
    "Ref",
    "OnClick",
    "OnKeyDown",
    "OnFocus",
    "OnBlur",
  ];
  const wanted = new Set([
    ...common,
    ...component.keyProps,
    ...(component.slug === "button" ? ["FocusableWhenDisabled", "FocusOnPointer"] : []),
  ]);
  const entries = props
    ? fields(name === "terminal.View" ? `terminal.${props}` : props)
    : declarations
        .filter((d) => d.receiver === "*Element" && wanted.has(d.name))
        .map((d) => ({
          name: d.name,
          type: d.signature,
          description: describe(d.name, d.description),
          source: d.source,
        }));
  return [
    {
      name,
      signature: `${name === "terminal.View" ? "terminal" : "ui"}.${constructor.signature}`,
      description: constructor.description,
      source: constructor.source,
      entries,
    },
  ];
}
const data: Record<string, ComponentApi> = {};
for (const component of ALL_COMPONENT_DOCS)
  for (const frontend of ["go", "typescript", "rust"] as const) {
    const content = readFileSync(
      resolve(
        root,
        `website/src/content/docs/${frontend}/components/${component.kind}/${component.slug}.mdx`,
      ),
      "utf8",
    );
    data[`${frontend}/${component.kind}/${component.slug}`] = {
      language: frontend === "typescript" ? "tsx" : frontend,
      sections:
        frontend === "typescript"
          ? typescriptApi(component)
          : frontend === "rust"
            ? rustApi(component)
            : goApi(component),
      example:
        content
          .split("## Usage")[1]
          ?.match(/```\w*\n([\s\S]*?)```/)?.[1]
          ?.trim() ?? "",
    };
  }
const json = JSON.stringify(data) + "\n";
if (process.argv.includes("--check")) {
  if (readFileSync(output, "utf8") !== json)
    throw new Error("Component API is stale. Run bun run docs:api.");
} else {
  mkdirSync(resolve(output, ".."), { recursive: true });
  if (!existsSync(output) || readFileSync(output, "utf8") !== json) writeFileSync(output, json);
}
console.log(`Component API: ${Object.keys(data).length} frontend pages.`);

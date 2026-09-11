import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { ALL_COMPONENT_DOCS, type ComponentDoc } from "../src/lib/component-docs";

const root = resolve(import.meta.dir, "../..");
const goRoot = resolve(root, "website/src/content/docs/go/components");
const rustRoot = resolve(root, "website/src/content/docs/rust/components");

const snake = (s: string) =>
  s
    .replace(/([a-z0-9])([A-Z])/g, "$1_$2")
    .replaceAll("-", "_")
    .toLowerCase();

function rustPropName(name: string): string {
  if (name.includes("Style()")) return "bg";
  if (name === "children") return "child";
  if (name === "OnClick") return "on_click";
  if (name === "OnInput") return "on_input";
  if (name === "OnPress") return "action";
  if (name === "SystemImage") return "system_image";
  if (name === "MatchContents") return "match_contents";
  if (name === "IsOn") return "is_on";
  if (name === "OnIsOnChange") return "on_value_change";
  if (name === "OnValueChange") return "on_value_change";
  if (name === "OnCheckedChange") return "on_click";
  if (name === "OnOpenChange") return "on_open_change";
  if (name === "OnSelectionChange") return "on_value_change";
  if (name === "IsPresented") return "is_presented";
  if (name === "OnIsPresentedChange") return "on_is_presented_change";
  if (name === "AttachmentAnchor") return "attachment_anchor";
  if (name === "ArrowEdge") return "arrow_edge";
  if (name === "CurrentValueLabel") return "current_value_label";
  if (name === "DisplayedComponents") return "displayed_components";
  if (name === "SupportsOpacity") return "supports_opacity";
  if (name === "WorkingDirectory") return "working_directory";
  if (name === "InitialPath") return "initial_destination";
  return snake(name);
}

function rustType(component: ComponentDoc): string {
  if (component.kind === "swift-ui") {
    if (component.slug === "host") return "MacSwiftUiHost";
    if (component.slug === "quickgui-host-view") return "SwiftUiQuickGuiHost";
    if (component.slug === "segmented-control") return "SwiftUiPicker";
    if (component.slug === "secure-field") return "SwiftUiTextField";
    if (component.slug === "progress-view") return "SwiftUiProgressView";
    return `SwiftUi${component.name}`;
  }
  const names: Record<string, string> = {
    view: "div",
    text: "text",
    button: "button",
    input: "text_input",
    "text-area": "text_area",
    svg: "Svg",
    shader: "CustomShader",
    "virtual-list": "VirtualList",
    "checkbox-group": "CheckboxGroup",
    "radio-group": "RadioGroup",
    "toggle-group": "ToggleGroup",
    "number-field": "NumberField",
    "date-field": "DateField",
    "time-field": "TimeField",
    "otp-field": "OtpField",
    "scroll-area": "ScrollArea",
    "preview-card": "PreviewCard",
    "system-popover": "SystemPopover",
    "alert-dialog": "Dialog",
    "popover-menu": "PopoverMenu",
    "context-menu": "ContextMenuState",
    "navigation-menu": "NavigationMenu",
    menu: "PopoverMenu",
    separator: "separator",
    avatar: "avatar",
    progress: "progress",
    meter: "meter",
  };
  return names[component.slug] ?? component.name;
}

function anatomyName(component: ComponentDoc, part: string): string {
  if (component.slug === "router") {
    return (
      {
        Router: "Router",
        Route: "RouteDefinition",
        Layout: "RouteDefinition::layout",
        Link: "Router::push",
        Outlet: "Router::matched",
      }[part] ?? part
    );
  }
  if (component.kind === "swift-ui" && component.slug === "popover") {
    return part === "Trigger" ? "SwiftUiPopover.trigger" : "SwiftUiPopover.content";
  }
  return `${rustType(component)}::${snake(part)}_part`;
}

const examples: Record<string, string> = {
  "ui/view": `div()\n    .p(20.0)\n    .gap(12.0)\n    .child(text("Hello from Rust"))\n    .child(button().on_click(continue_clicked).child("Continue"))`,
  "ui/text": `text("Hello from Rust").text_lg()`,
  "ui/button": `button()\n    .h(40.0)\n    .px(16.0)\n    .bg(Color::rgb8(37, 99, 235))\n    .text_color(Color::WHITE)\n    .rounded(8.0)\n    .hover(|style| style.bg(Color::rgb8(59, 130, 246)))\n    .on_click(save)\n    .child("Save changes")`,
  "ui/input": `text_input(name.as_str())\n    .placeholder("Name")\n    .on_input(edit)`,
  "ui/text-area": `text_area(notes.as_str())\n    .placeholder("Notes")\n    .on_input(edit)`,
  "ui/markdown": `let mut markdown = Markdown::with_text("Build **native** interfaces.");\nmarkdown.element("note").w(290.0)`,
  "ui/image": `img(Image::from_rgba(128, 128, pixels).expect("image"))\n    .size(160.0, 160.0)\n    .rounded(12.0)`,
  "ui/svg": `svg(Svg::from_bytes(ICON).expect("svg"))\n    .size(72.0, 72.0)\n    .text_color(Color::rgb8(37, 99, 235))`,
  "ui/shader": `custom_shader(\n    CustomShader::new(\n        "fn quickgui_fragment(input: QuickGuiShaderInput) -> vec4<f32> { return vec4<f32>(input.uv.x, 0.3, input.uv.y, 1.0); }",\n    )\n    .expect("shader"),\n)\n.size(260.0, 150.0)`,
  "ui/virtual-list": `div()\n    .h(210.0)\n    .overflow_hidden()\n    .virtual_scroll(&list)\n    .children(list.visible_rows().range.map(|index| text(format!("Row {index}"))))`,
  "ui/terminal": `let terminal = Terminal::spawn(\n    TerminalOptions {\n        program: Some("/bin/zsh".into()),\n        arguments: vec!["-l".into()],\n        working_directory: Some(directory.into()),\n        ..TerminalOptions::default()\n    },\n    cx.window_invalidator(),\n)?;\nterminal.element(cx)`,
  "ui/checkbox": `let checkbox = Checkbox::new(checked);\ncheckbox\n    .root_part(div().flex_row().items_center().gap(8.0).on_click(toggle))\n    .child(checkbox.indicator_part(div().size(16.0, 16.0).rounded(4.0)))\n    .child(text("Remember me"))`,
  "ui/checkbox-group": `let group = CheckboxGroup::new("colors", &state);\ngroup.root_part(div().flex_col().gap(8.0))`,
  "ui/radio": `let radio = Radio::new(selected);\nradio\n    .root_part(div().flex_row().gap(8.0).on_click(select))\n    .child(radio.indicator_part(div().size(16.0, 16.0).rounded(8.0)))\n    .child(text("Option A"))`,
  "ui/radio-group": `RadioGroup::new()\n    .root_part(div().flex_col().gap(8.0))`,
  "ui/switch": `let control = Switch::new(enabled);\ncontrol\n    .root_part(div().w(40.0).h(24.0).rounded(12.0).on_click(toggle))\n    .child(control.thumb_part(div().size(18.0, 18.0).rounded(9.0)))`,
  "ui/toggle": `Toggle::new(pressed)\n    .root_part(button().on_click(toggle).child("Pin this item"))`,
  "ui/toggle-group": `let group = ToggleGroup::new("align", &state, &items);\ngroup.root_part(div().flex_row())`,
  "ui/slider": `let slider = Slider::new("volume", &state);\nslider\n    .root_part(div().flex_col().gap(14.0))\n    .child(text("Volume"))\n    .child(slider.track_part(div().w(240.0).h(24.0)))`,
  "ui/number-field": `let field = NumberField::new("amount", &state);\nfield.root_part(div()).child(field.input_part(text_input(state.text())))`,
  "ui/select": `let mut state = SelectState::new(items);\nstate.open(true);\ndiv().child(text(state.selected_label().unwrap_or("Choose")))`,
  "ui/combobox": `let mut state = ComboboxState::new(items);\nstate.set_query("pro");\ndiv().child(text_input(state.query()))`,
  "ui/autocomplete": `let mut state = AutocompleteState::new(items);\nstate.set_query("qui");\ndiv().child(text(state.query()))`,
  "ui/field": `let field = Field::new("email");\nfield\n    .root_part(div().flex_col().gap(6.0))\n    .child(field.label_part(text("Email")))\n    .child(text_input(value))`,
  "ui/fieldset": `Fieldset::new()\n    .root_part(div().flex_col().gap(12.0))\n    .child(text("Account"))`,
  "ui/date-field": `let field = DateField::new("start");\nfield.root_part(div().flex_row())`,
  "ui/time-field": `let field = TimeField::new("alarm");\nfield.root_part(div().flex_row())`,
  "ui/calendar": `let calendar = Calendar::new("month");\ncalendar.root_part(div().flex_col())`,
  "ui/otp-field": `let field = OtpField::new("code");\nfield.root_part(div().flex_row().gap(8.0))`,
  "ui/tabs": `let tabs = Tabs::new("settings", "general");\ntabs.root_part(div().flex_col()).child(tabs.list_part(div().flex_row()))`,
  "ui/accordion": `let accordion = Accordion::new("faq");\nlet item = accordion.item("one", 0, true);\nitem.root_part(div()).child(item.trigger_part(text("Details")))`,
  "ui/collapsible": `let disclosure = Collapsible::new("notes", open);\ndisclosure.root_part(div()).child(disclosure.trigger_part(text("Notes")))`,
  "ui/splitter": `let splitter = Splitter::new("workspace", &state);\nsplitter.root_part(div().flex_row())`,
  "ui/scroll-area": `let area = ScrollArea::new("log");\narea.viewport_part(div().h(160.0).overflow_hidden())`,
  "ui/table": `TableState::new(rows, columns)\n    .element("issues")`,
  "ui/tree": `TreeState::new(nodes)\n    .element("files")`,
  "ui/separator": `separator(SeparatorOrientation::Horizontal).w(240.0)`,
  "ui/avatar": `avatar("user", "Ada Lovelace").size(40.0, 40.0).rounded(20.0)`,
  "ui/progress": `progress(0.4, 1.0).w(240.0).h(8.0).rounded(4.0)`,
  "ui/meter": `meter(70.0, 0.0, 100.0).w(240.0).h(8.0)`,
  "ui/toolbar": `let bar = Toolbar::new("main", &state, &items);\nbar.root_part(div().flex_row().gap(8.0))`,
  "ui/popover": `let popover = Popover::new("help", open);\npopover.root_part(div()).child(popover.trigger_part(button().child("Help")))`,
  "ui/system-popover": `SystemPopover::new(320.0, 240.0)`,
  "ui/dialog": `let dialog = Dialog::new("save", open);\ndialog.root_part(div()).child(dialog.title_part(text("Save changes")))`,
  "ui/alert-dialog": `let dialog = Dialog::alert("quit", open);\ndialog.root_part(div()).child(dialog.title_part(text("Quit without saving?")))`,
  "ui/tooltip": `let tooltip = Tooltip::new("hint");\ntooltip.trigger_part(button().child("Save"))`,
  "ui/preview-card": `let card = PreviewCard::new("file", &state);\ncard.trigger_part(text("README.md"))`,
  "ui/toast": `toast_viewport("toasts").child(text("Saved"))`,
  "ui/menu": `let menu = PopoverMenu::new("actions", &state);\nmenu.root_part(div()).child(menu.trigger_part(button().child("Actions")))`,
  "ui/popover-menu": `let menu = PopoverMenu::new("edit", &state);\nmenu.root_part(div()).child(menu.trigger_part(button().child("Edit")))`,
  "ui/context-menu": `ContextMenuState::new()\n    .trigger_part(div().child(text("Right-click me")))`,
  "ui/menubar": `let bar = Menubar::new("app");\nbar.root_part(div().flex_row())`,
  "ui/navigation-menu": `let menu = NavigationMenu::new("docs", &state, &items);\nmenu.root_part(div().flex_row())`,
  "ui/router": `let mut router = Router::new(\n    [\n        RouteDefinition::new("home", "/"),\n        RouteDefinition::new("settings", "/settings"),\n    ],\n    "/",\n)?;\nmatch router.matched().and_then(|matched| matched.route_ids().last().map(|id| id.as_ref())) {\n    Some("settings") => text("Settings"),\n    _ => button().on_click(open_settings).child("Settings"),\n}`,
  "swift-ui/host": `let mut host = MacSwiftUiHost::new(|_id| {})?;\nhost.sync(&[SwiftUiButton::new(1).label("Continue").into()])?;\nnative_view(host.view())`,
  "swift-ui/button": `SwiftUiButton::new(1)\n    .label("Save")\n    .system_image("checkmark")\n    .style(SwiftUiButtonStyle::BorderedProminent)`,
  "swift-ui/slider": `SwiftUiSlider::new(1, volume)\n    .range(0.0, 100.0)\n    .step(1.0)\n    .label("Volume")`,
  "swift-ui/toggle": `SwiftUiToggle::new(1, enabled).label("Notifications")`,
  "swift-ui/progress-view": `SwiftUiProgressView::new(1, 0.4, 1.0).label("Downloading")`,
  "swift-ui/stepper": `SwiftUiStepper::new(1, count).range(0.0, 10.0).step(1.0).label("Copies")`,
  "swift-ui/text-field": `SwiftUiTextField::new(1, name).placeholder("Name")`,
  "swift-ui/secure-field": `SwiftUiTextField::new(1, secret).placeholder("Password").secure(true)`,
  "swift-ui/picker": `SwiftUiPicker::new(1, "space", [SwiftUiPickerOption::new("space", "Space")])\n    .label("Theme")`,
  "swift-ui/segmented-control": `SwiftUiPicker::segmented(\n    1,\n    "code",\n    [SwiftUiPickerOption::new("code", "Code"), SwiftUiPickerOption::new("preview", "Preview")],\n)`,
  "swift-ui/date-picker": `SwiftUiDatePicker::new(1, timestamp).label("Start")`,
  "swift-ui/color-picker": `SwiftUiColorPicker::new(1, "#2563eb").label("Accent")`,
  "swift-ui/gauge": `SwiftUiGauge::new(1, 0.7).label("Battery")`,
  "swift-ui/quickgui-host-view": `SwiftUiQuickGuiHost::pending(1)`,
  "swift-ui/popover": `SwiftUiPopover::new(1)`,
};

function defaultExample(component: ComponentDoc): string {
  if (component.kind === "swift-ui") {
    return `${rustType(component)}::new(1)`;
  }
  if (component.parts.includes("Root")) {
    return `${rustType(component)}::new("${component.slug}").root_part(div())`;
  }
  return `${snake(component.name)}().child("${component.name}")`;
}

function rustMdx(component: ComponentDoc): string {
  const family = component.kind === "swift-ui" ? "swift-ui" : "ui";
  const go = readFileSync(resolve(goRoot, family, `${component.slug}.mdx`), "utf8");
  const description = go.split("\n\n")[0]!.trim();
  const example = examples[`${component.kind}/${component.slug}`] ?? defaultExample(component);
  const importName = rustType(component);
  const props = [...go.matchAll(/^\| `([^`]+)` \| (.+?) \|$/gm)].filter(
    (match) => match[1] !== "Prop",
  );
  const anatomy = component.parts.length
    ? `\n## Anatomy\n\n${component.parts.map((part) => `- \`${anatomyName(component, part)}\``).join("\n")}\n`
    : "";
  const table = props
    .map(([, name, purpose]) => `| \`${rustPropName(name!)}\` | ${purpose} |`)
    .join("\n");
  return `${description}

## Import

Import \`${importName}\` from \`quickgui\`.

\`\`\`rust
use quickgui::*;
\`\`\`

## Usage

\`\`\`rust
${example}
\`\`\`
${anatomy}
## Key props

| Prop | Purpose |
| --- | --- |
${table}
`;
}

let written = 0;
for (const component of ALL_COMPONENT_DOCS) {
  const family = component.kind === "swift-ui" ? "swift-ui" : "ui";
  const destination = resolve(rustRoot, family, `${component.slug}.mdx`);
  mkdirSync(resolve(destination, ".."), { recursive: true });
  writeFileSync(destination, rustMdx(component));
  written += 1;
}
console.log(`Wrote ${written} English Rust component MDX files`);

# TypeScript with Bun and Solid 2

The TypeScript language uses Bun, Solid 2, and the same Rust renderer as Go. Bun calls the shared library through `bun:ffi` in the same process. JSX produces retained native nodes without a DOM or WebView.

## Start a project

```sh
quickgui init my-app --language typescript
cd my-app
bun run dev
```

In this source checkout, run `bun install` and `bun run build:native` once, then:

```sh
bun packages/cli/src/cli.ts dev --project examples/counter-typescript
```

Use Bun 1.4 or later. `solid-js`, `@solidjs/compiler`, and `@solidjs/universal` are pinned to `2.0.0-rc.8`. Solid 2 is a release candidate, and Bun labels its FFI API experimental. TypeScript targets the repository's macOS workflow; Windows/Linux native runtime acceptance remains outstanding.

## Components and windows

```tsx
import { createSignal } from "solid-js";
import { app, Window } from "@quickgui/native";
import { Button, Text, View, createRenderer } from "@quickgui/solid";

function Counter() {
  const [count, setCount] = createSignal(0);
  return (
    <View style={{ display: "flex", flexDirection: "column", padding: 24, gap: 12 }}>
      <Text style={{ fontSize: 24 }}>Count: {count()}</Text>
      <Button onClick={() => setCount(count() + 1)}>Increment</Button>
    </View>
  );
}

function openWindow() {
  new Window({ title: "Counter", width: 640, height: 460, renderer: createRenderer(Counter) });
}
app.on("reopen", ({ hasVisibleWindows }) => {
  if (!hasVisibleWindows) openWindow();
});
await app.whenReady();
openWindow();
```

`<View>some text</View>` accepts plain text directly. Intrinsic `<div>` and `<span>` are aliases for `View` and `Text`, and need no component import. Put all visual declarations in a camelCase `style` object:

```tsx
<div style={{ flexCol: true, gap2: true }}>
  some text
  <span style={{ textLg: true, fontSemibold: true }}>Styled text</span>
</div>
```

The JSX compiler supplies its runtime imports using the project's `jsxImportSource` setting. `JSX.Style` provides editor completion for native values and fixed presets. Direct style attributes, `class`, and `className` are unsupported.

Components construct once. Solid tracks JSX expressions and updates affected native properties or child edges. Use Solid 2's `Show`, `For`, signals, effects, and cleanup APIs. Each window has an independent Solid root, disposed when the native window closes. The Dock handler creates a fresh window after the last window closes on macOS.

The TypeScript binding exposes the complete native component families:

- Primitives: `View`, `Text`, `Button`, `Input`/`TextInput`, `TextArea`, `Image`, `Svg`, `Shader`, `VirtualList`, and optional `Terminal`, `Markdown`, `Editor`, `CodeBlock`, and `DiffView`.
- Forms: `Field`, `Fieldset`, `Checkbox`, `CheckboxGroup`, `Radio`, `RadioGroup`, `Switch`, `NumberField`, `OtpField`, `Select`, `Combobox`, and `Autocomplete`.
- Layout and data: `Tabs`, `Accordion`, `Collapsible`, `Separator`, `Splitter`, `ScrollArea`, `Table`, `Tree`, `Calendar`, `DateField`, and `TimeField`.
- Menus and overlays: `Menu`, `Menubar`, `ContextMenu`, `NavigationMenu`, `PopoverMenu`, `Popover`, `SystemPopover`, `Dialog`, `AlertDialog`, `Tooltip`, `PreviewCard`, and `Drawer`.
- Feedback: `Avatar`, `Progress`, `Meter`, `Toggle`, `ToggleGroup`, `Toolbar`, and `Toast`.

Compound components expose their named parts and typed state hooks. Rust owns selection, keyboard navigation, focus, layout, placement, and interaction; Solid owns construction and reactive bindings. The [components gallery](../examples/components-typescript/app.tsx) demonstrates the complete control set.

`@quickgui/solid/router` supplies routes, nested outlets, links, and navigation hooks backed by Rust route matching and memory history. `@quickgui/solid/swift-ui` exposes native controls, popovers, and reverse QuickGUI hosting; typed modifier factories are in `@quickgui/solid/swift-ui/modifiers`.

Use `style` for all layout, typography, and paint declarations. Style keys are camelCase. Fixed presets are boolean entries in the same object:

```tsx
<View style={{ flexCol: true, p3: true, gap2: true, roundedLg: true }}>
  <Text style={{ textLg: true, fontSemibold: true }}>Panel title</Text>
</View>
```

Preset entries include `roundedLg`, `flexCol`, `itemsCenter`, `p3`, `textLg`, and `fontBold`. `roundedLg` uses an 8-pixel radius; `p3` uses 12 pixels from the four-pixel spacing scale. Presets take `true`, `false`, `null`, or `undefined`. Custom values use fields such as `borderRadius`, `padding`, `gridTemplateColumns`, and `width`. `positionSticky` is the sticky-layout preset.

Styles and nested style arrays merge left to right. A falsey array entry is ignored, and a falsey preset entry withdraws that preset so an earlier value remains visible. `bun scripts/generate-style-helpers.ts --check` checks all 272 TypeScript presets against Rust.

```tsx
import type { JSX } from "@quickgui/solid";

const panel = { p3: true, roundedXl: true, bg: "#18181b" } satisfies JSX.Style;
<View style={[panel, { textColor: "#fafafa" }]}>
  <Text>Hello</Text>
</View>;
```

Removing a style field clears its native value. Colors accept CSS hex forms or an integer packed as `0xAABBGGRR`, matching Rust. Component state, accessibility, and event handlers remain ordinary props. Property IDs and bounds are generated from Rust with `bun scripts/generate-typescript.ts`.

Input handlers receive native events, with text in `event.value`:

```tsx
<TextInput value={name()} onInput={(event) => setName(event.value ?? "")} />
```

`Window.close()`, `Window.setTitle()`, `Window.getState()`, `app.quit()`, and `app.exit()` use the native host. Operations returning values are asynchronous. `Window.whenReady()` resolves when Rust has mounted the window, including hidden windows; snapshot getters await that event; setters queue their native work until creation. `app.command()` exposes existing native JSON commands for advanced use.
Native window snapshots use `bounds`, `viewportSize`, and `scaleFactor`. Observe close completion with `window.onClose()` or `window.on("closed", ...)`. Menus, clipboard, file and alert dialogs, display information, appearance, notifications, global shortcuts, tray icons, permissions, power assertions, secure storage, and metrics are available from `@quickgui/native`.

Declare optional extensions with `extensions: ["terminal"]` or `["updater"]`, and install their corresponding native packages. `ExtensionSession` and `invokeExtension` expose other extension services through the shared C ABI. Import `Updater` from `@quickgui/extension-updater`; the [TypeScript updater guide](../website/src/content/docs/typescript/en/updater.mdx) shows installation, a complete `quickgui.config.ts`, application startup, and signed release commands.

Install the optional code surfaces with `bun add @quickgui/extension-editor`. `Editor`,
`CodeBlock`, and `DiffView` need no config entry or second native image. Rust keeps their
implementation behind its `editor` crate feature; the hosted renderer enables the adapter
explicitly. See [editor, CodeBlock, and diff view](editor-and-diffs.md).

Install retained Markdown with `bun add @quickgui/extension-markdown`. Parsed code fences can opt
into the shared CodeBlock renderer without adding another native image. See [retained
Markdown](markdown.md).

The [Quick Git example](../examples/quick-git-typescript/README.md) includes changes and history, partial staging, branches, stashes, worktrees, native menus and dialogs, independent per-window stores, and cancellable Bun subprocesses. Its interface follows the current Go example, including native splitters, the QuickGUI toolbar and composer, virtualized commit files, and history selection that survives focus refresh.

## Build and check

Set `language: "typescript"` in `quickgui.config.ts`, or `language = "typescript"` in TOML. The previous `frontend` name remains accepted as an alias. The default entry is `app.tsx`. Use `jsx: "preserve"` and `jsxImportSource: "@quickgui/solid"` in TypeScript configuration, as in the scaffold.

```sh
bun run check
bun run test
bun run build
```

`quickgui check` runs the project's TypeScript compiler. `quickgui test` preloads the same Solid compiler and reactive client runtime as builds. `quickgui fmt` uses the project's `oxfmt`. Production builds embed Bun and both host/worker entrypoints in one executable and bundle the native library alongside it. App edits rebuild only the executable.

macOS signing includes Bun's JIT and unsigned-executable-memory entitlements by default. If you supply `macos.entitlements`, include those keys in your file. Developer ID signing/notarization and Mac App Store distribution require their own platform acceptance.

## Runtime ownership

The host waits for the worker startup handshake before entering the native loop on the process main thread. The worker owns Solid, handlers, promises, timers, and application I/O. Both isolates load the same Rust library; UI commands enter the existing bounded host queue directly through FFI.

Rust copies native spans into a queue bounded to 8,192 events and 32 MiB. A coalesced, zero-argument thread-safe callback wakes the worker, which drains events with synchronous callbacks that copy data before returning. No borrowed pointer survives a callback, and there is no polling timer. Overflow fails the application explicitly. Shutdown disposes windows and pending requests before terminating the worker.

Framework checks: `bun run typecheck:js`, `bun run test:js`, `bun run test:typescript`, `bun scripts/generate-typescript.ts --check`, and `scripts/with-macos-ghostty-zig.sh cargo test -p quickgui-host --lib`.

After staging the native library, `bun scripts/check-typescript.ts` checks compiled development and production workers against real native windows, asynchronous replies, and shutdown. Its windows remain hidden.

## Complete examples and reference

- [Components gallery](../examples/components-typescript/README.md): 44 demos, including all Go gallery entries and Drawer.
- [Quick Git](../examples/quick-git-typescript/README.md): changes, partial staging, history, branches, stashes, worktrees, and multiple windows.
- The website exposes `/docs/typescript` and 69 component API pages, with source-derived signatures and TypeScript examples.

Run `bun scripts/check-typescript-examples.ts` for native lifecycle/content validation of both applications.

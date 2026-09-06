# TypeScript UI

QuickGUI compiles TypeScript and TSX to native code with scriptc. The Rust core owns the main
application thread, retained trees, layout, rendering, input, accessibility, and platform services.
Application code runs on a separate native thread and communicates with the main thread through
bounded command and event queues.

- `@quickgui/native` exposes windows and platform APIs through a C ABI.
- `@quickgui/ui` supplies signals, JSX primitives, component bindings, and renderer ownership.
- `@quickgui/cli` uses TypeScript 7 to lower JSX, then links the compiled application and Rust host.

A cgo-free [Go frontend](go.md) sits beside this pipeline, loads the same host as a shared
library at runtime, and exposes the same signals and unstyled components.

Bun is development tooling. Applications do not embed Bun, Node.js, a webview, or a JavaScript
engine. scriptc supports a subset of TypeScript and Node-compatible APIs; arbitrary npm packages
and Bun application APIs are not supported. Native application builds currently require a matching
macOS 14+ arm64 or x64 host, Node.js 24 or later, and Xcode Command Line Tools.

## First window

```console
bunx @quickgui/cli init my-app
cd my-app
bun run dev
```

```tsx
import { app, Window, type NativeNode } from "@quickgui/native";
import { Button, Text, View, createRenderer, createSignal } from "@quickgui/ui";

function Counter(): NativeNode {
  const [count, setCount] = createSignal(0);
  return (
    <View style={{
      display: "flex", flexDirection: "column", width: "100%", height: "100%",
      alignItems: "center", justifyContent: "center", gap: 12,
    }}>
      <Text>Count: {count()}</Text>
      <Button onClick={() => setCount(count() + 1)}>Increment</Button>
    </View>
  );
}

function openMainWindow(): void {
  new Window({ title: "Counter", width: 480, height: 320, renderer: createRenderer(() => <Counter />) });
}

app.onReopen((event) => {
  if (!event.hasVisibleWindows) openMainWindow();
});
await app.whenReady();
openMainWindow();
```

Await readiness before constructing windows. The constructor allocates a stable handle and queues
native creation. Closing a window disposes its renderer root. With the default quit mode, macOS
keeps the app resident after its last window closes, so the reopen callback can create a window.

`Window.getCurrentWindow()` returns the window associated with rendering or event dispatch. Read
it during component setup and retain it for asynchronous work. Window state queries such as
`await window.getState()` return Promises; application code never waits synchronously for AppKit.

## Signals and component props

```tsx
const [count, setCount] = createSignal(0);
const doubled = createMemo(() => count() * 2);
createEffect(() => console.log(doubled()));
onCleanup(() => subscription.close());
```

Import these functions from `@quickgui/ui`. A setter accepts a value: `setCount(count() + 1)`.
`batch` groups writes, `untrack` reads without subscribing, and `createRoot` creates an explicit
lifetime. Effects track the signals they read and release their previous children and cleanups
when they rerun. A component runs once; effects update its existing native nodes.

JSX is lowered using the TypeScript 7 checker. Host properties and text expressions that read
signals become reactive setters. For your own components, declare changing values as accessors:

```tsx
import { Text, type Accessor, type NativeNode } from "@quickgui/ui";

function CountLabel(props: { count: Accessor<number> }): NativeNode {
  return <Text>{props.count()}</Text>;
}

// JSX wraps the value in an accessor because CountLabel declares one.
<CountLabel count={count()} />;
```

A plain `count: number` prop is a setup-time value. When calling a component directly instead of
through JSX, pass the accessor yourself: `CountLabel({ count })`. Callback props remain callbacks;
`children` for a compound component is a function returning a native node. Avoid runtime property
getters, dynamic object shapes, and dependencies that require a JavaScript engine.

## Conditional and list content

`Show`, `For`, `KeyedFor`, and `Index` come from `@quickgui/ui`.

```tsx
<Show when={visible()} fallback={() => <Text>Hidden</Text>}>
  {() => <Text>Visible</Text>}
</Show>

<For each={names()}>{(name) => <Text>{name}</Text>}</For>

<KeyedFor each={records()} key={(record) => record.id}>
  {(record) => <Text>{record().title}</Text>}
</KeyedFor>
```

`For` passes an item value and an index accessor. An optional `key` matches items during reorders;
replacing an item with a new object recreates that row. `KeyedFor` requires a string or number key
and passes the item as an accessor, so immutable updates keep the same row and update its content.
`Index` retains rows by position and also exposes an item accessor. Removed rows dispose their
owned effects and cleanup callbacks.

## Styling and input

Host primitives are `View`, `Text`, `Button`, `Input`, `TextArea`, `Markdown`, `VirtualList`,
`Terminal`, `Image`, `Svg`, and `Shader`. They are unstyled. Styles use typed camelCase properties:

```tsx
<Button
  disabled={busy()}
  onClick={() => save()}
  style={{
    padding: 12, borderRadius: 8, backgroundColor: "#2563eb", color: "#ffffff",
    hover: { backgroundColor: "#1d4ed8" },
    disabled: { opacity: 0.5 },
    focus: { outline: "2px solid #60a5fa" },
  }}
>
  Save
</Button>
```

Top-level `disabled` controls behavior; `style.disabled` declares the corresponding paint state.
The same distinction applies to `selected`, `checked`, and other state selectors. Native events
arrive asynchronously with the core's decision. Event payloads may be absent; check optional values
before using them. Compound components offer typed callbacks such as `onValueChange`.

See [layout](view-api.md), [input](input.md), [graphics](graphics.md), and the runnable
[styling example](../examples/styling) for the Rust-backed property model.

## Compound components

Import component families from their subpaths. Roots establish their own scope, so independent
instances do not need application-generated scope IDs.

| Module | Components |
| --- | --- |
| `@quickgui/ui/controls` | Checkbox, CheckboxGroup, Radio, RadioGroup, Switch, Toggle, ToggleGroup |
| `@quickgui/ui/tabs` | Tabs |
| `@quickgui/ui/disclosure` | Accordion, Collapsible |
| `@quickgui/ui/field` | Field, Fieldset |
| `@quickgui/ui/dialog` | Dialog, AlertDialog |
| `@quickgui/ui/popover` | Popover, SystemPopover |
| `@quickgui/ui/select` | Select, Combobox, Autocomplete |
| `@quickgui/ui/collections` | Table, Tree |
| `@quickgui/ui/range` | Slider, Splitter |
| `@quickgui/ui/gauges` | Progress, Meter |
| `@quickgui/ui/number-field` | NumberField |
| `@quickgui/ui/date-time` | Calendar, DateField, TimeField |
| `@quickgui/ui/toolbar` | Toolbar, Separator |
| `@quickgui/ui/menu` | Menu, Menubar, PopoverMenu, ContextMenu |
| `@quickgui/ui/tooltip` | Tooltip, PreviewCard |
| `@quickgui/ui/toast` | Toast |
| `@quickgui/ui/base-ui` | Avatar, ScrollArea, OtpField, NavigationMenu |

The [components example](../examples/components) demonstrates the bindings with caller-defined
styles. Calendar day cells inherit the root scope, and changes update the root's selected date.

## Routing

The route table, matching, parameters, queries, and bounded memory history live in Rust. Declare
a static route array with `route` or `layout`, and render it with `Router`:

```tsx
import { Link, Outlet, Router, route, useParams } from "@quickgui/ui/router";
import { Text, View } from "@quickgui/ui";

function Shell() {
  return <View><Link href="/" end>Home</Link><Outlet /></View>;
}
function Home() { return <Text>Home</Text>; }
function Project() {
  const params = useParams();
  return <Text>Project: {params().get("id") ?? ""}</Text>;
}

<Router initialPath="/" routes={[
  route("/", Shell, [route("", Home), route("projects/:id", Project)]),
]} />;
```

Parameters and search parameters are `Map<string, string>` accessors. Repeated query names keep
the last value in the UI map. `useRouter()` exposes navigation and a reactive state snapshot;
`useNavigate()` provides a navigation callback. Layouts place the matched child with `Outlet`.
See the [routing example](../examples/routing).

## Platform APIs

Import platform APIs from `@quickgui/native`. For native dialogs, options come first and the
optional owning window comes second:

```ts
const result = await Dialog.showOpenDialog({
  title: "Open project", properties: ["openDirectory"],
}, window);
if (!result.canceled) console.log(result.filePaths);
```

Native menus, file panels, clipboard, secure storage, notifications, and window queries complete
through Promises. `Menu.popup(items, { window, x, y })` retains callbacks until the menu closes.
CPU-only services, such as route matching and explicit synchronous Zig functions, do not dispatch
to the native main thread.

`FileWatcher.start(absolutePaths, listener)` asynchronously registers recursive Rust-backed
watchers. Events contain `paths`, `rescan`, and an optional `error`; coalesce changes in the
application and call `await watcher.close()` when finished. Quick Git demonstrates refresh and
shutdown handling.

## SwiftUI

Import `Host` and controls from `@quickgui/ui/swift-ui`, and modifier factories from
`@quickgui/ui/swift-ui/modifiers`. Use aliases when also importing QuickGUI's `Button` or `Text`.

```tsx
import { Host, Slider } from "@quickgui/ui/swift-ui";
import { controlSize } from "@quickgui/ui/swift-ui/modifiers";

<Host matchContents>
  <Slider value={volume()} min={0} max={1} onValueChange={setVolume} modifiers={[controlSize("regular")]} />
</Host>;
```

`DatePicker` values are Unix timestamps in milliseconds. SwiftUI popovers use `Popover.Root`,
`Popover.Trigger`, and `Popover.Content`. `QuickGUIHostView` embeds a separately owned QuickGUI
renderer in SwiftUI and queues native creation asynchronously. See the [SwiftUI example](../examples/swift-ui).

## Development and distribution

`quickgui dev` watches source and compiles a candidate app, then replaces the running process only
after the candidate's first native window is ready. `--once --no-launch` compiles without opening
a GUI. `quickgui build` emits a signed `.app` and `.dmg` for the current Mac architecture.

The additional TypeScript diagnostic pass is off by default in development. Set
`native: { typeCheck: true }` in `quickgui.config.ts` to enable it; JSX lowering always uses the
TypeScript 7 checker. See [the CLI guide](cli.md), [native modules](native-modules.md), and the
[AI Chat](../examples/ai-chat), [Herdr](../examples/herdr-gui), and [Quick Git](../examples/quick-git)
applications for larger examples.

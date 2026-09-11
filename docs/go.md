# Go applications

QuickGUI applications are ordinary Go programs. `purego` loads the prebuilt Rust shared library in the same process, with `CGO_ENABLED=0`. The Rust core owns layout, rendering, accessibility, native controls, windows, and platform services. Go owns application state and the fine-grained reactive graph. The TypeScript CLI handles development and packaging.

## Components

Components return a retained native node: `*ui.Element` for the fluent builder or `*native.Node` for lower-level code. Pass a root directly as `WindowOptions.Component`, and compose children as `ui.View(Child(...), ui.Text(count()))`. The compiler recognizes the return type and keeps props and native view expressions reactive without a children callback. Optional callbacks remain available for deferred construction and controls that establish child context.

```go
package main

import (
	"log"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
)

func main() {
	if err := native.Run(func() {
		open := func() {
			native.NewWindow(native.WindowOptions{
				Title:     "Counter",
				Width:     760,
				Height:    520,
				Component: Counter,
			})
		}
		native.App.OnReopen(func(event native.ReopenEvent) {
			if !event.HasVisibleWindows {
				open()
			}
		})
		open()
	}); err != nil {
		log.Fatal(err)
	}
}

func Counter() *ui.Element {
	count, setCount := ui.CreateSignal(0)
	return ui.View(
		ui.Text("Fine-grained native UI").FontSize(28).FontWeight(700),
		ui.Text("Count: ", count()),
		ui.Button("Increment").
			OnClick(func() { setCount(count() + 1) }).
			Padding(12).
			RoundedLg().
			Bg("#2563eb").
			Hover(func(s ui.StyleBuilder) ui.StyleBuilder { return s.BackgroundColor("#3b82f6") }),
	).FlexCol().
		SizeFull().
		ItemsCenter().
		JustifyCenter().
		Gap(20).
		Bg("#090d16").
		TextColor("#e2e8f0")
}
```

Components and deferred construction callbacks return a node and run once when mounted. UI nodes are used explicitly: return them, assign them, or pass them as children. Discarded construction calls and `func()` declaration blocks are rejected. Ordinary `if` and `for` statements are useful for static construction. Reading a signal inside an accessor subscribes that binding; setting it changes the affected native properties or text nodes. Event handlers batch writes automatically. Use `ui.Batch` to group writes outside an event. Pass string or numeric accessors directly as children: `ui.Text("Count: ", count)` retains the prefix and updates only the number. Numbers use typed `strconv` conversions. Use direct reads such as `ui.Text(count())` and `CounterLabel(count())` when building with QuickGUI. The compiler preserves their reactivity. A setup assignment such as `initial := count()` remains a snapshot. Use `strconv` and concatenation when a property needs one combined string, such as an input value or accessibility label.

Pass children or content to constructors, then chain properties, styles, and handlers:

```go
return ui.View(
	ui.Text("hello"),
	ui.Input().Value("xxx").OnInput(func(value string) { /* save value */ }),
).Flex().PaddingLeft(20).TextAlign("center").RoundedLg()
```

Go primitives return `*ui.Element`. `.Value`, `.Width`, `.PaddingLeft`, and the other modifiers return the same retained element. Accessors remain fine-grained bindings; later declarations replace earlier values without remounting children. `.OnClick(func() { ... })` handles clicks, and `.OnInput(func(value string) { ... })` receives text. Use `.OnClickEvent`, `.OnInputEvent`, or `.OnSubmitEvent` for the full native event. `.Style(shared)` applies a reusable style built with `ui.Style()`.

Rust's layout helpers are available with Go names: `.FlexCol()`, `.FlexRowReverse()`, `.Flex1()`, `.ItemsCenter()`, `.JustifyBetween()`, `.GridCols(3)`, `.ColSpanFull()`, `.P4()`, `.MxAuto()`, `.SizeFull()`, `.RoundedLg()`, and `.TextSm()`. The generator checks 325 layout and style conveniences against Rust, including its four-pixel spacing scale and radius values. `.Flex()` selects flex display; `.Flex1()` sets grow 1, shrink 1, and zero basis.

Use `ui.Style().Padding(12).BorderRadius(8)` to share styles or fill a compound control’s `PartProps.Style` field. It returns a reusable `ui.StyleBuilder` value; fluent modifiers and `.Merge(other)` return new values without mutating it. Scalar accessors are bound independently when mounted, without rerunning the component.

```go
var children []*native.Node
card := ui.Style().Padding(20).RoundedLg().
	Hover(func(s ui.StyleBuilder) ui.StyleBuilder { return s.Bg("#1e293b") })
children = append(children, ui.View("Hello").Style(card).Node)
children = append(children, ui.View("Another card").Style(card.PaddingLeft(28)).Node)
return ui.Fragment(children)
```

Builder `.When(condition, func(s ui.StyleBuilder) ui.StyleBuilder { ... })` takes a boolean setup value. Use the element’s `.When(getter, style)` or a part’s style accessor for reactive conditions. Save the returned value when deriving styles; applying a style keeps a snapshot with its scalar accessors.

## Conditional options

`.When` tracks a condition and applies one or more options while it is true. Conditional options merge with earlier options; later values win only for the same property. Turning a condition off restores an earlier value, or clears the property when there is no base value. Children stay mounted. Conditional event handlers and value bindings are released when their condition becomes false.

```go
selected, setSelected := ui.CreateSignal(false)
return ui.Button("Toggle selection").
	Padding(12).
	Bg("#ccc").
	RoundedLg().
	When(selected(), ui.Style().BackgroundColor("#2563eb"), ui.Style().TextColor("white")).
	OnClick(func() { setSelected(!selected()) })
```

For a computed condition, use `.When(func() bool { return count() >= 5 }, ...)`. Keep ref callbacks outside conditional options; refs run after mounting.

Run `quickgui fmt` to apply QuickGUI's formatting rule. Long UI calls (over 100 columns), multiline calls, and calls with callback arguments use one argument per line and a trailing comma. Multiline typed props use one field per line. Short calls stay compact; `gofmt` then handles ordinary Go spacing, indentation, and alignment. Comments and string contents are preserved, and generated files are left to their generator. Running `gofmt` afterward preserves the layout.

Use `quickgui fmt --check` in CI or `quickgui fmt --project path/to/app` for another project. The formatter uses the SDK version in the project's `go.mod`. In this repository, `bun run fmt:go` formats the SDK and examples; `bun run test:go` checks the same rule. The Go command can also read source on stdin for editor integration: `go run github.com/egoist/quickgui/go/cmd/quickguifmt`.

## Groups and named group hover

`.Group(true)` marks an unnamed hover group; `.Group("card")` names it. A descendant’s `.GroupHover(...)` follows its nearest ancestor group. `.GroupHoverNamed("card", ...)` follows the nearest ancestor with that name, skipping groups with other names. Elements and reusable style builders accept the same style callbacks.

```go
func GroupExample() *ui.Element {
	return ui.View(
		ui.Text("Changes when the card is hovered").TextColor("#64748b").
			GroupHover(func(s ui.StyleBuilder) ui.StyleBuilder { return s.TextColor("#2563eb") }),
		ui.View(
			ui.Text("Follows the card and the nested toolbar").
				GroupHoverNamed("card", func(s ui.StyleBuilder) ui.StyleBuilder { return s.TextColor("#2563eb") }).
				GroupHoverNamed("toolbar", func(s ui.StyleBuilder) ui.StyleBuilder { return s.Opacity(0.8) }),
		).Group("toolbar"),
	).Group("card").Padding(20)
}
```

Hovering group padding or descendants activates its rules. Repeated group rules accumulate in declaration order; a node’s own `.Hover(...)` wins for overlapping properties. Use paint properties such as colors, opacity, outlines, and transforms. Hover updates run natively without rerunning components or changing layout.

`.GroupActive(...)` and `.GroupActiveNamed(...)` follow pressed groups. `.FocusWithin(...)` follows focus in the node or its descendants. Group and focus-within states accept paint styles and cannot set the cursor.

## Conditional content and lists

```go
return ui.Show(
	count() >= 5,
	func() *ui.Element {
		return ui.Text("Five or more clicks")
	},
)
```

`Show` accepts component functions for its content and optional fallback. It creates children lazily and disposes them when hidden. `For` reuses unchanged rows by a comparable key; `KeyedFor` gives each retained row an item accessor so changing its data preserves local state. Keys must be unique. Removed rows release their effects and native listeners. Window closure disposes the entire component tree and outstanding component background work.

`ui.Dynamic(func() ui.Component { ... })` selects a component reactively. Only the selector reruns; bindings inside the selected component keep updating its retained nodes. Return `nil` from the selector to render nothing. List callbacks also return their row node:

```go
return ui.For(
	items,
	func(item Item, index func() int) *native.Node {
		return ui.Fragment([]*native.Node{ui.Text(item.Name).Node,
			ui.Text(index()).Node})
	},
	func(item Item) any { return item.ID },
	nil,
)
```

Use `.Ref(func(node *native.Node) { ... })` for a node handle, such as a popover anchor. It runs once at its position in the fluent chain. Pass `element.Node` to low-level native APIs. Components and construction callbacks must return `*ui.Element` or `*native.Node`.

## Background work

Components, signals, event callbacks, and CPU-only native router calls run on one Go goroutine pinned to an OS thread. `native.Run` must be called once from `main`; it reserves the process main thread for AppKit/Winit. Do not read or write signals from worker goroutines.

`native.Dispatch(func() { … })` queues a background result onto the UI goroutine. `ui.Async(work, done)` runs a context-aware worker and dispatches its completion, canceling it when its component is disposed. `ui.OnCleanup` releases component resources. Native dialogs and services use asynchronous callbacks: none synchronously waits for the native main thread.

Go and Rust exchange bounded binary mutation batches and copied event data through an in-process C ABI. No application host process or IPC bridge is involved. Clean windows sleep; property deduplication and batched effects avoid redundant native mutations. App-specific background jobs can still consume CPU.

## Packages and controls

- `native`: application/window lifecycle, nodes, events, asynchronous dialogs and services, menus, OS filesystem watchers, and the Rust router.
- `ui`: fluent styles, primitives, `Show`, `For`, `KeyedFor`, compound controls, routing, and macOS SwiftUI hosting.
- `reactive`: signals, memos, batching, effects, owners, and contexts.
- `host`: the purego C ABI adapter and a replaceable host interface for tests.

Compound controls accept node-returning callbacks after their typed props so descendants inherit their context. Return `ui.Fragment(...)` when a callback constructs multiple sibling nodes. Pass strings or string accessors directly as children to text and buttons. Primitives configure styles and events with fluent methods. Pass an existing node directly to a parent or to `ui.Fragment(...)`. There is no implicit child-declaration API. The `Props.Children` form remains available for programmatic composition. Families include `Checkbox`, `Switch`, `Tabs`, `Dialog`, `Popover`, `SystemPopover`, `Slider`, `Select`, `Combobox`, `Menu`, `Table`, `Tree`, and `Toast`. Their native behavior remains in Rust. `ui.SwiftUI` provides the macOS SwiftUI control gallery and reverse-hosted QuickGUI views.

See [counter](../examples/counter/main.go), [components](../examples/components/main.go), [routing](../examples/routing/main.go), [SwiftUI](../examples/swift-ui/main.go), and the full [Quick Git](../examples/quick-git/main.go) application.

## Native services

Import `github.com/egoist/quickgui/go/native` for application/window lifecycle and
platform services. Call them after `native.Run` has initialized the application.
Native mutations enqueue commands; operations with a result take a completion
callback rather than synchronously waiting for the main thread. Completion and
event callbacks run on the Go application goroutine.

| API | Capability |
| --- | --- |
| `App`, `Window` | Identity, paths, windows, lifecycle, menus, activation, relaunch |
| `ShowAlertDialog`, `ShowOpenDialog`, `ShowSaveDialog`, `Shell` | System dialogs, file panels, opening/revealing paths and URLs |
| `Clipboard` | Typed text, binary MIME data, images, files, and Find pasteboard |
| `Screen`, `SystemPreferences`, `Appearance`, `Keyboard` | Display and platform snapshots |
| `Notifications`, `GlobalShortcut`, `Tray` | Native notification, shortcut, and tray lifecycles |
| `PowerMonitor`, `PowerAssertion`, `Permissions` | Power/idle state, sleep assertions, and explicit permissions |
| `AutoStart`, `Protocol`, `DeepLink` | Startup registration and application links |
| `SecureStorage` | Native credential storage |
| `Updater`, `CrashReporter`, `Metrics` | Signed updates, core crash reports, and explicit metrics |

Check callback errors and retain/dispose subscription or resource handles for the
duration they are needed. Capture the owning window during component creation if
an asynchronous result needs to operate on it. Examples and platform limits are
documented in [clipboard](clipboard.md#go), [document windows](document-windows.md#go),
[updates](relaunch-and-updates.md#download-progress-from-go), and
[crash reporting and metrics](crash-reporting-and-metrics.md#go).

## Build

Go 1.23 or newer and Bun are required for development. On macOS, packaging also uses Xcode Command Line Tools. Published native assets currently target macOS arm64 and x64; Linux and Windows need a matching host shared library and native runtime validation.

Optional terminal support is imported from `github.com/egoist/quickgui/go/terminal` and rendered with `terminal.View(terminal.Props{…})`. The CLI bundles its separate prebuilt native extension only when the app imports that package. See the [extension guide](https://github.com/egoist/quickgui/blob/main/docs/architecture/extensions.md) for offline builds and source development.

From a source checkout, build the Rust library once:

```console
bun install
bun run build:native
bun packages/cli/src/cli.ts dev --project examples/counter
```

Application edits only rebuild Go. Use `quickgui check` and `quickgui test` so applications use the same view compiler as development builds.

Direct Go commands bypass the view compiler and require explicit accessors:

```console
CGO_ENABLED=0 go -C examples/counter build -o /tmp/quickgui-counter .
QUICKGUI_LIBRARY="$PWD/target/release/libquickgui_host.dylib" /tmp/quickgui-counter
```

A Go `replace` directive points each repository example at `../../go`. External applications depend on `github.com/egoist/quickgui/go`; releases use the submodule tag `go/v<version>`. The CLI bundles the matching library under macOS `Contents/Frameworks`, or beside the executable on Linux/Windows. `QUICKGUI_LIBRARY` can select an explicit library for development. Rust and Go protocol versions are checked at load time.

Run `bun run test:go` to check formatting, generated Rust protocol constants, the SDK, and every Go example with CGO disabled. Run `go -C go generate ./protocol` after changing Rust wire constants.

Automatic updates are a separate opt-in import: `github.com/egoist/quickgui/go/updater`. Call `updater.Start` once per app, configure `[updates]` in `quickgui.toml`, and publish signed appcasts with the Bun CLI. See [automatic updates](https://github.com/egoist/quickgui/blob/main/docs/updater.md).

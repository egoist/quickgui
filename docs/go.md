# Go UI

QuickGUI's Go frontend sits beside the TypeScript/scriptc pipeline and talks to the same Rust
core. The core owns the main application thread, retained trees, layout, rendering, input,
accessibility, and platform services. Application code runs on a dedicated goroutine and
communicates through the existing bounded command and event queues.

Go is a frontend, not a second renderer. Public behavior lives in the Rust core; the Go packages
expose it. The hosted Go/native boundary never waits on native main-thread execution: constructors
allocate handles locally, mutations are fire-and-forget, and every result arrives as an
asynchronous callback on the application goroutine.

`go build` does not compile Rust and does not use cgo. The process loads a prebuilt host shared
library at runtime (`packages/native/lib/<target>/libquickgui_host.dylib` on macOS). `quickgui dev`
and `quickgui build` compile the Go package and stage that library. Build the library once with
`bun run build:native` when iterating with a plain `go run`.

## First window

```console
cd examples/counter-go
bun run dev
```

```go
package main

import (
    "fmt"

    "github.com/egoist/quickgui/packages/go/native"
    "github.com/egoist/quickgui/packages/go/ui"
)

func main() {
    native.Run(func() {
        native.NewWindow(native.WindowOptions{
            Title:    "Counter",
            Width:    480,
            Height:   320,
            Renderer: ui.CreateRenderer(Counter),
        })
    })
}

func Counter() *native.Node {
    count, setCount := ui.CreateSignal(0)
    return ui.View(ui.Props{
        Style: ui.Style{
            Display: "flex", FlexDirection: "column", Width: "100%", Height: "100%",
            AlignItems: "center", JustifyContent: "center", Gap: 12,
        },
        Children: []any{
            ui.Text(ui.Props{Children: func() string { return fmt.Sprintf("Count: %d", count()) }}),
            ui.Button(ui.Props{
                OnClick:  func(*native.Event) { setCount(count() + 1) },
                Children: "Increment",
            }),
        },
    })
}
```

`native.Run` loads the host library, starts the application goroutine, and keeps the process main
thread for AppKit/Winit. The callback runs after the host is ready, the same way TypeScript awaits
`app.whenReady()`. Closing a window disposes its renderer root. With the default quit mode, macOS
keeps the app resident after its last window closes; `App.OnReopen` can create a window.

`native.CurrentWindow()` returns the window associated with rendering or event dispatch. Read it
during component setup and retain it for later work. Window commands such as `window.Close()`
enqueue a mutation; they never wait for AppKit.

## Signals and component props

```go
count, setCount := ui.CreateSignal(0)
doubled := ui.CreateMemo(func() int { return count() * 2 })
ui.CreateEffect(func() { fmt.Println(doubled()) })
ui.OnCleanup(func() { subscription.Close() })
```

A setter accepts a value: `setCount(count() + 1)`. `Batch` groups writes, `reactive.Untrack` reads
without subscribing, and `reactive.CreateRoot` creates an explicit lifetime. Effects track the
signals they read and release their previous children and cleanups when they rerun. A component
runs once; effects update its existing native nodes.

Go has no JSX lowering pass. Host properties that should update later are accessors. Text that
reads a signal is a `func() string`:

```go
func CountLabel(count func() int) *native.Node {
    return ui.Text(ui.Props{Children: func() string { return fmt.Sprintf("%d", count()) }})
}
```

A plain `count int` prop is a setup-time value. Callback props remain callbacks. `native.Dispatch`
queues work from a background goroutine onto the application goroutine.

## Router

Routing is the same core-owned route table and bounded memory history as TypeScript.
`native.Router` talks to the host through the synchronous CPU-only service channel
(`router-create`, `router-push`, …). `ui.Router` builds a static table, renders the matched
chain, and keeps a page mounted while only its parameters or query change.

```go
func App() *native.Node {
    return ui.Router(ui.RouterProps{
        InitialPath: "/",
        Routes: []*ui.RouteDeclaration{
            ui.Route("/", Shell, ui.Route("", Home), ui.Route("projects/:id", Project)),
        },
        Fallback: func() *native.Node { return ui.Text(ui.Props{Children: "Not found"}) },
    })
}

func Shell() *native.Node {
    return ui.View(ui.Props{
        Children: []any{
            ui.Link(ui.LinkProps{Href: "/", PartProps: ui.PartProps{Children: func() *native.Node {
                return ui.Text(ui.Props{Children: "Home"})
            }}}),
            ui.Outlet(),
        },
    })
}

func Project() *native.Node {
    id := ui.UseParam("id")
    return ui.Text(ui.Props{Children: func() string { return "Project " + id() }})
}
```

`UseRouter`, `UseNavigate`, `UseLocation`, `UseParams`, and `UseSearchParams` must be called
below `Router`. `Link` is a native button with `role="link"`; `ActiveStyle` layers over `Style`
while the destination is active.

## Conditional and list content

`Show`, `For`, and `KeyedFor` come from the `ui` package.

```go
ui.Show(func() bool { return visible() }, func() *native.Node {
    return ui.Text(ui.Props{Children: "Visible"})
}, func() *native.Node {
    return ui.Text(ui.Props{Children: "Hidden"})
})

ui.For(func() []string { return names() }, func(name string, index func() int) *native.Node {
    return ui.Text(ui.Props{Children: name})
}, nil, nil)

ui.KeyedFor(func() []Record { return records() }, func(record Record) any {
    return record.ID
}, func(record func() Record, index func() int) *native.Node {
    return ui.Text(ui.Props{Children: func() string { return record().Title }})
}, nil)
```

`For` passes an item value and an index accessor. An optional key matches items during reorders;
replacing an item with a new value recreates that row. `KeyedFor` requires a key and passes the
item as an accessor, so immutable updates keep the same row and update its content. Removed rows
dispose their owned effects and cleanup callbacks.

## Styling and input

Host primitives are `View`, `Text`, `Button`, `Input`, `TextArea`, `Markdown`, and `Image`. They
are unstyled. Compound families match `@quickgui/ui`: `Checkbox`, `Dialog`, `Table`, `Toast`,
`Tabs`, `Popover`, `Slider`, `Select`, `Menu`, and the rest of the Base UI-shaped NativePart
bindings. Each family declares parts and forwards `componentchange` / `commit` / `dismiss` to the
Rust core; Go does not reimplement their behavior.

The macOS SwiftUI host is `ui.SwiftUI`, matching `@quickgui/ui/swift-ui`. Wrap each control in
`SwiftUI.Host` and pass modifier factories (`SwiftUI.ButtonStyle`, `SwiftUI.ControlSize`,
`SwiftUI.Disabled`, …). `DatePicker` values are Unix timestamps in milliseconds.
`SwiftUI.Popover.Root` / `Trigger` / `Content` present a native popover.
`SwiftUI.QuickGUIHostView` reverse-hosts an independently owned QuickGUI renderer; native creation
is queued on the application goroutine and never waits for AppKit.

```go
ui.SwiftUI.Host(ui.SwiftUIHostProps{
    MatchContents: true,
    PartProps: ui.PartProps{Children: func() *native.Node {
        return ui.SwiftUI.Slider(ui.SwiftUISliderProps{
            Value: volume, Min: 0, Max: 1,
            Modifiers: []ui.SwiftUIModifier{ui.SwiftUI.ControlSize("regular")},
            OnValueChange: func(next float64, _ *native.Event) { setVolume(next) },
        })
    }},
})
```

See [the SwiftUI gallery](../examples/swift-ui-go).

Styles use the same camelCase properties as TypeScript:

```go
ui.Button(ui.Props{
    Disabled: busy(),
    OnClick:  func(*native.Event) { save() },
    Style: ui.Style{
        Padding: 12, BorderRadius: 8, BackgroundColor: "#2563eb", Color: "#ffffff",
        Hover:    &ui.Style{BackgroundColor: "#1d4ed8"},
        Disabled: &ui.Style{Opacity: 0.5},
        Focus:    &ui.Style{/* outline via later bindings */},
    },
    Children: "Save",
})
```

Native events arrive asynchronously with the core's decision. Event payloads may be absent; check
`event.ValueOK()` before using them.

## Quick Git

[Quick Git](../examples/quick-git-go) is the same git client as `examples/quick-git`, written against
the Go frontend. Git parsing and process control stay in ordinary Go. The UI uses the same compound
parts as TypeScript: `Table` for file, history, and diff lists; `Dialog` for in-window forms;
`Toast` for notices; `Checkbox` and `Select` for controls; `SwiftUI.Host` buttons and progress
in the toolbar.

```console
cd examples/quick-git-go
bun run dev
```

`language: "go"` in `quickgui.config.ts` makes `quickgui dev` and `quickgui build` run
`CGO_ENABLED=0 go build` and copy `libquickgui_host` next to the executable.

## Host library

`bun run build:native` stages both the scriptc static archive and a `dynamic-host` shared library
under `packages/native/lib/<target>/`. The Go process searches, in order:

1. `QUICKGUI_HOST_LIB`
2. the executable directory
3. `packages/native/lib/<goos>-<goarch>/` walking up from the working directory

The shared library is built with the `dynamic-host` feature so it does not import the process
`main` symbol that scriptc provides. `quickgui_run_host` runs the native loop on the calling
thread.

See the [TypeScript UI guide](ui.md) for the shared rendering model,
[the counter example](../examples/counter-go),
[the SwiftUI gallery](../examples/swift-ui-go), and
[Quick Git in Go](../examples/quick-git-go).

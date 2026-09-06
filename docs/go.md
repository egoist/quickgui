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
library at runtime (`packages/native/lib/<target>/libquickgui_host.dylib` on macOS). Build that
library once with `bun run build:native`, then iterate on the Go application with a plain compile.

## First window

```console
bun run build:native
cd examples/counter-go
CGO_ENABLED=0 go run .
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
are unstyled. Styles use the same camelCase properties as TypeScript:

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

## Host library

`bun run build:native` stages both the scriptc static archive and a `dynamic-host` shared library
under `packages/native/lib/<target>/`. The Go process searches, in order:

1. `QUICKGUI_HOST_LIB`
2. the executable directory
3. `packages/native/lib/<goos>-<goarch>/` walking up from the working directory

The shared library is built with the `dynamic-host` feature so it does not import the process
`main` symbol that scriptc provides. `quickgui_run_host` runs the native loop on the calling
thread.

See the [TypeScript UI guide](ui.md) for the shared rendering model and
[the counter example](../examples/counter-go).

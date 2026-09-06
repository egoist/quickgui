# Go frontend

Signals, unstyled native components, and the host C ABI for QuickGUI applications written in Go.

The TypeScript frontend compiles with scriptc and links the Rust host into the executable. This
package is **cgo-free**: `go build` is an ordinary, fast Go compile. At runtime the process loads
the prebuilt host shared library (`libquickgui_host`) and talks to it through the same fire-and-forget
C ABI. Application work stays on a dedicated goroutine; AppKit/Winit keeps the main thread.

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
            Renderer: ui.CreateRenderer(Counter),
        })
    })
}

func Counter() *native.Node {
    count, setCount := ui.CreateSignal(0)
    return ui.View(ui.Props{
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

A component function runs once. Values that change after setup are accessors (`func() T`);
setters take a value. `Show`, `For`, and `KeyedFor` replace only the region they own.
`Router`, `Route`, `Layout`, `Outlet`, and `Link` expose the core route table the same way
`@quickgui/ui/router` does. Compound families (`Checkbox`, `Dialog`, `Table`, `Toast`, `Tabs`,
`Popover`, `Slider`, `Select`, `Menu`, and the rest) are thin bindings over the same Rust
`NativePart`s as `@quickgui/ui`.

`quickgui dev` / `quickgui build` compile a `language: "go"` project. You can also build the
host library once and compile the application as often as you like:

```console
bun run build:native
CGO_ENABLED=0 go build ./examples/counter-go
```

See [the Go guide](../../docs/go.md), the [counter example](../../examples/counter-go), and
[Quick Git in Go](../../examples/quick-git-go).

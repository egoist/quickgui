# Go SwiftUI gallery

The TypeScript SwiftUI example, written against the Go frontend. The left rail is a vertical
`Tabs` list. Each page mounts a live `ui.SwiftUI` control whose state is owned by a Go signal.

```console
cd examples/swift-ui-go
bun run dev
```

`language: "go"` compiles with `CGO_ENABLED=0 go build` and stages `libquickgui_host`. SwiftUI
hosts are macOS-only; other platforms keep the QuickGUI chrome and hide the native controls.

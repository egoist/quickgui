# QuickGUI SwiftUI

This example is a sidebar gallery of the native SwiftUI components exposed through QuickGUI. The
left rail is a semantic vertical `Tabs` list, and the right pane mounts one live component demo per
page with the state reported back to QuickGUI.

The gallery includes real SwiftUI `Button`, `Slider`, `Toggle`, `ProgressView`, `Stepper`,
`TextField`, `SecureField`, `Picker`, segmented tabs, `DatePicker`, `ColorPicker`, and `Gauge`
controls. The Popover page presents a native popover that reverse-hosts ordinary QuickGUI `View`,
`Text`, `Input`, and `Button` components. That embedded subtree keeps its Rust renderer and native
input/accessibility surface instead of being translated into SwiftUI controls.

```console
cd examples/swift-ui
bun run dev
```

The Go gallery is the same pages on `ui.SwiftUI`:

```console
cd examples/swift-ui-go
bun run dev
```

The `glass` style uses Liquid Glass on macOS 26 and falls back to a bordered native button on older
systems. Dismiss the popover by clicking outside it, or use the QuickGUI Save button inside it.

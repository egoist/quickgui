# QuickGUI

QuickGUI is a damage-driven, GPU-accelerated GUI framework for Rust desktop applications. It combines a GPUI-style fluent view API, Taffy Flexbox, CSS Grid and parent-size container queries, WGPU rendering, retained Unicode text, native accessibility, and bounded virtual scrolling.

The current focus is production-quality macOS behavior with low idle CPU and bounded memory. Windows and Linux compile through Winit/WGPU but still need native runtime and visual acceptance.

## Performance model

- Clean windows sleep in `ControlFlow::Wait` and render no idle frames.
- Ordinary wheel scrolling and redraw requests coalesce at the event-loop boundary; opt-in
  element listeners preserve each native delta kind and gesture phase.
- Hover, scrolling, drag previews, and selection reuse retained layout where possible.
- Web-style hover, active, focus, validation, and drag transitions repaint retained geometry
  without rebuilding the view or rerunning layout.
- Element opacity multiplies through complete GPU/native subtrees, remains hit-testable at zero,
  and changes rich-text glyph alpha without invalidating shaping.
- Uniform and measured variable-height lists mount only visible rows and preserve logical anchors.
- Layout-isolated container queries redeclare only when their assigned parent size changes.
- Text, images, SVGs, paths, shaders, and GPU buffers use explicit retention bounds.
- Font families, OpenType features, and ordered application fallbacks inherit through ordinary and
  rich text while remaining part of canonical retained shaping keys.
- Compatible windows share the expensive WGPU instance, adapter, device, and queue.
- Native services, delayed UI, animation, and background completion wake only on events or exact deadlines.
- Native cursor declarations reuse retained hit testing and add no redraw or idle scheduling source.
- Raw touch contacts are captured per identity with a fixed per-window bound and no recognizer,
  monitor, polling task, or idle scheduling source.

## View API

Views are ordinary Rust with JSX-like composition and Tailwind-style helpers:

```rust
use quickgui::{
    Application, Color, EventContext, IntoElement, View, ViewContext, WindowOptions, div, text,
};

struct Counter {
    count: usize,
}

impl View for Counter {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        let increment = cx.listener("increment", |this, cx: &mut EventContext| {
            this.count += 1;
            cx.invalidate();
        });

        div()
            .size_full()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_3()
            .bg(Color::rgb8(18, 18, 20))
            .text_color(Color::rgb8(240, 241, 244))
            .child(text(format!("Count: {}", self.count)).text_2xl())
            .child(
                div()
                    .on_click(increment)
                    .px_4()
                    .py_2()
                    .rounded_lg()
                    .bg(Color::rgb8(45, 105, 180))
                    .opacity(0.9)
                    .hover(|style| {
                        style.bg(Color::rgb8(56, 122, 204)).opacity(1.0)
                    })
                    .child("Increment"),
            )
    }
}

fn main() -> Result<(), quickgui::AppError> {
    Application::new().run(|cx| {
        cx.open_window(
            WindowOptions::new("Counter").size(480.0, 320.0),
            Counter { count: 0 },
        );
    })
}
```

## Documentation

Start at the [documentation index](docs/README.md).

- [View API and layout](docs/view-api.md)
- [QuickGUI UI renderer](docs/ui.md)
- [Go UI](docs/go.md)
- [Project CLI and application packaging](docs/cli.md)
- [Native modules in Zig](docs/native-modules.md)
- [Windows and shared state](docs/windows.md)
- [Native document windows](docs/document-windows.md)
- [Displays and window placement](docs/displays.md)
- [macOS integration](docs/macos.md)
- [Text, editing, and forms](docs/text-and-forms.md)
- [Selection controls](docs/selection-controls.md)
- [Tabs](docs/tabs.md)
- [Popovers and popover menus](docs/popovers.md)
- [Select and autocomplete](docs/select-and-autocomplete.md)
- [Dialogs](docs/dialogs.md)
- [Base UI components (separator, avatar, checkbox group, preview card, scroll area, OTP field, drawer, navigation menu)](docs/base-ui-components.md)
- [Unstyled component roadmap](docs/component-roadmap.md)
- [Virtual tables and trees](docs/data-collections.md)
- [Range and feedback components](docs/range-and-feedback.md)
- [Toolbar, toggles, and toasts](docs/toolbar-and-toast.md)
- [Date and time fields](docs/date-and-time.md)
- [In-window menubar](docs/menubar.md)
- [Crash reporting and process metrics](docs/crash-reporting-and-metrics.md)
- [Application assets and custom fonts](docs/assets-and-fonts.md)
- [Clipboard](docs/clipboard.md)
- [Graphics and media](docs/graphics.md)
- [Declarative motion](docs/animations.md)
- [Input and interaction](docs/input.md)
- [Asynchronous work](docs/async.md)
- [Deterministic testing](docs/testing.md)
- [Retained-tree inspector](docs/inspector.md)
- [Examples and performance checks](docs/examples-and-performance.md)
- [Releasing QuickGUI 0.1](docs/releasing.md)
- [Changelog](CHANGELOG.md)
- [Status and roadmap](docs/status.md)
- [Architecture](docs/architecture/README.md)

## Try it

For a natively compiled TypeScript application (macOS, Node.js 24+, Bun tooling, and Xcode Command Line Tools):

```console
bun install
cd examples/counter
bun run dev
```

A matching Go counter uses the same host and fine-grained signals. `go build` does not compile
Rust or use cgo; the process loads the prebuilt host shared library at runtime:

```console
bun run build:native
cd examples/counter-go
CGO_ENABLED=0 go run .
```

The routing example uses the Rust core for matching and memory history while QuickGUI UI renders nested
layouts, links, dynamic parameters, queries, and fallback routes:

```console
cd examples/routing
bun run dev
```

The complete DeepSeek streaming example uses QuickGUI core Markdown:

```console
cd examples/ai-chat
bun run dev
```

The QuickGUI UI alert-dialog example presents native alerts with optional window ownership:

```console
cd examples/alert-dialog
bun run dev
```

The separate file-dialog example uses native open and save panels with Electron-shaped results:

```console
cd examples/file-dialog
bun run dev
```

The QuickGUI UI popover example compares the shared compound JSX API of a native `SystemPopover` and a
retained in-window `Popover`:

```console
cd examples/popover
bun run dev
```

The sidebar vibrancy example switches among every Electron-compatible macOS semantic material and
all three visual-effect activity states while keeping a translucent QuickGUI UI sidebar and opaque
content pane:

```console
cd examples/sidebar-vibrancy
bun run dev
```

The core-first system API example covers app environment, rich clipboard, displays, desktop
integrations, permissions, preferences, power, native menus, notifications, tray icons, shortcuts,
window controls, and updater metadata:

```console
cd examples/system-api
bun run dev
```

The styling example declares text alignment, the extended text styles, gradients, per-corner radii,
dashed borders, outlines, filters, transforms with a hover variant, blend modes, a backdrop blur,
right-to-left layout, sticky headers, and scroll snapping:

```console
cd examples/styling
bun run dev
```

The components example puts every compound component the QuickGUI UI package binds in one window: select,
combobox, autocomplete, virtual table and tree, slider, number field, splitter, toolbar, toggle
group, date and time fields, calendar, menubar, popover and context menus, dialog, tabs, and toasts:

```console
cd examples/components
bun run dev
```

Quick Git is a complete native git client: virtualized change lists and diffs, hunk and line
staging, commit messages drafted by a local Codex or Claude CLI, history with a lane graph,
branches, stashes, and first-class worktrees:

```console
cd examples/quick-git
bun run dev
```

The CLI lowers TSX with TypeScript 7 and compiles the application with scriptc, linking the Rust
host into a native `.app`. Bun runs the development tools; the application contains no Bun runtime.
Rust framework examples remain available directly through Cargo:

```console
cargo run --release --example gesture_input
cargo run --release --example cursors
cargo run --release --example appearance
cargo run --release --example window_background
cargo run --release --example clipboard
cargo run --release --example assets
cargo run --release --example styled_text
cargo run --release --example selection_controls
cargo run --release --example tabs
cargo run --release --example popovers
cargo run --release --example system_popover
cargo run --release --example comboboxes
cargo run --release --example autocomplete
cargo run --release --example dialogs
cargo run --release --example data_collections
cargo run --release --example displays
cargo run --release --example animations
cargo run --release --example effects
cargo run --release --example rtl_sticky_snap
cargo run --release --example text_styling
cargo run --release --example text_services
cargo run --release --example range_controls
cargo run --release --example toolbar_toast
cargo run --release --example date_fields
cargo run --release --example menubar
cargo run --release --example base_ui_components
cargo run --release --example platform_services
cargo run --release --example window_controls
cargo run --features inspector --example inspector
cargo run --release --example stress_scroll
cargo run --example hacker_news
```

The stress example mounts only the visible slice of a 100,000-row list and includes live CPU/render
telemetry. On macOS, `scripts/macos-performance-gate.sh` gates bidirectional scrolling and
`scripts/macos-acceptance-gate.sh` gates native resize, wrapped and ellipsized text, embedded
`NSView` lifecycle, context-menu interaction and a 128-cycle popover memory plateau, idle behavior,
whole-process CPU, peak RSS, and process-owned graphics footprint against a real WindowServer. The
scripted popovers are lifecycle evidence, not a human-visible visual recording.
`scripts/macos-display-acceptance-gate.sh` separately opens hidden, non-key windows on every active
display and requires exact first-render centering plus settled retained/AppKit screen identity,
scale, native-frame origin, work-area agreement, and full native-frame containment. Its report
marks unavailable mixed-scale hardware as skipped, never passed.

## Validate

```console
cargo test --all-targets --all-features --locked
cargo +1.90.0 check --all-targets --all-features --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo bench --bench virtual_list
cargo bench --bench animation
scripts/macos-performance-gate.sh
scripts/macos-acceptance-gate.sh
scripts/macos-display-acceptance-gate.sh
QUICKGUI_PACKAGE_TOOLCHAIN=1.90.0 scripts/package-release-gate.sh
```

## License

MIT OR Apache-2.0.

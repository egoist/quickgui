# Examples and performance checks

[Documentation index](README.md)

The QuickGUI UI system API example keeps desktop behavior in the Rust core while `@quickgui/native`
projects it into JavaScript and `@quickgui/ui` renders the controls. It covers application
identity and paths, system information and preferences, permissions, power, rich clipboard data,
notifications, menus, desktop integrations, global shortcuts, tray icons, and window controls:

```console
cd examples/system-api
bun run dev
```

The Go counter is the same fine-grained UI as `examples/counter`, compiled with `go build` and
no cgo. The process loads the prebuilt host shared library at runtime:

```console
bun run build:native
cd examples/counter-go
CGO_ENABLED=0 go run .
```

Quick Git is a native git client built as a product rather than a demo. Its pure-TypeScript git
layer (a bounded process runner plus porcelain v2, unified diff, log, ref, stash, and worktree
parsers with patch formatting for partial staging) is covered by `bun test`, including real git
runs in a temporary repository; the UI declares only the visible rows of every list through the
core-virtualized `Table`, drafts commit messages with the Codex or Claude CLI non-interactively,
and treats worktrees as first-class:

```console
cd examples/quick-git
bun run dev
```

The Go port (`examples/quick-git-go`) is the same client on the cgo-free frontend, complete on its
own. Parsers and the process runner live in `internal/git`; persistence, a poll-based watcher, and
the store live in `internal/model`. The UI binds the same core-owned Table, Dialog, Toast, Checkbox,
and Select parts as TypeScript:

```console
cd examples/quick-git-go
bun run dev
```

`quickgui dev` and `quickgui build` compile with `CGO_ENABLED=0 go build` and stage the host shared
library next to the executable. `CGO_ENABLED=0 go run .` still works after `bun run build:native`.

The QuickGUI UI routing example declares nested layouts, dynamic and wildcard routes, query-only
navigation, active `Link` styling, and back/forward controls. Rust owns pattern matching,
normalization, decoded values, and the bounded memory history; QuickGUI UI owns only declaration
collection and rendering:

```console
cd examples/routing
bun run dev
```

The cursor gallery covers every typed GPUI-compatible native cursor and its CSS/Tailwind name.
Hover changes use the retained hit stack and schedule no cursor-owned frame:

```console
cargo run --release --example cursors
```

The Flexbox gallery exercises reverse flow, grow/shrink/basis shorthands, independent axis gaps,
wrapped-content distribution, per-item alignment, aspect ratios, and auto margins in a resizable
application shell. Every helper is ordinary retained Taffy state and adds no scheduler source:

```console
cargo run --release --example flex_layout
```

The container-query example switches one reusable dashboard among compact, two-column, and
three-column layouts from its assigned parent width. Resize reconciliation is bounded and an
unchanged size creates no callback or scheduling work:

```console
cargo run --release --example container_queries
```

The live Hacker News example combines bounded background fetching, full wrapped comments,
fixed-height story virtualization, measured variable-height comment virtualization, independent
draggable overlay scrollbars, custom hidden-inset chrome, and explicit app regions:

```console
cargo run --example hacker_news
```

The native appearance example follows macOS light/dark changes, supports explicit per-window
overrides, and exposes a frame counter that remains still while the window is clean:

```console
cargo run --release --example appearance
```

The macOS background example switches one live window between opaque, transparent, and native
blurred composition. Its frame counter makes the one-transition-frame and zero-idle-frame behavior
visible:

```console
cargo run --release --example window_background
```

The document-window example exercises represented file URLs, edited-state indication, the system
character palette, and opt-in native tab creation, navigation, merging, detaching, bar visibility,
and overview. Its retained tab snapshot changes only at command or native lifecycle boundaries:

```console
cargo run --release --example document_window
```

The clipboard example exercises hash-bound text metadata, encoded PNG transfer, native file lists,
and macOS's shared Find pasteboard. It opens native services only when a button or editor shortcut
requests them:

```console
cargo run --release --example clipboard
```

The application-assets example builds a bounded source from static SVG and text bytes, retains the
parsed icon, and loads copy through the same `Assets` handle available to view callbacks. Static
bytes are shared without a payload copy and the source creates no watcher or idle work:

```console
cargo run --release --example assets
```

The display example shows bounded native monitor snapshots, current/primary display identity,
logical work areas, and display-targeted centered child windows. Connecting, disconnecting, or
rearranging a monitor updates observing views from one platform notification. Its separate macOS
gate creates hidden, non-key windows on every active display and compares retained placement with
live AppKit identity and scale. It requires exact first-render centering, settled native-frame
origin and screen/work-area agreement, and containment of the complete native outer frame:

```console
cargo run --release --example displays
scripts/macos-display-acceptance-gate.sh
```

The motion example covers paint-only hover/active/focus transitions, one-shot replay, interpolated
layout/color, a rapidly retargetable analytic spring, a synchronized repeating animation capped at
30 FPS, and live active-animation metrics. The tooltip/context-menu and drag/drop examples also
exercise independently owned animated tooltip and drag-preview trees. Repeating motion is opt-in
so the untouched example visibly returns to zero idle frames:

```console
cargo run --release --example animations
```

The native-input example demonstrates exact pixel/line wheel deltas with default prevention, raw
bounded multi-contact touch capture, Force Touch, pinch, rotation, and smart magnification on one
element. Its telemetry also makes the zero-idle-frame behavior visible:

```console
cargo run --release --example gesture_input
```

The text-overflow gallery compares full wrapping, one-line truncation, start and middle ellipsis,
and a two-line clamp. Resize it to exercise Unicode-safe projection and the retained width cache:

```console
cargo run --release --example text_overflow
```

The styled-text example applies one retained complete code font, disables contextual ligatures by
default, re-enables them for one highlighted run, and declares an ordered CJK/Arabic/emoji fallback
stack shared by static and controlled text. It also exercises rich backgrounds and decorations:

```console
cargo run --release --example styled_text
```

The selection-control gallery exercises caller-styled checked and mixed checkboxes, roving
radio-group Tab/arrow behavior, switches, disabled semantics, and light/dark application palettes.
The framework descriptors add no visual child, GPU asset, allocation, transition, or idle
scheduling source:

```console
cargo run --release --example selection_controls
```

The tabs gallery demonstrates caller-owned horizontal and vertical tab presentation. It covers
manual and automatic activation, disabled-item skipping, looping, retained and unmounted panels,
and application-owned indicators in one ordinary window; it opens no popover or native child
surface:

```console
cargo run --release --example tabs
```

The disclosure gallery exercises a standalone controlled collapsible, single- and multiple-value
accordions, ordinary Tab-order headings, Enter/Space activation, unmounted and retained closed
panels, disabled behavior, exact heading/region accessibility, and caller-owned presentation. The
descriptors add no timer, observer, task, registry, animation, GPU resource, or idle frame source:

```console
cargo run --release --example disclosures
```

The focus/accessibility gallery uses unstyled Field and Fieldset parts for a real submitted form.
It demonstrates visible labels that focus their inputs, group legend/description relationships,
required and invalid native state, simultaneous help/error descriptions, bounded validation
announcements, and entirely application-owned presentation. Settled descriptors retain no task,
timer, registry, or idle frame:

```console
cargo run --release --example focus_accessibility
```

The controlled-popover gallery combines caller-owned trigger/positioner/popover/title/description/
close parts, same-turn opening focus, independent Escape/outside dismissal, exact restoration,
nested topmost surfaces, edge-aware placement, and application-owned light/dark presentation. Open
settled popovers add no idle scheduling source:

```console
cargo run --release --example popovers
```

The dialog gallery composes an ordinary editing dialog and a nested consequential alert from
caller-styled portal, backdrop, popover, title, description, and close parts. It exercises topmost
focus trapping, wrapping Tab traversal, independent Escape/backdrop policy, exact restoration,
hidden-inset no-drag regions, and the overlay plane used above native children. Settled dialogs add
no idle frame:

```console
cargo run --release --example dialogs
```

The select/combobox gallery combines two unstyled contracts: a standalone Select and a constrained
20,000-option editable Combobox. It demonstrates caller-owned parts, preview-versus-commit
behavior, committed-label restoration, stable IDs, disabled choices, normal text editing and IME
ownership, never-key overflow placement, owner-tree listbox accessibility, visible-only rows, and
CPU/draw/shaping telemetry. Settled open controls and closed popovers add no idle frame:

```console
cargo run --release --example comboboxes
```

The free-form autocomplete gallery styles only application-owned input, popover, and option parts.
Its 20,000-item suggestion panel is a never-key native child that can cross the window edge while
keyboard and IME focus stay in the owner input. Escape preserves arbitrary text, Return commits
only an active row, visible-only rows and owner-tree accessibility proxies remain bounded, and both
windows sleep when settled:

```console
cargo run --release --example autocomplete
```

The data-collections gallery combines a bounded expandable project tree with a sortable
20,000-row table. Both use one composite keyboard focus target, exact native collection semantics,
visible-only row declarations, captured native-style scrollbars, and caller-owned light/dark
palettes applied through unstyled header, cell, row, and disclosure renderers.
The footer exposes frame CPU, draw calls, and shaping work; untouched collections add no idle
scheduling source:

```console
cargo run --release --example data_collections
```

The feature-gated inspector example opens the retained-tree picker over explicit IDs, a scroll
container, accessibility metadata, and overlapping z-index layers. Click freezes a selection and
wheel movement cycles through occluded nodes without adding an idle redraw source:

```console
cargo run --features inspector --example inspector
```

The effects gallery paints multi-stop linear, radial, and conic gradients, per-corner radii,
dashed and dotted borders, outlines, raster backgrounds, color filters, transforms with hover and
click on a rotated card, subtree blur and drop shadows, backdrop blur over scrolling content, and
every blend mode. A settled frame keeps its compositing textures and schedules nothing:

```console
cargo run --release --example effects
```

The direction gallery mirrors one layout between left-to-right and right-to-left, pins sticky
section headers inside their parents while a list scrolls, and snaps a horizontal carousel and a
vertical page stack at gesture end:

```console
cargo run --release --example rtl_sticky_snap
```

The text-styling gallery shows text alignment including `text_center`, `text_start`, and
`text_end`, text shadows, letter and word spacing, case mapping, overlines, and the word-break,
overflow-wrap, and soft-hyphen policies:

```console
cargo run --release --example text_styling
```

The text-services gallery wires the system spell checker, autocorrect, smart substitutions,
dictionary lookup, the unstyled find bar, and the application undo manager into one editor:

```console
cargo run --release --example text_services
```

The range and feedback gallery styles sliders, range sliders, progress bars, meters, number fields,
and splitters over the unstyled descriptors; the toolbar gallery adds roving-focus toolbars, toggle
groups, and the bounded toast queue; the date gallery shows segmented date and time fields with an
optional calendar; the menubar gallery composes an in-window menubar from popover menus; the Base UI
gallery styles separators, avatars, checkbox groups, preview cards, scroll areas, OTP fields,
drawers, and a navigation menu:

```console
cargo run --release --example range_controls
cargo run --release --example toolbar_toast
cargo run --release --example date_fields
cargo run --release --example menubar
cargo run --release --example base_ui_components
```

The platform-services and window-controls examples exercise the application shell: activation
policy, Dock attention, secure keyboard entry, message boxes with checkboxes, Quick Look, the color
and font panels, the share sheet, biometric authentication, window levels, click-through with hover
forwarding, aspect ratios, and restore state:

```console
cargo run --release --example platform_services
cargo run --release --example window_controls
```

## Run the stress test

```console
cargo run --release --example stress_scroll
```

The demo renders a selectable 100,000-row list while mounting only the visible rows plus two rows
of overscan. It supports trackpad/wheel scrolling, a captured overlay scrollbar that expands on
hover and auto-hides, hover and pressed feedback, click listeners, arrow/page navigation, Home/End,
and in-window CPU/render telemetry.

On macOS, run the self-terminating release gate against a real WindowServer with:

```console
scripts/macos-performance-gate.sh
```

The probe warms 120 presented frames, drives 360 frames across the data set in both directions,
lets scroll-owned transitions settle, verifies two seconds of true idle, and exits normally. Its
machine-readable result enforces at most 30% of one application-thread CPU core, 8 ms p95 and 20 ms
maximum application-thread frame work, 32 visible text areas, the renderer's 256 area/layout and
32 text-renderer retention caps, eight draw calls, the 512-entry example label cache, and zero
extra idle frames. `FrameMetrics::frame_time` separately reports surface/submission wall time so a
FIFO presentation wait is never mislabeled as CPU work.

The wrapper additionally measures all process threads, peak resident memory, and macOS physical
footprint (including owned graphics allocations) with `/usr/bin/time`: the defaults are 35%
whole-process CPU over the complete run, 192 MiB peak RSS, and 256 MiB peak footprint. Override
them for a documented reference machine with `QUICKGUI_PERF_MAX_PROCESS_CPU_PERCENT`,
`QUICKGUI_PERF_MAX_RSS_MIB`, and `QUICKGUI_PERF_MAX_FOOTPRINT_MIB`. The window is temporarily
floating and pinned inside the primary display because macOS correctly stops presenting a fully
occluded or off-space surface; a 30-second exact
foreground-task watchdog turns lost presentation into a failing result instead of a hung gate.
This live gate is intentionally not a headless CI timing assertion. The ordinary test suite still
executes thousands of production retained-scroll updates and proves that overflow offsets change
without rebuilding the view.

## Run the macOS 0.1 acceptance gate

The second live gate exercises the framework surfaces that a CPU-only test cannot reproduce:

```console
scripts/macos-acceptance-gate.sh
```

After 45 warm-up frames, the probe drives 180 bidirectional native resizes through
760×520–1180×720 while rendering 64 wrapped and 16 line-clamped/ellipsized Unicode text areas plus
one embedded `NSTextField`.
It detaches the native child during commands 60–74, remounts it, verifies its final AppKit
geometry, then exercises runtime minimum-size set/growth/clear through both `WindowState` and the
native `NSWindow.contentMinSize`. A resize below the former minimum must succeed only after clear.
The same live window receives a bounded AppKit mouse script which verifies exact left/right
capture and bubble order, outside down/up delivery, independent propagation/default prevention,
native double-click counts, pressed-button drag motion, hover entry/exit, and native-window exit.
It then drives eight full-interaction context-menu/submenu command cycles, four owner-press
dismissals, four native Escape dismissals, and 128 additional root-plus-submenu resource cycles at
an explicit 50 ms inter-cycle cadence. The resource phase samples current RSS and physical
footprint after cycles 32 and 128 and enforces 16 MiB and 24 MiB positive-growth ceilings. One final
open submenu is dismissed by cooperatively transferring application activation to Finder without
reclaiming owner focus. The probe then closes the window and verifies that teardown removed the
native child from its superview. A one-second settle period is followed by a two-second idle audit.

The in-process result requires at least 45 presented resize frames per second, at most 35% of one
application-thread CPU core, 10 ms p95 and 25 ms maximum application-thread frame work, no more
than 96 visible or 256 retained text areas/layouts, 32 retained text renderers, eight draw calls,
two simultaneous popover windows, the resource-growth ceilings above, and zero extra idle frames.
The wrapper independently limits all process threads to 35% CPU over the complete run, peak RSS to
192 MiB, and peak physical footprint to 320 MiB. A documented reference machine can override the
wrapper limits with
`QUICKGUI_ACCEPTANCE_MAX_PROCESS_CPU_PERCENT`, `QUICKGUI_ACCEPTANCE_MAX_RSS_MIB`, and
`QUICKGUI_ACCEPTANCE_MAX_FOOTPRINT_MIB`.

Like the scroll gate, this requires a real logged-in macOS WindowServer and is intentionally not a
headless CI timing assertion. Its schema-11 result records every constraint, mouse assertion,
popover lifecycle, cadence, and current-memory sample; the watchdog names the exact stalled native
phase. Direct AppKit/Winit responder delivery closes each scripted popover too quickly for this to be
a visual QA recording. It emits `QUICKGUI_ACCEPTANCE_RESULT`,
`QUICKGUI_LIFECYCLE_RESULT`, and `QUICKGUI_ACCEPTANCE_PROCESS_RESULT` JSON records so a release
job or local harness can archive exact evidence.

The `virtual_list` benchmark covers both the O(1) uniform list and the sparse variable-height
range/row-declaration path over 100,000 items. The `animation` benchmark covers analytic spring
steps after normal and delayed frames, phase/style projection, duration/spring declaration
construction, and paint-transition declaration construction.

Run validation and the CPU-side list benchmarks with:

```console
cargo test --all-targets
cargo bench --bench virtual_list
cargo bench --bench animation
cargo clippy --all-targets --all-features -- -D warnings
scripts/macos-performance-gate.sh
scripts/macos-acceptance-gate.sh
scripts/macos-display-acceptance-gate.sh
```

# QuickGUI documentation

QuickGUI's documentation is split by concern so public API guidance stays separate from renderer and platform internals.

## Guides

- [View API and layout](view-api.md) — declarative views, Tailwind-style helpers, Flexbox, CSS Grid, and parent-size container queries.
- [QuickGUI UI renderer](ui.md) — scriptc compilation and the Rust C ABI, unstyled native components, JSX compilation, reactive event boundaries, and the current binding scope.
- [Go UI](go.md) — the cgo-free Go frontend: the same Rust core and C ABI, Solid-style signals, and a fast `go build` that loads the host shared library at runtime.
- [Project CLI and application packaging](cli.md) — initialization, on-demand TSX development in a real native app, restart ownership, target builds, and macOS signing.
- [Native modules in Zig](native-modules.md) — `modules/<name>/main.zig` compiled into typed static native libraries: the type mapping, injected arenas, errors, synchronous and background calls, retained state behind handles, and what pays off natively.
- [Application identity, paths, and system information](application-environment.md) — package metadata, app-scoped standard directories, locale/language data, and immutable runtime access.
- [Relaunch and signed updates](relaunch-and-updates.md) — orderly Rust-core restart, bounded progress, signature re-verification, and platform installation strategies.
- [Crash reporting and process metrics](crash-reporting-and-metrics.md) — bounded panic and native-fault reports, retention and upload, an opt-in hang watchdog, and explicit process/system readings.
- [Power, idle, thermal, and session state](power-and-session.md) — RAII sleep assertions, bounded point-in-time snapshots, idle/session queries, and native transition events.
- [System preferences and permissions](system-preferences-and-permissions.md) — appearance and accessibility snapshots, declarative observation, Reduce Motion, and explicit privacy prompts.
- [Desktop integrations](desktop-integrations.md) — taskbar and Dock state, recent documents, About panels, file icons, Jump Lists, native menus, notifications, deep links, and the per-target capability matrix.
- [Windows and shared state](windows.md) — multiple native windows, entities, globals, roles, bounds, appearance, backgrounds, and commands.
- [Native document windows](document-windows.md) — represented files, edited state, character palette, system tabs, and bounded observation.
- [Displays and window placement](displays.md) — bounded monitor snapshots, work areas, scale factors, stable macOS identities, and targeted placement.
- [macOS integration](macos.md) — platform services, notifications, compositor backgrounds, hidden-inset chrome, native views, and first-frame presentation.
- [Text, editing, and forms](text-and-forms.md) — wrapping, rich text, selection, controlled editors, validation, and forms.
- [Retained Markdown](markdown.md) — native CommonMark rendering, incremental streaming updates, semantic styling, hard resource bounds, and QuickGUI UI usage.
- [Selection controls](selection-controls.md) — controlled checkboxes, radio groups, switches, checked/mixed accessibility, and roving keyboard behavior.
- [Collapsible and accordion](disclosures.md) — controlled unstyled disclosure parts, current Tab/Enter/Space behavior, exact heading/region relationships, bounded open values, and mounting policy.
- [Tabs](tabs.md) — controlled unstyled in-window tab parts, manual or automatic activation, horizontal or vertical roving focus, panel mounting, and exact accessibility relationships.
- [Range and feedback components](range-and-feedback.md) — controlled unstyled sliders, number fields, splitters, progress indicators, and meters with captured pointer arithmetic, typed keyboard actions, exact bounds, and numeric accessibility.
- [Toolbar, toggles, and toasts](toolbar-and-toast.md) — roving toolbar focus, pressed-state toggle buttons and groups, and a bounded live-region toast queue with exact one-shot auto-dismissal.
- [Popovers and popover menus](popovers.md) — controlled in-window overlays, cross-edge native WGPU hosts, unstyled menu parts, typed owner actions, nesting, placement, and popover accessibility.
- [In-window menubar](menubar.md) — roving menubar triggers over `PopoverMenu` surfaces, open-on-click, hover switching, Escape/arrow navigation, and menubar accessibility.
- [Context menus](context-menus.md) — unstyled cursor-point triggers, native overflow surfaces, popover-menu composition, exact lifecycle, nested actions, and resource bounds.
- [Select, autocomplete, and combobox](select-and-autocomplete.md) — three distinct unstyled value contracts with overflow-capable native children, never-key owner-IME behavior, accessibility portals, async source replacement, validation, and bounded virtualization.
- [Dialogs](dialogs.md) — unstyled modal portal/backdrop/popover/title/description/close parts, nested focus containment, independent dismissal, restoration, and exact accessibility.
- [Base UI components: separator, avatar, checkbox group, preview card, scroll area, OTP field, drawer, and navigation menu](base-ui-components.md) — orientation-only dividers, delayed avatar fallbacks, derived parent checkboxes, hover preview cards, caller-styled scroll areas, one-character OTP slots with paste distribution, swipe-and-snap drawers over the dialog focus machinery, and a navigation landmark with roving triggers and exact hover deadlines.
- [Unstyled component roadmap](component-roadmap.md) — Base UI-derived component inventory, behavior/presentation boundary, dependency order, and current gaps.
- [Date and time fields](date-and-time.md) — segmented civil date/time editors, leap-year validation, typed-digit entry with automatic advance, wrapping arrow steps, 12/24-hour presentation, a bounded month grid with roving day focus, and spin-button/grid accessibility.
- [Virtual tables and trees](data-collections.md) — controlled sortable grids, bounded hierarchies, composite keyboard focus, collection accessibility, and visible-only mounting.
- [Application assets and custom fonts](assets-and-fonts.md) — bounded bundled resources, stable image handles, view access, and startup font registration.
- [Clipboard](clipboard.md) — bounded text and metadata, encoded images, native file lists, macOS Find pasteboard, and deterministic tests.
- [Graphics and media](graphics.md) — shadows, raster and animated images, SVG, paths, canvas, and custom WGSL.
- [Declarative motion](animations.md) — paint-only web transitions, duration curves, chained stages, retained springs, independently owned tooltip/drag-preview motion, exact throttling, and Reduce Motion.
- [Input and interaction](input.md) — native cursors, exact wheel input, keyboard layouts, IME-safe commands, two-phase focused keys and typed actions, pickers, menus, overlays, tooltips, context menus, and drag/drop.
- [Asynchronous work](async.md) — cancellable foreground futures, exact timers, and bounded background work.
- [Deterministic testing](testing.md) — headless interaction, exact time, geometry assertions, and bounded WGPU screenshots.
- [Retained-tree inspector](inspector.md) — feature-gated picking, hierarchy/layout/hit/accessibility snapshots, frame damage, and resource bounds.
- [Examples and performance checks](examples-and-performance.md) — Hacker News, the 100,000-row stress case, benchmarks, and validation.
- [Releasing QuickGUI 0.1](releasing.md) — clean gates, registry authentication, tag automation, and registry smoke tests.
- [Changelog](../CHANGELOG.md) — user-facing changes in each published QuickGUI version.
- [Status and roadmap](status.md) — implemented surface, ordered release blockers, later work, and explicit non-goals.

## Architecture

The [architecture index](architecture/README.md) routes to focused notes:

- [Runtime and ownership](architecture/runtime.md)
- [Scheduling and performance](architecture/scheduling.md)
- [Input and interaction](architecture/input.md)
- [Layout and rendering](architecture/rendering.md)
- [macOS composition](architecture/macos.md)
- [Deterministic testing](architecture/testing.md)
- [Current boundaries](architecture/boundaries.md)

Return to the [project README](../README.md).

# Repository guidance

This is unreleased project, no migration or backward compatibility need at the moment.

## Architecture

- Applications use Go, TypeScript, or Rust. Go loads the shared library through `purego` with `CGO_ENABLED=0`; TypeScript uses Bun's `bun:ffi` and Solid 2; Rust apps `cargo build` the `quickgui` crate into the executable. All run in the same process. Do not introduce CGO, or an IPC bridge.
- Implement shared native framework capabilities in the Rust core. Language bindings expose core behavior; language-specific component construction and fine-grained reactivity belong in the respective application language. Go containers and controls start empty (`ui.View()`, `ui.Button()`) and append content with `.Child(...)` or `.Children(...)`, matching Rust. Dedicated text factories keep `ui.Text(value)` / `text(value)`. Compound Go controls use instances such as `ui.NewPopover()` with `.Root()` / `.Trigger()`; Rust parts use `.root()` / `.trigger()` without `_part` suffixes; styles, properties, and handlers use fluent methods. Use `ui.Style()` with fluent modifiers and `.Merge(other)` for reusable Go styles; apply them with `.Style(shared)` or compound-part `Style` fields. Do not restore the removed style-option or style-record APIs. Keep layout conveniences aligned with Rust through `scripts/generate-style-helpers.ts`. Treat the component API as unreleased: update consumers directly and document only the current surface.
- Ordinary Go and TypeScript application edits reuse native shared libraries and rebuild only the selected language. Rust application edits rebuild the crate. TypeScript keeps AppKit/Winit on Bun's main thread and Solid plus application I/O in a worker. Use the Solid universal compiler and explicit client-runtime resolution; native callbacks copy borrowed events into the bounded Rust queue before notifying JavaScript. TypeScript JSX puts every layout, typography, paint, and fixed preset declaration in `style` with camelCase keys; direct style attributes, `class`, and `className` are unsupported. See the [TypeScript guide](docs/typescript.md) and [Go guide](docs/go.md) for toolchains and checks.
- Go components and UI construction callbacks return `*ui.Element` or `*native.Node`. Consume constructed nodes explicitly by returning, assigning, or passing them as children. Implicit declarations and void construction callbacks are unsupported. Plain value props and direct reactive expressions remain compiled into retained bindings; no annotations are required. Event, effect, and lifecycle callbacks retain their ordinary signatures.

## Performance

Read and follow the [performance guide](docs/architecture/performance.md) before changing runtime scheduling, reactivity, native-host mutations, layout, rendering, or cache/resource reuse.

## Agent skills

- [QuickGUI application development](.agents/skills/quickgui-app-development/SKILL.md): language-specific component, reactivity, native-service and application build workflows.
- [QuickGUI core changes and review](.agents/skills/quickgui-core-change/SKILL.md): framework ownership, ABI and generated binding changes, packaging review, and focused validation.

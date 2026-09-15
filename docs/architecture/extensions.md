# Native extensions

User guides live on the website: [Go](../../website/src/content/docs/go/en/extensions.mdx) and [TypeScript](../../website/src/content/docs/typescript/en/extensions.mdx).

## Package boundary

Each first-party extension lives in `extensions/[name]/`. Its root contains `Cargo.toml`, `go.mod`, Go bindings, `package.json`, and `quickgui.extension.json`. Rust lives in `src/`; TypeScript bindings and tests live in `js/`; prebuilt native images are staged in `lib/[target]/`. Extension READMEs contain only the build command.

The base host contains one renderer, layout engine, text-input engine, and native event loop. It registers arbitrary extension names and independent versions through a generic descriptor. Editor, Markdown, and Terminal are component packages; Updater is an asynchronous service package. Their algorithms and dependencies are absent from the base host. The host has one generic extension node and does not enumerate component types or interpret their application properties.

Rust applications enable `editor`, `markdown`, or `terminal` crate features to compile the shared implementations into their executable. Go and TypeScript applications load the optional libraries in the same process through purego or Bun FFI. Application edits reuse the selected native images.

## Public ABI

`crates/quickgui-extension-sdk` provides renderer-independent declarations and lifecycle helpers. `include/quickgui_extension.h` exposes the equivalent C tables. The SDK depends only on serialization and the standard library.

A descriptor identifies an extension by name, exact version, ABI version, and table size. The registry accepts generic service and component tables, rejects mismatched identities and conflicting registrations, and caps the total at 32 packages. Package-specific types and function tables do not belong in the registry or SDK.

`ServiceApi` supplies asynchronous invoke/shutdown operations. Replies and session events use bounded JSON and the existing application event queue. Services release every transferred sink exactly once.

`ComponentApi` supplies create, update, render, event, and destroy operations. The extension owns an opaque instance. Its calls run serially on the UI thread; workers communicate through a thread-safe wake handle. Creation owns the wake context on success and failure. Destroy releases component state and initiates worker shutdown; the final worker releases the last wake reference.

`PackageApi` combines both tables under one name and counts as one package. The editor uses its
service capability to load language packs asynchronously; the host does not know what a grammar is.

Both contracts exchange borrowed C-layout spans and opaque handles. Rust objects, allocator ownership, and renderer instances never cross library boundaries. Libraries stay loaded for process lifetime; component/session state is released on unmount.

## Rendering and input

A component renders the SDK's bounded primitive tree: containers, styled text, inputs, buttons, SVG, rectangle drawing, virtual lists, and references to other registered components. Root style declarations remain ordinary host styles. Component output keys are namespaced by instance, preserving retained focus, selection, scroll state, and layout identity.

The host mounts virtual-list rows by requesting only the visible range plus overscan from the owning component. Mirrored columns share one retained metric and scroll state. Row declarations stay separate from viewport dimensions, so wide code does not resize its pane. Remeasurement preserves the logical scroll anchor; document reset uses a separate generation.

Input events return synchronously to the owning component, which may suppress a default, request repaint, use the clipboard, or emit an application event. These application events travel through the existing queued component-change route. Equivalent property updates are no-ops. Background wakes target the owning scope, and declared deadlines permit cursor blinking without a polling loop.

## Discovery and distribution

Go imports discover each module's manifest through `go list -deps`. Package initialization declares its native requirement. The CLI resolves exact-version artifacts from an installed npm package, the source checkout, an explicit artifact directory, or the npm registry cache. Go users do not manually install npm packages.

TypeScript applications install the extension's npm package and list it in their project extension configuration. QuickGUI runtime packages remain peer dependencies, with workspace copies only as development dependencies.

Only selected images and their declared resources enter the application bundle. Artifact downloads verify SHA-512 integrity, extract bounded declared members, and use a bounded cache. Native libraries load locally in-process, without CGO or IPC.

Release tooling builds each extension separately for every supported target, publishes the SDK before the Rust core, publishes each extension's Go module tag, and packages its npm artifact. Third-party publishers use their own names, versions, and release workflows without changing the host.

## Verification

`scripts/check-extensions.ts` checks dependency separation, selected-library packaging, and real purego loading. Core tests exercise an independently named component's lifecycle, virtual rows, and idle behavior through the public ABI. Extension tests exercise the shared algorithms and language bindings. Native GUI checks cover actual rendering, keyboard input, and scrolling.

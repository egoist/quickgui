# Native extensions

For the authoring walkthroughs, see the [Go guide](../../website/src/content/docs/go/en/extensions.mdx) and [TypeScript guide](../../website/src/content/docs/typescript/en/extensions.mdx). They cover reusable packages, independently authored native services, packaging, and registration.

Go applications load one QuickGUI core shared library through purego. Optional backends ship as separate libraries. Third-party services own their names, versions, and npm scopes; they do not need an extension-specific core or CLI change. The terminal extension separates rendering from its backend: the core keeps its retained terminal view while `quickgui-terminal` owns Ghostty, the PTY, and terminal workers. The updater extension uses Sparkle on macOS and a compatible signed-appcast backend on Windows/Linux. Its network and installer code is absent from the default core.

## Application usage

```go
import (
	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/terminal"
	"github.com/egoist/quickgui/go/ui"
)

func Console() *native.Node {
	return terminal.View(terminal.Props{
		Program: "/bin/zsh",
		Args:    []string{"-l"},
		Props: ui.Props{
			Style: ui.Style().Width("100%").Height(320),
		},
	})
}
```

`terminal.Palette`, `terminal.StatusDetails`, and `terminal.StatusFromEvent` replace their previous `ui.Terminal…` equivalents. Normal styles, children, reactive properties, event ownership, and PTY lifecycle remain supported. Do not import the terminal package from `ui`: an ordinary app must stay independent of it.

## Resolution and packaging

Each optional Go package contains a `quickgui.extension.json` manifest and calls `host.RequireExtension("extension-name", "1.0.0")` during package initialization. The manifest's exact version, npm artifact package, native descriptor, and Go requirement must agree. Omitting the version retains the built-in core-release requirement. Initialization only records the requirement. Before running the application, the Go loader opens the selected extension image, obtains `quickgui_extension_v1`, and registers its descriptor with the core.

The CLI runs `go list -deps` with the same entry, target, environment, and build tags as `go build`. A transitive import opts in too; excluded target/tag files do not. The resolver deduplicates requirements, rejects conflicting versions, and supports at most 32 extensions. No feature list is duplicated in application config.

Artifacts are resolved from an explicit `QUICKGUI_EXTENSION_DIR`, an installed matching npm package, or an exact-version npm download. Source-checkout staging is also available for built-in `@quickgui/extension-*` packages. Any valid npm scope or unscoped name is accepted, and cache identities include the publisher's package name. Downloaded tarballs require SHA-512 integrity; only declared target libraries and resources are extracted, with bounded input and decompressed sizes. Cached libraries have checked digests and a 512 MiB eviction budget. The extension loader only loads local files. An updater session starts update networking only after the application explicitly initializes it.

Manifests may declare bounded per-platform `resources`: installer helpers or `.qgr` resource bundles. All native images are staged before resources so extraction cannot overwrite another extension's library. Framework bundles validate every path and byte budget before writing files, then create confined symlinks. macOS preserves Sparkle framework links and signs its nested code with the app identity.

macOS bundles place core and extension images in `Contents/Frameworks`. Linux and Windows payloads place them beside the executable, including AppDir, Debian, and NSIS payloads. Removing a Go import removes the extension from the next fresh bundle. End users need only the packaged application.

## Generic services

`quickgui init-extension <directory> --type go|rust|zig` scaffolds reusable components (Go) or an independent native service (Rust, Zig). Native extensions export `quickgui_extension_v1` and do not depend on the renderer or an application-language runtime.

The same service artifact works from Go and TypeScript applications. Go opts in through an imported manifest and `host.RequireExtension`; TypeScript lists extension packages or directories in `extensions` in `quickgui.toml`. Paths resolve from the app project, and the CLI reads `quickgui.extension.json` inside each directory. Both package the exact extension release before application startup. All scaffold build scripts are TypeScript run with Bun.

Every request/reply/event extension uses `SERVICE_EXTENSION` (kind 2) and the same `ServiceApi`, keyed by its declared name. Registration copies metadata and function tables into a registry bounded to 32 extensions, rejects conflicting identities, and accepts identical repeated registration. Function pointers are copied out before invocation or shutdown, so foreign code never runs under the registry lock. `UPDATER_EXTENSION` remains an alias for binary/source compatibility.

`native.InvokeExtension(name, method, value, done)` supports one-shot operations. `native.OpenExtension` creates a persistent session; `session.Request` returns a JSON result and `session.Command` exposes only an error. Replies and events preserve arbitrary JSON, including explicit null fields, and reach the UI goroutine through the existing queue. Method names and 64 KiB request/reply limits are validated before calling extensions. Closing a session removes its Go subscription; the extension owns cancellation and exactly-once release of every sink.

The standalone [C SDK header](../../include/quickgui_extension.h) is vendorable and has no host, renderer, or language runtime dependency. The [native-extension example](../../examples/native-extension/) implements `acme-echo` version `1.0.0` with a third-party npm scope, compiled independently of the core. Visual features can expose native data through reactive Go components using existing primitives. The generic service ABI does not register new renderer node kinds.

## Updater service

Import `github.com/egoist/quickgui/go/updater` and call `updater.Start` once after application readiness. The core routes service replies and events through the existing Go event queue. `ServiceApi` copies bounded inputs and queues native work; a start sink persists for its session and releases exactly once after its last worker. Command replies do not tear down event subscriptions. Sparkle objects are confined to the macOS main queue. Portable service commands serialize, downloads run off-thread, and closing a session cancels outstanding network work without joining a worker on the UI thread.

The signed installer handoff uses a helper only after payload verification, allowing normal quit hooks to run before loaded application files are replaced. See [automatic updates](../updater.md) for configuration, publication, and platform limits.

## Native contract

`src/extension_api.rs` and `include/quickgui_extension.h` define ABI version 1 using C layouts, fixed-width fields, borrowed spans, function pointers, and opaque session handles. The core checks the descriptor header before reading its layout, then validates the requested extension, function table size, non-null callbacks, ABI version, and exact extension release. Generic service versions are independent of the core; terminal's typed frame adapter still requires the matching core release. Rust-owned allocations, traits, objects, and allocator ownership never cross libraries. Preserve published service layouts and semantics across core releases; an incompatible contract requires a new ABI rather than silently changing version 1.

A terminal creation call owns its wake callback context even on failure. The final worker releases that context exactly once. Destroying a session triggers the existing PTY shutdown path. Images remain loaded for process lifetime because worker callbacks retain function pointers; session resources do not remain alive just because the image does.

Commands are bounded byte spans or JSON metadata. A changed frame crosses in one typed callback, carrying text, style runs, graphics, cursor, selection, scroll, edge colors, and process metadata. The core copies a complete frame once per revision into an immutable snapshot. Unchanged revisions return without invoking the callback or copying screen data. Invalid frames become a terminal failure instead of silently keeping stale content.

The extension compiles the existing backend sources without depending on the core crate, WGPU, Taffy, or the application runtime. The core `terminal-extension` feature compiles the retained adapter without Ghostty or PTY dependencies. Existing Rust users can still opt into the `terminal` feature to link the backend statically. Keep both routes on the shared terminal algorithms.

## Development and releases

```console
bun packages/native/build.ts
bun packages/native/build.ts --extension terminal
bun packages/native/build.ts --extension updater
bun scripts/check-extensions.ts
```

The native build commands rebuild framework artifacts after native changes. Go application edits only run Go compilation and reuse them. Release builds pass `--target` for each published image: `darwin-arm64`, `darwin-x64`, `linux-arm64`, `linux-x64`, and `windows-x64`.

Version synchronization covers the backend crate, npm package, and Go manifest. The release workflow builds those host, terminal, and updater images on macOS, Linux, and Windows in parallel, packages the five npm archives, then publishes them in dependency order. `@quickgui/cli` depends on `@quickgui/native`; `@quickgui/extension-terminal` and `@quickgui/extension-updater` remain optional.

Third-party service authors ship a Go manifest/requirement, a library implementing the public service ABI, and an optional npm artifact package under their own release workflow. They do not extend the core registry or built-in release list. `scripts/check-extensions.ts` builds the standalone C extension, checks real request/reply behavior through purego, and packages an isolated consumer with a separately copied Go module and npm package. It then removes those build inputs before running the bundled executable.

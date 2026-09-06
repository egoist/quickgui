# @quickgui/native

The TypeScript C ABI bindings and static Rust host for QuickGUI applications compiled with scriptc.
Import application, window, dialog, menu, clipboard, storage, and other platform APIs here; use
`@quickgui/ui` for JSX and reactive composition.

```ts
import { app, Window } from "@quickgui/native";
await app.whenReady();
const window = new Window({ title: "QuickGUI", width: 640, height: 480 });
await window.getState();
```

The Rust host owns the AppKit/Winit main thread. Constructors allocate handles and queue creation;
mutations enqueue work; results and native request acceptance arrive through Promise-backed tasks.
CPU-only services execute on the caller's thread. The boundary never waits synchronously for
native main-thread execution.

The package ships `lib/<target>/libquickgui_host.a`, the `dynamic-host` shared library the Go
frontend loads at runtime, its `link.json` recipe, `ffi.json`, TypeScript bindings, and the Zig
module runtime. It does not ship a Node-API addon. Build applications through
`@quickgui/cli` on a matching macOS host. A source checkout builds the host with
`bun run build:native`; `--target` can stage a particular macOS Rust target.

See [the UI guide](../../docs/ui.md) and [native modules](../../docs/native-modules.md).

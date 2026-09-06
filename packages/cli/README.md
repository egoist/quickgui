# @quickgui/cli

Scaffold, compile, develop, and package native TypeScript applications with QuickGUI.

```console
bunx @quickgui/cli init my-app
cd my-app
bun run dev
bun run build
```

## Requirements

Use a matching macOS arm64 or x64 host, Bun for tooling, Node.js 24+ for scriptc, and Xcode Command
Line Tools. TypeScript 7 lowers JSX. The resulting application contains compiled native code and
the Rust host; it does not embed Bun or Node.js.

## Development

`quickgui dev` builds an ad-hoc-signed `.app` under `.quickgui/dev/<target>/`. Source changes compile
a candidate process; the CLI replaces the previous app only after the candidate's first native
window is ready. Compile or startup failures leave the previous app running.

AppKit/Winit owns the main thread and compiled application code runs on a separate native thread.
Bounded queues wake the event loop as work arrives. `--once --no-launch` builds without opening a
window. Enable the additional TypeScript diagnostic pass with `native: { typeCheck: true }`; it is
off by default, while JSX lowering always uses the TypeScript 7 checker.

## Production

`quickgui build` creates a signed `.app` and a versioned `.dmg` using `hdiutil`. Build on the target
Mac architecture; cross-compiling applications is not supported. The native package must contain
the matching static host library and link recipe.

```console
bun run build --target darwin-arm64
bun run build --sign "Developer ID Application: Example (TEAMID)" --notarize quickgui-notary
```

Notarization uses an existing `notarytool` Keychain profile, waits for acceptance, and staples and
validates the DMG. Development builds do not create DMGs.

## Native modules

Put `main.zig` in `modules/<name>/`. Zig 0.16+ compiles each module to a static library, and the CLI
generates typed synchronous and asynchronous wrappers in `index.ts`. The application links the
library through scriptc's C ABI. `quickgui modules` generates bindings and libraries separately;
the generated calls require a compiled application and cannot run directly under Bun.

## Configuration

```ts
import { defineConfig } from "@quickgui/cli";

export default defineConfig({
  name: "My App",
  identifier: "com.example.my-app",
  language: "typescript",
  entry: "src/app.tsx",
  version: "0.1.0",
  resources: ["assets"],
  protocols: ["my-app"],
  native: { typeCheck: false },
  macos: {
    icon: "assets/AppIcon.icns",
    minimumSystemVersion: "14.0",
    signingIdentity: "Developer ID Application: Example (TEAMID)",
    notarization: { keychainProfile: "quickgui-notary" },
  },
});
```

See the [CLI guide](../../docs/cli.md), [UI guide](../../docs/ui.md), and
[native modules guide](../../docs/native-modules.md).

# @quickgui/native

Prebuilt Rust shared libraries for QuickGUI's Go and TypeScript applications. The CLI selects the target asset and bundles it with an application. Both languages load it in process. Rust applications link the `quickgui` crate instead.

This package also exports Bun application bindings: `app`, `Window`, and retained native nodes. TypeScript applications use `@quickgui/solid` for Solid 2 JSX. See the [TypeScript guide](../../docs/typescript.md). Go APIs remain in `github.com/egoist/quickgui/go/native` and `github.com/egoist/quickgui/go/ui`.

Build from a source checkout with `bun run build:native`. Release images are staged in `lib/darwin-arm64`, `lib/darwin-x64`, `lib/linux-arm64`, `lib/linux-x64`, and `lib/windows-x64`. Linux uses `libquickgui_host.so`; Windows uses `quickgui_host.dll`. App code reuses these artifacts without rebuilding Rust. The SDK rejects incompatible protocol versions at startup.

The core library excludes Ghostty and PTY dependencies. Terminal support is a separate `@quickgui/extension-terminal` artifact selected in TypeScript with `extensions: ["terminal"]`, or in Go by importing `github.com/egoist/quickgui/go/terminal`. Updater and third-party service extensions use the same explicit extension registration. See the [extension guide](../../docs/architecture/extensions.md).

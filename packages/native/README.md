# @quickgui/native

Prebuilt Rust shared libraries for QuickGUI's Go and TypeScript applications. The CLI selects the target asset and bundles it with an application. Both languages load it in process. Rust applications link the `quickgui` crate instead.

This package also exports Bun application bindings: `app`, `Window`, and retained native nodes. TypeScript applications use `@quickgui/solid` for Solid 2 JSX. See the [TypeScript guide](../../docs/typescript.md). Go APIs remain in `github.com/egoist/quickgui/go/native` and `github.com/egoist/quickgui/go/ui`.

Build from a source checkout with `bun run build:native`. Release images are staged in `lib/darwin-arm64`, `lib/darwin-x64`, `lib/linux-arm64`, `lib/linux-x64`, and `lib/windows-x64`. Linux uses `libquickgui_host.so`; Windows uses `quickgui_host.dll`. App code reuses these artifacts without rebuilding Rust. The SDK rejects incompatible protocol versions at startup.

The core library excludes the editor grammars and diff engine, Markdown parser, and terminal backend. Editor, Markdown, Terminal, and Updater are separate extension artifacts, selected through the TypeScript `extensions` configuration or Go imports. Third-party components and services use the same generic SDK and registration path without host changes. See the [extension guide](../../docs/architecture/extensions.md).

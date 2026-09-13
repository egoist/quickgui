# Project CLI and application packaging

`@quickgui/cli` supports Go, TypeScript, and Rust applications. Go and TypeScript package the matching Rust shared library (`CGO_ENABLED=0` for Go). Rust apps compile the `quickgui` crate and do not load that library. See the [TypeScript guide](typescript.md) for Bun and Solid 2 JSX, worker ownership, and checks.

## Create a project

```console
bunx @quickgui/cli init my-app
cd my-app
bun run dev
```

The CLI asks you to choose Go, TypeScript, or Rust. Pass `--language go`, `--language typescript`, or `--language rust` to skip the prompt; an explicit language is required in non-interactive environments. `--frontend` is still accepted as an alias.

The Go scaffold contains `main.go`, `go.mod`, `quickgui.config.ts`, and `package.json`. Pass your root component as `native.WindowOptions{Component: Counter}`. `native.Run` owns application startup. The Rust scaffold contains `src/main.rs` and `Cargo.toml`; `Application::run` owns startup. Initialization refuses to overwrite a non-empty directory; `--no-install` skips dependency installation and preparation.

## Development

```console
quickgui dev
quickgui dev --project path/to/app
quickgui dev --once --no-launch
```

On macOS this creates an ad-hoc signed bundle under `.quickgui/dev/<target>/`. Source changes compile and launch a candidate app. The CLI replaces its previous process only after the candidate's first native window is ready; compile/startup failures leave the working app open. Quitting the active app stops that watcher. A small development readiness notification coordinates the CLI; application UI calls always stay inside the Go process through purego.

Components and events run on a dedicated Go goroutine pinned to an OS thread. AppKit/Winit owns the process main thread. Background work dispatches its UI result with `native.Dispatch` or `ui.Async`.

`quickgui fmt` formats Go files recursively using the project's SDK formatter. It wraps long and multiline QuickGUI calls, places callback arguments on separate lines, and then runs standard Go formatting. TypeScript projects use the project's formatter; Rust projects use `rustfmt`. Use `quickgui fmt --check` to check without writing, or `--project path/to/app` to select a project. Generated files, hidden directories, `vendor`, `target`, and `node_modules` are skipped. The SDK must be present in a Go project's `go.mod`.

From this repository, run `bun run build:native` once to stage the Rust library used by Go and TypeScript. Ordinary Go or TypeScript app edits only rebuild that language. Rust apps compile the crate on each change. A released `@quickgui/native` package supplies the shared library; Go and TypeScript consumers do not need a C compiler. Go 1.23+, a current stable Rust toolchain for Rust apps, Bun, and macOS Xcode Command Line Tools are required for development/packaging.

## Configuration

Both `quickgui.toml` and `quickgui.config.ts` are supported. `dev` and `build` look for `quickgui.toml` first, then `quickgui.config.ts`. Pass `--config path/to/file.toml` (or a TypeScript file) to select one explicitly. Both formats use the same option names and validation; relative paths are resolved from the project directory. The generated scaffold continues to use TypeScript.

A `quickgui.toml` can contain:

```toml
language = "go"
name = "My App"
identifier = "com.example.my-app"
entry = "."
version = "0.1.0"
fonts = ["assets/Custom.ttf"]
resources = ["legal/NOTICE.txt"]
protocols = ["my-app"]

[native]
tags = ["production"]

[macos]
icon = "assets/AppIcon.icns"
minimumSystemVersion = "14.0"
signingIdentity = "Developer ID Application: Example (TEAMID)"

[macos.notarization]
keychainProfile = "quickgui-notary"
```

Use quoted strings for `version` and `buildVersion`. Nested options use TOML tables; document types use `[[documentTypes]]` array entries. Config changes are reloaded during `dev`, and malformed TOML reports a parse error instead of falling back to another file.

The equivalent TypeScript configuration is:

```ts
import { defineConfig } from "@quickgui/cli";

export default defineConfig({
  language: "go",
  name: "My App",
  identifier: "com.example.my-app",
  entry: ".", // A Go main package, e.g. "cmd/app".
  version: "0.1.0",
  fonts: ["assets/Custom.ttf"],
  resources: ["legal/NOTICE.txt"],
  protocols: ["my-app"],
  native: { tags: ["production"] },
  macos: {
    icon: "assets/AppIcon.icns",
    minimumSystemVersion: "14.0",
    signingIdentity: "Developer ID Application: Example (TEAMID)",
    notarization: { keychainProfile: "quickgui-notary" },
  },
});
```

The project `resources/` directory is packaged automatically. Put the application icon at `resources/icon.png`. The `resources` array only adds extra files or folders.

`native.libraryPath` or `QUICKGUI_LIBRARY` selects a custom host library for Go and TypeScript. Otherwise the CLI finds the matching asset in `@quickgui/native` or the repository build output. Rust apps ignore that library and link the `quickgui` crate. `native.tags` passes Go build tags. Go application metadata and packaged font paths are injected at link time; Rust metadata is written to `quickgui.json`. There is no runtime TypeScript compiler, JSX lowering, or native-module code generator.

## Production

```console
quickgui build --target darwin-arm64
quickgui build --target darwin-x64
quickgui build --sign "Developer ID Application: Example (TEAMID)" --notarize quickgui-notary
quickgui build --update-manifest --update-base-url https://dl.example.com/demo
quickgui build --mas
```

Production Go builds use `-trimpath -ldflags='-s -w …'`. Production Rust builds use `cargo build --release`. macOS packages put the shared library for Go and TypeScript in `Contents/Frameworks` and resources in `Contents/Resources`. Linux AppDir/Debian and Windows installer payloads keep the shared library, fonts, and `resources` beside the executable. Rust apps omit that library and keep `quickgui.json` with the resources. The signed `.app` is packaged in a versioned DMG with an Applications link. Notarization uses an existing `notarytool` Keychain profile; development builds do not create DMGs. MAS builds use the configured app/installer identities and entitlements. Signed update manifests require the configured update signing key. The project `resources/` directory is packaged automatically; `resources/icon.png` is the application icon. The `resources` config option adds extra files. `macos.icon` / `windows.icon` / `linux.icon` override platform containers.

The Go compiler maps `darwin-x64`, `linux-x64`, and `windows-x64` to `GOARCH=amd64`; arm64 targets use `GOARCH=arm64`. A matching native library and target packaging tools are required. Linux AppDir/Debian and Windows installer payloads include the shared library beside the executable. Published native assets cover macOS arm64/x64, Linux arm64/x64, and Windows x64.

Use `quickgui dev --help` and `quickgui build --help` for all command flags, and [config.ts](../packages/cli/src/config.ts) for typed resource, signing, entitlements, file associations, update, and platform packaging options.

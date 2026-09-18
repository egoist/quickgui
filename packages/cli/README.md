# @quickgui/cli

`@quickgui/cli` supports Go, TypeScript, and Rust QuickGUI applications. It creates projects, watches source, compiles the selected language, and packages the executable. Go and TypeScript reuse matching prebuilt native libraries (`CGO_ENABLED=0` for Go; Bun and Solid 2 for TypeScript). Rust apps `cargo build` the `quickgui` crate and do not load that shared library. See the [TypeScript guide](../../docs/typescript.md).

## Create a project

```console
bunx @quickgui/cli init my-app
cd my-app
bun run dev
```

The CLI asks you to choose Go, TypeScript, or Rust. Pass `--language go`, `--language typescript`, or `--language rust` to skip the prompt; an explicit language is required in non-interactive environments. `--frontend` is still accepted as an alias.

The Go scaffold contains `main.go`, `go.mod`, `quickgui.config.ts`, and `package.json`. Pass your root component as `native.WindowOptions{Component: Counter}`. `native.Run` owns application startup. The Rust scaffold contains `src/main.rs` and `Cargo.toml`; `Application::run` owns startup. Initialization refuses to overwrite a non-empty directory; `--no-install` skips dependency installation and preparation.

## Create an extension

```console
quickgui init-extension my-components --type go
quickgui init-extension my-service --type zig
quickgui init-extension my-service --type rust --module github.com/acme/my-service --npm-package @acme/extension-my-service
```

`go` is the default. It creates a reusable component library and a reactive demo. `zig` and `rust` create independent service libraries, Go wrappers, manifests, and publishable native artifact packages. Each project includes a `cmd/demo` application and `quickgui.toml`; run `bun run dev` inside it. All build scripts are TypeScript executed with Bun.

Use `--name` to override the extension name, `--module` for the Go module path (default `example.com/<name>`), and `--npm-package` for a native artifact package (default `quickgui-extension-<name>`). `--no-install` skips both `bun install` and `go mod tidy`. Existing non-empty directories are preserved.

The Zig template targets Zig 0.16.x; the Rust template uses stable Cargo/Rust. Both produce native extensions usable by Go and TypeScript applications.

Native templates build for the current machine and stage their library in `artifacts/lib/<target>/`. The manifest controls the extension release and the generated native version constants. Application edits reuse the built library; restart `bun run dev` after native changes. Pure Go extensions need no native build toolchain or artifact package.

In this unpublished checkout, run `bun packages/cli/src/cli.ts init-extension <directory> --type <type> --no-install`. Before running the demo, point the generated Go SDK requirement to this checkout with a `replace` directive, set `@quickgui/cli` to a local `file:` dependency, and run the two installation commands. The integration check `bun scripts/check-extension-templates.ts` exercises all three templates from a packed CLI using the local SDK and staged core library.

## Development

```console
quickgui dev
quickgui dev --project path/to/app
quickgui dev --once --no-launch
```

On macOS this creates an ad-hoc signed bundle under `.quickgui/dev/<target>/`. Source changes compile and launch a candidate app. The CLI replaces its previous process only after the candidate's first native window is ready; compile/startup failures leave the working app open. Quitting the active app stops that watcher. A small development readiness notification coordinates the CLI; application UI calls always stay inside the Go process through purego.

Components and events run on a dedicated Go goroutine pinned to an OS thread. AppKit/Winit owns the process main thread. Background work dispatches its UI result with `native.Dispatch` or `ui.Async`.

`quickgui fmt` formats Go files recursively using the project's SDK formatter. It wraps long and multiline QuickGUI calls, places callback arguments on separate lines, breaks long or multiline fluent method chains, and then runs standard Go formatting. TypeScript projects use the project's formatter; Rust projects use `rustfmt`. Use `quickgui fmt --check` to check without writing, or `--project path/to/app` to select a project. Generated files, hidden directories, `vendor`, `target`, and `node_modules` are skipped. The SDK must be present in a Go project's `go.mod`.

From this repository, run `bun run build:native` once to stage the Rust library used by Go and TypeScript. Ordinary Go or TypeScript app edits only rebuild that language. Rust apps compile the crate on each change. A released `@quickgui/native` package supplies the shared library; Go and TypeScript consumers do not need a C compiler. Go 1.23+, a current stable Rust toolchain for Rust apps, Bun, and macOS Xcode Command Line Tools are required for development/packaging.

## Optional native extensions

Import `github.com/egoist/quickgui/extensions/terminal` to use `terminal.View(terminal.Props{…})`. The core UI package does not import the terminal backend. The CLI examines the actual Go dependency graph, including transitive imports, target files, and build tags, then bundles the required extension libraries beside the core library.

Terminal uses the separate `@quickgui/extension-terminal` package. The CLI uses an installed package or downloads the exact SDK-matched version on first use, verifies its SHA-512 integrity, and caches it. Apps without that import neither download nor bundle it. Ordinary Go edits reuse these prebuilt artifacts; no native build or feature-combination matrix is needed.

For offline builds, install the matching `@quickgui/extension-terminal` version beforehand, or put the target libraries in `QUICKGUI_EXTENSION_DIR`. `QUICKGUI_CACHE_DIR` selects the download cache root (default: `~/.cache/quickgui`). Downloads and extracted libraries are limited to 128 MiB each, and the extension cache evicts old libraries above 512 MiB. At runtime, libraries are loaded locally through purego; there is no network access or IPC.

From a source checkout, build each native artifact once:

```console
bun packages/native/build.ts
bun packages/native/build.ts --extension terminal
bun packages/cli/src/cli.ts dev --project examples/herdr-gui
```

The built-in terminal requires the matching core release. Third-party services can use their own names, release versions, and npm scopes with the public service ABI. Put a `quickgui.extension.json` manifest beside the imported Go package and call `host.RequireExtension("your-extension", "1.0.0")`; the CLI bundles it and the core registers it automatically. See the [authoring guide](../../website/src/content/docs/go/en/extensions.mdx) and [standalone C/Go example](../../examples/native-extension/). Packaged apps include their selected libraries and work without Bun, Go, Rust, or npm installed.

## Configuration

Both `quickgui.toml` and `quickgui.config.ts` are supported. `dev` and `build` look for `quickgui.toml` first, then `quickgui.config.ts`. Pass `--config path/to/file.toml` (or a TypeScript file) to select one explicitly. Both formats use the same option names and validation; relative paths are resolved from the project directory. The application scaffold uses TypeScript configuration; extension demos use TOML.

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

The project `resources/` directory is packaged automatically. Put the application icon at `resources/icon.png`. The CLI resizes that PNG to the sizes each platform installer needs. The `resources` array only adds extra files or folders.

`native.libraryPath` or `QUICKGUI_LIBRARY` selects a custom host library for Go and TypeScript. Otherwise the CLI finds the matching asset in `@quickgui/native` or the repository build output. Rust apps ignore that library and link the `quickgui` crate. `native.tags` passes Go build tags. Go application metadata and packaged font paths are injected at link time; Rust metadata is written to `quickgui.json`. There is no runtime TypeScript compiler, JSX lowering, or native-module code generator.

## Production

```console
quickgui build --target darwin-arm64
quickgui build --target darwin-x64
quickgui build --sign "Developer ID Application: Example (TEAMID)" --notarize quickgui-notary
quickgui build --update-manifest
quickgui build --upload
quickgui build --mas
```

Production Go builds use `-trimpath -ldflags='-s -w …'`. Production Rust builds use `cargo build --release`. macOS packages put the shared library for Go and TypeScript in `Contents/Frameworks` and resources in `Contents/Resources`. Linux and Windows keep fonts and `resources` beside the executable. Rust apps omit that library and keep `quickgui.json` with the resources. The signed `.app` is packaged in a versioned DMG with [create-dmg](https://github.com/create-dmg/create-dmg): a Finder window, icon positions, and an Applications drop link. Notarization uses an existing `notarytool` Keychain profile; development builds do not create DMGs. MAS builds use the configured app/installer identities and entitlements. Signed updates and `--upload` are described under [Signed updates and publishing](#signed-updates-and-publishing).

The Go compiler maps `darwin-x64`, `linux-x64`, and `windows-x64` to `GOARCH=amd64`; arm64 targets use `GOARCH=arm64`. A matching native library and target packaging tools are required. Linux AppDir/Debian and Windows installer payloads include the shared library beside the executable. Published native assets cover macOS arm64/x64, Linux arm64/x64, and Windows x64.

## Linux packages

A production Linux build writes these files to `dist/linux-<arch>/`:

| File | What it is | Needs |
| --- | --- | --- |
| `<Name>` plus libraries and resources | The plain executable | nothing |
| `<Name>.desktop` | Desktop entry template used by the packages below | nothing |
| `<Name>-<version>-<arch>.AppImage` | Single-file app. Without `appimagetool` the finished `<Name>.AppDir` is left instead, with the command that completes it | `appimagetool` on `PATH` |
| `<package>_<version>_<arch>.deb` | Debian package, written in pure TypeScript | `linux.maintainer` |
| `<Name>-<version>-linux-<arch>.tar.gz` | Per-user install with a `bin/` + `share/` layout | nothing |
| `install.sh` | Installs the tarball into `~/.local/<package>.app` without root, links `~/.local/bin/<package>`, and registers the desktop entry, icon, and file types | nothing |
| `latest-linux-<arch>.txt` | The version `install.sh` resolves "latest" to | nothing |

`<package>` is the lowercase executable name, for example `my-app`. A project without an icon gets a gray placeholder in the AppImage and tarball; add `resources/icon.png` to replace it.

```toml
[linux]
categories = ["Development"]             # freedesktop menu categories, default ["Utility"]
comment = "A short description"          # Comment= in the desktop entry and the .deb description
maintainer = "Example <hello@example.com>"   # required for, and enables, the .deb
section = "utils"                        # Debian section
depends = ["libgtk-3-0"]                 # Debian Depends
icon = "assets/linux.png"                # only when resources/icon.png and `icon` are absent
appImage = true                          # default true
deb = true                               # default: true when maintainer is set
tarball = true                           # default true; false skips the tarball, install.sh, latest-linux-<arch>.txt
```

Test the install script against a local build without publishing anything. The variable prefix is the package name in upper case:

```console
MY_APP_BUNDLE_PATH=dist/linux-x64/My-App-1.0.0-linux-x64.tar.gz sh dist/linux-x64/install.sh
sh dist/linux-x64/install.sh --uninstall
```

`MY_APP_VERSION` installs a specific version and `MY_APP_RELEASES_URL` downloads from another location. AppImage and tarball installs update themselves when the app includes the updater; `.deb` installs are updated by the package manager.

## Signed updates and publishing

```console
quickgui keygen --out-dir ~/.config/my-app/update-keys
```

`keygen` writes `quickgui-update.pub` and `quickgui-update.key`, an Ed25519 pair. Put the public key in the config and keep the private key out of the repository. `--force` overwrites an existing pair.

```toml
[updates]
target = "github"                        # "github" or "s3": where releases are published and read from
publicKey = "CONTENTS_OF_quickgui-update.pub"
automaticChecks = true
changelog = "CHANGELOG.md"              # optional; the default when the file exists

[updates.github]
repository = "example/my-app"            # public repository; tagPrefix = "v" by default

# With target = "s3":
# [updates.s3]
# bucket = "my-app-releases"
# publicUrl = "https://downloads.example.com"
# endpoint = "https://ACCOUNT_ID.r2.cloudflarestorage.com"   # non-AWS providers
# region = "auto"
# prefix = "stable"
```

| Command | Effect |
| --- | --- |
| `quickgui build` | Builds and packages. Nothing is signed or uploaded. |
| `quickgui build --update-manifest` | Also signs the update files and writes `appcast-<target>.xml`. `updates.manifest = true` does this on every production build. |
| `quickgui build --upload` | Also publishes the installers, update files, feed, `install.sh`, and `latest-linux-<arch>.txt` to the configured target. Implies `--update-manifest`. |

Release notes come from one Markdown changelog for all versions: the build publishes the section under the `## x.y.z` heading that equals `version` (a date may follow, `## x.y.z - 2026-09-19`), and fails when that section is missing. See [Release notes](../../docs/updater.md#release-notes).

Signing reads the private key from `QUICKGUI_UPDATER_PRIVATE_KEY` (or `SPARKLE_PRIVATE_KEY`), otherwise from the file named by `updates.ed25519SecretKey`. Uploading to GitHub uses the `gh` CLI with `GH_TOKEN` or `gh auth login`; S3 uses `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY` (or the `S3_*` names). Run the command once per target. The [updater guide](../../docs/updater.md) covers the application code, every option, a GitHub Actions workflow, and troubleshooting.

Use `quickgui dev --help` and `quickgui build --help` for all command flags, and [config.ts](src/config.ts) for typed resource, signing, entitlements, file associations, update, and platform packaging options.

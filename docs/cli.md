# Project CLI and application packaging

`@quickgui/cli` creates QuickGUI UI projects, runs a reloadable native development application, and
packages production applications for a target platform.

## Create a project

```console
bunx @quickgui/cli init my-app
cd my-app
bun run dev
```

The scaffold imports `app` and `Window` from `@quickgui/native`, then imports `View`, `Text`,
`Button`, and `createRenderer` from `@quickgui/ui`. Both QuickGUI packages are direct
dependencies. It awaits `app.whenReady()` before creating its first window; the CLI owns the
native application loop, and application code never calls `app.run()`. When readiness resolves,
new windows begin opening immediately. QuickGUI UI reactivity such as `createSignal` comes directly from
`@quickgui/ui`.

Initialization refuses to overwrite a non-empty directory. Use `--no-install` when dependency
installation belongs to another workflow.

## Development application

```console
quickgui dev
```

On macOS, development first creates and ad-hoc signs a real bundle at
`.quickgui/dev/darwin-arm64/<executable>.app`. The CLI directly owns the process created from that
bundle's `Contents/MacOS` executable, so the running GUI has normal bundle and `Info.plist` context.

The development executable links scriptc-compiled application code and the Rust host. TypeScript 7
lowers TSX into typed native UI operations. Ordinary source edits therefore follow this sequence:

1. compile the edited source and package a candidate `.app`;
2. start the candidate process with AppKit/Winit on its main thread and compiled application code on its own
   native thread;
3. wait until QuickGUI completes the first native window event-loop turn;
4. stop only the previous process owned by this CLI session.

There is no soft in-isolate reload. If the new source cannot compile or exits before a window is
ready, the candidate is discarded and the last working app stays open. The generated `.app` is
self-contained and also works when launched directly; use `quickgui dev` when the CLI should own
watching and candidate-first process replacement. Quitting the active app also stops the watcher;
an older app terminated as part of a successful reload does not.

AppKit/Winit permanently owns the process main thread. Timers, fetch, streaming, and other
application work run on scriptc's native event loop on the application thread. Bounded command/event queues and a Winit
proxy wake join the two without periodic native pumping.

Available development options:

```console
quickgui dev --project path/to/app
quickgui dev --config quickgui.config.ts
quickgui dev --sign "Apple Development: Example"
quickgui dev --once
quickgui dev --once --no-launch
```

Development and production application builds run on the matching macOS host architecture.
Bun is tooling only; Node.js 24+ runs the scriptc compiler. TypeScript 7 is required for JSX lowering.
The optional extra diagnostic pass is off by default; enable it with `native: { typeCheck: true }`.

Go applications set `language: "go"` in `quickgui.config.ts`. The default `entry` is the project
directory. `quickgui dev` and `quickgui build` run `CGO_ENABLED=0 go build` and copy the prebuilt
host shared library next to the executable (into `Contents/MacOS` on macOS). Zig native modules
and scriptc are skipped. `go` must be on PATH; build the host library with `bun run build:native`
once so `@quickgui/native/lib/<target>/` contains `libquickgui_host`.

## Production package

```console
quickgui build
quickgui build --target darwin-arm64
quickgui build --sign "Developer ID Application: Example (TEAMID)" --notarize quickgui-notary
quickgui build --update-manifest --update-base-url https://dl.example.com/demo
quickgui build --mas
```

Production compilation turns the application, the `@quickgui/ui` runtime, and the
`@quickgui/native` host into one native executable. macOS output includes both a signed `.app` and
a versioned `.dmg` created with `hdiutil`, holding the app next to an Applications link. Ad-hoc app signing is the macOS default, while a DMG built with
a real signing identity is timestamped and signed with the same identity.

Set `macos.notarization` or pass `--notarize <profile>` to submit the DMG with `notarytool --wait`,
then staple and validate the accepted ticket. The profile must already exist in the Keychain:

```console
xcrun notarytool store-credentials quickgui-notary \
  --apple-id developer@example.com \
  --team-id TEAMID
```

Notarization requires a Developer ID signing identity; QuickGUI rejects an ad-hoc notarization
attempt before compiling the application. Development builds remain `.app`-only and never create
or notarize a DMG.

Application compilation currently supports `darwin-arm64` and `darwin-x64` on a matching Mac.
The installed `@quickgui/native` package must contain that target's static host library and link
recipe. Other target names remain reserved for the existing packaging utilities; Linux and Windows
application compilation is not supported by this pipeline.

## Native modules

```console
quickgui modules
quickgui modules --release
quickgui modules --target darwin-x64
```

A directory `modules/<name>/` holding a `main.zig` is a native module: `quickgui dev` and
`quickgui build` compile it into a static library with Zig 0.16 or newer, write the typed
`modules/<name>/index.ts` the application imports, and link the library into the executable.
`quickgui modules` runs only that step, for generated bindings and native test builds; `--release` uses the
production optimization mode. The [native modules guide](native-modules.md) covers the type
mapping, the calling conventions, and the configuration.

## Icons

Point `icon` at one square PNG of at least 256x256 and QuickGUI generates every container it
needs: a `.icns` with the PNG-based `ic07`…`ic13` entries for macOS, a PNG-based `.ico` for the
Windows installer, and `hicolor` PNG sizes for Linux. Both writers are pure TypeScript.

Sizes are collected in this order and never invented:

1. `assets/icon.iconset/icon_<n>x<n>.png`, if that directory exists beside the configured icon;
2. the source PNG itself, when its own size matches;
3. `sips`, which only exists on macOS.

So a macOS host produces every size from one file, while Linux and Windows hosts need the
pre-sized `iconset` directory (`icon_16x16.png` through `icon_1024x1024.png`) for anything the
source PNG does not already cover. A size with no source is simply left out of the container.
`macos.icon` and `windows.icon` still take precedence when you have hand-built containers.

## File associations

`documentTypes` is declared once and reaches every packaging backend:

```ts
documentTypes: [
  {
    name: "Demo Project",
    extensions: ["demo", "demoproj"],
    role: "Editor",                        // or "Viewer"
    mimeTypes: ["application/x-demo-project"],
    utTypeIdentifier: "com.example.demo.project",
    conformsTo: ["public.data"],           // defaults to ["public.data"]
    exported: true,                        // false declares an imported UTI instead
    description: "A Demo project bundle",
  },
],
```

| Backend | Generated |
| --- | --- |
| macOS `Info.plist` | `CFBundleDocumentTypes` plus `UTExportedTypeDeclarations` / `UTImportedTypeDeclarations` |
| Linux desktop entry | `MimeType=` including `x-scheme-handler/<protocol>` entries |
| Linux `shared-mime-info` | `usr/share/mime/packages/<name>.xml` with one `<glob>` per extension |
| Windows NSIS | `Software\Classes` ProgID, `DefaultIcon`, and `shell\open\command` per extension, removed on uninstall |

At most 64 document types, 64 extensions each; extensions are lowercased, stripped of leading
dots, and may not repeat across types.

## Linux packaging utilities

These utilities are retained for future native target support; the application compiler currently
rejects Linux targets.

A production Linux build always writes `<name>.desktop` and, when any document type declares a
MIME type, a `shared-mime-info` XML package. Beyond that:

- an AppDir with `AppRun`, the desktop entry, and the `hicolor` icon tree is assembled whenever
  `linux.appImage` is true (the default). When `appimagetool` is on PATH it is invoked and the
  `.AppImage` is kept; otherwise the finished AppDir is kept and the CLI prints the exact
  `appimagetool` command that completes it;
- a `.deb` is written when `linux.deb` is true, which defaults to true as soon as
  `linux.maintainer` is set. The package is built in pure TypeScript — an `ar` container holding
  `debian-binary`, `control.tar.gz`, and `data.tar.gz` written as ustar and compressed with
  `Bun.gzipSync` — so no `dpkg-deb` is required. `control` carries `Installed-Size`, `Section`
  (default `utils`), and `Depends`, and `md5sums` covers every payload file.

```ts
linux: {
  categories: ["Utility", "Development"],
  comment: "A QuickGUI demo",
  maintainer: "Demo Team <demo@example.com>",
  section: "utils",
  depends: ["libc6"],
},
```

## Windows packaging utilities

These utilities are retained for future native target support; the application compiler currently
rejects Windows targets.

A production Windows build writes an NSIS script next to the executable and runs `makensis` when
it is on PATH; when it is not, the script is kept and the CLI prints the command that compiles it.
The script installs the executable, writes an uninstaller and its
`Software\Microsoft\Windows\CurrentVersion\Uninstall\<identifier>` entry, creates Start Menu and
desktop shortcuts, and registers every `protocols` scheme and `documentTypes` extension.

```ts
windows: {
  publisher: "Example Inc",
  nsis: {
    installDirectory: "$LOCALAPPDATA\\Demo",   // an NSIS expression, used verbatim
    perMachine: false,
    createDesktopShortcut: true,
    createStartMenuShortcut: true,
  },
  signing: {
    subjectName: "Example Inc",              // or certificateFile: "keys/demo.pfx"
    passwordEnvironmentVariable: "WINDOWS_CERT_PASSWORD",
    timestampUrl: "http://timestamp.digicert.com",
    digest: "sha256",
  },
},
```

Authenticode signing runs `signtool` and therefore only executes when the build host is Windows;
on any other host the build succeeds and prints that signing was skipped. Exactly one of
`certificateFile` or `subjectName` must be set.

## Mac App Store

```console
quickgui build --mas
```

`--mas` signs the bundle with the `3rd Party Mac Developer Application` identity and the App
Sandbox entitlements, embeds `Contents/embedded.provisionprofile` before signing, and runs
`productbuild --component <app> /Applications --sign "3rd Party Mac Developer Installer" <pkg>`.
It replaces the DMG rather than adding to it.

```ts
macos: {
  teamIdentifier: "TEAMID",
  appStore: {
    applicationIdentity: "3rd Party Mac Developer Application: Example (TEAMID)",
    installerIdentity: "3rd Party Mac Developer Installer: Example (TEAMID)",
    provisioningProfile: "keys/demo.provisionprofile",
    // entitlements: "MacAppStore.entitlements",
  },
},
```

Both identities are checked against the prefixes Apple requires, and the profile must end in
`.provisionprofile`. When `entitlements` is omitted QuickGUI writes a minimal App Sandbox template
enabling `com.apple.security.app-sandbox`, user-selected file access, and outbound networking,
plus an application group derived from `macos.teamIdentifier` when that is set.

## Signed update manifests

```console
quickgui keygen
quickgui build --update-manifest
quickgui build --update-manifest --update-base-url https://dl.example.com/demo
```

`quickgui keygen` shells out to `minisign -G` or `rsign generate` (whichever is on PATH) and
writes `quickgui-update.pub` / `quickgui-update.key`. `--password` encrypts the secret key;
`--force` overwrites an existing pair.

`quickgui build --update-manifest` produces the exact artifact the Rust updater installs for the
target, signs it with `minisign -S` or `rsign sign`, and writes `latest.json` beside it. See
[Relaunch and signed updates](relaunch-and-updates.md) for the end-to-end flow, the artifact
layout per platform, and the manifest shape.

```ts
updates: {
  manifest: true,
  baseUrl: "https://dl.example.com/demo",
  minisignSecretKey: "keys/quickgui-update.key",  // or QUICKGUI_MINISIGN_SECRET_KEY
  notesFile: "RELEASE_NOTES.md",
},
```

## Configuration

Create `quickgui.config.ts` at the project root:

```ts
import { defineConfig } from "@quickgui/cli";

export default defineConfig({
  name: "My App",
  identifier: "com.example.my-app",
  entry: "src/app.tsx",
  outDir: "dist",
  version: "0.1.0",
  buildVersion: "1",
  resources: ["assets"],
  macos: {
    icon: "assets/AppIcon.icns",
    minimumSystemVersion: "14.0",
    category: "public.app-category.developer-tools",
    signingIdentity: "Developer ID Application: Example (TEAMID)",
    entitlements: "Entitlements.plist",
    dmgTitle: "My App",
    notarization: {
      keychainProfile: "quickgui-notary",
      // keychain: "ci.keychain-db", // Optional non-default Keychain.
    },
  },
  windows: {
    icon: "assets/app.ico",
    hideConsole: true,
  },
  icon: "assets/icon.png",
  documentTypes: [
    {
      name: "Demo Project",
      extensions: ["demo"],
      mimeTypes: ["application/x-demo-project"],
      utTypeIdentifier: "com.example.my-app.project",
    },
  ],
  updates: {
    manifest: true,
    baseUrl: "https://dl.example.com/my-app",
    minisignSecretKey: "keys/quickgui-update.key",
  },
  modules: {
    directory: "modules",     // <directory>/<name>/main.zig, written in Zig
    optimize: "ReleaseFast",  // default: ReleaseSafe for dev, ReleaseFast for build
  },
  linux: {
    maintainer: "My Team <team@example.com>",
    categories: ["Utility"],
  },
});
```

Resource files and directories are copied by basename into the application resources directory.
Two configured resources may not use the same destination name.

Return to the [documentation index](README.md).

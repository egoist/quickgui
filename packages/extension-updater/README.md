# @quickgui/extension-updater

Optional automatic updates for QuickGUI TypeScript and Go applications. The TypeScript API and native updater artifacts live in this package.

For TypeScript, install it with `bun add @quickgui/extension-updater`, register `extensions: ["@quickgui/extension-updater"]` in `quickgui.config.ts`, and import:

```ts
import { app } from "@quickgui/native";
import Updater from "@quickgui/extension-updater";

await app.whenReady();
const updater = new Updater({}, event => console.log(event.status));
await updater.ready;
```

The [TypeScript updater guide](../../website/src/content/docs/typescript/en/updater.mdx) includes a complete configuration and release workflow. The package also exports the `UpdaterOptions` and `UpdateEvent` types.

For Go, import `github.com/egoist/quickgui/go/updater`; the CLI resolves this package at the same release version as the Go SDK. The updater is not a dependency of the default native core.

macOS uses the bundled Sparkle 2.9.4 framework. Windows and Linux implement its signed appcast contract with a native installer helper. App/extension calls use the existing in-process bridge: Bun FFI for TypeScript and purego for Go. The helper runs separately only to install after the app releases its executable and libraries.

Build from the repository with `bun packages/native/build.ts --extension updater`. npm release artifacts cover macOS arm64/x64, Linux arm64/x64, and Windows x64. Configure, sign, and publish updates using the [updater guide](https://github.com/egoist/quickgui/blob/master/docs/updater.md).

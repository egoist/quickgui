# Automatic updates

The optional `github.com/egoist/quickgui/extensions/updater` package provides automatic application updates. Importing it includes `@quickgui/extension-updater`; apps without the import include neither that library nor Sparkle. Ordinary Go edits reuse the prebuilt native artifacts.

## Configure

Generate an Ed25519 key pair with Bun tooling:

```sh
quickgui keygen --sparkle --out-dir /secure/my-app-update-keys
```

Keep the private `.key` outside version control and copy the contents of `.pub` into `quickgui.toml`:

```toml
[updates]
baseUrl = "https://downloads.example.com/my-app"
publicKey = "BASE64_PUBLIC_KEY_FROM_THE_PUB_FILE"
automaticChecks = true
# Optional; {target} expands to darwin-arm64, windows-x64, etc.
feedUrl = "https://downloads.example.com/my-app/appcast-{target}.xml"
# Optional release notes, at most 16 KiB of plain text.
notesFile = "RELEASE_NOTES.md"
```

The CLI embeds the current version, identifier, target-specific feed URL, default preference, and public key in the Go executable. On macOS it also generates Sparkle's Info.plist settings and signs the nested Sparkle helpers with the app's identity. Secret keys are never embedded. Developer ID apps are supported; App Store and sandboxed builds must exclude the updater import using a Go build tag or platform file.

## Start once per application

Start after `native.Run` reports readiness, outside components. Share the handle between settings, menus, and any update banner. An updater belongs to the application, so closing a window must not close it.

```go
import (
	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/extensions/updater"
	"log"
)

var updates *updater.Updater

func startUpdater() {
	updates = updater.Start(updater.Options{}, func(event updater.Event) {
		if event.Error != "" {
			log.Print(event.Error)
		}
		if event.QuitRequired {
			// Runs the app's ordinary quit hooks; save or cancel there as usual.
			native.App.Quit(false, nil)
		}
		// Feed event.Status and event.Version into UI signals for an update banner.
	}, func(err error) {
		if err != nil {
			log.Print(err)
		}
	})
}
```

Call `updates.Check(done)` from a Check for Updates menu item, `updates.Install(done)` from an available-update banner, and `updates.SetAutomaticChecks(enabled, done)` from settings. Callbacks always run on the Go UI goroutine. `State()` reads the latest cached event without calling native code or polling. Initialization and commands report errors through their completion callbacks.

`Options` can override the feed URL and public key. Windows/Linux also allow current version and identifier overrides; Sparkle reads those from the app bundle, and its public key must match the embedded `SUPublicEDKey`. Development builds are disabled by default, including when using a release-built native library. Use `AllowDevelopment: true` only with a deliberately configured test feed and disposable app bundle.

## State and lifecycle

`Event.Status` is `idle`, `checking`, `available`, `downloading`, `installing`, or `disabled`. Events include version, plain-text notes, progress byte counts, the automatic-check preference, and an optional error. An explicit check that finds no update reports `Kind == "up-to-date"` on the portable backend. macOS also uses Sparkle's standard result windows.

On macOS, scheduled discoveries are retained for an in-app banner. Manual checks promote a retained result into Sparkle's standard UI. Sparkle owns download, verification, installation, scheduling, and relaunch. Its persisted preference takes precedence over the packaged default.

Windows and Linux perform one quiet check per launch when enabled, and another when automatic checking is enabled. There is no idle polling timer. Download progress is throttled to at most ten events per second plus completion. Checks and installation cannot overlap; an explicit check adopts an in-flight automatic check.

The portable installer verifies the artifact before preparing a handoff. `QuitRequired` means the helper has validated its inputs and is waiting for the current process to exit. Call `native.App.Quit(false, nil)` after dealing with unsaved work. If quit is cancelled, the helper times out after two minutes and leaves the current application intact. Closing an updater cancels checks and pre-handoff downloads; an accepted installer handoff is already owned by the helper.

## Platform behavior

| Platform | Update payload | Installation |
| --- | --- | --- |
| macOS | ZIP containing the signed `.app` | Sparkle 2.9.4, embedded only with the extension |
| Windows | QuickGUI NSIS `.exe` installer | A helper waits for app exit, runs the verified installer with `/S` and the current install directory, then relaunches |
| Linux | Type-2 `.AppImage` | A helper waits for app exit, replaces the user-owned AppImage atomically, and restores the old image if replacement or startup fails |

Linux `.deb`, system binaries, unpacked AppDirs, and root-owned installations report `disabled`; use their package manager. A new AppImage must retain the updater import so the SDK can acknowledge startup to the helper. Linux backups are deleted only after that acknowledgement and a short startup grace period. Windows installer failures are reported; rollback remains the installer's responsibility.

The portable backend supports HTTPS Sparkle RSS appcasts, raw Ed25519 enclosure signatures, semantic-version ordering, target-specific feeds, and Windows minimum-system requirements. It skips deltas and unsupported channel/rollout constraints. It does not implement every Sparkle feature. Feed reads are limited to 1 MiB, downloads to 512 MiB, and presentation notes to 16 KiB. Signatures and declared lengths are checked again by the installer helper before installation. No unsigned fallback is available.

## Publish

Place the private key in `QUICKGUI_UPDATER_PRIVATE_KEY` (or `SPARKLE_PRIVATE_KEY`), using your CI secret store. Alternatively, `updates.ed25519SecretKey` points to a local key file. Keys use Sparkle's current base64-encoded 32-byte Ed25519 seed format and also work with Sparkle's `sign_update` tool.

```sh
quickgui build --update-manifest
```

The CLI signs the finished platform artifact and writes `appcast-<target>.xml`. Upload the versioned artifact first, then replace the feed at its configured HTTPS URL. Feeds are per target, so architectures cannot accidentally install each other's binaries. The generated feed contains the current release; it does not generate Sparkle delta archives or staged rollouts. `updates.manifest = true` enables generation for every production build.

macOS archives preserve framework symlinks and should contain the Developer ID signed, notarized app. Windows requires `makensis`; sign the installer with the configured Authenticode identity. Linux requires `appimagetool` to produce an updateable AppImage. Public-key mismatches, missing signing keys, missing installers, and unsupported payloads fail the build.

Existing Rust users can explicitly enable the legacy `updater` Cargo feature for the Minisign JSON `UpdateClient`. It is no longer a default feature or part of the Go native core. That legacy client and its CLI manifest path remain separate from this Sparkle-compatible extension; use `updater.Start`, not the removed `native.Updater` facade.

## Build the framework extension

```sh
bun packages/native/build.ts --extension updater
```

The builder stages the shared library plus Sparkle resources on macOS, or the installer helper on Windows/Linux. The CLI discovers the import transitively and fetches the exact-version extension package when necessary. End users need only the packaged application.

The current npm release matrix ships macOS artifacts. Windows/Linux implementations can be built from source on their target hosts; publishing those prebuilt targets follows the framework's broader platform release support.

See the [runnable example](../examples/updater/) and [native extension contract](architecture/extensions.md). Sparkle configuration and appcast details are documented by [Sparkle](https://sparkle-project.org/documentation/).

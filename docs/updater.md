# Automatic updates

QuickGUI's updater lives in the Rust core behind the `updater` Cargo feature. Rust applications enable that feature and call `quickgui::updater` directly. Go and TypeScript applications import `github.com/egoist/quickgui/extensions/updater` or `@quickgui/extension-updater`, a native extension that compiles the same sources behind the service ABI; apps without the import include neither that library nor Sparkle, and ordinary Go edits reuse the prebuilt native artifacts. Every language shares one feed format, key, install helper, and set of platform rules.

## Configure

### 1. Create a signing key

Every update is signed with an Ed25519 key. Create the pair once and keep it for the life of the app: installs only accept updates signed by the key they were built with.

```sh
quickgui keygen --out-dir ~/.config/my-app/update-keys
```

The command writes `quickgui-update.pub` and `quickgui-update.key`. The `.pub` contents go into your config. The `.key` file is the secret: store it in a password manager or your CI secret store, never in the repository. If you lose it, existing installs can never update again.

### 2. Choose where releases are published

Set `updates.target` to `"github"` or `"s3"`, and fill in the section with the same name. The target decides where `--upload` sends files and which URLs the app checks, so you never write a feed or download URL yourself. Both sections may stay in the config; only the one `target` names is used. The feed address is built into the app, so changing `target` later only affects versions built after the change. Keep publishing to the old target until users have moved over.

**GitHub Releases** needs only the repository. It must be public, because installed apps download updates without credentials.

```toml
[updates]
target = "github"                 # publish to and update from GitHub Releases
publicKey = "PASTE_THE_CONTENTS_OF_quickgui-update.pub"
automaticChecks = true            # default for new installs; users can change it
changelog = "CHANGELOG.md"        # optional; this file is used by default when it exists

[updates.github]
repository = "example/my-app"     # owner/name of a PUBLIC repository
# tagPrefix = "v"                 # release tag = tagPrefix + version, e.g. v1.2.0
```

**Amazon S3** needs the bucket, its region, and the public HTTPS address that serves it (the bucket's own URL, or a CloudFront domain in front of it).

```toml
[updates]
target = "s3"                     # publish to and update from the bucket below
publicKey = "PASTE_THE_CONTENTS_OF_quickgui-update.pub"
automaticChecks = true            # default for new installs; users can change it
changelog = "CHANGELOG.md"        # optional; this file is used by default when it exists

[updates.s3]
bucket = "my-app-releases"
region = "us-east-1"
publicUrl = "https://my-app-releases.s3.us-east-1.amazonaws.com"
# prefix = "stable"               # optional folder inside the bucket
```

**Cloudflare R2, MinIO, and other S3-compatible stores** add `endpoint`, the address of the S3 API. `publicUrl` is still the address users download from, which for R2 is a custom domain or the bucket's `r2.dev` URL.

```toml
[updates]
target = "s3"                     # publish to and update from the bucket below
publicKey = "PASTE_THE_CONTENTS_OF_quickgui-update.pub"
automaticChecks = true            # default for new installs; users can change it
changelog = "CHANGELOG.md"        # optional; this file is used by default when it exists

[updates.s3]
bucket = "my-app-releases"
endpoint = "https://ACCOUNT_ID.r2.cloudflarestorage.com"   # S3 API endpoint
region = "auto"
publicUrl = "https://downloads.example.com"                # custom domain or https://pub-….r2.dev
# prefix = "stable"
```

The bucket has to allow public reads of the uploaded files; QuickGUI uploads with your credentials but does not change bucket policy.

### 3. All `updates` options

| Option | Required | Meaning |
| --- | --- | --- |
| `publicKey` | yes | Contents of `quickgui-update.pub`. Embedded in the app and used to verify every update. |
| `target` | yes | `"github"` or `"s3"`. Chooses which section is used. |
| `github.repository` | with `target = "github"` | `owner/name` of a public GitHub repository. |
| `github.tagPrefix` | no | Release tag is this plus the version. Default `"v"`, so version `1.2.0` is released as `v1.2.0`. |
| `s3.bucket` | with `target = "s3"` | Bucket name. |
| `s3.publicUrl` | with `target = "s3"` | Public HTTPS address that serves the bucket. No query string or credentials. |
| `s3.endpoint` | no | S3 API endpoint for non-AWS providers. Omit for AWS. |
| `s3.region` | no | Bucket region, or `"auto"` for R2. |
| `s3.prefix` | no | Folder inside the bucket. It is also appended to `publicUrl`. |
| `automaticChecks` | no | Whether new installs check at launch. Default `true`. A user's own choice is remembered and wins. |
| `changelog` | no | Markdown changelog for all versions. The `## x.y.z` section matching `version` becomes the release notes. Default `CHANGELOG.md` when that file exists. |
| `ed25519SecretKey` | no | Path to the private key file for local builds. The `QUICKGUI_UPDATER_PRIVATE_KEY` environment variable overrides it. |
| `manifest` | no | `true` signs and writes the update feed on every production build, as `--update-manifest` does. |

### Where the URLs come from

The target fixes every address. `<file>` is a name such as `appcast-darwin-arm64.xml`, and `<tag>` is `tagPrefix` plus the version.

| | GitHub | S3 |
| --- | --- | --- |
| Update feed, `install.sh`, `latest-linux-<arch>.txt` | `https://github.com/<repository>/releases/latest/download/<file>` | `<publicUrl>/<prefix>/<file>` |
| Installers and update archives | `https://github.com/<repository>/releases/download/<tag>/<file>` | `<publicUrl>/<prefix>/<file>` |

On GitHub the feed always comes from the release marked **Latest**. Drafts and pre-releases are never latest, so they do not reach users until you publish them as a normal release.

The CLI embeds the version, identifier, feed URL, and public key in the app. Secret keys are never embedded. App Store and sandboxed builds should exclude the updater import with a Go build tag.

On macOS the CLI also generates Sparkle's Info.plist settings and signs the nested Sparkle helpers with the app's identity. Developer ID apps are supported; App Store and sandboxed builds must exclude the updater.

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

## Rust applications

```toml
[dependencies]
quickgui = { version = "...", features = ["updater"] }
```

Start one updater after the application is ready and read its events from the application-thread executor. `quickgui::updater_options!()` expands in your crate to the settings `quickgui build` embedded from `[updates]`; without the CLI (plain `cargo run`) it is empty and the updater reports `Disabled`, like every development build.

```rust
use quickgui::updater::{UpdateStatus, Updater};

let task = cx.spawn(|task_cx: quickgui::AsyncViewContext<Self>| async move {
    let Ok((updater, mut events)) = Updater::start(quickgui::updater_options!()).await else {
        return Ok(());
    };
    let updater = std::rc::Rc::new(updater);
    task_cx.update({
        let updater = updater.clone();
        move |this, _| this.updater = Some(updater)
    }).await?;
    while let Some(event) = events.next().await {
        task_cx.update(move |this, cx| {
            if event.quit_required {
                cx.exit(); // Runs the ordinary quit lifecycle; save or cancel there as usual.
            }
            this.update_available = event.status == UpdateStatus::Available;
            cx.invalidate();
        }).await?;
    }
    Ok::<(), quickgui::AsyncContextError>(())
})?;
```

`updater.check().await`, `updater.install().await`, and `updater.set_automatic_checks(enabled).await` resolve once the command is accepted or rejected; results arrive as events. `updater.state()` returns the latest event without native calls. `Options` fields override the embedded feed URL, public key, version, and identifier, and `allow_development` enables a development build against a deliberately configured test feed. Dropping the `Updater` cancels checks and downloads; an accepted installer handoff is already owned by the helper. The core acknowledges startup to the install helper when the application becomes ready, so no application code is needed for rollback protection.

`quickgui build` detects the feature from Cargo's build output and stages the install helper (and `Sparkle.framework` on macOS) beside the executable, exactly as it does for the extension. See `examples/updater.rs`.

## State and lifecycle

`Event.Status` is `idle`, `checking`, `available`, `downloading`, `installing`, or `disabled`. Events include version, Markdown notes (the changelog section), progress byte counts, the automatic-check preference, and an optional error. An explicit check that finds no update reports `Kind == "up-to-date"` on the portable backend. macOS also uses Sparkle's standard result windows.

On macOS, scheduled discoveries are retained for an in-app banner. Manual checks promote a retained result into Sparkle's standard UI. Sparkle owns download, verification, installation, scheduling, and relaunch. Its persisted preference takes precedence over the packaged default.

Windows and Linux perform one quiet check per launch when enabled, and another when automatic checking is enabled. There is no idle polling timer. Download progress is throttled to at most ten events per second plus completion. Checks and installation cannot overlap; an explicit check adopts an in-flight automatic check.

The portable installer verifies the artifact before preparing a handoff. `QuitRequired` means the helper has validated its inputs and is waiting for the current process to exit. Call `native.App.Quit(false, nil)` after dealing with unsaved work. If quit is cancelled, the helper times out after two minutes and leaves the current application intact. Closing an updater cancels checks and pre-handoff downloads; an accepted installer handoff is already owned by the helper.

## Platform behavior

| Platform | Update payload | Installation |
| --- | --- | --- |
| macOS | ZIP containing the signed `.app` | Sparkle 2.9.4, embedded only with the extension |
| Windows | QuickGUI NSIS `.exe` installer | A helper waits for app exit, runs the verified installer with `/S` and the current install directory, then relaunches |
| Linux | Type-2 `.AppImage`, or the `install.sh` tarball | A helper waits for app exit, replaces the user-owned AppImage or the complete install prefix atomically, and restores the old one if replacement or startup fails |

A Linux release lists both payloads as enclosures of one appcast item; each installation downloads the one it can replace. The running application is an AppImage when `APPIMAGE`/`APPDIR` hold its executable, and a managed prefix when `../share/quickgui/install.json` beside its `bin/` names it and the current user owns the directory. Linux `.deb`, system binaries, unpacked AppDirs, and root-owned installations report `disabled`; use their package manager. A new release must retain the updater import so the SDK can acknowledge startup to the helper. Linux backups are deleted only after that acknowledgement and a short startup grace period.

### The Linux tarball install

`quickgui build` writes `<Name>-<version>-linux-<arch>.tar.gz`, `install.sh`, and `latest-linux-<arch>.txt` unless `linux.tarball = false`. None of them needs an external tool. The archive holds one versioned directory with an install-prefix layout: the executable, native libraries, and resources in `bin/`, and the desktop entry, icons, MIME package, and managed-install marker in `share/`.

`--upload` publishes all three, which gives the script a stable URL at the destination:

```sh
curl -fsSL https://github.com/example/my-app/releases/latest/download/install.sh | sh
curl -fsSL https://github.com/example/my-app/releases/latest/download/install.sh | sh -s -- --uninstall
```

The script needs no root. It resolves the version from `latest-linux-<arch>.txt` (one per architecture, because targets publish independently), refuses a bundle that holds anything but regular files and directories under one top-level directory, unpacks into `~/.local/<package>.app`, links `~/.local/bin/<package>`, and registers the desktop entry (and MIME package) under `$XDG_DATA_HOME` with absolute `Exec=` and `Icon=` paths, so the app appears in the applications menu and its URL schemes and document types resolve. `<PREFIX>_VERSION`, `<PREFIX>_BUNDLE_PATH`, and `<PREFIX>_RELEASES_URL` override the version, install a local tarball, or change the download origin; the prefix is the uppercased package name, such as `MY_APP`. Running the script again upgrades in place.

Updates unpack the verified archive beside the prefix and swap the whole directory, so files dropped from a later layout do not survive. Archives may hold only regular files and directories under one top-level directory; permissions are reduced to `0755`/`0644`, and the marker's identifier must match the installed one. After a successful swap the helper refreshes the registered desktop entry from the new release when it still points at this prefix. Uninstalling leaves settings and data alone. Windows installer failures are reported; rollback remains the installer's responsibility.

The portable backend supports HTTPS Sparkle RSS appcasts, raw Ed25519 enclosure signatures, semantic-version ordering, target-specific feeds, and Windows minimum-system requirements. It skips deltas and unsupported channel/rollout constraints. It does not implement every Sparkle feature. Feed reads are limited to 1 MiB, downloads to 512 MiB, and presentation notes to 16 KiB. Signatures and declared lengths are checked again by the installer helper before installation. No unsigned fallback is available.

## Publish

### 1. Release from your machine

Raise `version` in your config first: apps only install a version higher than their own. Add a `## <version>` section to `CHANGELOG.md` for it. Then give the build the private key and run it with `--upload`:

```sh
export QUICKGUI_UPDATER_PRIVATE_KEY="$(cat ~/.config/my-app/update-keys/quickgui-update.key)"
quickgui build --upload
```

For GitHub, sign in once with `gh auth login` (or set `GH_TOKEN`). For S3, set `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY`.

`--upload` builds the app, signs the update files, writes the feed, and publishes everything. It prints each public URL when it finishes. Use `--update-manifest` instead to sign and write the feed without uploading, for example to inspect `dist/` first. Run the command once per target; every target has its own feed, so a Mac never downloads a Linux build.

### 2. What gets uploaded

| Target | Files |
| --- | --- |
| macOS | `<Name>.dmg` for new users, `<Name>-<version>-<target>.zip` for updates, `appcast-<target>.xml` |
| Windows | `<Name>-<version>-<target>.exe` (installer and update), `appcast-<target>.xml` |
| Linux | `.AppImage`, `<Name>-<version>-<target>.tar.gz`, `.deb` if configured, `install.sh`, `latest-linux-<arch>.txt`, `appcast-<target>.xml` |

Versioned files are uploaded first and the feed last, so the feed never points at a file that is not there yet. On GitHub the first target creates the release `<tag>` (with this version's changelog section as its description) and later targets add their files to it. Re-running replaces files of the same name.

### 3. Release from GitHub Actions

Add the private key as a repository secret named `QUICKGUI_UPDATER_PRIVATE_KEY` (Settings → Secrets and variables → Actions), then push a tag that matches the version in your config:

```yaml
# .github/workflows/release.yml
name: Release
on:
  push:
    tags: ["v*"]

permissions:
  contents: write # lets GITHUB_TOKEN create the release and upload assets

jobs:
  build:
    strategy:
      fail-fast: false
      matrix:
        include:
          - os: macos-latest
            target: darwin-arm64
          - os: ubuntu-22.04
            target: linux-x64
          - os: windows-latest
            target: windows-x64
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: oven-sh/setup-bun@v2
      - uses: actions/setup-go@v5
        with:
          go-version: stable
      - run: bun install
      - run: bunx @quickgui/cli build --target ${{ matrix.target }} --upload
        env:
          QUICKGUI_UPDATER_PRIVATE_KEY: ${{ secrets.QUICKGUI_UPDATER_PRIVATE_KEY }}
          GH_TOKEN: ${{ github.token }}
          # For updates.s3 instead of GH_TOKEN:
          # AWS_ACCESS_KEY_ID: ${{ secrets.AWS_ACCESS_KEY_ID }}
          # AWS_SECRET_ACCESS_KEY: ${{ secrets.AWS_SECRET_ACCESS_KEY }}
```

The three jobs upload into the same release. The `gh` CLI is preinstalled on GitHub's runners. macOS builds that you distribute should also pass `--sign` and `--notarize`; Windows needs NSIS (`makensis`) on the runner, and Linux produces an AppImage only when `appimagetool` is installed (the tarball needs nothing).

### Release notes

Release notes come from one Markdown changelog that covers every version. When you publish version `1.2.0`, the build takes the section under the `## 1.2.0` heading and ships it with the update.

`updates.changelog` names the file, relative to the project. Leave it out and `CHANGELOG.md` in the project directory is used when it exists.

```md
# Changelog

## 1.2.0 - 2026-09-19

Faster startup and a fix for lost drafts.

- Search inside a project with **Ctrl+Shift+F**
- Windows remember their size and position
- Fix drafts being lost when the app quits during a save

## 1.1.0

- Add a tray icon
- Fix a crash when opening an empty project
```

- A release heading is `## x.y.z`, where `x.y.z` is the `version` in your config. A date may follow: `## x.y.z - 2026-09-19`.
- The notes are everything below that heading up to the next `## ` heading. Text above the first release heading, such as the title or an introduction, is never published.
- Use `###` for groups such as "Added" and "Fixed" if you like; they stay inside the section.
- A section may hold at most 16 KiB.
- Put the newest release at the top. Write for the people who use the app: what changed for them, not the commit history.

Publishing fails with ``CHANGELOG.md has no notes for this release. Add a `## 1.2.0` section`` when the heading is missing or its section is empty, so a release cannot go out with the wrong notes. A project with no changelog at all publishes without notes; the update is still offered with its version number. The build also writes the extracted section to `dist/<target>/release-notes.md`.

| Where it appears | How it is shown |
| --- | --- |
| Windows and Linux | Your app receives the Markdown unchanged as `event.Notes`. Render it with the Markdown component, or show it as text. |
| macOS | Sparkle's update window renders it as Markdown. |
| GitHub release page (`target = "github"`) | It becomes the release description. |

### S3 credentials

| Variable | Meaning |
| --- | --- |
| `AWS_ACCESS_KEY_ID` or `S3_ACCESS_KEY_ID` | Access key with permission to write objects in the bucket |
| `AWS_SECRET_ACCESS_KEY` or `S3_SECRET_ACCESS_KEY` | Its secret |
| `AWS_SESSION_TOKEN` or `S3_SESSION_TOKEN` | Only for temporary credentials |

For R2, create an API token with **Object Read & Write** on the bucket and use its access key ID and secret. If a CDN caches your bucket, give `appcast-*.xml`, `latest-linux-<arch>.txt`, and `install.sh` a short cache lifetime, because they change with every release.

### Linux install command

Linux builds also publish `install.sh`. It installs the tarball into `~/.local/<package>.app` without root, adds the app to the applications menu, and that install keeps itself up to date. Give users one line:

```sh
# updates.github
curl -fsSL https://github.com/example/my-app/releases/latest/download/install.sh | sh
# updates.s3
curl -fsSL https://downloads.example.com/install.sh | sh
```

Add `| sh -s -- --uninstall` to the same URL to remove it. Set `linux.tarball = false` to skip the tarball and script.

### Troubleshooting

- **The app says it is up to date.** The published `version` must be higher than the installed one, the release must not be a draft or pre-release on GitHub, and the feed for that target must exist. Open the feed URL from the table above in a browser to check.
- **`Set updates.publicKey and QUICKGUI_UPDATER_PRIVATE_KEY`.** The build has no private key. Export the variable or set `ed25519SecretKey`.
- **`Update signing key does not match updates.publicKey`.** The private key is not the pair of the configured public key. Use the original key; apps in the field only trust that one.
- **`CHANGELOG.md has no notes for this release`.** Add a `## x.y.z` heading that equals `version` exactly, with at least one line under it.
- **The status is `disabled`.** Development builds never update, and neither do `.deb`, system-wide, or root-owned Linux installs. Test with a production build.
- **`GitHub upload failed`.** Check `gh auth status`, that the token can write to the repository (`contents: write` in Actions), and that the repository name is right.
- **`S3 upload failed`.** Check the credentials, `region`, and `endpoint`. A 403 on download means the bucket does not allow public reads.

macOS archives preserve framework symlinks and should contain the Developer ID signed, notarized app. Keys use Sparkle's base64 32-byte Ed25519 seed format and also work with Sparkle's `sign_update` tool. The generated feed describes the current release only; it has no Sparkle delta archives or staged rollouts. Public-key mismatches, missing signing keys, missing installers, and unsupported payloads fail the build.

## Build the helper and extension

```sh
bun packages/native/build.ts --extension updater
```

The builder stages the shared library plus Sparkle resources on macOS, or the installer helper on Windows/Linux. The CLI discovers the Go or TypeScript import transitively, or the Rust `updater` feature, and fetches the exact-version extension package when necessary; Rust applications take only the helper files from it. End users need only the packaged application.

The current npm release matrix ships macOS artifacts. Windows/Linux implementations can be built from source on their target hosts; publishing those prebuilt targets follows the framework's broader platform release support.

See the [runnable example](../examples/updater/) and [native extension contract](architecture/extensions.md). Sparkle configuration and appcast details are documented by [Sparkle](https://sparkle-project.org/documentation/).

# Releasing QuickGUI 0.1

[Documentation index](README.md)

QuickGUI 0.1 stays on the stable Winit 0.30 type universe used by `accesskit_winit` 0.33.2.
The versioned `quickgui-winit` support package carries macOS panel allocation, touch delivery, and
the pre-buffer AppKit mouse click count required by the runtime, while
`quickgui-accesskit-winit` changes only the adapter's Winit dependency.
`quickgui-cosmic-text` adds ordered per-style fallback families to shaping and owned cache keys;
`quickgui-glyphon` adds a per-text-area opacity multiplier after rich-run color resolution so
opacity transitions remain paint-only rather than invalidating shaping and depends on that exact
Cosmic Text support version. Winit 0.31 beta exposes
native panels upstream, but also changes the window and event-loop interfaces; migrating to that
beta is not part of the 0.1 release boundary.

The main crate's minimum supported Rust version is 1.90, matching `libghostty-vt` 0.2.1 as selected
by the optional terminal feature. CI compiles every target and feature with that exact toolchain on
Linux in addition to the stable macOS test, backend, and package jobs.

## Automated release gate

Start from a clean checkout of the intended tag and run:

```console
cargo fmt --all -- --check
cargo test --all-targets --all-features --locked
cargo check --all-targets --all-features --locked
cargo +1.90.0 check --all-targets --all-features --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features --locked
bash -n scripts/*.sh
git diff --check
QUICKGUI_PACKAGE_TOOLCHAIN=1.90.0 scripts/package-release-gate.sh
```

The package gate builds all seven `.crate` archives, extracts the exact normalized contents, and
compiles `tests/downstream_smoke` with only those extracted packages patched into the registry
graph. This catches missing files, accidental path-only dependencies, mismatched renamed-crate
types, missing license/notice files, duplicated vendor sources, and a public API that cannot be
consumed outside this repository. Each archive is also restricted to a per-crate top-level
allowlist, so examples, benches, tests, repository docs, scripts, and workflow files cannot leak
into future crates.io releases. It rejects a dirty source tree by default;
`QUICKGUI_PACKAGE_ALLOW_DIRTY=1` exists only for development verification.
CI sets `QUICKGUI_PACKAGE_TOOLCHAIN=1.90.0`, making both package creation and the fresh downstream
resolution use the declared MSRV rather than the runner's newer default compiler.

The GitHub `CI` workflow runs this complete non-interactive gate on clean commits. Every non-PR
invocation uploads one immutable `quickgui-<version>-crates-<commit>` artifact containing all seven
verified `.crate` archives, `SHA256SUMS`, this release guide, and the changelog. Pull requests verify
the same packages but do not retain release artifacts. The CI workflow never publishes.

A pushed `v*` tag starts the separate `Release` workflow. It does not rerun the CI quality
gates. Ordinary manual dispatch rebuilds natives the same way. `publish_only` plus a prior
run id reuses that run's native artifacts instead of compiling them again. Native host,
terminal, and updater images build in parallel on macOS (arm64 and x64, including Sparkle),
Linux x64, Linux arm64, and Windows x64. Each native job copies those images under
`target/native-libs/packages` with framework symlinks left intact, so
`actions/upload-artifact` cannot strip the `packages/` prefix and Sparkle's `Current` /
`Resources` links do not copy into themselves. The publish job merges the four artifacts,
restores `packages/*/lib` if a flattened layout is present, rejects a tag that is not exactly
`v<root-package-version>` or lacks a dated changelog section, packs the five npm archives from
the downloaded libraries, and publishes crates.io,
the Go module tag, and npm in dependency order. The root `package.json` version is the source
of truth; every published crate, backend, and npm package must match it. The workflow then
creates the GitHub Release. It does not wait for public registry consumers to resolve the
just-published packages; npm can still report a version as missing after a successful publish.

## macOS acceptance evidence

The live probes are separate because they create native windows and measure a real WindowServer:

```console
scripts/macos-performance-gate.sh
scripts/macos-acceptance-gate.sh
scripts/macos-display-acceptance-gate.sh
```

Run a focused probe after changing the subsystem it covers. Do not repeat the complete composition
and 128-cycle popover soak merely because documentation, packaging, examples, or unrelated component
code changed. A release decision may combine the last passing full composition probe with newer
focused performance or display evidence, provided the intervening change and its focused coverage
are recorded in [the status ledger](status.md).

## Registry authentication setup

The private GitHub repository cannot use crates.io trusted publishing. Create a crates.io API token
that can publish these seven crates and store it as the `CARGO_REGISTRY_TOKEN` GitHub Actions secret:

- `quickgui-extension-sdk`
- `quickgui-winit`
- `quickgui-accesskit-winit`
- `quickgui-cosmic-text`
- `quickgui-glyphon`
- `quickgui-system`
- `quickgui`

For each npm package (`@quickgui/native`, `@quickgui/extension-terminal`, `@quickgui/extension-updater`, `@quickgui/solid`, `@quickgui/extension-editor`, `@quickgui/extension-markdown`, and `@quickgui/cli`), add an
[npm Trusted Publisher](https://docs.npmjs.com/trusted-publishers/) with GitHub owner `egoist`,
repository `quickgui`, workflow filename `release.yml`, and no environment. Allow `npm publish`.
npm requires Node 22.14 or newer and npm 11.5.1 or newer for OIDC; the workflow uses Node 24 and
verifies the npm CLI before publication. No `NPM_TOKEN` secret is required.

Creating the workflow does not create the npm registry-side trust records. A missing or misspelled
record makes npm authentication fail with a 404 on `PUT` even when the package already exists.
Each package needs that Trusted Publisher record before OIDC can publish it. From
an npm login with 2FA:

```console
for name in native extension-terminal extension-updater solid extension-editor extension-markdown cli; do
  npm trust github "@quickgui/$name" --file release.yml --repo egoist/quickgui
done
```

Or add the same record in each package's Trusted publishing settings on npmjs.com. This
repository already has those records.

## Version-driven publication

Set only the root `package.json` version, move the shipped changes out of `Unreleased` into a dated
`## <version> - YYYY-MM-DD` section, and commit those changes. Then create the matching annotated
tag and push it:

```console
version=$(bun -p 'require("./package.json").version')
git tag -a "v$version" -m "QuickGUI $version"
git push origin main "v$version"
```

Pushing the tag starts the release automatically. To release manually, open the `Release` workflow,
choose **Run workflow**, and select the branch containing the release commit. The workflow reads the
root `package.json` version and uses `v<version>` for the GitHub Release, creating that tag at the
selected branch commit if it does not already exist. If the tag already points elsewhere, the
workflow stops before publishing. Manual dispatch does not change the root version or changelog.

Every QuickGUI Cargo, Go SDK, and npm release package uses this one version. CI and the release workflow run
the version synchronizer in their checkout before compiling or packaging; it updates every package
manifest, internal dependency pin, generated binding check, and lockfile from the root version.
You never update the Cargo or npm package versions by hand. `bun run version:check` is available to
verify an already synchronized checkout without changing files.

The workflow publishes crates.io packages in this dependency order:

1. `quickgui-extension-sdk`
2. `quickgui-winit`
3. `quickgui-accesskit-winit`
4. `quickgui-cosmic-text`
5. `quickgui-glyphon`
6. `quickgui-system`
7. `quickgui`

The Go SDK is published from the same source commit with a `go/v<version>` tag, as required for
the nested `github.com/egoist/quickgui/go` module. The workflow refuses to move an existing SDK tag
and continues when that tag already exists, so a later npm-only recovery can keep the published
Go module bytes unchanged.
The repository must be readable by Go consumers; a tag alone does not grant access to a private repository.

It then publishes npm packages in the order `@quickgui/native`, `@quickgui/extension-terminal`, `@quickgui/extension-updater`, `@quickgui/solid`, `@quickgui/extension-editor`, `@quickgui/extension-markdown`, and `@quickgui/cli`.
npm 11 refuses a prerelease without `--tag`, so every version is published with `--tag latest`.
A rerun skips a version that is already on the registry instead of republishing or moving
dist-tags; OIDC cannot run `npm dist-tag`.
The terminal and updater packages are optional; the CLI resolves its exact version only when a Go import requires it.
Packages are published back-to-back; npm does not need a prior package to
finish indexing before the next `npm publish`. A rerun skips an existing, non-yanked crate version and skips an npm version that is already
on the registry. Native images are not bit-identical across rebuilds, so a later recovery of the
same version leaves the published tarball in place instead of failing on a checksum mismatch. This
permits safe recovery from a partial registry release without attempting to overwrite immutable versions.

The npm tarballs and their SHA-256 checksums are retained as a workflow artifact and attached to the
GitHub Release. npm trusted publishing works for the private repository, but does not generate
provenance for it.

## Manual recovery

If automation is unavailable, use the same dependency order from a clean, fully verified tag:

```console
cargo publish --manifest-path crates/quickgui-extension-sdk/Cargo.toml --locked
cargo publish --manifest-path vendor/winit/Cargo.toml
cargo publish --manifest-path vendor/accesskit_winit/Cargo.toml
cargo publish --manifest-path vendor/cosmic_text/Cargo.toml
cargo publish --manifest-path vendor/glyphon/Cargo.toml
cargo publish --manifest-path crates/quickgui-system/Cargo.toml --locked
cargo publish --locked
```

Pack npm packages with `bun pm pack`, which resolves `workspace:*` dependencies to their exact
workspace versions, and publish the resulting tarballs with npm 11.5.1 or newer. Pass
`--tag latest` (required for prereleases on npm 11). Do not publish the
workspace directories with npm directly.
Never rerun a successful manual publish; first inspect the public registry and continue after the
last completed package.

Optionally, run `bun scripts/release-registry-smoke.ts <version>` after the registries have
indexed the new version. That compiles a fresh Rust 1.90 consumer without patches, installs the
core and CLI npm packages at that same version in a fresh project, resolves the tagged Go SDK,
and compiles apps with and without the optional terminal import using `CGO_ENABLED=0` using the
installed CLI. npm may still fail to resolve a version that is already published; inspect the
registry directly if that happens. Run the live macOS gates once more from the tagged source if
the published artifacts differ from the previously recorded candidates.

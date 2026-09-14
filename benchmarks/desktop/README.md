# Desktop app comparison

The homepage charts come from real release applications built and measured on
one macOS arm64 machine. Each fixture implements **Orbit**, an offline issue
tracker in a 1100 × 720 content area:

- 1,000 issues with titles, descriptions, owners, priorities, statuses, and notes.
- All, open, and completed filters, plus search by issue ID, title, project, or owner.
- 100 retained rows per page, a scrolling list, and previous/next controls.
- A detail pane with editable notes and complete/reopen actions.

Edits are kept in memory for the session. The initial view contains the first
100 issues and the first issue's details. There is no network, database, or
background synchronization. No fixture includes optional extensions or plugins.
Electron and Tauri use the exact same bundled TypeScript, HTML, and CSS. QuickGUI
uses Go, Bun/Solid 2 TypeScript, and Rust components over the same native core.
GPUI is Zed's GPU UI framework, built as a standalone Rust app.

[`workload.ts`](workload.ts) generates one deterministic dataset embedded in all
six builds; generated copies are ignored by Git. The result records the dataset
SHA-256, record count, page size, and content dimensions.

## Reproduce

Install Bun, Go 1.23 or later, Rust/Cargo, and Xcode Command Line Tools. Stage the
QuickGUI native runtime with `bun run build:native`. Run from the repository root:

```sh
bun install --cwd benchmarks/desktop --frozen-lockfile
bun scripts/benchmark-desktop.ts --publish
```

This builds all six production apps and then launches them sequentially through
LaunchServices. Keep the machine awake and avoid interacting with the benchmark
windows during sampling. It only closes the benchmark processes it started.
Existing user applications are left running. Build output, app bundles, and the
complete result are saved under `target/desktop-benchmarks/`.
The terminal running the benchmark needs macOS Screen Recording access to
inspect window titles and capture each issue tracker window.

Run or build only the [QuickGUI TypeScript fixture](quickgui-typescript) from the repository root:

```sh
bun install
bun run --cwd benchmarks/desktop/quickgui-typescript check
bun run --cwd benchmarks/desktop/quickgui-typescript test
bun run --cwd benchmarks/desktop/quickgui-typescript dev
bun run --cwd benchmarks/desktop/quickgui-typescript build
```

It embeds the same generated JSON, retains 100 rows per page, and checks the dataset,
mounted row count, and native viewport before setting the readiness title. Its bundle
includes Bun and the Rust shared library; its worker runs inside the measured app process.
The homepage retains a measurement date and toolchain for each framework. The
rows can be refreshed independently while retaining the other measurements from
the same machine, OS, workload, and idle policy.

```sh
# Build without opening the apps.
bun scripts/benchmark-desktop.ts --build-only

# Rerun the already-built apps and regenerate the homepage's data files.
bun scripts/benchmark-desktop.ts --measure-only --publish

# Build and measure only GPUI, merging it with the published rows.
bun scripts/benchmark-desktop.ts --only gpui --publish

# Open the packaged Electron app for manual use.
open "target/desktop-benchmarks/electron/Benchmark Electron-darwin-arm64/Benchmark Electron.app"
```

`--publish` writes local website data files and the first launch's screenshot for
each measured framework, shown in the homepage's app preview. It does not deploy
the website. A partial `--only` publish verifies that the existing result used the
same schema, machine, OS, and workload before replacing that framework's row. Every
requested framework must finish successfully before those files are replaced. The
runner identifies the fresh process from each launch; it does not close a copy that
you opened manually.

The runner waits for the packaged app's visible issue tracker window. Electron and
Tauri change the window title only after validating all 1,000 records, the 100
rendered rows, selected issue, inputs, layout, and viewport size, and allowing two
animation frames to paint. Tauri adjusts for
the macOS title bar to keep its WebView content at 1100 × 720. Electron loads HTML from its installed
app path inside the ASAR and exits on load failure. Every launch is captured in
`target/desktop-benchmarks/screenshots/` after memory sampling; inspect these images when
changing a fixture. A browser renderer process alone is not proof that its page
loaded. Run the isolated Electron packaging regression with:

```sh
bun test benchmarks/desktop/electron/main.test.ts
```

## Memory

Each framework is launched three times. The app stays open, with its first issue
selected and no interaction. Sampling starts only after at least 30 seconds and
a 15-second stable interval:

- App and renderer CPU stays at or below 1% of one core in each one-second interval.
- The set of app and renderer processes remains unchanged.
- The memory range stays within 1 MB or 1% of the median, whichever is larger,
  for both the main process and its rendering helpers.

The runner then takes ten further samples, one second apart, requiring continued
stability. Activity restarts the observation, within the same three-minute
deadline. If no complete idle interval is found, the run fails. It does not force
garbage collection, purge caches, close the window, or inject memory pressure. Screenshots happen after the measurements
because capturing a window can allocate temporary surfaces.

Each run is summarized by its median; the chart uses the median of those three
medians. Whiskers show the smallest and largest launch median. Raw data retains
the entire startup trace, the time idle was reached, measured CPU deltas, process
roles, and the final idle samples. The classifier and idle rules have regressions:

```sh
bun test scripts/benchmark-memory.test.ts
```

The metric is **physical footprint**, read through macOS `proc_pid_rusage`, the
[API Apple documents for memory footprint](https://developer.apple.com/videos/play/wwdc2022/10106/).
It includes compressed memory and memory charged to CPU/GPU resources. It is not
JavaScript heap size or RSS. Charts use decimal **MB (1,000,000 bytes)**, matching
Activity Monitor's units; raw data is stored in bytes.

The app total includes its main process, helper executables in its bundle, and
its WebKit renderer, GPU, and networking processes. **AutoFill
(`com.apple.SafariPlatformSupport.Helper`) and other macOS services are excluded.**
They remain in the raw observations with `role: "os-service"` for audit, but do
not enter memory totals or the app's idle CPU calculation. Main-process and
framework-helper medians are also reported separately.

[Tauri uses WKWebView on macOS](https://v2.tauri.app/concept/process-model/),
whose renderer, networking, and GPU processes can have launchd as their parent.
[Electron also has multiple processes](https://www.electronjs.org/docs/latest/tutorial/process-model).
Walking only the main process's children would miss some memory. The runner
finds app-associated processes through the resource coalition using macOS `proc_pidinfo`
([XNU definition](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/sys/proc_info_private.h))
and requires the browser renderer to be present. Membership alone does not make
a process part of the app total: the classification rules above apply afterward.
Missing app/renderer processes and failed samples fail the run.

Disk and shader caches are not purged. These are fresh app processes, not
guaranteed cold-cache launches. System services such as `MTLCompilerService`
are excluded consistently across frameworks.
The machine, OS version/build, toolchains, app versions, executable hashes, and
QuickGUI checkout revision are recorded in the result. The current data uses an
unreleased QuickGUI development checkout and macOS 27.0 build 26A5425a.

## Bundle size

The second chart counts uncompressed file bytes in the complete `.app`, including
the executable, assets, native libraries, and bundled frameworks. Framework
symlinks are not followed and hard-linked files are deduplicated. Directory
allocation, extended attributes, and compression are excluded.

Tauri uses the OS's WebKit, so WebKit is not part of its distributed bundle.
Electron ships Chromium and Node. QuickGUI Go bundles its shared native runtime,
QuickGUI TypeScript also embeds Bun, QuickGUI Rust links the core into its executable,
and GPUI links Zed's GPU UI framework into its executable.
This measures what each fixture ships, rather than adding system libraries to
one framework or removing included libraries from another. It is installed
bundle size, not DMG, ZIP, or installer download size.

The CLI uses its normal Go and Rust release builds. GPUI and Tauri use the default Cargo
release profile. Electron is packaged with ASAR using `@electron/packager`.
Dependency versions are pinned in the fixture manifests and lockfiles.

## Data and limitations

- [Raw samples](../../website/public/benchmarks/desktop-macos-arm64.json)
- [Homepage summary](../../website/src/data/desktop-benchmarks.json)
- [Runner](../../scripts/benchmark-desktop.ts)

This measures a loaded issue tracker **at idle**, on one machine. It measures
neither interaction or rendering throughput, startup latency, build time, nor
the cost of database or network activity. Cross-platform comparisons need new
measurements on each target.
The page preserves the measured values even when another framework is smaller.

### Text rendering comparison

On a Mac with a 2× display, run `bun scripts/compare-text-rendering.ts` after installing
the Electron fixture dependencies. It captures the same text in Electron and QuickGUI's
actual GPU renderer, at several sizes, weights, colors, and opacities. PNGs and measured
stroke coverage are written to `target/desktop-benchmarks/text-reference/`.

This is a diagnostic comparison, not a pixel-equivalence test: QuickGUI follows GPUI's
CoreText alpha-mask treatment, while Chromium applies its own color-dependent correction.
Native mask/cache tests and exact sRGB compositing regressions run in the Rust test suite.

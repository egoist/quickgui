#!/usr/bin/env bun
/** Reproduce the homepage's release-app comparison on macOS arm64. */
import { createHash } from "node:crypto";
import {
  chmodSync,
  cpSync,
  existsSync,
  lstatSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  realpathSync,
  writeFileSync,
} from "node:fs";
import { basename, join, resolve } from "node:path";
import { dlopen, ptr } from "bun:ffi";
import { buildProject } from "../packages/cli/src/build.ts";
import { loadConfig } from "../packages/cli/src/config.ts";
import { createIssues, workload } from "../benchmarks/desktop/workload.ts";
import {
  idlePolicy,
  idleMeasurement,
  median,
  memorySample,
  processRole,
  type MemorySample,
} from "./benchmark-memory.ts";

const root = resolve(import.meta.dir, "..");
const fixtures = join(root, "benchmarks/desktop");
const output = join(root, "target/desktop-benchmarks");
const manifestPath = join(output, "apps.json");
const runs = 3;
const supportedAppIds = [
  "quickgui-go",
  "quickgui-typescript",
  "quickgui-rust",
  "gpui",
  "tauri",
  "electron",
] as const;
type AppId = (typeof supportedAppIds)[number];

interface App {
  id: AppId;
  name: string;
  version: string;
  appPath: string;
  executablePath: string;
}
interface Process {
  pid: number;
  parent: number;
  rssBytes: number;
  path: string;
}

function selectedAppIds(argv: string[]): AppId[] {
  const equals = argv.find((argument) => argument.startsWith("--only="));
  const index = argv.indexOf("--only");
  if (equals && index !== -1) throw new Error("Pass --only only once");
  const value = equals?.slice("--only=".length) ?? (index === -1 ? undefined : argv[index + 1]);
  if (index !== -1 && (!value || value.startsWith("--")))
    throw new Error(`--only requires one of: ${supportedAppIds.join(", ")}`);
  if (!value) return [...supportedAppIds];
  if (!supportedAppIds.includes(value as AppId))
    throw new Error(
      `Unsupported --only value ${JSON.stringify(value)}; expected one of: ${supportedAppIds.join(", ")}`,
    );
  return [value as AppId];
}

async function command(argv: string[], cwd = root): Promise<string> {
  const child = Bun.spawn(argv, {
    cwd,
    env: process.env,
    stdout: "pipe",
    stderr: "pipe",
    stdin: "ignore",
  });
  const [status, stdout, stderr] = await Promise.all([
    child.exited,
    new Response(child.stdout).text(),
    new Response(child.stderr).text(),
  ]);
  if (status !== 0) throw new Error(`${argv.join(" ")} (${status})\n${stdout}\n${stderr}`);
  return stdout.trim();
}

function packageMacApp(options: {
  appPath: string;
  binaryPath: string;
  name: string;
  executableName: string;
  identifier: string;
  version: string;
}) {
  const contents = join(options.appPath, "Contents");
  const macos = join(contents, "MacOS");
  mkdirSync(macos, { recursive: true });
  const executablePath = join(macos, options.executableName);
  cpSync(options.binaryPath, executablePath);
  chmodSync(executablePath, 0o755);
  writeFileSync(
    join(contents, "Info.plist"),
    `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>en</string>
  <key>CFBundleDisplayName</key>
  <string>${options.name}</string>
  <key>CFBundleExecutable</key>
  <string>${options.executableName}</string>
  <key>CFBundleIdentifier</key>
  <string>${options.identifier}</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundleName</key>
  <string>${options.name}</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>${options.version}</string>
  <key>CFBundleVersion</key>
  <string>${options.version}</string>
  <key>LSApplicationCategoryType</key>
  <string>public.app-category.developer-tools</string>
  <key>LSMinimumSystemVersion</key>
  <string>14.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
</dict>
</plist>
`,
  );
  writeFileSync(join(contents, "PkgInfo"), "APPL????");
}

function bytesInBundle(path: string, inodes = new Set<string>()): number {
  const stat = lstatSync(path);
  if (stat.isDirectory())
    return readdirSync(path).reduce(
      (sum, name) => sum + bytesInBundle(join(path, name), inodes),
      0,
    );
  // Framework symlinks must not make the same binary count more than once.
  if (stat.isSymbolicLink()) return stat.size;
  const key = `${stat.dev}:${stat.ino}`;
  if (inodes.has(key)) return 0;
  inodes.add(key);
  return stat.size;
}

async function build(selectedIds: readonly AppId[]): Promise<App[]> {
  const selected = new Set<AppId>(selectedIds);
  const dataset = JSON.stringify(createIssues(), null, 2);
  for (const language of ["go", "typescript", "rust"] as const) {
    const id = `quickgui-${language}` as AppId;
    if (selected.has(id)) writeFileSync(join(fixtures, `${id}/issues.generated.json`), dataset);
  }
  if (selected.has("gpui")) writeFileSync(join(fixtures, "gpui/issues.generated.json"), dataset);
  const webOutput = join(output, "web");
  if (selected.has("tauri") || selected.has("electron")) {
    writeFileSync(join(fixtures, "web/issues.generated.json"), dataset);
    mkdirSync(webOutput, { recursive: true });
    const web = await Bun.build({
      entrypoints: [join(fixtures, "web/app.ts")],
      outdir: webOutput,
      target: "browser",
      minify: true,
    });
    if (!web.success) throw new AggregateError(web.logs, "Could not build the issue tracker");
    cpSync(join(fixtures, "web/index.html"), join(webOutput, "index.html"));
  }
  const apps: App[] = [];
  const quickguiVersion = JSON.parse(
    readFileSync(join(root, "packages/cli/package.json"), "utf8"),
  ).version;
  for (const language of ["go", "typescript", "rust"] as const) {
    const id = `quickgui-${language}` as AppId;
    if (!selected.has(id)) continue;
    console.log(`[benchmark] Building QuickGUI ${language} (release)`);
    const project = join(fixtures, id);
    if (language === "go") await command(["go", "mod", "tidy"], project);
    const result = await buildProject(await loadConfig(project), {
      mode: "production",
      target: "darwin-arm64",
      outDir: join(output, id),
    });
    apps.push({
      id,
      name: `QuickGUI ${language === "go" ? "Go" : language === "rust" ? "Rust" : "TypeScript"}`,
      version: quickguiVersion,
      appPath: result.artifactPath,
      executablePath: result.executablePath,
    });
  }
  if (selected.has("gpui")) {
    console.log("[benchmark] Building GPUI (release)");
    await command(["cargo", "build", "--release", "--manifest-path", join(fixtures, "gpui/Cargo.toml")]);
    const gpuiBinary = join(output, "cargo/release/benchmark-gpui");
    const gpuiApp = join(output, "gpui/Benchmark-GPUI.app");
    packageMacApp({
      appPath: gpuiApp,
      binaryPath: gpuiBinary,
      name: "Benchmark GPUI",
      executableName: "Benchmark-GPUI",
      identifier: "dev.quickgui.benchmark.gpui",
      version: "0.2.2",
    });
    await command(["codesign", "--force", "--deep", "--sign", "-", gpuiApp]);
    await command(["codesign", "--verify", "--deep", "--strict", gpuiApp]);
    apps.push({
      id: "gpui",
      name: "GPUI",
      version: "0.2.2",
      appPath: gpuiApp,
      executablePath: join(gpuiApp, "Contents/MacOS/Benchmark-GPUI"),
    });
  }
  if (selected.has("tauri")) {
    console.log("[benchmark] Building Tauri (release)");
    await command(
      [
        join(fixtures, "node_modules/.bin/tauri"),
        "build",
        "--bundles",
        "app",
        "--target",
        "aarch64-apple-darwin",
      ],
      join(fixtures, "tauri"),
    );
    const tauriApp = join(
      output,
      "cargo/aarch64-apple-darwin/release/bundle/macos/Benchmark Tauri.app",
    );
    apps.push({
      id: "tauri",
      name: "Tauri",
      version: "2.11.5",
      appPath: tauriApp,
      executablePath: join(tauriApp, "Contents/MacOS/benchmark-tauri"),
    });
  }
  if (selected.has("electron")) {
    console.log("[benchmark] Packaging Electron (release)");
    const source = join(output, "electron-source");
    mkdirSync(source, { recursive: true });
    const bundled = await Bun.build({
      entrypoints: [join(fixtures, "electron/main.ts")],
      outdir: source,
      target: "node",
      format: "cjs",
      external: ["electron"],
      minify: true,
    });
    if (!bundled.success) throw new AggregateError(bundled.logs, "Could not bundle Electron entry");
    cpSync(webOutput, source, { recursive: true });
    writeFileSync(
      join(source, "package.json"),
      JSON.stringify({
        name: "benchmark-electron",
        version: "0.1.0",
        main: "main.js",
        private: true,
      }),
    );
    await command(
      [
        join(fixtures, "node_modules/.bin/electron-packager"),
        source,
        "Benchmark Electron",
        "--platform=darwin",
        "--arch=arm64",
        "--electron-version=44.2.0",
        "--asar",
        `--out=${join(output, "electron")}`,
        "--overwrite",
        "--app-bundle-id=dev.quickgui.benchmark.electron",
      ],
      fixtures,
    );
    const electronApp = join(
      output,
      "electron/Benchmark Electron-darwin-arm64/Benchmark Electron.app",
    );
    apps.push({
      id: "electron",
      name: "Electron",
      version: "44.2.0",
      appPath: electronApp,
      executablePath: join(electronApp, "Contents/MacOS/Benchmark Electron"),
    });
  }
  const previousApps: App[] = existsSync(manifestPath)
    ? JSON.parse(readFileSync(manifestPath, "utf8"))
    : [];
  const manifestApps = new Map(previousApps.map((app) => [app.id, app]));
  for (const app of apps) manifestApps.set(app.id, app);
  writeFileSync(
    manifestPath,
    JSON.stringify(
      supportedAppIds.flatMap((id) => manifestApps.get(id) ?? []),
      null,
      2,
    ) + "\n",
  );
  return apps;
}

async function processes(): Promise<Process[]> {
  return (await command(["/bin/ps", "-axo", "pid=,ppid=,rss=,comm="]))
    .split("\n")
    .flatMap((line) => {
      const match = /^\s*(\d+)\s+(\d+)\s+(\d+)\s+(.+)$/.exec(line);
      return match
        ? [{ pid: +match[1]!, parent: +match[2]!, rssBytes: +match[3]! * 1024, path: match[4]! }]
        : [];
    });
}

async function readyWindow(pid: number) {
  // Quartz reports the window actually shown by the packaged process. Both
  // web fixtures set this title only after laying out their issue tracker.
  const inspect = `function run(args) {
    ObjC.import('CoreGraphics');
    const windows = ObjC.deepUnwrap(ObjC.castRefToObject($.CGWindowListCopyWindowInfo(1, 0)));
    return JSON.stringify(windows.filter(w => w.kCGWindowOwnerPID === Number(args[0]) && w.kCGWindowLayer === 0));
  }`;
  for (let attempt = 0; attempt < 100; attempt++) {
    if (!(await processes()).some((item) => item.pid === pid))
      throw new Error("Benchmark exited before its issue tracker was ready");
    const windows = JSON.parse(
      await command(["/usr/bin/osascript", "-l", "JavaScript", "-e", inspect, String(pid)]),
    );
    const window =
      windows.find(
        (item: { kCGWindowName?: string }) => item.kCGWindowName === workload.readyTitle,
      ) ??
      windows.find(
        (item: { kCGWindowBounds?: { Width?: number; Height?: number } }) =>
          item.kCGWindowBounds?.Width === workload.width &&
          (item.kCGWindowBounds?.Height ?? 0) >= workload.height,
      ) ??
      (windows.length === 1 ? windows[0] : undefined);
    if (window) return { id: window.kCGWindowNumber as number, bounds: window.kCGWindowBounds };
    await Bun.sleep(100);
  }
  throw new Error(
    "The issue tracker did not become visible. Check its content and macOS screen-recording access; refusing to measure an empty window.",
  );
}

async function measure(apps: App[]) {
  // Fail on missing tools before spending several minutes sampling applications.
  const machine = {
    chip: await command(["sysctl", "-n", "machdep.cpu.brand_string"]),
    model: await command(["sysctl", "-n", "hw.model"]),
    memoryBytes: Number(await command(["sysctl", "-n", "hw.memsize"])),
    osVersion: await command(["sw_vers", "-productVersion"]),
    osBuild: await command(["sw_vers", "-buildVersion"]),
  };
  const toolchain = {
    go: await command(["go", "version"]),
    rust: await command(["rustc", "--version"]),
    bun: Bun.version,
    solid: JSON.parse(readFileSync(join(fixtures, "quickgui-typescript/package.json"), "utf8"))
      .dependencies["solid-js"],
    quickguiRevision: await command(["git", "rev-parse", "HEAD"]),
    quickguiWorkingTreeDirty: !!(await command(["git", "status", "--porcelain"])),
  };
  // XNU's resource coalition includes launchd-owned WebKit XPC processes.
  // Parent/child PID traversal alone would undercount Tauri on macOS.
  // https://github.com/apple-oss-distributions/xnu/blob/main/bsd/sys/proc_info_private.h
  const library = dlopen("/usr/lib/libproc.dylib", {
    proc_pidinfo: { args: ["i32", "i32", "u64", "ptr", "i32"], returns: "i32" },
    proc_pid_rusage: { args: ["i32", "i32", "ptr"], returns: "i32" },
  });
  const system = dlopen("/usr/lib/libSystem.B.dylib", {
    mach_timebase_info: { args: ["ptr"], returns: "i32" },
  });
  const timebase = new Uint32Array(2);
  if (system.symbols.mach_timebase_info(ptr(timebase)) !== 0 || !timebase[1])
    throw new Error("Could not read the CPU clock timebase");
  const nanosPerTick = timebase[0]! / timebase[1]!;
  function coalition(pid: number): string | null {
    const buffer = new BigUint64Array(5);
    const length = library.symbols.proc_pidinfo(pid, 20, 0, ptr(buffer), buffer.byteLength);
    return length === buffer.byteLength && buffer[0] !== 0n ? String(buffer[0]) : null;
  }
  const ownCoalition = coalition(process.pid);
  const results = [];
  try {
    for (const app of apps) {
      const executable = realpathSync(app.executablePath);
      const appRuns = [];
      for (let run = 0; run < runs; run++) {
        const existingPids = new Set((await processes()).map((item) => item.pid));
        console.log(`[benchmark] ${app.name}: launch ${run + 1}/${runs}`);
        const logs = join(output, "logs");
        mkdirSync(logs, { recursive: true });
        const logPrefix = join(logs, `${app.id}-${run + 1}`);
        await command([
          "/usr/bin/open",
          "-n",
          "--stdout",
          `${logPrefix}.stdout.log`,
          "--stderr",
          `${logPrefix}.stderr.log`,
          app.appPath,
        ]);
        let main: Process | undefined;
        for (let attempt = 0; attempt < 100 && !main; attempt++) {
          main = (await processes()).find(
            (item) => item.path === executable && !existingPids.has(item.pid),
          );
          if (!main) await Bun.sleep(100);
        }
        if (!main) throw new Error(`Application did not start: ${app.name}`);
        const mainPid = main.pid;
        const history: MemorySample[] = [];
        try {
          const resourceCoalition = coalition(mainPid);
          if (!resourceCoalition || resourceCoalition === ownCoalition)
            throw new Error(
              `LaunchServices did not isolate the application coalition: ${app.name}`,
            );
          await readyWindow(mainPid);
          const readyAt = performance.now();
          const observe = async () => {
            const members = (await processes()).filter(
              (item) => coalition(item.pid) === resourceCoalition,
            );
            if (!members.some((item) => item.pid === mainPid))
              throw new Error(`${app.name} exited before the measurement finished`);
            if (
              app.id === "tauri" &&
              !members.some((item) => item.path.includes("WebKit.WebContent"))
            )
              throw new Error(
                "Tauri's WebContent process is missing; refusing to publish an incomplete total",
              );
            if (
              app.id === "electron" &&
              !members.some((item) => item.path.includes("Helper (Renderer)"))
            )
              throw new Error(
                "Electron's renderer process is missing; refusing to publish an incomplete total",
              );
            const observed = members.flatMap((item) => {
              const role = processRole(item.path, item.pid, mainPid, app.appPath);
              // rusage_info_v0: UUID[16], user/system CPU ticks, wakeups, pageins,
              // wired bytes, resident bytes, physical footprint, start/exit times.
              const usage = new BigUint64Array(12);
              if (library.symbols.proc_pid_rusage(item.pid, 0, ptr(usage)) !== 0) {
                if (role === "os-service") return []; // Idle XPC service exited between reads.
                throw new Error(`Could not read app process memory: ${item.path} (${item.pid})`);
              }
              return [
                {
                  pid: item.pid,
                  name: basename(item.path),
                  path: item.path,
                  role,
                  footprintBytes: Number(usage[9]),
                  rssBytes: Number(usage[8]),
                  cpuTimeNs: Number(usage[2]! + usage[3]!) * nanosPerTick,
                },
              ];
            });
            const snapshot = memorySample(
              Math.round(performance.now() - readyAt),
              observed,
              history.at(-1),
            );
            history.push(snapshot);
            return snapshot;
          };
          let measurement;
          while (!(measurement = idleMeasurement(history))) {
            if (performance.now() - readyAt > idlePolicy.maximumWaitMs) {
              throw new Error(
                "App did not reach stable idle memory/CPU; refusing to publish startup samples",
              );
            }
            if (history.length) await Bun.sleep(idlePolicy.intervalMs);
            await observe();
          }
          const { idleDetectedAtMs, startupSamples, samples } = measurement;
          console.log(
            `[benchmark] ${app.name}: idle after ${(idleDetectedAtMs / 1000).toFixed(1)} s`,
          );
          // Capturing a window can allocate temporary surfaces. Do it after all memory
          // samples so preview generation cannot inflate the idle result.
          const window = await readyWindow(mainPid);
          const screenshots = join(output, "screenshots");
          mkdirSync(screenshots, { recursive: true });
          try {
            await command([
              "/usr/sbin/screencapture",
              "-x",
              "-o",
              "-l",
              String(window.id),
              join(screenshots, `${app.id}-${run + 1}.png`),
            ]);
          } catch (cause) {
            console.warn(
              `[benchmark] ${app.name}: could not capture a window screenshot; continuing with memory samples`,
            );
            console.warn(cause);
          }
          appRuns.push({
            run: run + 1,
            windowBounds: window.bounds,
            idleDetectedAtMs,
            medianFootprintBytes: median(samples.map((s) => s.footprintBytes)),
            medianMainProcessBytes: median(samples.map((s) => s.mainProcessBytes)),
            medianFrameworkHelperBytes: median(samples.map((s) => s.frameworkHelperBytes)),
            startupSamples,
            samples,
          });
          writeFileSync(
            `${logPrefix}.measurement.json`,
            JSON.stringify(appRuns.at(-1), null, 2) + "\n",
          );
        } catch (cause) {
          writeFileSync(`${logPrefix}.idle.json`, JSON.stringify(history, null, 2) + "\n");
          throw new Error(`${app.name} launch ${run + 1} failed; see ${logPrefix}.*.log`, {
            cause,
          });
        } finally {
          // Only stop the benchmark process launched by this invocation.
          try {
            process.kill(mainPid, "SIGTERM");
          } catch {
            /* Already exited. */
          }
          for (let attempt = 0; attempt < 50; attempt++) {
            if (!(await processes()).some((item) => item.pid === mainPid)) break;
            await Bun.sleep(100);
          }
        }
      }
      const medians = appRuns.map((run) => run.medianFootprintBytes);
      results.push({
        id: app.id,
        name: app.name,
        version: app.version,
        measuredAt: new Date().toISOString(),
        toolchain,
        bundleBytes: bytesInBundle(app.appPath),
        executableSha256: createHash("sha256")
          .update(readFileSync(app.executablePath))
          .digest("hex"),
        memoryBytes: median(medians),
        memoryMinBytes: Math.min(...medians),
        memoryMaxBytes: Math.max(...medians),
        mainProcessMemoryBytes: median(appRuns.map((run) => run.medianMainProcessBytes)),
        frameworkHelperMemoryBytes: median(appRuns.map((run) => run.medianFrameworkHelperBytes)),
        runs: appRuns,
      });
      console.log(
        `[benchmark] ${app.name}: ${(median(medians) / 1_000_000).toFixed(1)} MB idle app footprint`,
      );
    }
  } finally {
    library.close();
    system.close();
  }
  const report = {
    schemaVersion: 2,
    measuredAt: new Date().toISOString(),
    platform: "macOS arm64",
    workload: {
      ...workload,
      datasetSha256: createHash("sha256")
        .update(JSON.stringify(createIssues(), null, 2))
        .digest("hex"),
    },
    machine,
    toolchain,
    methodology: {
      workload:
        "A 1100 × 720 issue tracker with 1,000 identical records, three sidebar filters, search, 100 retained rows per page, pagination, issue details, editable notes, and completion actions. Idle on the first page with the first issue selected. Edits stay in memory for the session; no network or database service.",
      build:
        "Production builds; no optional plugins. QuickGUI Go uses the CLI release build, QuickGUI TypeScript embeds Bun and a Solid 2 worker with the same native library, and QuickGUI Rust links the crate into its executable. GPUI and Tauri use the default Cargo release profile; Electron is packaged with ASAR.",
      memory:
        "Sum of proc_pid_rusage physical footprints for the main app, bundled helper executables, and coalition-associated WebKit WebContent/GPU/Networking processes. AutoFill/SafariPlatformSupport and other macOS services are recorded but excluded. Includes compressed memory; this is not JavaScript heap size or RSS. Charts use decimal MB (1,000,000 bytes), as in Activity Monitor.",
      bundle:
        "Uncompressed .app file bytes, including bundled libraries/frameworks. Symlinks counted once; OS-provided frameworks such as WebKit are excluded. This is not installer/download size.",
      aggregation:
        "Wait at least 30 seconds after readiness, then require 15 seconds with app/renderer CPU at most 1% of one core, unchanged process membership, and memory spread at most 1 MB or 1% for both main process and framework helpers. Take ten further one-second samples, requiring continued stability; activity restarts the idle observation within the same 180-second deadline. Median per launch, then median of three launch medians; whiskers show the launch-median range. No forced GC, cache purge, or memory-pressure injection.",
      readiness:
        "Wait for a visible issue tracker before observing idle CPU/memory. Capture screenshots only after memory samples, to avoid capture-induced allocations. Tauri and Electron validate the 1,000-record dataset, 100 mounted rows, selected details, input controls, and 1100 × 720 viewport after two animation frames. Web content failures, load failures, or missing renderer processes fail the run.",
      scope:
        "Local measurements on this machine and OS; these values do not predict every app, workload, or platform.",
      runs,
      warmupMs: idlePolicy.minimumWaitMs,
      samplesPerRun: idlePolicy.samplesPerRun,
      intervalMs: idlePolicy.intervalMs,
      idlePolicy,
      measurementDates:
        "Each result records its own measurement date and toolchain; every row uses the same machine, OS, workload, and idle policy.",
    },
    results,
  };
  const publicDir = join(root, "website/public/benchmarks");
  const publishedPath = join(publicDir, "desktop-macos-arm64.json");
  const publish = process.argv.includes("--publish");
  let finalReport = report;
  if (publish && apps.length < supportedAppIds.length) {
    if (!existsSync(publishedPath))
      throw new Error("A partial publish needs the existing website benchmark data");
    const existing = JSON.parse(readFileSync(publishedPath, "utf8")) as typeof report;
    if (
      existing.schemaVersion !== report.schemaVersion ||
      existing.platform !== report.platform ||
      JSON.stringify(existing.workload) !== JSON.stringify(report.workload) ||
      JSON.stringify(existing.machine) !== JSON.stringify(report.machine)
    )
      throw new Error(
        "The existing website data uses a different schema, machine, OS, or workload; rerun every framework instead of merging incomparable results",
      );
    const merged = new Map(existing.results.map((result) => [result.id, result]));
    for (const result of report.results) merged.set(result.id, result);
    const mergedResults = supportedAppIds.map((id) => merged.get(id));
    const missing = supportedAppIds.filter((_, index) => !mergedResults[index]);
    if (missing.length)
      throw new Error(`The existing website data is missing ${missing.join(", ")}`);
    finalReport = {
      ...report,
      results: mergedResults.filter((result) => result !== undefined),
    };
  }
  const json = JSON.stringify(finalReport, null, 2) + "\n";
  writeFileSync(join(output, "results.json"), json);
  if (publish) {
    const dataDir = join(root, "website/src/data");
    mkdirSync(publicDir, { recursive: true });
    mkdirSync(dataDir, { recursive: true });
    for (const app of apps) {
      const screenshot = join(output, "screenshots", `${app.id}-1.png`);
      if (existsSync(screenshot)) cpSync(screenshot, join(publicDir, `${app.id}.png`));
    }
    writeFileSync(publishedPath, json);
    writeFileSync(
      join(dataDir, "desktop-benchmarks.json"),
      JSON.stringify(
        {
          ...finalReport,
          results: finalReport.results.map(({ runs: _runs, ...summary }) => summary),
        },
        null,
        2,
      ) + "\n",
    );
  }
  console.log(`[benchmark] Wrote ${join(output, "results.json")}`);
}

if (import.meta.main) {
  if (process.platform !== "darwin" || process.arch !== "arm64")
    throw new Error(
      "This comparison currently measures macOS arm64; do not label other platforms with these results.",
    );
  mkdirSync(output, { recursive: true });
  process.env.GOWORK = "off";
  process.env.CGO_ENABLED = "0";
  process.env.CARGO_TARGET_DIR = join(output, "cargo");
  const requestedIds = selectedAppIds(process.argv);
  const builtOrManifestApps: App[] = process.argv.includes("--measure-only")
    ? JSON.parse(readFileSync(manifestPath, "utf8"))
    : await build(requestedIds);
  if (builtOrManifestApps.some((app) => !supportedAppIds.includes(app.id)))
    throw new Error(
      "Benchmark manifest contains an unsupported app; rebuild the fixtures before measuring or publishing",
    );
  const apps = builtOrManifestApps.filter((app) => requestedIds.includes(app.id));
  for (const id of requestedIds) {
    if (!apps.some((app) => app.id === id))
      throw new Error(
        `Benchmark manifest is missing ${id}; build that fixture before measuring or publishing`,
      );
  }
  if (!process.argv.includes("--build-only")) await measure(apps);
}

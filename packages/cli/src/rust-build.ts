import { copyFileSync, existsSync, mkdirSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

import type { NativeCompileOptions } from "./native-build.ts";
import type { ResolvedQuickGuiConfig } from "./config.ts";
import { CliError } from "./error.ts";
import { stageExtensionResources, type ExtensionManifest } from "./extensions.ts";
import { updaterMetadata } from "./packaging/appcast.ts";
import { hostTarget, targetInfo, type QuickGuiTarget } from "./targets.ts";

export interface RustBuildPlan {
  argv: string[];
  env: Record<string, string | undefined>;
  manifestPath: string;
  manifestDir: string;
}

export function rustTargetTriple(target: QuickGuiTarget): string {
  const { platform, architecture } = targetInfo(target);
  const arch = architecture === "x64" ? "x86_64" : "aarch64";
  switch (platform) {
    case "darwin":
      return `${arch}-apple-darwin`;
    case "windows":
      return `${arch}-pc-windows-msvc`;
    default:
      return `${arch}-unknown-linux-gnu`;
  }
}

export function rustManifestPath(config: ResolvedQuickGuiConfig): string {
  const fromEntry = config.entry.endsWith("Cargo.toml")
    ? config.entry
    : join(config.entry, "Cargo.toml");
  if (existsSync(fromEntry)) return fromEntry;
  if (
    !config.entry.endsWith("Cargo.toml") &&
    existsSync(config.entry) &&
    statSync(config.entry).isFile()
  ) {
    const beside = join(dirname(config.entry), "Cargo.toml");
    if (existsSync(beside)) return beside;
  }
  const fromRoot = join(config.projectRoot, "Cargo.toml");
  if (existsSync(fromRoot)) return fromRoot;
  return fromEntry;
}

export function requireRustManifest(config: ResolvedQuickGuiConfig): string {
  const path = rustManifestPath(config);
  if (!existsSync(path)) throw new CliError(`Rust Cargo.toml not found for ${config.entry}`);
  return path;
}

export function rustBuildPlan(options: NativeCompileOptions): RustBuildPlan {
  const { config, target, mode } = options;
  const manifestPath = rustManifestPath(config);
  const cross = target !== hostTarget();
  const rustflags = rustFlags(config, target, mode);
  return {
    argv: [
      "cargo",
      "build",
      "--manifest-path",
      manifestPath,
      "--message-format=json-render-diagnostics",
      ...(mode === "production" ? ["--release"] : []),
      ...(cross ? ["--target", rustTargetTriple(target)] : []),
    ],
    env: {
      ...process.env,
      ...(rustflags ? { RUSTFLAGS: rustflags } : {}),
      // Read by `quickgui::updater_options!()`; inert unless the application uses it.
      QUICKGUI_UPDATER_METADATA: Buffer.from(
        JSON.stringify(updaterMetadata(config, target, mode)),
      ).toString("base64url"),
    },
    manifestPath,
    manifestDir: dirname(manifestPath),
  };
}

export function packagedAppMetadata(
  config: ResolvedQuickGuiConfig,
  mode: NativeCompileOptions["mode"],
  fonts: string[],
): { name: string; version: string; identifier: string; fonts: string[] } {
  return {
    name: config.name,
    version: config.version,
    identifier: mode === "development" ? `${config.identifier}.dev` : config.identifier,
    fonts,
  };
}

export function packagedResourceDir(executablePath: string, target: QuickGuiTarget): string {
  return targetInfo(target).platform === "darwin"
    ? resolve(dirname(executablePath), "..", "Resources")
    : dirname(executablePath);
}

/** Files and directories Linux/Windows packaging must keep beside the executable. */
export function rustPackagedSidecars(
  executablePath: string,
  target: QuickGuiTarget,
  fonts: string[],
): string[] {
  if (targetInfo(target).platform === "darwin") return [];
  const resourceDir = packagedResourceDir(executablePath, target);
  return [
    join(resourceDir, "quickgui.json"),
    ...(fonts.length > 0 ? [join(resourceDir, "fonts")] : []),
  ];
}

export function parseCargoExecutable(stdout: string): string {
  const artifacts: Array<{
    reason?: string;
    executable?: string | null;
    target?: { kind?: string[] };
  }> = [];
  for (const line of stdout.split("\n")) {
    if (!line.trim()) continue;
    try {
      artifacts.push(JSON.parse(line) as (typeof artifacts)[number]);
    } catch {
      // Cargo may print a leftover non-JSON line; diagnostics go to stderr.
    }
  }
  const bins = artifacts.filter(
    (message) =>
      message.reason === "compiler-artifact" &&
      typeof message.executable === "string" &&
      message.executable.length > 0 &&
      message.target?.kind?.includes("bin"),
  );
  const executable = bins.at(-1)?.executable;
  if (!executable) throw new CliError("Cargo did not produce an executable");
  return executable;
}

/**
 * Helper files the core's `updater` feature needs beside the application. They ship in
 * `@quickgui/extension-updater` at the core's exact version; the library there is not used,
 * because a Rust application links the updater. Mirrors `extensions/updater/quickgui.extension.json`.
 */
export function rustUpdaterResources(version: string): ExtensionManifest {
  return {
    schema: 1,
    name: "updater",
    abi: 1,
    package: "@quickgui/extension-updater",
    version,
    library: "quickgui_updater",
    resources: {
      darwin: ["Sparkle.framework.qgr"],
      linux: ["quickgui-updater-helper"],
      windows: ["quickgui-updater-helper.exe"],
    },
  };
}

/** The `quickgui` crate Cargo linked into the build: its version and enabled features. */
export function parseCargoCore(stdout: string): { version: string; features: string[] } | undefined {
  for (const line of stdout.split("\n")) {
    if (!line.includes('"compiler-artifact"')) continue;
    try {
      const message = JSON.parse(line) as {
        reason?: string;
        package_id?: string;
        features?: string[];
        target?: { kind?: string[]; name?: string };
      };
      // Package IDs read `<source>#quickgui@<version>`, or `<source>#<version>` when the source
      // directory already carries the package name.
      const version = /#(?:[^#@]+@)?(\d+\.\d+\.\d+[^\s#@]*)$/.exec(message.package_id ?? "")?.[1];
      if (
        message.reason === "compiler-artifact" &&
        version &&
        message.target?.name === "quickgui" &&
        message.target.kind?.some((kind) => kind === "lib" || kind === "rlib")
      )
        return { version, features: message.features ?? [] };
    } catch {
      // Same leftover non-JSON lines parseCargoExecutable skips.
    }
  }
  return undefined;
}

export async function compileRustApplication(options: NativeCompileOptions): Promise<string[]> {
  requireRustManifest(options.config);
  if (!Bun.which("cargo")) throw new CliError("Rust's cargo is required on PATH");
  const plan = rustBuildPlan(options);
  const child = Bun.spawn(plan.argv, {
    cwd: plan.manifestDir,
    stdin: "ignore",
    stdout: "pipe",
    // Cargo keeps machine-readable artifact messages on stdout and writes its
    // human-readable progress and rendered diagnostics to stderr. Keep stdout
    // available for parseCargoExecutable while showing Cargo output live.
    stderr: "inherit",
    env: plan.env,
  });
  const [status, stdout] = await Promise.all([child.exited, new Response(child.stdout).text()]);
  if (status !== 0) {
    throw new CliError("Rust compilation failed");
  }
  const built = parseCargoExecutable(stdout);
  if (!existsSync(built)) throw new CliError(`Cargo did not write ${built}`);
  mkdirSync(dirname(options.executablePath), { recursive: true });
  copyFileSync(built, options.executablePath);
  const resourceDir = packagedResourceDir(options.executablePath, options.target);
  mkdirSync(resourceDir, { recursive: true });
  writeFileSync(
    join(resourceDir, "quickgui.json"),
    `${JSON.stringify(packagedAppMetadata(options.config, options.mode, options.fonts), null, 2)}\n`,
  );
  const core = parseCargoCore(stdout);
  const extensions = core?.features.includes("updater") ? [rustUpdaterResources(core.version)] : [];
  if (extensions.length > 0 && options.mode === "production" && !options.config.updates?.publicKey)
    throw new CliError(
      "The updater requires [updates] with publicKey and a target (github or s3) for production builds",
    );
  // The updater is linked into the executable; only its helper files travel beside it.
  const destination =
    targetInfo(options.target).platform === "darwin"
      ? resolve(dirname(options.executablePath), "..", "Frameworks")
      : dirname(options.executablePath);
  if (extensions.some((extension) => extension.resources?.[targetInfo(options.target).platform]))
    mkdirSync(destination, { recursive: true });
  return [
    ...rustPackagedSidecars(options.executablePath, options.target, options.fonts),
    ...(await stageExtensionResources(
      extensions,
      options.target,
      options.config.projectRoot,
      destination,
    )),
  ];
}

export async function runRust(
  project: string,
  command: "check" | "test" | "fmt",
  options: { check?: boolean; release?: boolean; manifestPath?: string } = {},
): Promise<number> {
  if (!Bun.which("cargo")) throw new CliError("Rust's cargo is required on PATH");
  const manifest = options.manifestPath
    ? ["--manifest-path", options.manifestPath]
    : [];
  const argv =
    command === "fmt"
      ? ["cargo", "fmt", "--all", ...manifest, "--", ...(options.check ? ["--check"] : [])]
      : ["cargo", command, ...manifest, ...(options.release ? ["--release"] : [])];
  const child = Bun.spawn(argv, {
    cwd: project,
    stdin: "inherit",
    stdout: "inherit",
    stderr: "inherit",
  });
  return child.exited;
}

function rustFlags(
  config: ResolvedQuickGuiConfig,
  target: QuickGuiTarget,
  mode: NativeCompileOptions["mode"],
): string | undefined {
  const flags = [
    process.env.RUSTFLAGS,
    targetInfo(target).platform === "windows" && config.windows.hideConsole && mode === "production"
      ? "-C link-arg=/SUBSYSTEM:WINDOWS"
      : undefined,
  ].filter((flag): flag is string => Boolean(flag));
  return flags.length > 0 ? flags.join(" ") : undefined;
}

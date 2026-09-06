/**
 * Compile a Go QuickGUI application with `CGO_ENABLED=0 go build` and stage the
 * prebuilt host shared library next to the executable.
 */

import { cpSync, existsSync, readFileSync } from "node:fs";
import { basename, dirname, join, relative, resolve } from "node:path";

import type { ResolvedQuickGuiConfig } from "./config.ts";
import { CliError } from "./error.ts";
import { targetInfo, type QuickGuiTarget } from "./targets.ts";

export interface GoTargetEnv {
  GOOS: "darwin" | "linux" | "windows";
  GOARCH: "arm64" | "amd64";
}

/** Map a QuickGUI target onto the GOOS/GOARCH pair `go build` expects. */
export function goTargetEnv(target: QuickGuiTarget): GoTargetEnv {
  const info = targetInfo(target);
  return {
    GOOS: info.platform,
    GOARCH: info.architecture === "x64" ? "amd64" : "arm64",
  };
}

/** Filename of the staged `dynamic-host` library for a platform. */
export function hostLibraryName(platform: GoTargetEnv["GOOS"]): string {
  if (platform === "darwin") return "libquickgui_host.dylib";
  if (platform === "windows") return "quickgui_host.dll";
  return "libquickgui_host.so";
}

export function goBuildArgs(options: {
  mode: "development" | "production";
  hideConsole: boolean;
  platform: GoTargetEnv["GOOS"];
  output: string;
  packageArg: string;
}): string[] {
  const flags: string[] = [];
  if (options.mode === "production") flags.push("-s", "-w");
  if (options.hideConsole && options.platform === "windows") flags.push("-H", "windowsgui");
  const args = ["build", "-o", options.output];
  if (flags.length > 0) args.push("-ldflags", flags.join(" "));
  args.push(options.packageArg);
  return args;
}

/** Package path or `.go` file passed to `go build`, relative to the project root. */
export function goPackageArg(config: ResolvedQuickGuiConfig): string {
  const relativePath = relative(config.projectRoot, config.entry);
  if (relativePath === "") return ".";
  return relativePath;
}

export function resolveNativePackageDir(from: string): string {
  try {
    return dirname(Bun.resolveSync("@quickgui/native/package.json", from));
  } catch {
    throw new CliError(
      "Could not resolve `@quickgui/native`; add it as a dependency of the Go application",
    );
  }
}

/** Locate the prebuilt host shared library for `target`. */
export function resolveHostLibrary(projectRoot: string, target: QuickGuiTarget): string {
  const nativeRoot = resolveNativePackageDir(projectRoot);
  const directory = join(nativeRoot, "lib", target);
  const fallback = hostLibraryName(targetInfo(target).platform);
  const linkPath = join(directory, "link.json");
  let name = fallback;
  if (existsSync(linkPath)) {
    const recipe = JSON.parse(readFileSync(linkPath, "utf8")) as { shared?: unknown };
    if (typeof recipe.shared === "string" && recipe.shared.length > 0) {
      name = recipe.shared;
    }
  }
  const path = join(directory, name);
  if (!existsSync(path)) {
    throw new CliError(
      `Host shared library not found at ${path}; run \`bun run build:native\``,
    );
  }
  return path;
}

export function stageHostLibrary(
  projectRoot: string,
  target: QuickGuiTarget,
  executablePath: string,
): string {
  const source = resolveHostLibrary(projectRoot, target);
  const destination = resolve(dirname(executablePath), basename(source));
  cpSync(source, destination);
  return destination;
}

export async function compileGoApplication(options: {
  config: ResolvedQuickGuiConfig;
  mode: "development" | "production";
  target: QuickGuiTarget;
  executablePath: string;
}): Promise<string> {
  const go = Bun.which("go");
  if (!go) {
    throw new CliError("Go compilation needs `go` on PATH");
  }
  const env = goTargetEnv(options.target);
  const args = goBuildArgs({
    mode: options.mode,
    hideConsole: options.config.windows.hideConsole,
    platform: env.GOOS,
    output: options.executablePath,
    packageArg: goPackageArg(options.config),
  });
  const child = Bun.spawn([go, ...args], {
    cwd: options.config.projectRoot,
    env: {
      ...process.env,
      CGO_ENABLED: "0",
      GOOS: env.GOOS,
      GOARCH: env.GOARCH,
    },
    stdin: "ignore",
    stdout: "pipe",
    stderr: "pipe",
  });
  const [status, stdout, stderr] = await Promise.all([
    child.exited,
    new Response(child.stdout).text(),
    new Response(child.stderr).text(),
  ]);
  if (status !== 0) {
    const detail = stderr.trim() || stdout.trim();
    throw new CliError(
      `go build failed${detail ? `:\n${detail}` : ""}`,
    );
  }
  if (!existsSync(options.executablePath)) {
    throw new CliError(`go build reported success but ${options.executablePath} is missing`);
  }
  return stageHostLibrary(options.config.projectRoot, options.target, options.executablePath);
}

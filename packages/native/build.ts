/** Build the Rust shared library once and stage it for purego application builds. */

import { cpSync, existsSync, mkdirSync, realpathSync } from "node:fs";
import { join, resolve } from "node:path";
import { stageSparkle } from "../../scripts/sparkle.ts";

const packageRoot = resolve(import.meta.dir);
const repoRoot = resolve(packageRoot, "..", "..");
const debug = process.argv.includes("--debug");
const extensionIndex = process.argv.indexOf("--extension");
const extension = extensionIndex < 0 ? undefined : process.argv[extensionIndex + 1];
if (extensionIndex >= 0 && extension !== "terminal" && extension !== "updater")
  throw new Error(`Unknown native extension: ${extension ?? "(missing)"}`);

const hostArchitecture =
  process.arch === "arm64" ? "arm64" : process.arch === "x64" ? "x64" : undefined;
if (hostArchitecture === undefined) throw new Error(`Unsupported host architecture: ${process.arch}`);
const hostPlatform =
  process.platform === "darwin"
    ? "darwin"
    : process.platform === "linux"
      ? "linux"
      : process.platform === "win32"
        ? "windows"
        : undefined;
if (hostPlatform === undefined) throw new Error(`Unsupported host platform: ${process.platform}`);

const triples: Record<
  string,
  { triple: string; stage: string; platform: "darwin" | "linux" | "windows" }
> = {
  "aarch64-apple-darwin": {
    triple: "aarch64-apple-darwin",
    stage: "darwin-arm64",
    platform: "darwin",
  },
  "x86_64-apple-darwin": { triple: "x86_64-apple-darwin", stage: "darwin-x64", platform: "darwin" },
  "darwin-arm64": { triple: "aarch64-apple-darwin", stage: "darwin-arm64", platform: "darwin" },
  "darwin-x64": { triple: "x86_64-apple-darwin", stage: "darwin-x64", platform: "darwin" },
  "aarch64-unknown-linux-gnu": {
    triple: "aarch64-unknown-linux-gnu",
    stage: "linux-arm64",
    platform: "linux",
  },
  "x86_64-unknown-linux-gnu": {
    triple: "x86_64-unknown-linux-gnu",
    stage: "linux-x64",
    platform: "linux",
  },
  "linux-arm64": { triple: "aarch64-unknown-linux-gnu", stage: "linux-arm64", platform: "linux" },
  "linux-x64": { triple: "x86_64-unknown-linux-gnu", stage: "linux-x64", platform: "linux" },
  "aarch64-pc-windows-msvc": {
    triple: "aarch64-pc-windows-msvc",
    stage: "windows-arm64",
    platform: "windows",
  },
  "x86_64-pc-windows-msvc": {
    triple: "x86_64-pc-windows-msvc",
    stage: "windows-x64",
    platform: "windows",
  },
  "windows-arm64": { triple: "aarch64-pc-windows-msvc", stage: "windows-arm64", platform: "windows" },
  "windows-x64": { triple: "x86_64-pc-windows-msvc", stage: "windows-x64", platform: "windows" },
};
const requestedIndex = process.argv.indexOf("--target");
const requested = requestedIndex < 0 ? undefined : process.argv[requestedIndex + 1];
const selected = requested === undefined ? undefined : triples[requested];
if (requestedIndex >= 0 && selected === undefined)
  throw new Error(`Unsupported Rust host target: ${requested ?? "(missing)"}`);

const platform = selected?.platform ?? hostPlatform;
const target = selected?.stage ?? `${hostPlatform}-${hostArchitecture}`;
const profile = debug ? "debug" : "release";
const crate = extension ? `quickgui-${extension}` : "quickgui-host";
const cargo = [
  "cargo",
  "build",
  "-p",
  crate,
  ...(extension === "updater" && platform !== "darwin"
    ? ["--lib", "--bin", "quickgui-updater-helper"]
    : ["--lib"]),
  ...(selected === undefined ? [] : ["--target", selected.triple]),
  ...(debug ? [] : ["--release"]),
];
const command =
  platform === "darwin" && extension === "terminal"
    ? [join(repoRoot, "scripts", "with-macos-ghostty-zig.sh"), ...cargo]
    : cargo;
console.log(`[native] ${command.join(" ")}`);
const build = Bun.spawnSync(command, {
  cwd: repoRoot,
  stdin: "inherit",
  stdout: "inherit",
  stderr: "pipe",
  env: process.env,
});
const stderr = build.stderr.toString();
process.stderr.write(stderr);
if (build.exitCode !== 0) process.exit(build.exitCode);

const targetDir = process.env.CARGO_TARGET_DIR
  ? resolve(process.env.CARGO_TARGET_DIR)
  : join(repoRoot, "target");
const basename = extension ? `quickgui_${extension}` : "quickgui_host";
const name =
  platform === "darwin"
    ? `lib${basename}.dylib`
    : platform === "windows"
      ? `${basename}.dll`
      : `lib${basename}.so`;
const library = join(targetDir, ...(selected === undefined ? [] : [selected.triple]), profile, name);
if (!existsSync(library)) throw new Error(`Expected the shared library at ${library}`);
const stage = join(
  extension ? resolve(packageRoot, "..", `extension-${extension}`) : packageRoot,
  "lib",
  target,
);
mkdirSync(stage, { recursive: true });
cpSync(library, join(stage, name));
console.log(`[native] Staged ${realpathSync(join(stage, name))}`);

if (extension === "updater") {
  if (platform === "darwin") await stageSparkle(stage);
  else {
    const helper = "quickgui-updater-helper" + (platform === "windows" ? ".exe" : "");
    cpSync(
      join(targetDir, ...(selected ? [selected.triple] : []), profile, helper),
      join(stage, helper),
    );
  }
}

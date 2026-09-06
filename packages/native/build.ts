/**
 * Build the Rust host static library and stage it, with its link recipe, under `lib/<target>/`.
 *
 * Run with `bun run build` inside `packages/native` (or `bun run build:native` at the root).
 * The CLI links every application against the staged archive, so a source checkout builds it once
 * and the published package ships one archive per supported target.
 */

import { cpSync, existsSync, mkdirSync, realpathSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

const packageRoot = resolve(import.meta.dir);
const repoRoot = resolve(packageRoot, "..", "..");
const debug = process.argv.includes("--debug");

const architecture = process.arch === "arm64" ? "arm64" : process.arch === "x64" ? "x64" : undefined;
if (architecture === undefined) throw new Error(`Unsupported host architecture: ${process.arch}`);
const platform = process.platform === "darwin" ? "darwin" : process.platform === "linux" ? "linux" : process.platform === "win32" ? "windows" : undefined;
if (platform === undefined) throw new Error(`Unsupported host platform: ${process.platform}`);
const requestedIndex = process.argv.indexOf("--target");
const requested = requestedIndex < 0 ? undefined : process.argv[requestedIndex + 1];
const triples: Record<string, { triple: string; stage: string }> = {
  "aarch64-apple-darwin": { triple: "aarch64-apple-darwin", stage: "darwin-arm64" },
  "x86_64-apple-darwin": { triple: "x86_64-apple-darwin", stage: "darwin-x64" },
  "darwin-arm64": { triple: "aarch64-apple-darwin", stage: "darwin-arm64" },
  "darwin-x64": { triple: "x86_64-apple-darwin", stage: "darwin-x64" },
};
const selected = requested === undefined ? undefined : triples[requested];
if (requestedIndex >= 0 && selected === undefined) throw new Error(`Unsupported Rust host target: ${requested ?? "(missing)"}`);
const target = selected?.stage ?? `${platform}-${architecture}`;

const profile = debug ? "debug" : "release";
const cargo = ["cargo", "rustc", "-p", "quickgui-host", "--lib", "--crate-type", "staticlib", ...(selected === undefined ? [] : ["--target", selected.triple]), ...(debug ? [] : ["--release"]), "--", "--print", "native-static-libs"];
const command = platform === "darwin" ? [join(repoRoot, "scripts", "with-macos-ghostty-zig.sh"), ...cargo] : cargo;
console.log(`[native] ${command.join(" ")}`);
const build = Bun.spawnSync(command, { cwd: repoRoot, stdin: "inherit", stdout: "inherit", stderr: "pipe", env: process.env });
const stderr = build.stderr.toString();
process.stderr.write(stderr);
if (build.exitCode !== 0) process.exit(build.exitCode);

const match = stderr.match(/native-static-libs:\s*(.*)/);
const flags = match === null ? [] : match[1]!.trim().split(/\s+/).filter(Boolean);
const frameworks: string[] = [];
const libraries: string[] = [];
for (let index = 0; index < flags.length; index += 1) {
  const flag = flags[index]!;
  if (flag === "-framework") {
    const name = flags[index + 1];
    if (name !== undefined && !frameworks.includes(name)) frameworks.push(name);
    index += 1;
  } else if (flag.startsWith("-l")) {
    const name = flag.slice(2);
    if (name !== "System" && name !== "c" && name !== "m" && !libraries.includes(name)) libraries.push(name);
  }
}

const targetDir = process.env.CARGO_TARGET_DIR ? resolve(process.env.CARGO_TARGET_DIR) : join(repoRoot, "target");
const archive = join(targetDir, ...(selected === undefined ? [] : [selected.triple]), profile, "libquickgui_host.a");
if (!existsSync(archive)) throw new Error(`Expected the host archive at ${archive}`);
const stage = join(packageRoot, "lib", target);
mkdirSync(stage, { recursive: true });
cpSync(archive, join(stage, "libquickgui_host.a"));

const sharedName = platform === "darwin" ? "libquickgui_host.dylib" : platform === "windows" ? "quickgui_host.dll" : "libquickgui_host.so";
const sharedCargo = [
  "cargo",
  "rustc",
  "-p",
  "quickgui-host",
  "--lib",
  "--crate-type",
  "cdylib",
  "--features",
  "dynamic-host",
  ...(selected === undefined ? [] : ["--target", selected.triple]),
  ...(debug ? [] : ["--release"]),
];
const sharedCommand = platform === "darwin" ? [join(repoRoot, "scripts", "with-macos-ghostty-zig.sh"), ...sharedCargo] : sharedCargo;
console.log(`[native] ${sharedCommand.join(" ")}`);
const sharedBuild = Bun.spawnSync(sharedCommand, { cwd: repoRoot, stdin: "inherit", stdout: "inherit", stderr: "inherit", env: process.env });
if (sharedBuild.exitCode !== 0) process.exit(sharedBuild.exitCode);
const sharedArchive = join(targetDir, ...(selected === undefined ? [] : [selected.triple]), profile, sharedName);
if (!existsSync(sharedArchive)) throw new Error(`Expected the host shared library at ${sharedArchive}`);
cpSync(sharedArchive, join(stage, sharedName));
console.log(`[native] Staged ${realpathSync(join(stage, sharedName))}`);

writeFileSync(
  join(stage, "link.json"),
  `${JSON.stringify(
    {
      target,
      entry: platform === "darwin" ? "_quickgui_main" : "quickgui_main",
      shared: sharedName,
      frameworks,
      libraries,
      searchPaths: platform === "darwin" ? ["/usr/lib/swift"] : [],
    },
    null,
    2,
  )}\n`,
);
console.log(`[native] Staged ${realpathSync(join(stage, "libquickgui_host.a"))}`);
console.log(`[native] Frameworks: ${frameworks.join(", ")}`);
console.log(`[native] Libraries: ${libraries.join(", ")}`);

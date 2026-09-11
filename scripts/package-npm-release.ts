#!/usr/bin/env bun
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dir, "..");
const output = resolve(root, process.argv[2] ?? "target/npm-release");
const version = JSON.parse(readFileSync(join(root, "package.json"), "utf8")).version as string;

function run(argv: string[], cwd = root): string {
  const child = Bun.spawnSync(argv, { cwd, stdin: "ignore", stdout: "pipe", stderr: "pipe" });
  if (child.exitCode !== 0) throw new Error(`${argv.join(" ")}: ${child.stderr.toString()}`);
  return child.stdout.toString();
}

run(["bun", "scripts/release-metadata.ts"]);
mkdirSync(output, { recursive: true });
for (const file of readdirSync(output)) {
  if (/^quickgui-.*\.tgz$/.test(file) || file === "NPM_SHA256SUMS") rmSync(join(output, file));
}

const packages = [
  { name: "native", library: "quickgui_host" },
  { name: "extension-terminal", library: "quickgui_terminal" },
  { name: "extension-updater", library: "quickgui_updater" },
  { name: "solid", library: undefined },
  { name: "cli", library: undefined },
];
const archives: Record<string, string> = {};
const checksums: string[] = [];
for (const pkg of packages) {
  const directory = join(root, "packages", pkg.name);
  const expected = pkg.library
    ? ["arm64", "x64"].map((arch) => `package/lib/darwin-${arch}/lib${pkg.library}.dylib`)
    : [];
  for (const entry of expected) {
    const binary = join(directory, entry.slice("package/".length));
    if (!existsSync(binary)) throw new Error(`Missing native binary: ${binary}`);
    if (process.platform === "darwin")
      run(["lipo", binary, "-verify_arch", entry.includes("arm64") ? "arm64" : "x86_64"]);
  }
  run(["bun", "pm", "pack", "--destination", output, "--quiet"], directory);
  const filename = `quickgui-${pkg.name}-${version}.tgz`;
  const archive = join(output, filename);
  const manifest = JSON.parse(run(["tar", "-xOf", archive, "package/package.json"]));
  if (
    manifest.name !== `@quickgui/${pkg.name}` ||
    manifest.version !== version ||
    JSON.stringify(manifest.os) !== '["darwin"]' ||
    manifest.publishConfig?.access !== "public"
  ) {
    throw new Error(`Incorrect release metadata in ${filename}`);
  }
  if (
    pkg.name === "cli" &&
    (manifest.dependencies?.["@quickgui/native"] !== version ||
      manifest.dependencies?.["@quickgui/extension-terminal"] ||
      manifest.dependencies?.["@quickgui/extension-updater"] ||
      manifest.bin?.quickgui !== "src/cli.ts")
  ) {
    throw new Error("CLI must depend only on the core native package; extensions are optional");
  }
  const entries = run(["tar", "-tzf", archive]).trim().split("\n");
  if (pkg.name === "extension-updater") {
    if (
      manifest.exports?.["."] !== "./src/index.ts" ||
      !entries.includes("package/src/index.ts") ||
      manifest.dependencies?.["@quickgui/native"] !== version
    )
      throw new Error(
        "Updater package must include its TypeScript API and matching native core dependency",
      );
    for (const arch of ["arm64", "x64"])
      if (!entries.includes("package/lib/darwin-" + arch + "/Sparkle.framework.qgr"))
        throw new Error("Missing Sparkle resources in updater package");
  }
  const binaries = entries.filter((entry) => /\.(dylib|dll|so)$/.test(entry));
  if (binaries.length !== expected.length || expected.some((entry) => !binaries.includes(entry))) {
    throw new Error(`Incorrect native library set in ${filename}: ${binaries.join(", ")}`);
  }
  if (pkg.name === "cli") {
    for (const required of [
      "src/extensions.ts",
      "src/typescript-build.ts",
      "src/typescript-compiler.ts",
      "src/rust-build.ts",
      "templates/typescript/app.tsx",
      "templates/rust/src/main.rs",
      "templates/rust/Cargo.toml",
      "src/extension-resources.ts",
      "src/init-extension.ts",
      "templates/extension/common/go.mod",
      "templates/extension/go/extension.go",
      "templates/extension/native/scripts/build.ts",
      "templates/extension/zig/native/quickgui_extension.h",
      "templates/extension/zig/native/extension.zig",
      "templates/extension/rust/native/src/lib.rs",
    ])
      if (!entries.includes(`package/${required}`))
        throw new Error(`CLI archive is missing ${required}`);
  }
  checksums.push(
    `${createHash("sha256").update(readFileSync(archive)).digest("hex")}  ${filename}`,
  );
  archives[pkg.name] = filename;
}
writeFileSync(join(output, "NPM_SHA256SUMS"), checksums.join("\n") + "\n");
console.log(
  `QUICKGUI_NPM_PACKAGE_RESULT ${JSON.stringify({ version, ...archives, checksums: "NPM_SHA256SUMS", passed: true })}`,
);

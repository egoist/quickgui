#!/usr/bin/env bun
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dir, "..");
const output = resolve(root, process.argv[2] ?? "target/npm-release");
const version = JSON.parse(readFileSync(join(root, "package.json"), "utf8")).version as string;
const releasedOs = ["darwin", "linux", "win32"] as const;
const releasedNativeTargets = [
  { stage: "darwin-arm64", platform: "darwin" },
  { stage: "darwin-x64", platform: "darwin" },
  { stage: "linux-arm64", platform: "linux" },
  { stage: "linux-x64", platform: "linux" },
  { stage: "windows-x64", platform: "windows" },
] as const;

function nativeLibraryName(
  library: string,
  platform: (typeof releasedNativeTargets)[number]["platform"],
) {
  if (platform === "darwin") return `lib${library}.dylib`;
  if (platform === "windows") return `${library}.dll`;
  return `lib${library}.so`;
}

function expectedLibraries(library: string) {
  return releasedNativeTargets.map(
    (target) => `package/lib/${target.stage}/${nativeLibraryName(library, target.platform)}`,
  );
}

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
  { name: "extension-editor", library: "quickgui_editor" },
  { name: "extension-markdown", library: "quickgui_markdown" },
  { name: "solid", library: undefined },
  { name: "cli", library: undefined },
];
const archives: Record<string, string> = {};
const checksums: string[] = [];
for (const pkg of packages) {
  const directory = pkg.name.startsWith("extension-")
    ? join(root, "extensions", pkg.name.slice("extension-".length))
    : join(root, "packages", pkg.name);
  const expected = pkg.library ? expectedLibraries(pkg.library) : [];
  for (const entry of expected) {
    const binary = join(directory, entry.slice("package/".length));
    if (!existsSync(binary)) throw new Error(`Missing native binary: ${binary}`);
    if (process.platform === "darwin" && entry.includes("/darwin-"))
      run(["lipo", binary, "-verify_arch", entry.includes("arm64") ? "arm64" : "x86_64"]);
  }
  run(["bun", "pm", "pack", "--destination", output, "--quiet"], directory);
  const filename = `quickgui-${pkg.name}-${version}.tgz`;
  const archive = join(output, filename);
  const manifest = JSON.parse(run(["tar", "-xOf", archive, "package/package.json"]));
  if (
    manifest.name !== `@quickgui/${pkg.name}` ||
    manifest.version !== version ||
    JSON.stringify(manifest.os) !== JSON.stringify(releasedOs) ||
    manifest.publishConfig?.access !== "public" ||
    manifest.repository?.url !== "https://github.com/egoist/quickgui.git"
  ) {
    throw new Error(`Incorrect release metadata in ${filename}`);
  }
  if (
    pkg.name === "cli" &&
    (manifest.dependencies?.["@quickgui/native"] !== version ||
      Object.keys(manifest.dependencies ?? {}).some((name) => name.startsWith("@quickgui/extension-")) ||
      manifest.bin?.quickgui !== "src/cli.ts")
  ) {
    throw new Error("CLI must depend only on the core native package; extensions are optional");
  }
  const entries = run(["tar", "-tzf", archive]).trim().split("\n");
  if (pkg.name === "extension-updater") {
    if (
      manifest.exports?.["."] !== "./js/index.ts" ||
      !entries.includes("package/js/index.ts") ||
      manifest.dependencies?.["@quickgui/native"] ||
      manifest.peerDependencies?.["@quickgui/native"] !== version ||
      manifest.devDependencies?.["@quickgui/native"] !== version
    )
      throw new Error(
        "Updater package must include its TypeScript API and peer-only native core relationship",
      );
    for (const target of releasedNativeTargets) {
      const resource =
        target.platform === "darwin"
          ? "Sparkle.framework.qgr"
          : target.platform === "windows"
            ? "quickgui-updater-helper.exe"
            : "quickgui-updater-helper";
      if (!entries.includes(`package/lib/${target.stage}/${resource}`))
        throw new Error(`Missing ${target.stage} updater resources in updater package`);
    }
  }
  if (["extension-editor", "extension-markdown", "extension-terminal"].includes(pkg.name)) {
    if (
      manifest.exports?.["."] !== "./js/index.ts" ||
      !entries.includes("package/js/index.ts") ||
      manifest.dependencies?.["@quickgui/native"] ||
      manifest.dependencies?.["@quickgui/solid"] ||
      manifest.dependencies?.["solid-js"] ||
      manifest.peerDependencies?.["@quickgui/native"] !== version ||
      manifest.peerDependencies?.["@quickgui/solid"] !== version ||
      manifest.devDependencies?.["@quickgui/native"] !== version ||
      manifest.devDependencies?.["@quickgui/solid"] !== version ||
      manifest.peerDependencies?.["solid-js"] !== "2.0.0-rc.8" ||
      manifest.devDependencies?.["solid-js"] !== "2.0.0-rc.8"
    )
      throw new Error(
        "Component extension must include its TypeScript API and peer-only QuickGUI relationships",
      );
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
      "templates/rust/Cargo.toml.tmpl",
      "src/extension-resources.ts",
      "src/init-extension.ts",
      "templates/extension/common/go.mod.tmpl",
      "templates/extension/go/extension.go.tmpl",
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

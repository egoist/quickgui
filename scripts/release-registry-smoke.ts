#!/usr/bin/env bun
import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const version = process.argv[2];
if (!version || !/^\d+\.\d+\.\d+(?:-[\w.-]+)?$/.test(version) || process.argv.length !== 3) {
  throw new Error("usage: bun scripts/release-registry-smoke.ts <version>");
}
const root = resolve(import.meta.dir, "..");
const scratch = mkdtempSync(join(tmpdir(), "quickgui-registry-smoke-"));
async function run(
  argv: string[],
  cwd: string,
  extra: Record<string, string> = {},
): Promise<string> {
  const child = Bun.spawn(argv, {
    cwd,
    stdin: "ignore",
    stdout: "pipe",
    stderr: "inherit",
    env: { ...process.env, CGO_ENABLED: "0", QUICKGUI_CACHE_DIR: join(scratch, "cache"), ...extra },
  });
  const [status, stdout] = await Promise.all([child.exited, new Response(child.stdout).text()]);
  if (status !== 0) throw new Error(`${argv.join(" ")} exited ${status}`);
  return stdout;
}
try {
  const rust = join(scratch, "rust-consumer");
  await run(
    ["cargo", "+1.90.0", "init", "--bin", "--name", "quickgui-release-smoke", rust],
    scratch,
  );
  await run(["cargo", "+1.90.0", "add", `quickgui@=${version}`], rust);
  await run(["cargo", "+1.90.0", "check"], rust, {
    CARGO_TARGET_DIR: join(root, "target/release-registry-smoke"),
  });

  const npm = join(scratch, "npm-consumer");
  mkdirSync(npm);
  writeFileSync(join(npm, "package.json"), '{"name":"quickgui-registry-smoke","private":true}');
  await run(["bun", "add", `@quickgui/native@${version}`, `@quickgui/cli@${version}`], npm);
  const cli = join(npm, "node_modules/@quickgui/cli/src/cli.ts");
  await run(
    ["bun", cli, "init", "compiled-consumer", "--language", "go", "--no-install"],
    npm,
  );
  const app = join(npm, "compiled-consumer");
  await run(["bun", "install"], app);
  await run(["go", "mod", "tidy"], app);
  await run(["bun", cli, "dev", "--project", app, "--once", "--no-launch"], npm);
  const target = process.arch === "arm64" ? "darwin-arm64" : "darwin-x64";
  const bundled = (resource: string) =>
    [
      ...new Bun.Glob(`.quickgui/dev/${target}/*.app/Contents/Frameworks/${resource}`).scanSync({
        cwd: app,
      }),
    ].length;
  const componentPackages = ["editor", "markdown", "terminal", "updater"];
  if (
    [...componentPackages.map((name) => `libquickgui_${name}.dylib`), "Sparkle.framework/Sparkle"].some(
      (resource) => bundled(resource) !== 0,
    )
  )
    throw new Error("Core-only registry app bundled an optional extension");
  const selected = new Set<string>();
  for (const name of componentPackages) {
    const module = `github.com/egoist/quickgui/extensions/${name}`;
    writeFileSync(join(app, `${name}.go`), `package main\nimport _ "${module}"\n`);
    await run(["go", "get", `${module}@v${version}`], app);
    await run(["go", "mod", "tidy"], app);
    await run(["bun", cli, "dev", "--project", app, "--once", "--no-launch"], npm);
    selected.add(name);
    for (const candidate of componentPackages) {
      if (bundled(`libquickgui_${candidate}.dylib`) !== Number(selected.has(candidate)))
        throw new Error(`${name} import produced an incorrect ${candidate} library set`);
    }
  }
  if (
    [
      "libquickgui_terminal.dylib",
      "libquickgui_updater.dylib",
      "libquickgui_editor.dylib",
      "libquickgui_markdown.dylib",
      "Sparkle.framework/Versions/B/Sparkle",
    ].some((resource) => bundled(resource) !== 1)
  )
    throw new Error(
      "Combined imports did not resolve the extension libraries and Sparkle resource bundle",
    );
  if (!existsSync(join(scratch, "cache/extensions")))
    throw new Error("Registry extension download was not cached");
  const help = await run(["bun", cli, "--help"], npm);
  if (!help.includes(`QuickGUI CLI ${version}`)) throw new Error("Installed CLI release differs");
  console.log(`QUICKGUI_REGISTRY_SMOKE ${JSON.stringify({ version, passed: true })}`);
} finally {
  rmSync(scratch, { recursive: true, force: true });
}

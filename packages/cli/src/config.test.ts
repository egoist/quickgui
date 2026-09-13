import { afterEach, expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { loadConfig, parseLanguage, resolveConfig } from "./config.ts";
import { parseCliArgs } from "./args.ts";
import { shouldIgnoreChange } from "./dev.ts";
import { errorMessage } from "./error.ts";

const roots: string[] = [];
const minimal = 'name = "TOML App"\nidentifier = "com.example.toml"\n';

test("Go, Rust, and TypeScript can be selected as application languages", () => {
  expect(parseLanguage("go")).toBe("go");
  expect(parseLanguage("rust")).toBe("rust");
  expect(parseLanguage("typescript")).toBe("typescript");
  for (const language of ["zig", "moonbit"]) {
    expect(() => parseLanguage(language)).toThrow("expected go, rust, or typescript");
    expect(() => parseCliArgs(["init", "app", "--language", language])).toThrow(
      "expected go, rust, or typescript",
    );
  }
  expect(
    resolveConfig({ name: "Demo", identifier: "com.example.demo", frontend: "typescript" }, "/p")
      .language,
  ).toBe("typescript");
  expect(() =>
    resolveConfig(
      {
        name: "Demo",
        identifier: "com.example.demo",
        language: "rust",
        frontend: "go",
      },
      "/p",
    ),
  ).toThrow("must be the same value");
});

function project(): string {
  const root = mkdtempSync(join(tmpdir(), "quickgui-toml-test-"));
  roots.push(root);
  return root;
}

afterEach(() => {
  for (const root of roots.splice(0)) rmSync(root, { recursive: true, force: true });
});

test("TOML shares config validation and path resolution with TypeScript", async () => {
  const root = project();
  const input = {
    name: "Example App", identifier: "com.example.app", version: "1.2.3", buildVersion: "7",
    entry: "cmd/app", outDir: "artifacts", target: "darwin-arm64",
    resources: ["assets"], fonts: ["assets/Custom.ttf"], protocols: ["EXAMPLE", "example"],
    native: { tags: ["release", "custom"], libraryPath: "lib/host.dylib" },
    macos: { minimumSystemVersion: "14.0", notarization: { keychainProfile: "release", keychain: "ci.keychain-db" } },
    windows: { hideConsole: false, nsis: { createDesktopShortcut: false, perMachine: true } },
    linux: { maintainer: "Example <hello@example.com>", categories: ["Development"], deb: false },
    updates: { manifest: true, baseUrl: "https://example.com/releases/", notesFile: "notes.md" },
    documentTypes: [{ name: "Project", extensions: [".QG"], mimeTypes: ["application/x-quickgui"], role: "Viewer", utTypeIdentifier: "com.example.project", exported: false }],
  };
  writeFileSync(join(root, "quickgui.config.ts"), `export default ${JSON.stringify(input)}`);
  writeFileSync(join(root, "quickgui.toml"), `
# Application metadata
name = "Example App"
identifier = "com.example.app"
version = "1.2.3"
buildVersion = "7"
entry = "cmd/app"
outDir = "artifacts"
target = "darwin-arm64"
resources = ["assets"]
fonts = ["assets/Custom.ttf"]
protocols = ["EXAMPLE", "example"]

[native]
tags = ["release", "custom"]
libraryPath = "lib/host.dylib"
[macos]
minimumSystemVersion = "14.0"
[macos.notarization]
keychainProfile = "release"
keychain = "ci.keychain-db"
[windows]
hideConsole = false
[windows.nsis]
createDesktopShortcut = false
perMachine = true
[linux]
maintainer = "Example <hello@example.com>"
categories = ["Development"]
deb = false
[updates]
manifest = true
baseUrl = "https://example.com/releases/"
notesFile = "notes.md"
[[documentTypes]]
name = "Project"
extensions = [".QG"]
mimeTypes = ["application/x-quickgui"]
role = "Viewer"
utTypeIdentifier = "com.example.project"
exported = false
`);
  const { configPath: tomlPath, ...toml } = await loadConfig(root);
  const { configPath: tsPath, ...typescript } = await loadConfig(root, "quickgui.config.ts");
  expect(toml).toEqual(typescript);
  expect(tomlPath).toBe(join(root, "quickgui.toml"));
  expect(tsPath).toBe(join(root, "quickgui.config.ts"));
  expect(toml.native.libraryPath).toBe(join(root, "lib/host.dylib"));
  expect(toml.windows.hideConsole).toBe(false);
  expect(toml.documentTypes[0]?.extensions).toEqual(["qg"]);
});

test("discovery retains TypeScript support and gives TOML precedence", async () => {
  const root = project();
  writeFileSync(join(root, "quickgui.config.ts"), 'export default { name: "TS App", identifier: "com.example.ts" }');
  expect((await loadConfig(root)).name).toBe("TS App");
  writeFileSync(join(root, "quickgui.toml"), minimal);
  expect((await loadConfig(root)).name).toBe("TOML App");
  expect((await loadConfig(root, "quickgui.config.ts")).name).toBe("TS App");
});

test("TOML and TypeScript resolve top-level extension directories from the project", async () => {
  const root = project();
  const extensions = ["vendor/my-service", join(root, "vendor/another-service")];
  writeFileSync(
    join(root, "quickgui.toml"),
    minimal + `language = "typescript"\nextensions = ${JSON.stringify(extensions)}\n`,
  );
  writeFileSync(
    join(root, "quickgui.config.ts"),
    `export default ${JSON.stringify({
      name: "Zig App",
      identifier: "com.example.typescript",
      language: "typescript",
      extensions,
    })}`,
  );
  for (const path of ["quickgui.toml", "quickgui.config.ts"]) {
    const config = await loadConfig(root, path);
    expect(config.extensions).toEqual([
      join(root, "vendor/my-service"),
      join(root, "vendor/another-service"),
    ]);
  }
});

test("extensions validate at the top level and Go keeps import-based discovery", () => {
  const input = { name: "Example", identifier: "com.example.app", language: "typescript" };
  expect(resolveConfig(input, "/project").extensions).toEqual([]);
  expect(() => resolveConfig({ ...input, extensions: "updater" }, "/project")).toThrow("extensions");
  expect(() => resolveConfig({ ...input, native: { extensions: ["updater"] } }, "/project")).toThrow("top-level extensions");
  expect(() => resolveConfig({ ...input, language: "go", extensions: ["updater"] }, "/project")).toThrow("Go extensions are discovered from imports");
});

test("explicit relative and absolute TOML paths resolve resources from the project", async () => {
  const root = project();
  mkdirSync(join(root, "config"));
  const path = join(root, "config/release.toml");
  writeFileSync(path, minimal + 'resources = ["assets"]');
  for (const requested of ["config/release.toml", path]) {
    const config = await loadConfig(root, requested);
    expect(config.configPath).toBe(path);
    expect(config.resources).toEqual([join(root, "assets")]);
  }
});

test("the project resources directory and icon.png are picked up automatically", () => {
  const root = project();
  mkdirSync(join(root, "resources"));
  writeFileSync(join(root, "resources", "icon.png"), "png");
  writeFileSync(join(root, "resources", "hero.png"), "hero");
  writeFileSync(join(root, "notes.txt"), "extra");
  const config = resolveConfig(
    { name: "Demo", identifier: "com.example.demo", resources: ["notes.txt"] },
    root,
  );
  expect(config.resourceDir).toBe(join(root, "resources"));
  expect(config.icon).toBe(join(root, "resources", "icon.png"));
  expect(config.resources).toEqual([join(root, "notes.txt")]);
  expect(() =>
    resolveConfig(
      { name: "Demo", identifier: "com.example.demo", resources: ["resources"] },
      root,
    ),
  ).toThrow("included automatically");
});

test("TOML edits are read again on reload and are not ignored by the watcher", async () => {
  const root = project();
  const path = join(root, "quickgui.toml");
  writeFileSync(path, minimal);
  expect((await loadConfig(root)).name).toBe("TOML App");
  writeFileSync(path, minimal.replace("TOML App", "Changed App"));
  expect((await loadConfig(root)).name).toBe("Changed App");
  expect(shouldIgnoreChange(root, path, join(root, "dist"))).toBe(false);
});

test("missing explicit configs do not silently fall back to another file", async () => {
  const root = project();
  await expect(loadConfig(root)).rejects.toThrow("expected quickgui.toml or quickgui.config.ts");
  writeFileSync(join(root, "quickgui.toml"), minimal);
  await expect(loadConfig(root, "quickgui.config.ts")).rejects.toThrow("quickgui.config.ts");
});

test("malformed TOML reports its path and parser cause without falling back", async () => {
  const root = project();
  writeFileSync(join(root, "quickgui.config.ts"), 'throw new Error("must not evaluate fallback")');
  writeFileSync(join(root, "quickgui.toml"), 'name = "unfinished');
  const error = await loadConfig(root).catch((error: unknown) => error);
  expect(error).toBeInstanceOf(Error);
  expect(errorMessage(error)).toContain(`Could not load ${join(root, "quickgui.toml")}`);
  expect((error as Error).cause).toBeInstanceOf(Error);
  expect(errorMessage(error)).not.toContain("must not evaluate fallback");
});

test("TOML values must satisfy the same typed configuration rules", async () => {
  const root = project();
  writeFileSync(join(root, "quickgui.toml"), minimal + 'version = 1.2');
  await expect(loadConfig(root)).rejects.toThrow("`version` must be a non-empty string");
  writeFileSync(join(root, "quickgui.toml"), minimal + '[windows]\nhideConsole = "false"');
  await expect(loadConfig(root)).rejects.toThrow("`windows.hideConsole` must be a boolean");
});

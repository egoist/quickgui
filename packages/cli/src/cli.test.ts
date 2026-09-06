import { afterEach, describe, expect, test } from "bun:test";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";

import { parseCliArgs } from "./args.ts";
import {
  macDmgFilename,
  macInfoPlist,
  macNotarytoolArguments,
} from "./build.ts";
import { resolveConfig } from "./config.ts";
import { ActiveProcessMonitor, shouldIgnoreChange } from "./dev.ts";
import { initProject } from "./init.ts";
import { hostTarget, parseTarget } from "./targets.ts";

const temporaryRoots: string[] = [];

afterEach(() => {
  for (const root of temporaryRoots.splice(0)) {
    rmSync(root, { recursive: true, force: true });
  }
});

describe("CLI arguments", () => {
  test("parses init, dev, and build options", () => {
    expect(parseCliArgs(["init", "hello", "--no-install", "--name=Hello"])).toEqual({
      command: "init",
      directory: "hello",
      install: false,
      name: "Hello",
    });
    expect(parseCliArgs(["dev", "--target", hostTarget(), "--once", "--no-launch"])).toEqual({
      command: "dev",
      project: ".",
      configFile: "quickgui.config.ts",
      target: hostTarget(),
      once: true,
      launch: false,
    });
    expect(
      parseCliArgs([
        "build",
        "--project",
        "demo",
        "--out-dir=artifacts",
        "--sign",
        "Developer ID Application: Example",
        "--notarize",
        "quickgui-notary",
      ]),
    ).toEqual({
      command: "build",
      project: "demo",
      configFile: "quickgui.config.ts",
      outDir: "artifacts",
      signingIdentity: "Developer ID Application: Example",
      notarizationProfile: "quickgui-notary",
      updateManifest: false,
      macAppStore: false,
    });
  });

  test("parses the packaging and updater build flags", () => {
    expect(
      parseCliArgs([
        "build",
        "--update-manifest",
        "--update-base-url",
        "https://dl.example.com/demo",
        "--mas",
      ]),
    ).toEqual({
      command: "build",
      project: ".",
      configFile: "quickgui.config.ts",
      updateManifest: true,
      updateBaseUrl: "https://dl.example.com/demo",
      macAppStore: true,
    });
    // A base URL alone implies the manifest.
    expect(
      parseCliArgs(["build", "--update-base-url=https://dl.example.com/demo"]),
    ).toMatchObject({ updateManifest: true });
    expect(() => parseCliArgs(["build", "--update-base-url", "http://dl.example.com"])).toThrow(
      "HTTPS",
    );
    expect(() => parseCliArgs(["build", "--update-manifest=yes"])).toThrow(
      "does not take a value",
    );
  });

  test("parses keygen options and help topics", () => {
    expect(parseCliArgs(["keygen"])).toEqual({
      command: "keygen",
      outDir: ".",
      force: false,
      passwordless: true,
    });
    expect(parseCliArgs(["keygen", "--out-dir", "keys", "--force", "--password"])).toEqual({
      command: "keygen",
      outDir: "keys",
      force: true,
      passwordless: false,
    });
    expect(parseCliArgs(["help", "keygen"])).toEqual({ command: "help", topic: "keygen" });
    expect(parseCliArgs(["keygen", "--help"])).toEqual({ command: "help", topic: "keygen" });
    expect(() => parseCliArgs(["keygen", "extra"])).toThrow("positional");
  });

  test("rejects unknown and duplicate options", () => {
    expect(() => parseCliArgs(["dev", "--wat"])).toThrow("Unknown option");
    expect(() => parseCliArgs(["build", "--target", hostTarget(), "--target", hostTarget()])).toThrow(
      "only be specified once",
    );
    expect(() => parseTarget("plan9-x64")).toThrow("Unsupported target");
  });
});

test("dev process monitoring ignores replaced and cleanup exits", async () => {
  const first = Promise.withResolvers<number>();
  const second = Promise.withResolvers<number>();
  const cleanup = Promise.withResolvers<number>();
  const statuses: number[] = [];
  const monitor = new ActiveProcessMonitor<{ exited: Promise<number> }>((status) => {
    statuses.push(status);
  });

  monitor.activate({ exited: first.promise });
  monitor.activate({ exited: second.promise });
  first.resolve(0);
  await first.promise;
  expect(statuses).toEqual([]);

  second.resolve(7);
  await second.promise;
  expect(statuses).toEqual([7]);

  monitor.close();
  monitor.activate({ exited: cleanup.promise });
  cleanup.resolve(0);
  await cleanup.promise;
  expect(statuses).toEqual([7]);
});

test("dev watcher ignores Bun compile transients without ignoring source", () => {
  const root = join(tmpdir(), "quickgui-dev-watch");
  const outDir = join(root, "dist");

  expect(
    shouldIgnoreChange(root, join(root, ".e5a5df0030510628-00000001.bun-build"), outDir),
  ).toBe(true);
  expect(shouldIgnoreChange(root, join(root, "app.tsx"), outDir)).toBe(false);
});

describe("project configuration", () => {
  test("normalizes paths and creates a filesystem-safe executable name", () => {
    const root = temporaryRoot();
    const config = resolveConfig(
      {
        name: "My / Great App",
        identifier: "com.example.great-app",
        entry: "ui/main.tsx",
        resources: ["assets"],
        fonts: ["assets/JetBrainsMonoNerdFontMono-Regular.ttf"],
        protocols: ["QuickGUI", "quickgui+preview", "quickgui"],
        macos: {
          dmgTitle: "Great App",
          notarization: {
            keychainProfile: "quickgui-notary",
            keychain: "ci.keychain-db",
          },
        },
      },
      root,
    );
    expect(config.executableName).toBe("My-Great-App");
    expect(config.entry).toBe(join(root, "ui/main.tsx"));
    expect(config.resources).toEqual([join(root, "assets")]);
    expect(config.fonts).toEqual([
      join(root, "assets/JetBrainsMonoNerdFontMono-Regular.ttf"),
    ]);
    expect(config.protocols).toEqual(["quickgui", "quickgui+preview"]);
    expect(config.macos.minimumSystemVersion).toBe("14.0");
    expect(config.macos.dmgTitle).toBe("Great App");
    expect(config.macos.notarization).toEqual({
      keychainProfile: "quickgui-notary",
      keychain: join(root, "ci.keychain-db"),
    });
  });

  test("Go projects default the entry to the project root", () => {
    const root = temporaryRoot();
    const config = resolveConfig(
      { name: "Go App", identifier: "com.example.go-app", language: "go" },
      root,
    );
    expect(config.language).toBe("go");
    expect(config.entry).toBe(root);
  });

  test("Go projects accept a .go file entry and reject a TypeScript path", () => {
    const root = temporaryRoot();
    const config = resolveConfig(
      {
        name: "Go App",
        identifier: "com.example.go-app",
        language: "go",
        entry: "main.go",
      },
      root,
    );
    expect(config.entry).toBe(join(root, "main.go"));
    expect(() =>
      resolveConfig(
        {
          name: "Go App",
          identifier: "com.example.go-app",
          language: "go",
          entry: "src/app.tsx",
        },
        root,
      ),
    ).toThrow("directory or a .go file");
    expect(() =>
      resolveConfig(
        { name: "Bad", identifier: "com.example.bad", language: "rust" },
        root,
      ),
    ).toThrow('`language` must be "typescript" or "go"');
  });

  test("rejects an invalid bundle identifier", () => {
    expect(() =>
      resolveConfig({ name: "Bad", identifier: "not a reverse dns identifier" }, temporaryRoot()),
    ).toThrow("Invalid application identifier");
  });

  test("rejects an invalid URL scheme", () => {
    expect(() =>
      resolveConfig(
        { name: "Bad", identifier: "com.example.bad", protocols: ["1bad"] },
        temporaryRoot(),
      ),
    ).toThrow("Invalid URL scheme");
  });

  test("validates macOS DMG and notarization configuration", () => {
    expect(() =>
      resolveConfig(
        {
          name: "Bad",
          identifier: "com.example.bad",
          macos: { dmgTitle: "This disk image title is much too long" },
        },
        temporaryRoot(),
      ),
    ).toThrow("macos.dmgTitle");
    expect(() =>
      resolveConfig(
        {
          name: "Bad",
          identifier: "com.example.bad",
          macos: { notarization: {} },
        },
        temporaryRoot(),
      ),
    ).toThrow("macos.notarization.keychainProfile");
  });
});

test("macOS packaging derives its DMG name and notarytool command", () => {
  expect(macDmgFilename("My App", "1.2.3")).toBe("My App 1.2.3.dmg");
  expect(() => macDmgFilename("Bad/App", "1.2.3")).toThrow("path separators");
  expect(
    macNotarytoolArguments("/tmp/My App 1.2.3.dmg", {
      keychainProfile: "quickgui-notary",
      keychain: "/tmp/ci.keychain-db",
    }),
  ).toEqual([
    "xcrun",
    "notarytool",
    "submit",
    "/tmp/My App 1.2.3.dmg",
    "--keychain-profile",
    "quickgui-notary",
    "--keychain",
    "/tmp/ci.keychain-db",
    "--wait",
  ]);
});

test("macOS metadata is escaped and complete", () => {
  const plist = macInfoPlist({
    name: "A & B",
    displayName: "A < B",
    executableName: "A-B",
    identifier: "com.example.a-b",
    version: "1.2.3",
    buildVersion: "7",
    minimumSystemVersion: "13.0",
    category: "public.app-category.developer-tools",
    urlSchemes: ["a-and-b", "a+b"],
    iconFile: "AppIcon.icns",
  });
  expect(plist).toContain("<string>A &amp; B</string>");
  expect(plist).toContain("<string>A &lt; B</string>");
  expect(plist).toContain("<key>CFBundleExecutable</key>");
  expect(plist).toContain("<string>AppIcon.icns</string>");
  expect(plist).toContain("<key>CFBundleURLTypes</key>");
  expect(plist).toContain("<string>a+b</string>");
});

test("project initialization renders a complete native scaffold", async () => {
  const root = temporaryRoot();
  const project = join(root, "sample-app");
  await initProject({
    directory: project,
    install: false,
    name: "Sample App",
    identifier: "com.example.sample-app",
  });

  expect(JSON.parse(readFileSync(join(project, "package.json"), "utf8"))).toMatchObject({
    name: "sample-app",
    scripts: { dev: "quickgui dev", build: "quickgui build" },
    dependencies: {
      "@quickgui/native": "^0.1.3",
      "@quickgui/ui": "^0.1.3",
    },
  });
  expect(readFileSync(join(project, "quickgui.config.ts"), "utf8")).toContain(
    'identifier: "com.example.sample-app"',
  );
  const applicationSource = readFileSync(join(project, "src/app.tsx"), "utf8");
  expect(applicationSource).toContain('from "@quickgui/native"');
  expect(applicationSource).toContain('from "@quickgui/ui"');
  expect(applicationSource).toContain("await app.whenReady()");
  expect(applicationSource).toContain("app.onReopen(");
  expect(applicationSource).toContain("if (!event.hasVisibleWindows) openMainWindow()");
  expect(applicationSource).toContain("renderer: createRenderer(");
  expect(applicationSource).not.toContain("quitMode");
  expect(applicationSource).not.toContain("app.run()");
  expect(readFileSync(join(project, ".gitignore"), "utf8")).toContain(".quickgui");
  expect(readFileSync(join(project, "README.md"), "utf8")).not.toContain("{{");
});

test("project initialization never overwrites a non-empty destination", async () => {
  const root = temporaryRoot();
  writeFileSync(join(root, "keep.txt"), "mine");
  await expect(initProject({ directory: root, install: false })).rejects.toThrow("is not empty");
  expect(readFileSync(join(root, "keep.txt"), "utf8")).toBe("mine");
});

function temporaryRoot(): string {
  const root = mkdtempSync(join(tmpdir(), "quickgui-cli-test-"));
  temporaryRoots.push(root);
  return root;
}

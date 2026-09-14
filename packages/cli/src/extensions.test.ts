import { afterEach, expect, test } from "bun:test";
import { createHash } from "node:crypto";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { resolveConfig } from "./config.ts";
import {
  discoverExtensions,
  extensionLibraryName,
  extensionManifests,
  parseGoPackages,
  readBounded,
  resolveExtension,
  verifyArchive,
  type ExtensionManifest,
} from "./extensions.ts";

const directories: string[] = [];
const originalDirectory = process.env.QUICKGUI_EXTENSION_DIR;
afterEach(() => {
  for (const directory of directories.splice(0))
    rmSync(directory, { recursive: true, force: true });
  if (originalDirectory === undefined) delete process.env.QUICKGUI_EXTENSION_DIR;
  else process.env.QUICKGUI_EXTENSION_DIR = originalDirectory;
});
function temporary(): string {
  const path = mkdtempSync(join(tmpdir(), "quickgui-extension-test-"));
  directories.push(path);
  return path;
}
const terminal: ExtensionManifest = {
  schema: 1,
  name: "terminal",
  abi: 1,
  package: "@quickgui/extension-terminal",
  version: "0.1.3",
  library: "quickgui_terminal",
};
function manifest(directory: string, value: unknown = terminal): void {
  mkdirSync(directory, { recursive: true });
  writeFileSync(join(directory, "quickgui.extension.json"), JSON.stringify(value));
}

test("Go package stream supports braces and quotes inside nested strings", () => {
  const packages = [{ Dir: '/path/"{nested}', Module: { Path: "example" } }, { Dir: "/other" }];
  expect(parseGoPackages(packages.map((value) => JSON.stringify(value)).join("\n"))).toEqual(
    packages,
  );
  expect(() => parseGoPackages('{"Dir":"unfinished')).toThrow();
  expect(() => parseGoPackages("{}garbage")).toThrow();
});

test("only imported packages opt in, and equivalent transitive requirements deduplicate", () => {
  const root = temporary();
  const normal = join(root, "ui");
  const first = join(root, "terminal");
  const second = join(root, "transitive");
  mkdirSync(normal);
  manifest(first);
  manifest(second, { ...terminal, description: "Extra metadata does not change the dependency" });
  expect(extensionManifests([{ Dir: normal }])).toEqual([]);
  expect(extensionManifests([{ Dir: normal }, { Dir: first }, { Dir: second }])).toEqual([
    terminal,
  ]);
  manifest(second, { ...terminal, version: "0.2.0" });
  expect(() => extensionManifests([{ Dir: first }, { Dir: second }])).toThrow("Conflicting");
  manifest(second, { ...terminal, library: "../../arbitrary" });
  expect(() => extensionManifests([{ Dir: second }])).toThrow("Invalid");
});

test("Go dependency discovery respects build tags, target files, and transitive imports", async () => {
  if (!Bun.which("go"))
    throw new Error("Go must be on PATH for extension import integration tests");
  const root = temporary();
  writeFileSync(join(root, "go.mod"), "module example.test/extensions\n\ngo 1.23\n");
  writeFileSync(join(root, "main.go"), "package main\nfunc main() {}\n");
  writeFileSync(
    join(root, "terminal.go"),
    '//go:build terminal\n\npackage main\nimport _ "example.test/extensions/widget"\n',
  );
  const widget = join(root, "widget");
  mkdirSync(widget);
  writeFileSync(
    join(widget, "widget_linux.go"),
    'package widget\nimport _ "example.test/extensions/terminal"\n',
  );
  writeFileSync(join(widget, "widget_darwin.go"), "package widget\n");
  const extension = join(root, "terminal");
  manifest(extension);
  writeFileSync(join(extension, "terminal.go"), "package terminal\n");
  const config = resolveConfig({ name: "Example", identifier: "test.extensions" }, root);
  const env = { ...process.env, CGO_ENABLED: "0", GOOS: "linux", GOARCH: "arm64", GOWORK: "off" };
  expect(await discoverExtensions(config, ".", env)).toEqual([]);
  config.native.tags = ["terminal"];
  expect(await discoverExtensions(config, ".", env)).toEqual([terminal]);
  expect(await discoverExtensions(config, ".", { ...env, GOOS: "darwin" })).toEqual([]);
}, 30_000);

test("third-party extensions own their package names and release versions", () => {
  const root = temporary();
  const extension = {
    ...terminal,
    name: "acme-echo",
    library: "quickgui_acme_echo",
    version: "7.2.1-beta.2+native.3",
  };
  for (const packageName of ["@acme/extension-echo", "quickgui-extension-echo"]) {
    manifest(root, { ...extension, package: packageName });
    expect(extensionManifests([{ Dir: root }])).toEqual([{ ...extension, package: packageName }]);
  }
  const other = join(root, "other-publisher");
  manifest(other, { ...extension, package: "@other/extension-echo" });
  expect(() => extensionManifests([{ Dir: root }, { Dir: other }])).toThrow("Conflicting");
  for (const packageName of [
    "../escape",
    "@scope/../escape",
    "@scope/echo/extra",
    "https://example.com/echo",
    "UPPERCASE",
    "@scope/.hidden",
    "x".repeat(215),
  ]) {
    manifest(root, { ...extension, package: packageName });
    expect(() => extensionManifests([{ Dir: root }])).toThrow("Invalid");
  }
  manifest(root, { ...terminal, name: "host", library: "quickgui_host" });
  expect(() => extensionManifests([{ Dir: root }])).toThrow("Invalid");
  manifest(root, { ...terminal, version: "1.0.0+" + "x".repeat(64) });
  expect(() => extensionManifests([{ Dir: root }])).toThrow("Invalid");
});

test("installed third-party packages resolve without core checkout integration", async () => {
  const root = temporary();
  delete process.env.QUICKGUI_EXTENSION_DIR;
  const extension = {
    ...terminal,
    name: "acme-echo",
    library: "quickgui_acme_echo",
    package: "@acme/extension-echo",
    version: "7.2.1",
  };
  const directory = join(root, "node_modules/@acme/extension-echo");
  const stage = join(directory, "lib/darwin-arm64");
  mkdirSync(stage, { recursive: true });
  const metadata = {
    name: extension.package,
    version: extension.version,
    exports: { "./package.json": "./package.json" },
  };
  writeFileSync(join(directory, "package.json"), JSON.stringify(metadata));
  const library = join(stage, extensionLibraryName(extension, "darwin-arm64"));
  writeFileSync(library, "third-party artifact");
  expect(realpathSync(await resolveExtension(extension, "darwin-arm64", root))).toBe(
    realpathSync(library),
  );
  writeFileSync(join(directory, "package.json"), JSON.stringify({ ...metadata, version: "7.2.0" }));
  await expect(resolveExtension(extension, "darwin-arm64", root)).rejects.toThrow(
    "requires @acme/extension-echo@7.2.1",
  );
});

test("a source checkout takes precedence over a matching cached package artifact", async () => {
  const root = temporary();
  delete process.env.QUICKGUI_EXTENSION_DIR;
  const requested = {
    ...terminal,
    version: JSON.parse(
      readFileSync(new URL("../../../extensions/terminal/package.json", import.meta.url), "utf8"),
    ).version as string,
  };
  const stale = join(root, "node_modules/@quickgui/extension-terminal");
  const staleStage = join(stale, "lib/darwin-arm64");
  mkdirSync(staleStage, { recursive: true });
  writeFileSync(
    join(stale, "package.json"),
    JSON.stringify({
      name: requested.package,
      version: requested.version,
      exports: { "./package.json": "./package.json" },
    }),
  );
  writeFileSync(join(staleStage, extensionLibraryName(requested, "darwin-arm64")), "stale");
  const checkout = join(import.meta.dir, "..", "..", "..", "extensions", "terminal");
  const checkoutStage = join(checkout, "lib/darwin-arm64");
  const checkoutLibrary = join(checkoutStage, extensionLibraryName(requested, "darwin-arm64"));
  const previous = existsSync(checkoutLibrary) ? readFileSync(checkoutLibrary) : undefined;
  mkdirSync(checkoutStage, { recursive: true });
  writeFileSync(checkoutLibrary, "checkout");
  try {
    expect(readFileSync(await resolveExtension(requested, "darwin-arm64", root), "utf8")).toBe(
      "checkout",
    );
  } finally {
    if (previous === undefined) rmSync(checkoutLibrary, { force: true });
    else writeFileSync(checkoutLibrary, previous);
  }
});

test("a third-party namespace never falls back to a built-in extension with the same name", async () => {
  const { spyOn } = await import("bun:test");
  const root = temporary();
  const extension = { ...terminal, package: "@acme/extension-alternate-terminal", version: "6.4.2" };
  delete process.env.QUICKGUI_EXTENSION_DIR;
  const originalCache = process.env.QUICKGUI_CACHE_DIR;
  process.env.QUICKGUI_CACHE_DIR = join(root, "cache");
  const urls: string[] = [];
  const fetchMock = spyOn(globalThis, "fetch").mockImplementation(
    Object.assign(
      async (input: string | Request | URL) => {
        urls.push(String(input));
        return new Response("not published", { status: 404 });
      },
      { preconnect: globalThis.fetch.preconnect },
    ),
  );
  try {
    await expect(resolveExtension(extension, "darwin-arm64", root)).rejects.toThrow("404");
    expect(urls).toEqual(["https://registry.npmjs.org/%40acme%2Fextension-alternate-terminal/6.4.2"]);
  } finally {
    fetchMock.mockRestore();
    if (originalCache === undefined) delete process.env.QUICKGUI_CACHE_DIR;
    else process.env.QUICKGUI_CACHE_DIR = originalCache;
  }
});

test("extension names and explicit offline directories resolve per target", async () => {
  const root = temporary();
  const filename = extensionLibraryName(terminal, "darwin-arm64");
  expect(filename).toBe("libquickgui_terminal.dylib");
  expect(extensionLibraryName(terminal, "windows-x64")).toBe("quickgui_terminal.dll");
  expect(extensionLibraryName(terminal, "linux-x64")).toBe("libquickgui_terminal.so");
  writeFileSync(join(root, filename), "test-native-library");
  process.env.QUICKGUI_EXTENSION_DIR = root;
  expect(readFileSync(await resolveExtension(terminal, "darwin-arm64", root), "utf8")).toBe(
    "test-native-library",
  );
  await expect(resolveExtension(terminal, "windows-x64", root)).rejects.toThrow("missing");
});

test("archive bytes must match their SHA512 integrity", () => {
  const archive = new TextEncoder().encode("native archive");
  const integrity = `sha512-${createHash("sha512").update(archive).digest("base64")}`;
  expect(() => verifyArchive(archive, integrity)).not.toThrow();
  expect(() => verifyArchive(new TextEncoder().encode("corrupt"), integrity)).toThrow("integrity");
  expect(() => verifyArchive(archive, "sha1-not-accepted")).toThrow("integrity");
});

test("stream limits apply while reading decompressed data, cancelling oversized streams", async () => {
  let cancelled = false;
  const stream = new ReadableStream<Uint8Array>({
    pull(controller) {
      controller.enqueue(new Uint8Array(8));
    },
    cancel() {
      cancelled = true;
    },
  });
  await expect(readBounded(stream, 12)).rejects.toThrow("size limit");
  expect(cancelled).toBe(true);
  expect(await readBounded(new Response("abc").body!, 3)).toEqual(Buffer.from("abc"));
});

test("exact-version downloads verify integrity and repair a corrupt cache", async () => {
  const { spyOn } = await import("bun:test");
  const root = temporary();
  const extension = {
    ...terminal,
    name: "cache-test",
    package: "@quickgui/extension-cache-test",
    library: "quickgui_cache_test",
    resources: { darwin: ["Sparkle.framework.qgr"] },
  };
  const filename = extensionLibraryName(extension, "darwin-arm64");
  const payload = join(root, "package/lib/darwin-arm64");
  mkdirSync(payload, { recursive: true });
  writeFileSync(join(payload, filename), "verified extension payload");
  writeFileSync(join(payload, "Sparkle.framework.qgr"), "verified extension resource");
  const archive = join(root, "extension.tgz");
  const tar = Bun.spawnSync(["tar", "-czf", archive, "package"], { cwd: root, stderr: "pipe" });
  expect(tar.exitCode).toBe(0);
  const bytes = readFileSync(archive);
  const url =
    "https://registry.npmjs.org/@quickgui/extension-cache-test/-/extension-cache-test-0.1.3.tgz";
  let calls = 0;
  let publisher = extension.package;
  const fetchMock = spyOn(globalThis, "fetch").mockImplementation(
    Object.assign(
      async (input: string | Request | URL) => {
        calls++;
        return String(input) === url
          ? new Response(bytes)
          : Response.json({
              name: publisher,
              version: extension.version,
              dist: {
                tarball: url,
                integrity: `sha512-${createHash("sha512").update(bytes).digest("base64")}`,
              },
            });
      },
      { preconnect: globalThis.fetch.preconnect },
    ),
  );
  const originalCache = process.env.QUICKGUI_CACHE_DIR;
  delete process.env.QUICKGUI_EXTENSION_DIR;
  process.env.QUICKGUI_CACHE_DIR = join(root, "cache");
  try {
    const path = await resolveExtension(extension, "darwin-arm64", root);
    expect(readFileSync(path, "utf8")).toBe("verified extension payload");
    expect(calls).toBe(2);
    expect(await resolveExtension(extension, "darwin-arm64", root)).toBe(path);
    expect(calls).toBe(2);
    writeFileSync(path, "corrupted");
    await resolveExtension(extension, "darwin-arm64", root);
    expect(readFileSync(path, "utf8")).toBe("verified extension payload");
    expect(calls).toBe(4);
    const resource = await resolveExtension(
      extension,
      "darwin-arm64",
      root,
      "Sparkle.framework.qgr",
    );
    expect(readFileSync(resource, "utf8")).toBe("verified extension resource");
    expect(calls).toBe(6);
    expect(await resolveExtension(extension, "darwin-arm64", root, "Sparkle.framework.qgr")).toBe(
      resource,
    );
    expect(calls).toBe(6);
    await expect(
      resolveExtension(extension, "darwin-arm64", root, "undeclared.qgr"),
    ).rejects.toThrow();
    publisher = "@another/extension-cache-test";
    const independent = { ...extension, package: publisher };
    const otherPath = await resolveExtension(independent, "darwin-arm64", root);
    expect(otherPath).not.toBe(path);
    expect(calls).toBe(8);
    expect(await resolveExtension(extension, "darwin-arm64", root)).toBe(path);
    expect(calls).toBe(8);
  } finally {
    fetchMock.mockRestore();
    if (originalCache === undefined) delete process.env.QUICKGUI_CACHE_DIR;
    else process.env.QUICKGUI_CACHE_DIR = originalCache;
  }
});

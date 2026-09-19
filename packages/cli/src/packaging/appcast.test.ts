import { expect, test } from "bun:test";
import { createPublicKey, sign, verify } from "node:crypto";
import { mkdtempSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  renderAppcast,
  sparklePrivateKey,
  sparklePublicKey,
  generateUpdaterKeys,
  updaterMetadata,
  writeAppcast,
} from "./appcast.ts";
import { resolveConfig } from "../config.ts";
import { macInfoPlist } from "../build.ts";

const secret = Buffer.alloc(32, 42).toString("base64"); // Test-only signing seed.
const key = sparklePrivateKey(secret),
  publicKey = sparklePublicKey(key);

test("signatures match the official Sparkle 2.9.4 sign_update tool", () => {
  // Produced by sign_update --ed-key-file with the test-only 32-byte seed above.
  const bytes = Buffer.from("QuickGUI Sparkle interoperability fixture\n");
  expect(sign(null, bytes, key).toString("base64")).toBe(
    "l6eLWtdZpa/jHx5TIueI982CxS7ApKYT6OrkBbP0tAcZSRraInQ6CbN1KJ4yYwDL9owrRJifMljQ9m6vlc4YBQ==",
  );
  expect(() => sparklePrivateKey(Buffer.alloc(64).toString("base64"))).toThrow("32-byte");
});

test("appcasts use Sparkle raw Ed25519 signatures on every platform", () => {
  const bytes = Buffer.from("test update payload");
  for (const target of ["darwin-arm64", "linux-x64", "windows-arm64"] as const) {
    const xml = renderAppcast({
      name: "Test & App",
      version: "2.0.0",
      target,
      url: "https://example.com/app?x=1&y=2",
      bytes,
      privateKey: secret,
      publicKey,
      notes: "<not markup>",
    });
    const signature = xml.match(/sparkle:edSignature="([^"]+)"/)![1]!;
    expect(verify(null, bytes, createPublicKey(key), Buffer.from(signature, "base64"))).toBe(true);
    expect(
      verify(null, Buffer.from("tampered"), createPublicKey(key), Buffer.from(signature, "base64")),
    ).toBe(false);
    expect(xml).toContain("Test &amp; App");
    expect(xml).toContain("&lt;not markup&gt;");
    // Notes are the changelog's Markdown; without the format Sparkle would read them as HTML.
    expect(xml).toContain('<description sparkle:format="markdown">&lt;not markup&gt;</description>');
  }
  expect(() =>
    renderAppcast({
      name: "Test",
      version: "1.0.0",
      target: "linux-x64",
      url: "https://example.com/update",
      bytes,
      privateKey: secret,
      publicKey: Buffer.alloc(32).toString("base64"),
    }),
  ).toThrow("does not match");
});
test("alternate enclosures follow the primary one, each with its own signature", () => {
  const appImage = Buffer.from("appimage payload");
  const tarball = Buffer.from("tarball payload");
  const xml = renderAppcast({
    name: "Test",
    version: "2.0.0",
    target: "linux-x64",
    url: "https://example.com/Test-2.0.0-linux-x64.AppImage",
    bytes: appImage,
    alternates: [{ url: "https://example.com/Test-2.0.0-linux-x64.tar.gz", bytes: tarball }],
    privateKey: secret,
    publicKey,
  });
  const enclosures = [...xml.matchAll(/<enclosure url="([^"]+)"[^>]*sparkle:edSignature="([^"]+)"/g)];
  expect(enclosures.map((match) => match[1])).toEqual([
    "https://example.com/Test-2.0.0-linux-x64.AppImage",
    "https://example.com/Test-2.0.0-linux-x64.tar.gz",
  ]);
  for (const [index, bytes] of [appImage, tarball].entries()) {
    const signature = Buffer.from(enclosures[index]![2]!, "base64");
    expect(verify(null, bytes, createPublicKey(key), signature)).toBe(true);
  }
  expect(xml.match(/<item>/g)).toHaveLength(1);
});
test("keygen writes Sparkle-compatible key material without overwriting existing keys", () => {
  const dir = mkdtempSync(join(tmpdir(), "quickgui-keygen-"));
  try {
    const paths = generateUpdaterKeys(dir);
    expect(Buffer.from(readFileSync(paths.secretKeyPath, "utf8").trim(), "base64").length).toBe(32);
    const privateKey = sparklePrivateKey(readFileSync(paths.secretKeyPath, "utf8"));
    expect(sparklePublicKey(privateKey)).toBe(readFileSync(paths.publicKeyPath, "utf8").trim());
    if (process.platform !== "win32")
      expect(statSync(paths.secretKeyPath).mode & 0o777).toBe(0o600);
    expect(() => generateUpdaterKeys(dir)).toThrow("already exists");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
test("TOML updater defaults are shared by Go metadata and Sparkle Info.plist", () => {
  const config = resolveConfig(
    {
      name: "Test",
      identifier: "test.app",
      updates: {
        target: "github",
        github: { repository: "example/app" },
        publicKey,
        automaticChecks: false,
      },
    },
    "/tmp",
  );
  const metadata = updaterMetadata(config, "darwin-arm64", "production");
  expect(metadata.feedUrl).toBe(
    "https://github.com/example/app/releases/latest/download/appcast-darwin-arm64.xml",
  );
  expect(metadata.automaticChecks).toBe(false);
  expect(metadata.development).toBe(false);
  const plist = macInfoPlist({
    name: config.name,
    displayName: config.name,
    executableName: config.executableName,
    identifier: config.identifier,
    version: config.version,
    buildVersion: config.buildVersion,
    minimumSystemVersion: config.macos.minimumSystemVersion,
    category: config.macos.category,
    updater: metadata,
  });
  expect(plist).toContain("<key>SUPublicEDKey</key><string>" + publicKey);
  expect(plist).toContain("<key>SUEnableAutomaticChecks</key><false/>");
  expect(plist).toContain("<key>SUVerifyUpdateBeforeExtraction</key><true/>");
  expect(updaterMetadata(config, "linux-x64", "development").development).toBe(true);
});

test("portable appcasts publish separate signed artifacts for each architecture", async () => {
  const dir = mkdtempSync(join(tmpdir(), "quickgui-appcast-targets-"));
  try {
    const secretPath = join(dir, "test.key"),
      source = join(dir, "setup.exe");
    writeFileSync(secretPath, secret);
    // Found by its conventional name; only the section of the version being built is published.
    writeFileSync(
      join(dir, "CHANGELOG.md"),
      "# Changelog\n\n## 2.0.0 - 2026-09-19\n\n- Faster & <safer>\n\n## 1.0.0\n\n- First release\n",
    );
    const config = resolveConfig(
      {
        name: "Test",
        identifier: "test.app",
        version: "2.0.0",
        updates: {
          target: "github",
          github: { repository: "example/app" },
          publicKey,
          ed25519SecretKey: secretPath,
        },
      },
      dir,
    );
    const paths: string[] = [];
    for (const target of ["windows-x64", "windows-arm64"] as const) {
      writeFileSync(source, target);
      const result = await writeAppcast({
        config,
        target,
        outputDirectory: dir,
        source,
        run: async () => {
          throw new Error("unexpected tool call");
        },
      });
      paths.push(result.artifactPath);
      expect(result.url).toBe(
        `https://github.com/example/app/releases/download/v2.0.0/Test-2.0.0-${target}.exe`,
      );
      expect(readFileSync(result.manifestPath, "utf8")).toContain(result.url);
      expect(readFileSync(result.manifestPath, "utf8")).toContain(
        '<description sparkle:format="markdown">- Faster &amp; &lt;safer&gt;</description>',
      );
      expect(readFileSync(result.notesPath!, "utf8")).toBe("- Faster & <safer>\n");
    }
    expect(paths[0]).not.toBe(paths[1]);
    await expect(
      writeAppcast({
        config: { ...config, version: "2.1.0" },
        target: "windows-x64",
        outputDirectory: dir,
        source,
        run: async () => {},
      }),
    ).rejects.toThrow("Add a `## 2.1.0` section");
    expect(readFileSync(paths[0]!, "utf8")).toBe("windows-x64");
    expect(readFileSync(paths[1]!, "utf8")).toBe("windows-arm64");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

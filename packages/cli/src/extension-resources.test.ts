import { expect, test } from "bun:test";
import {
  existsSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { gzipSync } from "node:zlib";
import { packResources, unpackResources } from "./extension-resources.ts";

test("resource bundles preserve framework links and executable permissions", () => {
  const dir = mkdtempSync(join(tmpdir(), "quickgui-resources-"));
  try {
    const framework = join(dir, "input/Sparkle.framework");
    mkdirSync(join(framework, "Versions/B"), { recursive: true });
    writeFileSync(join(framework, "Versions/B/Sparkle"), "native-image", { mode: 0o755 });
    symlinkSync("B", join(framework, "Versions/Current"));
    symlinkSync("Versions/Current/Sparkle", join(framework, "Sparkle"));
    const archive = join(dir, "framework.qgr");
    writeFileSync(archive, packResources(framework));
    const out = unpackResources(archive, join(dir, "output"));
    expect(readFileSync(join(out, "Sparkle"), "utf8")).toBe("native-image");
    expect(lstatSync(join(out, "Sparkle")).isSymbolicLink()).toBe(true);
    expect(lstatSync(join(out, "Versions/B/Sparkle")).mode & 0o111).toBe(0o111);
    expect(() => unpackResources(archive, join(dir, "output"))).toThrow("collision");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("resource bundles round-trip at the decoded content limit", () => {
  const dir = mkdtempSync(join(tmpdir(), "quickgui-resources-"));
  try {
    const framework = join(dir, "Test.framework");
    mkdirSync(framework);
    const payload = Buffer.alloc(128 * 1024 * 1024, 0x61);
    writeFileSync(join(framework, "payload"), payload);
    const archive = join(dir, "resource.qgr");
    writeFileSync(archive, packResources(framework));
    const out = unpackResources(archive, join(dir, "output"));
    expect(readFileSync(join(out, "payload")).equals(payload)).toBe(true);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("oversized resource envelopes are rejected before creating output", () => {
  const dir = mkdtempSync(join(tmpdir(), "quickgui-resources-"));
  try {
    const archive = join(dir, "resource.qgr");
    writeFileSync(archive, gzipSync(" ".repeat(176 * 1024 * 1024)));
    expect(() => unpackResources(archive, join(dir, "output"))).toThrow(
      "Cannot create a Buffer larger than",
    );
    expect(existsSync(join(dir, "output"))).toBe(false);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("decoded resource content remains bounded independently of the envelope", () => {
  const dir = mkdtempSync(join(tmpdir(), "quickgui-resources-"));
  try {
    const framework = join(dir, "Test.framework");
    mkdirSync(framework);
    const payload = Buffer.alloc(128 * 1024 * 1024 + 1);
    writeFileSync(join(framework, "payload"), payload);
    expect(() => packResources(framework)).toThrow("Native resources exceed their size limit");
    const archive = join(dir, "resource.qgr");
    writeFileSync(
      archive,
      gzipSync(
        JSON.stringify({
          schema: 1,
          root: "Test.framework",
          entries: [{ path: "payload", data: payload.toString("base64") }],
        }),
      ),
    );
    expect(() => unpackResources(archive, join(dir, "output"))).toThrow(
      "Native resources exceed their size limit",
    );
    expect(existsSync(join(dir, "output"))).toBe(false);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("resource paths, link traversal and duplicate paths are rejected before any writes", () => {
  const dir = mkdtempSync(join(tmpdir(), "quickgui-resources-"));
  try {
    const archive = join(dir, "resource.qgr");
    for (const entries of [
      [{ path: "../escape", data: "" }],
      [{ path: "link", link: "../../escape" }],
      [
        { path: "a", data: "" },
        { path: "a", data: "" },
      ],
      [
        { path: "link", link: "Versions" },
        { path: "link/file", data: "" },
      ],
    ]) {
      writeFileSync(
        archive,
        gzipSync(JSON.stringify({ schema: 1, root: "Test.framework", entries })),
      );
      expect(() => unpackResources(archive, join(dir, "output"))).toThrow();
    }
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

import { expect, test } from "bun:test";
import { lstatSync, mkdirSync, mkdtempSync, readFileSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  nativeLibPackages,
  packageLibDir,
  restoreNativeLibArtifacts,
  stageNativeLibArtifacts,
} from "./native-lib-artifacts.ts";

function scratch() {
  return mkdtempSync(join(tmpdir(), "quickgui-native-libs-"));
}

function writeLib(root: string, name: typeof nativeLibPackages[number], target: string, file: string) {
  const directory = join(packageLibDir(root, name), target);
  mkdirSync(directory, { recursive: true });
  writeFileSync(join(directory, file), name);
}

test("stages package lib trees under a wrapper that keeps the packages/ prefix", async () => {
  const root = scratch();
  writeLib(root, "native", "windows-x64", "quickgui_host.dll");
  writeLib(root, "extension-terminal", "windows-x64", "quickgui_terminal.dll");
  writeLib(root, "extension-updater", "windows-x64", "quickgui_updater.dll");
  writeLib(root, "extension-editor", "windows-x64", "quickgui_editor.dll");
  writeLib(root, "extension-markdown", "windows-x64", "quickgui_markdown.dll");
  const staged = join(root, "target", "native-libs");
  stageNativeLibArtifacts(root, staged);
  expect(await Bun.file(join(staged, "packages/native/lib/windows-x64/quickgui_host.dll")).text()).toBe(
    "native",
  );
  expect(
    await Bun.file(join(staged, "extensions/terminal/lib/windows-x64/quickgui_terminal.dll")).text(),
  ).toBe("extension-terminal");
});

test("restores artifacts that lost the packages/ prefix", async () => {
  const root = scratch();
  for (const name of nativeLibPackages) {
    const directory = join(root, name, "lib", "linux-x64");
    mkdirSync(directory, { recursive: true });
    writeFileSync(join(directory, "lib.bin"), name);
  }
  expect(restoreNativeLibArtifacts(root)).toEqual([...nativeLibPackages]);
  for (const name of nativeLibPackages) {
    expect(await Bun.file(join(packageLibDir(root, name), "linux-x64", "lib.bin")).text()).toBe(name);
  }
});

test("restore is a no-op when packages already hold the libraries", async () => {
  const root = scratch();
  writeLib(root, "native", "darwin-arm64", "libquickgui_host.dylib");
  expect(restoreNativeLibArtifacts(root)).toEqual([]);
  expect(
    await Bun.file(join(packageLibDir(root, "native"), "darwin-arm64", "libquickgui_host.dylib")).text(),
  ).toBe("native");
});

test("stages Sparkle-style framework bundles without following internal symlinks", () => {
  const root = scratch();
  writeLib(root, "native", "darwin-arm64", "libquickgui_host.dylib");
  writeLib(root, "extension-terminal", "darwin-arm64", "libquickgui_terminal.dylib");
  writeLib(root, "extension-editor", "darwin-arm64", "libquickgui_editor.dylib");
  writeLib(root, "extension-markdown", "darwin-arm64", "libquickgui_markdown.dylib");
  const framework = join(
    root,
    "extensions/updater/lib/darwin-arm64/Sparkle.framework",
  );
  const versionB = join(framework, "Versions/B");
  mkdirSync(join(versionB, "Resources"), { recursive: true });
  writeFileSync(join(versionB, "Sparkle"), "binary");
  writeFileSync(join(versionB, "Resources/Info.plist"), "plist");
  symlinkSync("B", join(framework, "Versions/Current"));
  symlinkSync("Versions/Current/Sparkle", join(framework, "Sparkle"));
  symlinkSync("Versions/Current/Resources", join(framework, "Resources"));

  const staged = join(root, "target", "native-libs");
  stageNativeLibArtifacts(root, staged);
  const dest = join(staged, "extensions/updater/lib/darwin-arm64/Sparkle.framework");
  expect(lstatSync(join(dest, "Sparkle")).isSymbolicLink()).toBe(true);
  expect(lstatSync(join(dest, "Resources")).isSymbolicLink()).toBe(true);
  expect(lstatSync(join(dest, "Versions/Current")).isSymbolicLink()).toBe(true);
  expect(readFileSync(join(dest, "Versions/B/Sparkle"), "utf8")).toBe("binary");
  expect(readFileSync(join(dest, "Versions/Current/Sparkle"), "utf8")).toBe("binary");
});

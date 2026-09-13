import { expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
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

function writeLib(root: string, name: string, target: string, file: string) {
  const directory = join(root, "packages", name, "lib", target);
  mkdirSync(directory, { recursive: true });
  writeFileSync(join(directory, file), name);
}

test("stages package lib trees under a wrapper that keeps the packages/ prefix", async () => {
  const root = scratch();
  writeLib(root, "native", "windows-x64", "quickgui_host.dll");
  writeLib(root, "extension-terminal", "windows-x64", "quickgui_terminal.dll");
  writeLib(root, "extension-updater", "windows-x64", "quickgui_updater.dll");
  const staged = join(root, "target", "native-libs");
  stageNativeLibArtifacts(root, staged);
  expect(await Bun.file(join(staged, "packages/native/lib/windows-x64/quickgui_host.dll")).text()).toBe(
    "native",
  );
  expect(
    await Bun.file(join(staged, "packages/extension-terminal/lib/windows-x64/quickgui_terminal.dll")).text(),
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

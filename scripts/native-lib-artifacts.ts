/** Stage and restore native library trees for the Release workflow artifacts. */

import { cpSync, existsSync, mkdirSync, renameSync, statSync } from "node:fs";
import { join, resolve } from "node:path";

export const nativeLibPackages = ["native", "extension-terminal", "extension-updater"] as const;

function isDirectory(path: string) {
  return existsSync(path) && statSync(path).isDirectory();
}

export function packageLibDir(root: string, name: (typeof nativeLibPackages)[number]) {
  return join(root, "packages", name, "lib");
}

/** Copy package lib trees under `destination/packages/<name>/lib`. */
export function stageNativeLibArtifacts(root: string, destination: string) {
  const destRoot = resolve(destination);
  for (const name of nativeLibPackages) {
    const source = packageLibDir(root, name);
    if (!isDirectory(source)) throw new Error(`missing ${source}`);
    const dest = join(destRoot, "packages", name, "lib");
    mkdirSync(dest, { recursive: true });
    cpSync(source, dest, { recursive: true });
  }
  return destRoot;
}

/**
 * `actions/upload-artifact` strips the longest shared prefix. Uploading
 * package lib globs therefore lands as `native/lib/...` after download.
 * Move those trees back onto `packages/<name>/lib` when needed.
 */
export function restoreNativeLibArtifacts(root: string) {
  const restored: string[] = [];
  for (const name of nativeLibPackages) {
    const dest = packageLibDir(root, name);
    if (isDirectory(dest)) continue;
    const stripped = join(root, name, "lib");
    if (!isDirectory(stripped)) continue;
    mkdirSync(join(root, "packages", name), { recursive: true });
    renameSync(stripped, dest);
    restored.push(name);
  }
  return restored;
}

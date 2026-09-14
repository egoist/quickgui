/** Stage and restore native library trees for the Release workflow artifacts. */

import { cpSync, existsSync, mkdirSync, renameSync, statSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

export const nativeLibPackages = ["native", "extension-terminal", "extension-updater", "extension-editor", "extension-markdown"] as const;

function isDirectory(path: string) {
  return existsSync(path) && statSync(path).isDirectory();
}

export function packageLibDir(root: string, name: (typeof nativeLibPackages)[number]) {
  return name === "native" ? join(root, "packages", "native", "lib")
    : join(root, "extensions", name.slice("extension-".length), "lib");
}

/** Preserve the repository's package and extension paths inside the artifact wrapper. */
export function stageNativeLibArtifacts(root: string, destination: string) {
  const destRoot = resolve(destination);
  for (const name of nativeLibPackages) {
    const source = packageLibDir(root, name);
    if (!isDirectory(source)) throw new Error(`missing ${source}`);
    const dest = packageLibDir(destRoot, name);
    mkdirSync(dest, { recursive: true });
    cpSync(source, dest, { recursive: true, verbatimSymlinks: true });
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
    mkdirSync(dirname(dest), { recursive: true });
    renameSync(stripped, dest);
    restored.push(name);
  }
  return restored;
}

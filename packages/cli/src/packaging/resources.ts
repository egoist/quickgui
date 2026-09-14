/** Copy and archive helpers for application files that sit next to the packaged executable. */

import { cpSync, mkdirSync, readdirSync, readFileSync, statSync } from "node:fs";
import { basename, join, posix, resolve } from "node:path";

import { CliError } from "../error.ts";
import type { TarEntry } from "./archive.ts";

const SKIPPED_RESOURCE_NAMES = new Set(["icon.iconset"]);

function isSkippedResourceName(name: string): boolean {
  return name.startsWith(".") || SKIPPED_RESOURCE_NAMES.has(name.toLocaleLowerCase("en-US"));
}

/** Copy the contents of the project `resources/` directory into `destination`. */
export function copyResourceDirectory(
  source: string,
  destination: string,
  reservedNames: ReadonlySet<string> = new Set(),
): string[] {
  const names = new Map(
    [...reservedNames].map((name) => [name.toLocaleLowerCase("en-US"), name] as const),
  );
  const staged: string[] = [];
  mkdirSync(destination, { recursive: true });
  for (const name of readdirSync(source).sort()) {
    if (isSkippedResourceName(name)) continue;
    const normalizedName = name.toLocaleLowerCase("en-US");
    const previous = names.get(normalizedName);
    if (previous) {
      throw new CliError(
        `Resource destination name is reserved or duplicated: ${name} conflicts with ${previous}`,
      );
    }
    names.set(normalizedName, name);
    const target = resolve(destination, name);
    cpSync(join(source, name), target, { recursive: statSync(join(source, name)).isDirectory() });
    staged.push(target);
  }
  return staged;
}

/**
 * Copy the convention `resources/` directory, then extra configured files, into `destination`.
 * Destination names must be unique and must not replace reserved packaging files.
 */
export function stageApplicationResources(
  resourceDir: string | undefined,
  extras: readonly string[],
  destination: string,
  reservedNames: ReadonlySet<string> = new Set(),
): string[] {
  const reserved = new Set(reservedNames);
  const staged: string[] = [];
  if (resourceDir) {
    const copied = copyResourceDirectory(resourceDir, destination, reserved);
    staged.push(...copied);
    for (const path of copied) reserved.add(basename(path));
  }
  staged.push(...copyResources(extras, destination, reserved));
  return staged;
}

/** Copy each extra configured resource into `destination` using its basename. */
export function copyResources(
  paths: readonly string[],
  destination: string,
  reservedNames: ReadonlySet<string> = new Set(),
): string[] {
  const names = new Map(
    [...reservedNames].map((name) => [name.toLocaleLowerCase("en-US"), name] as const),
  );
  const staged: string[] = [];
  for (const path of paths) {
    const name = basename(path);
    const normalizedName = name.toLocaleLowerCase("en-US");
    const previous = names.get(normalizedName);
    if (previous) {
      throw new CliError(
        `Resource destination name is reserved or duplicated: ${name} conflicts with ${previous}`,
      );
    }
    names.set(normalizedName, name);
    const target = resolve(destination, name);
    cpSync(path, target, { recursive: statSync(path).isDirectory() });
    staged.push(target);
  }
  return staged;
}

/** Copy a file or directory beside an executable, preserving the source basename. */
export function copyBesideExecutable(source: string, directory: string): void {
  cpSync(source, join(directory, basename(source)), { recursive: statSync(source).isDirectory() });
}

/**
 * Expand files and directories into deterministic ustar entries under `destination`.
 *
 * Directory trees keep their relative layout. Paths use forward slashes so the same archive
 * is produced on every host.
 */
export function extraPayloadEntries(
  sources: readonly string[],
  destination: string,
): TarEntry[] {
  const entries: TarEntry[] = [];
  for (const source of sources) {
    collectPayload(source, posix.join(posixNormalize(destination), basename(source)), entries);
  }
  return entries;
}

function collectPayload(source: string, destination: string, entries: TarEntry[]): void {
  const info = statSync(source);
  if (info.isDirectory()) {
    entries.push({ path: destination, type: "directory", mode: 0o755 });
    for (const child of readdirSync(source).sort()) {
      collectPayload(join(source, child), posix.join(destination, child), entries);
    }
    return;
  }
  entries.push({
    path: destination,
    data: new Uint8Array(readFileSync(source)),
    mode: info.mode & 0o777 || 0o644,
  });
}

function posixNormalize(path: string): string {
  return path.replaceAll("\\", "/").replace(/\/+$/, "");
}

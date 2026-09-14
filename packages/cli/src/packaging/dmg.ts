/**
 * macOS disk-image packaging through the vendored `create-dmg` script.
 *
 * Argument lists are built here so tests can assert the Finder layout without
 * running `hdiutil` or AppleScript.
 */

import { existsSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { CliError } from "../error.ts";

/** Finder window origin used for production DMGs, in screen points. */
export const DMG_WINDOW_POSITION = { x: 200, y: 120 } as const;
/** Finder window size that fits the app and Applications drop link. */
export const DMG_WINDOW_SIZE = { width: 660, height: 400 } as const;
/** Icon size `create-dmg` applies in the Finder window (maximum 128). */
export const DMG_ICON_SIZE = 128;
/** Center of the application icon inside the Finder window. */
export const DMG_APP_POSITION = { x: 180, y: 190 } as const;
/** Center of the Applications drop link inside the Finder window. */
export const DMG_APPLICATIONS_POSITION = { x: 480, y: 185 } as const;

export interface CreateDmgInput {
  dmgPath: string;
  /** Folder whose contents are copied into the image. Should contain only the `.app`. */
  sourceFolder: string;
  volumeName: string;
  /** Basename of the `.app` inside `sourceFolder`. */
  appFileName: string;
  /** Optional `.icns` used as the mounted volume icon. */
  volumeIcon?: string;
}

/** Absolute path of the vendored `create-dmg` script. */
export function resolveCreateDmgScript(): string {
  const vendored = fileURLToPath(new URL("../../vendor/create-dmg/create-dmg", import.meta.url));
  if (!existsSync(vendored)) {
    throw new CliError(`Vendored create-dmg script is missing: ${vendored}`);
  }
  return vendored;
}

/** Flags passed to `create-dmg` after the script path. */
export function createDmgFlags(input: CreateDmgInput): string[] {
  return [
    "--volname",
    input.volumeName,
    ...(input.volumeIcon ? ["--volicon", input.volumeIcon] : []),
    "--window-pos",
    String(DMG_WINDOW_POSITION.x),
    String(DMG_WINDOW_POSITION.y),
    "--window-size",
    String(DMG_WINDOW_SIZE.width),
    String(DMG_WINDOW_SIZE.height),
    "--icon-size",
    String(DMG_ICON_SIZE),
    "--icon",
    input.appFileName,
    String(DMG_APP_POSITION.x),
    String(DMG_APP_POSITION.y),
    "--hide-extension",
    input.appFileName,
    "--app-drop-link",
    String(DMG_APPLICATIONS_POSITION.x),
    String(DMG_APPLICATIONS_POSITION.y),
    "--overwrite",
    "--hdiutil-quiet",
    input.dmgPath,
    input.sourceFolder,
  ];
}

/** Full `create-dmg` invocation for a production macOS disk image. */
export function createDmgArguments(input: CreateDmgInput): string[] {
  return [resolveCreateDmgScript(), ...createDmgFlags(input)];
}

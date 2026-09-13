/**
 * Icon container writers and PNG resizing.
 *
 * `.icns` and `.ico` copy PNG bytes. Missing sizes are generated with `Bun.Image` from one
 * square source PNG on every host. An optional `<icon>.iconset` folder can replace individual
 * sizes with hand-drawn files.
 */

import { existsSync, readFileSync } from "node:fs";
import { basename, dirname, extname, join } from "node:path";

import { CliError, errorMessage } from "../error.ts";
import { concat } from "./archive.ts";

/** Largest source icon QuickGUI reads. */
export const MAX_ICON_SOURCE_BYTES = 16 * 1024 * 1024;
/** Sizes written into a generated `.icns`, in the `ic07`…`ic13` PNG family. */
export const ICNS_ENTRIES: ReadonlyArray<{ type: string; size: number }> = [
  { type: "ic11", size: 32 },
  { type: "ic12", size: 64 },
  { type: "ic07", size: 128 },
  { type: "ic13", size: 256 },
  { type: "ic08", size: 256 },
  { type: "ic09", size: 512 },
  { type: "ic10", size: 1024 },
];
/** Sizes written into a generated `.ico`. */
export const ICO_SIZES: readonly number[] = [16, 24, 32, 48, 64, 128, 256];
/** Sizes installed into the Linux `hicolor` icon theme. */
export const LINUX_ICON_SIZES: readonly number[] = [16, 32, 48, 64, 128, 256, 512];
/** Union of sizes written into `.icns`, `.ico`, and Linux `hicolor`. */
export const PACKAGED_ICON_SIZES: readonly number[] = [
  ...new Set([...ICNS_ENTRIES.map((entry) => entry.size), ...ICO_SIZES, ...LINUX_ICON_SIZES]),
].sort((left, right) => left - right);

const PNG_SIGNATURE = Uint8Array.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);

/** Read a PNG's `IHDR` dimensions, rejecting anything that is not a PNG. */
export function pngDimensions(data: Uint8Array): { width: number; height: number } {
  if (data.byteLength < 24) throw new CliError("Icon file is too small to be a PNG");
  for (const [index, byte] of PNG_SIGNATURE.entries()) {
    if (data[index] !== byte) throw new CliError("Icon file is not a PNG");
  }
  const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
  if (view.getUint32(12) !== 0x49_48_44_52) throw new CliError("PNG is missing its IHDR chunk");
  return { width: view.getUint32(16), height: view.getUint32(20) };
}

/** Build an `.icns` container from square PNGs keyed by pixel size. */
export function createIcns(sources: ReadonlyMap<number, Uint8Array>): Uint8Array {
  const encoder = new TextEncoder();
  const parts: Uint8Array[] = [];
  let used = 0;
  for (const entry of ICNS_ENTRIES) {
    const png = sources.get(entry.size);
    if (!png) continue;
    used += 1;
    const header = new Uint8Array(8);
    header.set(encoder.encode(entry.type), 0);
    new DataView(header.buffer).setUint32(4, png.byteLength + 8);
    parts.push(header, png);
  }
  if (used === 0) throw new CliError("No PNG sizes were available for the .icns icon");
  const body = concat(parts);
  const container = new Uint8Array(8 + body.byteLength);
  container.set(encoder.encode("icns"), 0);
  new DataView(container.buffer).setUint32(4, container.byteLength);
  container.set(body, 8);
  return container;
}

/** Build a PNG-based `.ico` container from square PNGs keyed by pixel size. */
export function createIco(sources: ReadonlyMap<number, Uint8Array>): Uint8Array {
  const images = ICO_SIZES.filter((size) => sources.has(size)).map(
    (size) => [size, sources.get(size)!] as const,
  );
  if (images.length === 0) throw new CliError("No PNG sizes were available for the .ico icon");
  if (images.length > 255) throw new CliError("An .ico may contain at most 255 images");
  const directory = new Uint8Array(6 + images.length * 16);
  const view = new DataView(directory.buffer);
  view.setUint16(0, 0, true);
  view.setUint16(2, 1, true);
  view.setUint16(4, images.length, true);
  let offset = directory.byteLength;
  for (const [index, [size, png]] of images.entries()) {
    const entry = 6 + index * 16;
    directory[entry] = size >= 256 ? 0 : size;
    directory[entry + 1] = size >= 256 ? 0 : size;
    directory[entry + 2] = 0;
    directory[entry + 3] = 0;
    view.setUint16(entry + 4, 1, true);
    view.setUint16(entry + 6, 32, true);
    view.setUint32(entry + 8, png.byteLength, true);
    view.setUint32(entry + 12, offset, true);
    offset += png.byteLength;
  }
  return concat([directory, ...images.map(([, png]) => png)]);
}

/** Path of the optional hand-drawn PNG that replaces a generated size. */
export function iconsetEntryPath(icon: string, size: number): string {
  const directory = join(dirname(icon), `${basename(icon, extname(icon))}.iconset`);
  return join(directory, `icon_${size}x${size}.png`);
}

/** Validate a configured source icon and return its bytes. */
export function readSourceIcon(icon: string): Uint8Array {
  if (!existsSync(icon)) throw new CliError(`Icon not found: ${icon}`);
  const data = new Uint8Array(readFileSync(icon));
  if (data.byteLength > MAX_ICON_SOURCE_BYTES) {
    throw new CliError(`Icon exceeds ${MAX_ICON_SOURCE_BYTES} bytes: ${icon}`);
  }
  const { width, height } = pngDimensions(data);
  if (width !== height) throw new CliError(`Icon must be square, got ${width}x${height}: ${icon}`);
  if (width < 256) {
    throw new CliError(`Icon must be at least 256x256 pixels, got ${width}x${width}: ${icon}`);
  }
  return data;
}

/** Resize a square PNG to `size`×`size` with Bun's built-in image pipeline. */
export async function resizePng(source: Uint8Array, size: number): Promise<Uint8Array> {
  if (!Number.isInteger(size) || size < 1) {
    throw new CliError(`Invalid icon size ${size}`);
  }
  try {
    const bytes = await new Bun.Image(source).resize(size, size).png().bytes();
    const { width, height } = pngDimensions(bytes);
    if (width !== size || height !== size) {
      throw new CliError(`Resized icon is ${width}x${height}, expected ${size}x${size}`);
    }
    return bytes;
  } catch (error) {
    if (error instanceof CliError) throw error;
    throw new CliError(`Failed to resize icon to ${size}x${size}: ${errorMessage(error)}`);
  }
}

/**
 * Collect square PNGs for `sizes`.
 *
 * Matching `<icon>.iconset/icon_<n>x<n>.png` files win, then the source icon when it already
 * matches, then `Bun.Image` on every host.
 */
export async function collectIconSizes(
  icon: string,
  source: Uint8Array,
  sizes: readonly number[],
): Promise<Map<number, Uint8Array>> {
  const collected = new Map<number, Uint8Array>();
  const sourceSize = pngDimensions(source).width;
  const missing: number[] = [];
  for (const size of sizes) {
    const preSized = iconsetEntryPath(icon, size);
    if (existsSync(preSized)) {
      const data = new Uint8Array(readFileSync(preSized));
      try {
        const { width, height } = pngDimensions(data);
        if (width === size && height === size) {
          collected.set(size, data);
          continue;
        }
      } catch {
        // Fall through and generate from the source icon.
      }
    }
    if (size === sourceSize) {
      collected.set(size, source);
      continue;
    }
    missing.push(size);
  }
  if (missing.length > 0) {
    try {
      const generated = await Promise.all(missing.map((size) => resizePng(source, size)));
      for (const [index, size] of missing.entries()) collected.set(size, generated[index]!);
    } catch (error) {
      if (error instanceof CliError) throw error;
      throw new CliError(`Could not generate icon sizes from ${icon}: ${errorMessage(error)}`);
    }
  }
  const ordered = new Map<number, Uint8Array>();
  for (const size of sizes) {
    const png = collected.get(size);
    if (png) ordered.set(size, png);
  }
  return ordered;
}

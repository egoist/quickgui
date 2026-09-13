/** Bounded extension resources preserve framework symlinks without extracting archive paths.
 * All files are validated and written before links; links cannot traverse '..' or absolute paths.
 */
import {
  chmodSync,
  existsSync,
  lstatSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  readlinkSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { basename, dirname, join } from "node:path";
import { gzipSync, gunzipSync } from "node:zlib";
import { CliError } from "./error.ts";

const maximumBytes = 128 * 1024 * 1024;
const maximumEntries = 4096;
const maximumPathLength = 512;
// Base64 expands content by 4/3, with padding per entry. Reserve two bounded ASCII
// paths plus JSON fields per entry, and space for the root and envelope fields.
const maximumEnvelopeBytes =
  Math.ceil(maximumBytes / 3) * 4 + maximumEntries * (2 * maximumPathLength + 128) + 1024;
interface Entry {
  path: string;
  data?: string;
  link?: string;
  executable?: boolean;
}
interface Resources {
  schema: 1;
  root: string;
  entries: Entry[];
}
const safePath = (path: string): boolean =>
  path.length <= maximumPathLength &&
  path
    .split("/")
    .every((part) => /^[A-Za-z0-9_. +@-]+$/.test(part) && part !== "." && part !== "..");

export function packResources(root: string): Buffer {
  const entries: Entry[] = [];
  const visit = (directory: string, prefix: string): void => {
    for (const name of readdirSync(directory).sort()) {
      const path = join(directory, name),
        relative = prefix + name,
        stat = lstatSync(path);
      if (stat.isSymbolicLink()) entries.push({ path: relative, link: readlinkSync(path) });
      else if (stat.isDirectory()) visit(path, relative + "/");
      else if (stat.isFile())
        entries.push({
          path: relative,
          data: readFileSync(path).toString("base64"),
          executable: Boolean(stat.mode & 0o111),
        });
      else throw new CliError(`Unsupported native resource: ${path}`);
    }
  };
  visit(root, "");
  const value: Resources = { schema: 1, root: basename(root), entries };
  validate(value);
  const envelope = JSON.stringify(value);
  if (Buffer.byteLength(envelope) > maximumEnvelopeBytes)
    throw new CliError("Native resource envelope exceeds its size limit");
  return gzipSync(envelope);
}
function validate(value: Resources): void {
  if (
    value.schema !== 1 ||
    !safePath(value.root) ||
    value.root.includes("/") ||
    !Array.isArray(value.entries) ||
    value.entries.length > maximumEntries
  )
    throw new CliError("Invalid native resource bundle");
  const paths = new Set<string>();
  let size = 0;
  for (const entry of value.entries) {
    if (!entry || typeof entry.path !== "string" || !safePath(entry.path) || paths.has(entry.path))
      throw new CliError("Unsafe or duplicate native resource path");
    paths.add(entry.path);
    if (entry.link !== undefined) {
      if (entry.data !== undefined || typeof entry.link !== "string" || !safePath(entry.link))
        throw new CliError("Unsafe native resource symlink");
    } else {
      if (
        typeof entry.data !== "string" ||
        Buffer.from(entry.data, "base64").toString("base64") !== entry.data
      )
        throw new CliError("Invalid native resource bytes");
      size += Buffer.byteLength(entry.data, "base64");
      if (size > maximumBytes) throw new CliError("Native resources exceed their size limit");
    }
  }
  for (const path of paths) {
    let parent = dirname(path);
    while (parent !== ".") {
      if (paths.has(parent)) throw new CliError("Native resource path traverses a file or link");
      parent = dirname(parent);
    }
  }
}
export function unpackResources(archive: string, destination: string): string {
  const value = JSON.parse(
    gunzipSync(readFileSync(archive), { maxOutputLength: maximumEnvelopeBytes }).toString("utf8"),
  ) as Resources;
  validate(value);
  const root = join(destination, value.root);
  if (existsSync(root)) throw new CliError(`Native resource collision: ${value.root}`);
  mkdirSync(root, { recursive: true });
  for (const entry of value.entries.filter((entry) => entry.link === undefined)) {
    const path = join(root, entry.path);
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, Buffer.from(entry.data!, "base64"), {
      flag: "wx",
      mode: entry.executable ? 0o755 : 0o644,
    });
    chmodSync(path, entry.executable ? 0o755 : 0o644);
  }
  for (const entry of value.entries.filter((entry) => entry.link !== undefined)) {
    const path = join(root, entry.path);
    mkdirSync(dirname(path), { recursive: true });
    symlinkSync(entry.link!, path);
  }
  return root;
}

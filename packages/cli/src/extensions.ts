import { createHash } from "node:crypto";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  renameSync,
  rmSync,
  statSync,
  utimesSync,
  writeFileSync,
} from "node:fs";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import type { ResolvedQuickGuiConfig } from "./config.ts";
import { CliError } from "./error.ts";
import { targetInfo, type QuickGuiTarget } from "./targets.ts";

const manifestName = "quickgui.extension.json";
const maxExtensions = 32;
const maxArchiveBytes = 128 * 1024 * 1024;
const maxCacheBytes = 512 * 1024 * 1024;

function validPackageName(name: unknown): name is string {
  return (
    typeof name === "string" &&
    name.length <= 214 &&
    /^(?:@[a-z0-9][a-z0-9._-]*\/)?[a-z0-9][a-z0-9._-]*$/.test(name)
  );
}

export interface ExtensionManifest {
  schema: 1;
  name: string;
  abi: 1;
  package: string;
  version: string;
  library: string;
  resources?: Partial<Record<"darwin" | "linux" | "windows", string[]>>;
}

interface GoPackage {
  ImportPath?: string;
  Dir?: string;
  Module?: { Path?: string; Version?: string };
}

/** go list emits a stream of JSON objects, including nested objects and escaped strings. */
export function parseGoPackages(source: string): GoPackage[] {
  const packages: GoPackage[] = [];
  let start = -1,
    depth = 0,
    quoted = false,
    escaped = false;
  for (let i = 0; i < source.length; i++) {
    const char = source[i]!;
    if (quoted) {
      if (escaped) escaped = false;
      else if (char === "\\") escaped = true;
      else if (char === '"') quoted = false;
      continue;
    }
    if (char === '"') quoted = true;
    else if (char === "{") {
      if (depth++ === 0) start = i;
    } else if (char === "}") {
      if (--depth < 0) throw new CliError("Invalid go list output");
      if (depth === 0) {
        packages.push(JSON.parse(source.slice(start, i + 1)));
        start = -1;
      }
    } else if (depth === 0 && !/\s/.test(char)) throw new CliError("Invalid go list output");
  }
  if (depth || quoted || start !== -1) throw new CliError("Incomplete go list output");
  return packages;
}

export function extensionManifests(packages: GoPackage[]): ExtensionManifest[] {
  const found = new Map<string, ExtensionManifest>();
  for (const pkg of packages) {
    if (!pkg.Dir) continue;
    const path = join(pkg.Dir, manifestName);
    if (!existsSync(path)) continue;
    if (statSync(path).size > 16 * 1024)
      throw new CliError(`Extension manifest is too large: ${path}`);
    const value = JSON.parse(readFileSync(path, "utf8"));
    if (
      !value ||
      value.schema !== 1 ||
      value.abi !== 1 ||
      typeof value.name !== "string" ||
      !/^[a-z][a-z0-9-]{0,63}$/.test(value.name) ||
      value.name === "host" ||
      !validPackageName(value.package) ||
      value.library !== `quickgui_${value.name.replaceAll("-", "_")}` ||
      typeof value.version !== "string" ||
      value.version.length > 64 ||
      !/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(value.version)
    ) {
      throw new CliError(`Invalid QuickGUI extension manifest: ${path}`);
    }
    const resources: NonNullable<ExtensionManifest["resources"]> = {};
    if (value.resources !== undefined) {
      if (!value.resources || typeof value.resources !== "object" || Array.isArray(value.resources))
        throw new CliError("Invalid extension resources");
      for (const platform of ["darwin", "linux", "windows"] as const) {
        const files = value.resources[platform];
        if (files === undefined) continue;
        if (
          !Array.isArray(files) ||
          files.length > 16 ||
          files.some(
            (file) => typeof file !== "string" || !/^[A-Za-z][A-Za-z0-9_.-]{0,127}$/.test(file),
          ) ||
          new Set(files).size !== files.length
        )
          throw new CliError("Invalid extension resource names");
        resources[platform] = [...files].sort();
      }
      if (Object.keys(value.resources).some((key) => !["darwin", "linux", "windows"].includes(key)))
        throw new CliError("Unknown extension resource platform");
    }
    const manifest: ExtensionManifest = {
      schema: value.schema,
      name: value.name,
      abi: value.abi,
      package: value.package,
      version: value.version,
      library: value.library,
      ...(Object.keys(resources).length ? { resources } : {}),
    };
    const previous = found.get(value.name);
    if (previous && JSON.stringify(previous) !== JSON.stringify(manifest))
      throw new CliError(`Conflicting requirements for QuickGUI extension ${value.name}`);
    found.set(value.name, manifest);
    if (found.size > maxExtensions)
      throw new CliError(
        `QuickGUI applications support at most ${maxExtensions} native extensions`,
      );
  }
  return [...found.values()].sort((a, b) => a.name.localeCompare(b.name));
}

export async function discoverExtensions(
  config: ResolvedQuickGuiConfig,
  entry: string,
  env: Record<string, string | undefined>,
): Promise<ExtensionManifest[]> {
  const child = Bun.spawn(
    [
      "go",
      "list",
      "-deps",
      "-json=ImportPath,Dir",
      ...(config.native.tags.length ? ["-tags", config.native.tags.join(",")] : []),
      entry,
    ],
    {
      cwd: config.projectRoot,
      env,
      stdin: "ignore",
      stdout: "pipe",
      stderr: "pipe",
    },
  );
  const [status, stdout, stderr] = await Promise.all([
    child.exited,
    new Response(child.stdout).text(),
    new Response(child.stderr).text(),
  ]);
  if (status !== 0) throw new CliError(`Could not resolve Go extension imports\n${stderr.trim()}`);
  return extensionManifests(parseGoPackages(stdout));
}

export function extensionLibraryName(manifest: ExtensionManifest, target: QuickGuiTarget): string {
  switch (targetInfo(target).platform) {
    case "darwin":
      return `lib${manifest.library}.dylib`;
    case "windows":
      return `${manifest.library}.dll`;
    default:
      return `lib${manifest.library}.so`;
  }
}

function hash(bytes: Uint8Array): string {
  return createHash("sha256").update(bytes).digest("hex");
}

export function verifyArchive(bytes: Uint8Array, integrity: string): void {
  const match = /(?:^|\s)sha512-([A-Za-z0-9+/]+={0,2})(?:\s|$)/.exec(integrity);
  if (!match || createHash("sha512").update(bytes).digest("base64") !== match[1])
    throw new CliError("Native extension archive integrity check failed");
}

/** Bound decompressed/downloaded data before buffering it. */
export async function readBounded(
  stream: ReadableStream<Uint8Array>,
  maximum: number,
): Promise<Uint8Array> {
  const reader = stream.getReader();
  const chunks: Uint8Array[] = [];
  let size = 0;
  try {
    for (;;) {
      const next = await reader.read();
      if (next.done) break;
      size += next.value.byteLength;
      if (size > maximum) {
        await reader.cancel();
        throw new CliError("Native extension data exceeds its size limit");
      }
      chunks.push(next.value);
    }
  } finally {
    reader.releaseLock();
  }
  return Buffer.concat(chunks, size);
}

async function boundedFetch(url: string, maximum: number): Promise<Uint8Array> {
  const response = await fetch(url, { signal: AbortSignal.timeout(60_000), redirect: "error" });
  if (!response.ok)
    throw new CliError(`Could not download native extension (${response.status}): ${url}`);
  if (Number(response.headers.get("content-length")) > maximum) {
    await response.body?.cancel();
    throw new CliError("Native extension download exceeds its size limit");
  }
  if (!response.body) throw new CliError("Native extension download returned no data");
  return readBounded(response.body, maximum);
}

export async function resolveExtension(
  manifest: ExtensionManifest,
  target: QuickGuiTarget,
  projectRoot: string,
  resource?: string,
): Promise<string> {
  if (resource && !manifest.resources?.[targetInfo(target).platform]?.includes(resource))
    throw new CliError("Undeclared extension resource");
  const filename = resource ?? extensionLibraryName(manifest, target);
  if (process.env.QUICKGUI_EXTENSION_DIR) {
    const explicit = resolve(projectRoot, process.env.QUICKGUI_EXTENSION_DIR, filename);
    if (!existsSync(explicit))
      throw new CliError(`Extension ${manifest.name} is missing from QUICKGUI_EXTENSION_DIR`);
    return explicit;
  }
  const candidates: string[] = [];
  try {
    candidates.push(dirname(Bun.resolveSync(`${manifest.package}/package.json`, projectRoot)));
  } catch {
    /* optional package */
  }
  if (manifest.package === `@quickgui/extension-${manifest.name}`)
    candidates.push(resolve(import.meta.dir, "..", "..", `extension-${manifest.name}`));
  let foundVersion: string | undefined;
  for (const directory of candidates) {
    const metadata = join(directory, "package.json");
    const file = join(directory, "lib", target, filename);
    if (!existsSync(metadata) || !existsSync(file)) continue;
    const pkg = JSON.parse(readFileSync(metadata, "utf8"));
    if (pkg.name !== manifest.package || pkg.version !== manifest.version) {
      foundVersion = typeof pkg.version === "string" ? pkg.version : String(pkg.version);
      continue;
    }
    return file;
  }
  if (foundVersion !== undefined)
    throw new CliError(
      `Extension ${manifest.name} requires ${manifest.package}@${manifest.version}; found ${foundVersion}`,
    );
  return downloadExtension(manifest, target, filename);
}

async function downloadExtension(
  manifest: ExtensionManifest,
  target: QuickGuiTarget,
  filename: string,
): Promise<string> {
  const cacheRoot = process.env.QUICKGUI_CACHE_DIR || join(homedir(), ".cache", "quickgui");
  const cache = join(cacheRoot, "extensions");
  mkdirSync(cache, { recursive: true });
  // Independent publishers can choose the same logical name and version.
  // Namespace the cache by the full artifact identity, not just that name.
  const identity = hash(
    Buffer.from(
      JSON.stringify([
        manifest.package,
        manifest.name,
        manifest.version,
        manifest.abi,
        manifest.library,
        target,
      ]),
    ),
  );
  const destination = join(cache, `${identity}-${filename}`);
  const checksum = `${destination}.sha256`;
  if (
    existsSync(destination) &&
    existsSync(checksum) &&
    statSync(destination).size <= maxArchiveBytes &&
    statSync(checksum).size <= 128 &&
    hash(readFileSync(destination)) === readFileSync(checksum, "utf8").trim()
  ) {
    const now = new Date();
    utimesSync(destination, now, now);
    return destination;
  }
  console.log(`[quickgui] Downloading ${manifest.package}@${manifest.version} (${target})`);
  const registry = `https://registry.npmjs.org/${encodeURIComponent(manifest.package)}/${encodeURIComponent(manifest.version)}`;
  const pkg = JSON.parse(Buffer.from(await boundedFetch(registry, 1024 * 1024)).toString("utf8"));
  if (
    pkg.name !== manifest.package ||
    pkg.version !== manifest.version ||
    typeof pkg.dist?.tarball !== "string" ||
    typeof pkg.dist?.integrity !== "string"
  )
    throw new CliError("Native extension registry metadata does not match the requested release");
  const url = new URL(pkg.dist.tarball);
  if (url.protocol !== "https:" || url.hostname !== "registry.npmjs.org")
    throw new CliError("Unexpected native extension archive origin");
  const archive = await boundedFetch(url.href, maxArchiveBytes);
  verifyArchive(archive, pkg.dist.integrity);
  const temporary = mkdtempSync(join(cache, ".download-"));
  try {
    const tarball = join(temporary, "archive.tgz");
    writeFileSync(tarball, archive);
    // Extract exactly the requested member to stdout, never paths supplied by the archive.
    const child = Bun.spawn(["tar", "-xOf", tarball, `package/lib/${target}/${filename}`], {
      stdin: "ignore",
      stdout: "pipe",
      stderr: "pipe",
    });
    const timer = setTimeout(() => child.kill(), 60_000);
    let status: number, bytes: Uint8Array, stderr: Uint8Array;
    try {
      [status, bytes, stderr] = await Promise.all([
        child.exited,
        readBounded(child.stdout, maxArchiveBytes),
        readBounded(child.stderr, 64 * 1024),
      ]);
    } catch (error) {
      child.kill();
      await child.exited;
      throw error;
    } finally {
      clearTimeout(timer);
    }
    if (status !== 0 || bytes.byteLength === 0 || bytes.byteLength > maxArchiveBytes)
      throw new CliError(
        `Native extension does not contain ${target}/${filename}: ${Buffer.from(stderr).toString().trim()}`,
      );
    const file = join(temporary, filename);
    const digest = hash(new Uint8Array(bytes));
    writeFileSync(file, new Uint8Array(bytes));
    renameSync(file, destination);
    writeFileSync(join(temporary, "checksum"), digest + "\n");
    renameSync(join(temporary, "checksum"), checksum);
    pruneCache(cache, destination);
    return destination;
  } finally {
    rmSync(temporary, { recursive: true, force: true });
  }
}

function pruneCache(directory: string, keep: string): void {
  const files = readdirSync(directory)
    .filter((name) => !/^(\.|.*\.sha256$)/.test(name))
    .map((name) => {
      const path = join(directory, name);
      try {
        return { path, stat: statSync(path) };
      } catch {
        return undefined;
      }
    })
    .filter((file) => file !== undefined)
    .filter((file) => file.stat.isFile())
    .sort((a, b) => a.stat.mtimeMs - b.stat.mtimeMs);
  let size = files.reduce((total, file) => total + file.stat.size, 0);
  for (const file of files) {
    if (size <= maxCacheBytes) break;
    if (file.path === keep) continue;
    rmSync(file.path, { force: true });
    rmSync(`${file.path}.sha256`, { force: true });
    size -= file.stat.size;
  }
}

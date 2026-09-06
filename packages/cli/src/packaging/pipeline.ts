/**
 * Filesystem side of QuickGUI packaging.
 *
 * The generators in the sibling modules are pure; this module is the only place that touches the
 * filesystem or spawns a tool, so tests can exercise every generated file and argument list
 * without running `appimagetool`, `makensis`, `minisign`, or `productbuild`.
 */

import {
  chmodSync,
  cpSync,
  existsSync,
  mkdirSync,
  readFileSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { basename, dirname, join, resolve } from "node:path";

import type { ResolvedQuickGuiConfig } from "../config.ts";
import { CliError } from "../error.ts";
import { targetInfo, type QuickGuiTarget } from "../targets.ts";
import type { TarEntry } from "./archive.ts";
import {
  collectIconSizes,
  createIcns,
  createIco,
  ICNS_ENTRIES,
  ICO_SIZES,
  LINUX_ICON_SIZES,
  readSourceIcon,
  sipsResizeArguments,
} from "./icons.ts";
import { sharedMimeInfoXml } from "./documents.ts";
import {
  appImageArguments,
  appRunScript,
  createDebianPackage,
  debianControl,
  debianMd5Sums,
  debianPayloadPaths,
  DEBIAN_ARCHITECTURES,
  desktopEntry,
} from "./linux.ts";
import { makensisArguments, nsisScript, signToolArguments } from "./windows.ts";
import {
  buildUpdateManifest,
  joinUrl,
  minisignSignArguments,
  readSignature,
  resolveSecretKey,
  serializeUpdateManifest,
  updateArchiveArguments,
  updateTarget,
  type MinisignTool,
  type UpdateManifest,
} from "./updates.ts";

export type Runner = (command: string[], cwd: string) => Promise<void>;

/** Generated packaging outputs, in the order they should be reported to the user. */
export interface PackagingOutput {
  /** Files that must be moved from staging into the target output directory. */
  artifacts: string[];
  /** Advisory lines printed after a successful build. */
  notes: string[];
}

function toolPath(name: string): string | undefined {
  return Bun.which(name) ?? undefined;
}

/** The first Minisign-compatible signer on PATH, if any. */
export function findMinisignTool(): MinisignTool | undefined {
  if (toolPath("minisign")) return "minisign";
  if (toolPath("rsign")) return "rsign";
  return undefined;
}

/** Resize a square PNG with `sips`, which only exists on macOS. */
function sipsResizer(
  icon: string,
  scratchDirectory: string,
  run: (command: string[]) => void,
): ((size: number) => Uint8Array | undefined) | undefined {
  if (process.platform !== "darwin" || !toolPath("sips")) return undefined;
  return (size: number) => {
    const destination = join(scratchDirectory, `icon-${size}.png`);
    try {
      run(sipsResizeArguments(icon, destination, size));
    } catch {
      return undefined;
    }
    return existsSync(destination) ? new Uint8Array(readFileSync(destination)) : undefined;
  };
}

export interface IconBuildResult {
  icns?: Uint8Array;
  ico?: Uint8Array;
  png: Map<number, Uint8Array>;
}

/**
 * Turn one configured square PNG into the containers each platform needs.
 *
 * Sizes come from `<icon>.iconset/icon_<n>x<n>.png` first, then the source itself, then `sips`.
 * Nothing is invented: a size with no source is simply absent from the container.
 */
export function buildIcons(
  icon: string,
  scratchDirectory: string,
  runSync: (command: string[]) => void,
): IconBuildResult {
  const source = readSourceIcon(icon);
  mkdirSync(scratchDirectory, { recursive: true });
  const resize = sipsResizer(icon, scratchDirectory, runSync);
  const sizes = [
    ...new Set([...ICNS_ENTRIES.map((entry) => entry.size), ...ICO_SIZES, ...LINUX_ICON_SIZES]),
  ].sort((left, right) => left - right);
  const png = collectIconSizes(icon, source, sizes, resize);
  if (png.size === 0) {
    throw new CliError(
      `No usable icon sizes for ${icon}. Provide pre-sized PNGs in ${basename(icon, ".png")}.iconset/ ` +
        "(icon_16x16.png … icon_1024x1024.png) or build on macOS where `sips` can resize.",
    );
  }
  const icns = safeContainer(() => createIcns(png));
  const ico = safeContainer(() => createIco(png));
  return { png, ...(icns ? { icns } : {}), ...(ico ? { ico } : {}) };
}

function safeContainer(build: () => Uint8Array): Uint8Array | undefined {
  try {
    return build();
  } catch {
    return undefined;
  }
}

export interface LinuxPackagingInput {
  config: ResolvedQuickGuiConfig;
  target: QuickGuiTarget;
  /** Compiled executable inside the staging directory. */
  executablePath: string;
  stagingRoot: string;
  icons?: IconBuildResult;
  run: Runner;
  /** Extra files installed next to the executable (the Go host shared library). */
  extraFiles?: ReadonlyArray<{ path: string; name: string }>;
}

/** Build the desktop entry, AppDir, optional AppImage, and optional `.deb`. */
export async function packageLinux(input: LinuxPackagingInput): Promise<PackagingOutput> {
  const { config } = input;
  const artifacts: string[] = [];
  const notes: string[] = [];
  const paths = debianPayloadPaths(config.executableName);
  const entry = desktopEntry({
    name: config.name,
    executableName: config.executableName,
    identifier: config.identifier,
    categories: config.linux.categories,
    protocols: config.protocols,
    documentTypes: config.documentTypes,
    ...(config.linux.comment ? { comment: config.linux.comment } : {}),
  });
  const mimeXml =
    config.documentTypes.some((type) => type.mimeTypes.length > 0)
      ? sharedMimeInfoXml(config.documentTypes)
      : undefined;

  const desktopPath = resolve(input.stagingRoot, `${config.executableName}.desktop`);
  writeFileSync(desktopPath, entry);
  artifacts.push(desktopPath);
  if (mimeXml) {
    const mimePath = resolve(input.stagingRoot, `${config.executableName}.mime.xml`);
    writeFileSync(mimePath, mimeXml);
    artifacts.push(mimePath);
  }

  if (config.linux.appImage) {
    const appDir = resolve(input.stagingRoot, `${config.executableName}.AppDir`);
    mkdirSync(join(appDir, "usr", "bin"), { recursive: true });
    cpSync(input.executablePath, join(appDir, "usr", "bin", config.executableName));
    chmodSync(join(appDir, "usr", "bin", config.executableName), 0o755);
    for (const extra of input.extraFiles ?? []) {
      cpSync(extra.path, join(appDir, "usr", "bin", extra.name));
    }
    writeFileSync(join(appDir, `${config.executableName}.desktop`), entry);
    writeFileSync(join(appDir, "AppRun"), appRunScript(config.executableName));
    chmodSync(join(appDir, "AppRun"), 0o755);
    const largest = largestIcon(input.icons);
    if (largest) writeFileSync(join(appDir, `${config.executableName}.png`), largest);
    for (const [size, png] of input.icons?.png ?? []) {
      const directory = join(appDir, "usr", "share", "icons", "hicolor", `${size}x${size}`, "apps");
      mkdirSync(directory, { recursive: true });
      writeFileSync(join(directory, `${config.executableName}.png`), png);
    }
    if (toolPath("appimagetool")) {
      const architecture = targetInfo(input.target).architecture === "arm64" ? "aarch64" : "x86_64";
      const appImage = resolve(
        input.stagingRoot,
        `${config.executableName}-${config.version}-${architecture}.AppImage`,
      );
      await input.run(appImageArguments(appDir, appImage), config.projectRoot);
      artifacts.push(appImage);
    } else {
      artifacts.push(appDir);
      notes.push(
        `appimagetool was not found on PATH. The finished AppDir is at ` +
          `${basename(appDir)}; run \`appimagetool ${basename(appDir)}\` to produce an AppImage.`,
      );
    }
  }

  if (config.linux.deb) {
    if (!config.linux.maintainer) {
      throw new CliError("Building a .deb requires `linux.maintainer` (\"Name <email>\")");
    }
    const architecture = DEBIAN_ARCHITECTURES[targetInfo(input.target).architecture];
    const executable = new Uint8Array(readFileSync(input.executablePath));
    const data: TarEntry[] = [
      { path: paths.executable, data: executable, mode: 0o755 },
      ...(input.extraFiles ?? []).map((extra) => ({
        path: `${dirname(paths.executable)}/${extra.name}`,
        data: new Uint8Array(readFileSync(extra.path)),
        mode: 0o644,
      })),
      { path: paths.desktopEntry, data: new TextEncoder().encode(entry) },
      ...(mimeXml
        ? [{ path: paths.mimePackage, data: new TextEncoder().encode(mimeXml) }]
        : []),
      ...[...(input.icons?.png ?? [])]
        .filter(([size]) => LINUX_ICON_SIZES.includes(size))
        .map(([size, png]) => ({ path: paths.icon(size), data: png })),
    ];
    const control = debianControl({
      packageName: debianPackageName(config.executableName),
      version: config.version,
      architecture,
      maintainer: config.linux.maintainer,
      description: config.linux.comment ?? config.name,
      section: config.linux.section,
      depends: config.linux.depends,
      installedSizeKilobytes: data.reduce(
        (total, member) => total + (member.data?.byteLength ?? 0),
        0,
      ) / 1024,
    });
    const md5sums = debianMd5Sums(
      data.map((member) => ({
        path: member.path,
        md5: new Bun.CryptoHasher("md5")
          .update(Buffer.from(member.data ?? new Uint8Array()))
          .digest("hex"),
      })),
    );
    const debian = createDebianPackage({
      control,
      md5sums,
      data,
      gzip: (bytes) => new Uint8Array(Bun.gzipSync(Buffer.from(bytes))),
    });
    const debPath = resolve(
      input.stagingRoot,
      `${debianPackageName(config.executableName)}_${config.version}_${architecture}.deb`,
    );
    writeFileSync(debPath, debian);
    artifacts.push(debPath);
  }

  return { artifacts, notes };
}

export function debianPackageName(executableName: string): string {
  const name = executableName
    .toLowerCase()
    .replace(/[^a-z0-9+.-]+/g, "-")
    .replace(/^[^a-z0-9]+/, "")
    .replace(/[^a-z0-9+.]+$/, "");
  if (name.length < 2) {
    throw new CliError(`Application name does not produce a valid Debian package name`);
  }
  return name;
}

function largestIcon(icons: IconBuildResult | undefined): Uint8Array | undefined {
  if (!icons) return undefined;
  let best: [number, Uint8Array] | undefined;
  for (const entry of icons.png) {
    if (!best || entry[0] > best[0]) best = entry;
  }
  return best?.[1];
}

export interface WindowsPackagingInput {
  config: ResolvedQuickGuiConfig;
  executablePath: string;
  stagingRoot: string;
  icons?: IconBuildResult;
  run: Runner;
  /** Extra files installed next to the executable (the Go host shared library). */
  extraFiles?: ReadonlyArray<readonly [string, string]>;
}

/** Write the NSIS script and, when `makensis` is available, compile and sign the installer. */
export async function packageWindows(input: WindowsPackagingInput): Promise<PackagingOutput> {
  const { config } = input;
  const artifacts: string[] = [];
  const notes: string[] = [];
  const installerName = `${config.executableName}-${config.version}-setup.exe`;
  const installerPath = resolve(input.stagingRoot, installerName);
  let iconPath: string | undefined = config.windows.icon;
  if (!iconPath && input.icons?.ico) {
    iconPath = resolve(input.stagingRoot, `${config.executableName}.ico`);
    writeFileSync(iconPath, input.icons.ico);
    artifacts.push(iconPath);
  }
  const script = nsisScript({
    name: config.name,
    executableName: `${config.executableName}.exe`,
    identifier: config.identifier,
    version: config.version,
    publisher: config.windows.publisher ?? config.name,
    executablePath: input.executablePath,
    outputFile: installerPath,
    protocols: config.protocols,
    ...(input.extraFiles && input.extraFiles.length > 0 ? { extraFiles: input.extraFiles } : {}),
    documentTypes: config.documentTypes,
    ...(iconPath ? { iconPath } : {}),
    ...(config.windows.nsis?.installDirectory
      ? { installDirectory: config.windows.nsis.installDirectory }
      : {}),
    ...(config.windows.nsis?.perMachine !== undefined
      ? { perMachine: config.windows.nsis.perMachine }
      : {}),
    ...(config.windows.nsis?.createDesktopShortcut !== undefined
      ? { createDesktopShortcut: config.windows.nsis.createDesktopShortcut }
      : {}),
    ...(config.windows.nsis?.createStartMenuShortcut !== undefined
      ? { createStartMenuShortcut: config.windows.nsis.createStartMenuShortcut }
      : {}),
  });
  const scriptPath = resolve(input.stagingRoot, `${config.executableName}.nsi`);
  writeFileSync(scriptPath, script);
  artifacts.push(scriptPath);

  if (!toolPath("makensis")) {
    notes.push(
      `makensis was not found on PATH. The generated NSIS script is at ${basename(scriptPath)}; ` +
        `run \`makensis ${basename(scriptPath)}\` to produce ${installerName}.`,
    );
    return { artifacts, notes };
  }
  await input.run(makensisArguments(scriptPath), config.projectRoot);
  if (!existsSync(installerPath)) {
    throw new CliError(`makensis did not produce the expected installer: ${installerPath}`);
  }
  artifacts.push(installerPath);

  const signing = config.windows.signing;
  if (signing) {
    if (process.platform !== "win32") {
      notes.push(
        "Authenticode signing was skipped: `signtool` only runs on Windows. Re-run the build on a " +
          "Windows host to sign the installer.",
      );
    } else {
      const password = signing.passwordEnvironmentVariable
        ? process.env[signing.passwordEnvironmentVariable]
        : undefined;
      if (signing.passwordEnvironmentVariable && password === undefined) {
        throw new CliError(
          `\`windows.signing.passwordEnvironmentVariable\` names ${signing.passwordEnvironmentVariable}, which is not set`,
        );
      }
      await input.run(
        signToolArguments({
          artifact: installerPath,
          ...(signing.certificateFile ? { certificateFile: signing.certificateFile } : {}),
          ...(signing.subjectName ? { subjectName: signing.subjectName } : {}),
          ...(password ? { password } : {}),
          ...(signing.timestampUrl ? { timestampUrl: signing.timestampUrl } : {}),
          ...(signing.digest ? { digest: signing.digest } : {}),
        }),
        config.projectRoot,
      );
    }
  }
  return { artifacts, notes };
}

export interface UpdateManifestInput {
  config: ResolvedQuickGuiConfig;
  target: QuickGuiTarget;
  outputDirectory: string;
  /** The final `.app` bundle, AppImage, executable, or installer to publish. */
  source: string;
  baseUrl: string;
  run: Runner;
}

export interface UpdateManifestOutput {
  artifactPath: string;
  manifestPath: string;
  url: string;
}

/**
 * Produce and sign the artifact the Rust updater installs, then write `latest.json`.
 *
 * The archive layout matches `install_staged` exactly: macOS takes a gzip tar holding one
 * top-level `.app`, Linux takes a raw executable or a gzip tar holding one AppImage, and Windows
 * takes the `.exe` installer as-is.
 */
export async function writeUpdateManifest(
  input: UpdateManifestInput,
): Promise<UpdateManifestOutput> {
  const { config } = input;
  const platform = targetInfo(input.target).platform;
  let artifactPath: string;
  if (platform === "darwin" || input.source.endsWith(".AppImage")) {
    const archiveName = `${basename(input.source)}.tar.gz`;
    artifactPath = resolve(input.outputDirectory, archiveName);
    await input.run(
      updateArchiveArguments(dirname(input.source), basename(input.source), artifactPath),
      config.projectRoot,
    );
  } else {
    artifactPath = input.source;
  }
  if (!existsSync(artifactPath)) {
    throw new CliError(`The update artifact was not created: ${artifactPath}`);
  }

  const tool = findMinisignTool();
  if (!tool) {
    throw new CliError(
      "Signing an update needs `minisign` or `rsign` on PATH. Install one " +
        "(https://jedisct1.github.io/minisign/) and re-run, or drop --update-manifest.",
    );
  }
  const secretKey = resolveSecretKey(config.updates?.minisignSecretKey);
  const signaturePath = `${artifactPath}.minisig`;
  await runSigner(
    minisignSignArguments({
      tool,
      secretKey,
      artifact: artifactPath,
      signaturePath,
      comment: `${config.name} ${config.version}`,
      trustedComment: `${config.name} ${config.version} ${updateTarget(input.target)}`,
    }),
    config.projectRoot,
    process.env.QUICKGUI_MINISIGN_PASSWORD,
  );

  const manifestPath = resolve(input.outputDirectory, "latest.json");
  const existing = readExistingManifest(manifestPath);
  const notes = config.updates?.notesFile
    ? readFileSync(config.updates.notesFile, "utf8")
    : undefined;
  const manifest = buildUpdateManifest({
    version: config.version,
    baseUrl: input.baseUrl,
    target: input.target,
    artifactName: basename(artifactPath),
    signature: readSignature(signaturePath),
    ...(notes ? { notes } : {}),
    ...(existing ? { existing } : {}),
  });
  writeFileSync(manifestPath, serializeUpdateManifest(manifest));
  return {
    artifactPath,
    manifestPath,
    url: joinUrl(input.baseUrl, basename(artifactPath)),
  };
}

/**
 * Run a Minisign-compatible signer, feeding an encrypted key's password through standard input.
 *
 * Neither `minisign -S` nor `rsign sign` accepts a password flag, so `QUICKGUI_MINISIGN_PASSWORD`
 * is written to the child's stdin; without it stdin is closed and an unencrypted key signs
 * without prompting.
 */
async function runSigner(command: string[], cwd: string, password?: string): Promise<void> {
  const child = Bun.spawn(command, {
    cwd,
    stdin:
      password === undefined ? "ignore" : new TextEncoder().encode(`${password}\n${password}\n`),
    stdout: "pipe",
    stderr: "pipe",
  });
  const [status, stdout, stderr] = await Promise.all([
    child.exited,
    new Response(child.stdout).text(),
    new Response(child.stderr).text(),
  ]);
  if (status !== 0) {
    const detail = stderr.trim() || stdout.trim();
    throw new CliError(
      `Signing failed: ${command[0]} exited with ${status}${detail ? `\n${detail}` : ""}`,
    );
  }
}

function readExistingManifest(path: string): UpdateManifest | undefined {
  if (!existsSync(path) || !statSync(path).isFile()) return undefined;
  try {
    return JSON.parse(readFileSync(path, "utf8")) as UpdateManifest;
  } catch {
    return undefined;
  }
}

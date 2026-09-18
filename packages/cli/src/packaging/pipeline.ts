/**
 * Filesystem side of QuickGUI packaging.
 *
 * The generators in the sibling modules are pure; this module is the only place that touches the
 * filesystem or spawns a tool, so tests can exercise every generated file and argument list
 * without running `appimagetool`, `makensis`, or `productbuild`.
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

import { sharedLibraryName } from "../native-build.ts";
import type { ResolvedQuickGuiConfig } from "../config.ts";
import { CliError } from "../error.ts";
import { targetInfo, type QuickGuiTarget } from "../targets.ts";
import { createTar, type TarEntry } from "./archive.ts";
import {
  collectIconSizes,
  createIcns,
  createIco,
  LINUX_ICON_SIZES,
  PACKAGED_ICON_SIZES,
  placeholderPng,
  readSourceIcon,
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
  installScript,
  latestVersionFile,
  MANAGED_INSTALL_ICON_SIZE,
  MANAGED_INSTALL_MARKER,
  managedInstallMarker,
  tarballName,
} from "./linux.ts";
import { writeAppcast } from "./appcast.ts";
import { copyBesideExecutable, extraPayloadEntries } from "./resources.ts";
import { makensisArguments, nsisScript, signToolArguments } from "./windows.ts";

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

export interface IconBuildResult {
  icns?: Uint8Array;
  ico?: Uint8Array;
  png: Map<number, Uint8Array>;
}

/**
 * Turn one configured square PNG into the containers each platform needs.
 *
 * Sizes come from `<icon>.iconset/icon_<n>x<n>.png` first, then the source itself, then
 * `Bun.Image` on every host.
 */
export async function buildIcons(icon: string): Promise<IconBuildResult> {
  const source = readSourceIcon(icon);
  const png = await collectIconSizes(icon, source, PACKAGED_ICON_SIZES);
  if (png.size === 0) throw new CliError(`No usable icon sizes for ${icon}`);
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
  /** Core and selected extension libraries staged beside the executable. */
  libraries?: string[];
  target: QuickGuiTarget;
  /** Compiled executable inside the staging directory. */
  executablePath: string;
  stagingRoot: string;
  icons?: IconBuildResult;
  run: Runner;
}

/** Build the desktop entry, AppDir, optional AppImage, optional `.deb`, and optional tarball. */
export async function packageLinux(input: LinuxPackagingInput): Promise<PackagingOutput> {
  const { config } = input;
  const artifacts: string[] = [];
  const notes: string[] = [];
  const entry = desktopEntry({
    name: config.name,
    executableName: config.executableName,
    identifier: config.identifier,
    categories: config.linux.categories,
    protocols: config.protocols,
    documentTypes: config.documentTypes,
    ...(config.linux.comment ? { comment: config.linux.comment } : {}),
  });
  const mimeXml = config.documentTypes.some((type) => type.mimeTypes.length > 0)
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
    for (const library of input.libraries ?? [
      join(input.stagingRoot, sharedLibraryName(input.target)),
    ]) {
      copyBesideExecutable(library, join(appDir, "usr", "bin"));
    }
    writeFileSync(join(appDir, `${config.executableName}.desktop`), entry);
    writeFileSync(join(appDir, "AppRun"), appRunScript(config.executableName));
    chmodSync(join(appDir, "AppRun"), 0o755);
    // appimagetool rejects an AppDir whose desktop entry names an icon that is not there.
    const largest = largestIcon(input.icons);
    writeFileSync(join(appDir, `${config.executableName}.png`), largest ?? placeholderPng());
    if (!largest) {
      notes.push("No `icon` is configured, so the AppImage uses a placeholder icon.");
    }
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
      throw new CliError('Building a .deb requires `linux.maintainer` ("Name <email>")');
    }
    const architecture = DEBIAN_ARCHITECTURES[targetInfo(input.target).architecture];
    const data = prefixPayload(input, "usr", entry, mimeXml, false);
    const control = debianControl({
      packageName: debianPackageName(config.executableName),
      version: config.version,
      architecture,
      maintainer: config.linux.maintainer,
      description: config.linux.comment ?? config.name,
      section: config.linux.section,
      depends: config.linux.depends,
      installedSizeKilobytes:
        data.reduce((total, member) => total + (member.data?.byteLength ?? 0), 0) / 1024,
    });
    const md5sums = payloadMd5Sums(data);
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

  if (config.linux.tarball) {
    // One versioned top-level directory, which `install.sh` and the updater both strip.
    const root = `${config.executableName}-${config.version}`;
    const data = prefixPayload(input, root, entry, mimeXml, true);
    data.push({
      path: `${root}/${MANAGED_INSTALL_MARKER}`,
      data: new TextEncoder().encode(
        managedInstallMarker(config.identifier, config.executableName),
      ),
    });
    const tarballPath = resolve(
      input.stagingRoot,
      tarballName(config.executableName, config.version, input.target),
    );
    writeFileSync(tarballPath, Bun.gzipSync(Buffer.from(createTar(data))));
    const scriptPath = resolve(input.stagingRoot, "install.sh");
    writeFileSync(
      scriptPath,
      installScript({
        name: config.name,
        executableName: config.executableName,
        identifier: config.identifier,
        packageName: debianPackageName(config.executableName),
        ...(config.updates ? { destination: config.updates.destination } : {}),
      }),
    );
    chmodSync(scriptPath, 0o755);
    const latestPath = resolve(input.stagingRoot, latestVersionFile(input.target));
    writeFileSync(latestPath, `${config.version}\n`);
    artifacts.push(tarballPath, scriptPath, latestPath);
  }

  return { artifacts, notes };
}

/**
 * Executable, sidecars, desktop entry, MIME package, and icons laid out under one install prefix.
 *
 * A tarball install pins its desktop entry to one icon file, so it gets the placeholder when the
 * project has no icon; a `.deb` leaves the theme lookup to the desktop.
 */
function prefixPayload(
  input: LinuxPackagingInput,
  prefix: string,
  entry: string,
  mimeXml: string | undefined,
  placeholderIcon: boolean,
): TarEntry[] {
  const paths = debianPayloadPaths(input.config.executableName, prefix);
  const encoder = new TextEncoder();
  const icons = [...(input.icons?.png ?? [])].filter(([size]) => LINUX_ICON_SIZES.includes(size));
  if (icons.length === 0 && placeholderIcon) {
    icons.push([MANAGED_INSTALL_ICON_SIZE, placeholderPng(MANAGED_INSTALL_ICON_SIZE)]);
  }
  return [
    {
      path: paths.executable,
      data: new Uint8Array(readFileSync(input.executablePath)),
      mode: 0o755,
    },
    ...extraPayloadEntries(
      input.libraries ?? [join(input.stagingRoot, sharedLibraryName(input.target))],
      dirname(paths.executable),
    ),
    { path: paths.desktopEntry, data: encoder.encode(entry) },
    ...(mimeXml ? [{ path: paths.mimePackage, data: encoder.encode(mimeXml) }] : []),
    ...icons.map(([size, png]) => ({ path: paths.icon(size), data: png })),
  ];
}

/** Hash file members for `md5sums`. Directory entries stay in `data.tar.gz` only. */
export function payloadMd5Sums(data: readonly TarEntry[]): string {
  return debianMd5Sums(
    data
      .filter((member) => member.type !== "directory")
      .map((member) => ({
        path: member.path,
        md5: new Bun.CryptoHasher("md5")
          .update(Buffer.from(member.data ?? new Uint8Array()))
          .digest("hex"),
      })),
  );
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
  libraries?: string[];
  executablePath: string;
  stagingRoot: string;
  icons?: IconBuildResult;
  run: Runner;
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
    extraFiles: (input.libraries ?? [join(input.stagingRoot, "quickgui_host.dll")]).map(
      (library) => [library, basename(library)],
    ),
    outputFile: installerPath,
    protocols: config.protocols,
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
  /** Further artifacts for the same appcast item, such as the Linux tarball beside an AppImage. */
  alternates?: string[];
  run: Runner;
}

export interface UpdateManifestOutput {
  artifactPath: string;
  manifestPath: string;
  /** This release's changelog section, written beside the appcast. */
  notesPath?: string;
  url: string;
}

/** Sign the update artifacts and write the target's appcast. */
export async function writeUpdateManifest(
  input: UpdateManifestInput,
): Promise<UpdateManifestOutput> {
  return writeAppcast(input);
}

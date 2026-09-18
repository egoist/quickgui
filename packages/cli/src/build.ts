import {
  chmodSync,
  cpSync,
  existsSync,
  mkdirSync,
  readFileSync,
  mkdtempSync,
  renameSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { basename, extname, join, relative, resolve } from "node:path";

import {
  languageLabel,
  type MacOSNotarizationConfig,
  type ResolvedQuickGuiConfig,
} from "./config.ts";
import { CliError, errorMessage } from "./error.ts";
import { compileNativeApplication } from "./native-build.ts";
import { requireRustManifest } from "./rust-build.ts";
import {
  macDocumentTypesPlist,
  macTypeDeclarationsPlist,
  type ResolvedDocumentType,
} from "./packaging/documents.ts";
import { createDmgArguments } from "./packaging/dmg.ts";
import {
  masCodesignArguments,
  masEntitlementsTemplate,
  masPackageFilename,
  productBuildArguments,
  validateMasConfig,
} from "./packaging/mas.ts";
import {
  buildIcons,
  debianPackageName,
  packageLinux,
  packageWindows,
  writeUpdateManifest,
  type IconBuildResult,
} from "./packaging/pipeline.ts";
import { stageApplicationResources } from "./packaging/resources.ts";
import { updaterMetadata } from "./packaging/appcast.ts";
import { LATEST_LINUX_VERSION_FILE, tarballName } from "./packaging/linux.ts";
import { uploadRelease } from "./packaging/publish.ts";
import { targetInfo, type QuickGuiTarget } from "./targets.ts";

export type BuildMode = "development" | "production";

export interface BuildProjectOptions {
  mode: BuildMode;
  target: QuickGuiTarget;
  outDir?: string;
  signingIdentity?: string;
  notarization?: MacOSNotarizationConfig;
  /** Sign the update artifacts and write the target's appcast. */
  updateManifest?: boolean;
  /** Publish the release files to `updates.github` or `updates.s3`. Implies `updateManifest`. */
  upload?: boolean;
  /** Sign for the Mac App Store and produce a `.pkg` instead of a DMG. */
  macAppStore?: boolean;
}

export interface BuildResult {
  artifactPath: string;
  executablePath: string;
  target: QuickGuiTarget;
  mode: BuildMode;
  dmgPath?: string;
  /** Installers, packages, desktop entries, and scripts produced beside the main artifact. */
  packagePaths?: string[];
  /** The signed artifact published to the updater, when `--update-manifest` was requested. */
  updateArtifactPath?: string;
  /** The appcast, when `--update-manifest` was requested. */
  manifestPath?: string;
  /** Public URLs of the files `--upload` published. */
  uploadedUrls?: string[];
  /** Advisory lines describing tools that were missing. */
  notes?: string[];
}

interface StagedBuild extends BuildResult {
  /** Staged files moved into the target output directory alongside the main artifact. */
  extraArtifacts?: string[];
  /** Installers, packages, desktop entries, and scripts. Never application sidecars. */
  packagedArtifacts?: string[];
}

export async function buildProject(
  config: ResolvedQuickGuiConfig,
  options: BuildProjectOptions,
): Promise<BuildResult> {
  const info = targetInfo(options.target);
  validateBuildInputs(config, info.platform);
  if (info.platform === "darwin" && options.mode === "production") {
    validateMacPackaging(config, options);
  }
  const baseOutDir = options.outDir
    ? resolve(config.projectRoot, options.outDir)
    : options.mode === "development"
      ? resolve(config.projectRoot, ".quickgui", "dev")
      : config.outDir;
  const targetOutDir = resolve(baseOutDir, options.target);
  mkdirSync(targetOutDir, { recursive: true });
  const stagingRoot = mkdtempSync(join(targetOutDir, ".quickgui-staging-"));

  try {
    const staged: StagedBuild =
      info.platform === "darwin"
        ? await buildMacApp(config, options, stagingRoot)
        : await buildExecutable(config, options, stagingRoot);
    const finalPath = resolve(targetOutDir, basename(staged.artifactPath));
    const finalDmgPath = staged.dmgPath
      ? resolve(targetOutDir, basename(staged.dmgPath))
      : undefined;
    const extras = (staged.extraArtifacts ?? []).map((stagedPath) => ({
      stagedPath,
      finalPath: resolve(targetOutDir, basename(stagedPath)),
    }));
    replaceArtifacts(
      [
        { stagedPath: staged.artifactPath, finalPath },
        ...(staged.dmgPath && finalDmgPath
          ? [{ stagedPath: staged.dmgPath, finalPath: finalDmgPath }]
          : []),
        ...extras,
      ],
      stagingRoot,
    );
    const executablePath = resolve(finalPath, relative(staged.artifactPath, staged.executablePath));
    const packagePaths = (staged.packagedArtifacts ?? []).map((stagedPath) =>
      resolve(targetOutDir, basename(stagedPath)),
    );
    let updates: { artifactPath: string; manifestPath: string; notesPath?: string } | undefined;
    let uploadedUrls: string[] | undefined;
    if ((options.updateManifest || options.upload) && options.mode === "production") {
      updates = await writeUpdateManifest({
        config,
        target: options.target,
        outputDirectory: targetOutDir,
        source: updateSource(options, finalPath, packagePaths),
        alternates: updateAlternates(options, packagePaths),
        run: (command, cwd) => run(command, cwd),
      });
      if (options.upload && config.updates) {
        const release = releaseFiles(
          [finalDmgPath, ...packagePaths, updates.artifactPath].filter(
            (path): path is string => path !== undefined,
          ),
          updates.manifestPath,
        );
        uploadedUrls = await uploadRelease({
          destination: config.updates.destination,
          name: config.name,
          version: config.version,
          ...release,
          ...(updates.notesPath ? { notesFile: updates.notesPath } : {}),
          cwd: config.projectRoot,
        });
      }
    }
    return {
      artifactPath: finalPath,
      executablePath,
      target: options.target,
      mode: options.mode,
      ...(finalDmgPath ? { dmgPath: finalDmgPath } : {}),
      ...(packagePaths.length > 0 ? { packagePaths } : {}),
      ...(staged.notes?.length ? { notes: staged.notes } : {}),
      ...(updates
        ? { updateArtifactPath: updates.artifactPath, manifestPath: updates.manifestPath }
        : {}),
      ...(uploadedUrls ? { uploadedUrls } : {}),
    };
  } finally {
    if (existsSync(stagingRoot)) rmSync(stagingRoot, { recursive: true, force: true });
  }
}

async function buildMacApp(
  config: ResolvedQuickGuiConfig,
  options: BuildProjectOptions,
  stagingRoot: string,
): Promise<StagedBuild> {
  if (process.platform !== "darwin") {
    throw new CliError("macOS .app bundles must currently be assembled and signed on macOS");
  }
  const identity = options.signingIdentity ?? config.macos.signingIdentity ?? "-";
  const notarization =
    options.mode === "production" ? (options.notarization ?? config.macos.notarization) : undefined;
  const displayName = options.mode === "development" ? `${config.name} Dev` : config.name;
  const identifier =
    options.mode === "development" ? `${config.identifier}.dev` : config.identifier;
  const appPath = resolve(stagingRoot, `${config.executableName}.app`);
  const contents = resolve(appPath, "Contents");
  const macos = resolve(contents, "MacOS");
  const resources = resolve(contents, "Resources");
  mkdirSync(macos, { recursive: true });
  mkdirSync(resources, { recursive: true });
  const executablePath = resolve(macos, config.executableName);
  const fonts = stageFonts(config, resolve(resources, "fonts"));
  const libraries = await compileExecutable(config, options, executablePath, fonts);
  const sparkle = libraries.find((path) => path.endsWith("Sparkle.framework"));
  if (
    sparkle &&
    (options.macAppStore ||
      (config.macos.entitlements &&
        /<key>com\.apple\.security\.app-sandbox<\/key>\s*<true\s*\//.test(
          readFileSync(config.macos.entitlements, "utf8"),
        )))
  )
    throw new CliError(
      "The updater extension targets unsandboxed apps; omit its Go import from App Store/sandboxed builds",
    );
  chmodSync(executablePath, 0o755);

  let iconFile: string | undefined;
  let generatedIcns: Uint8Array | undefined;
  if (config.macos.icon) {
    if (extname(config.macos.icon).toLowerCase() !== ".icns") {
      throw new CliError("macos.icon must point to an .icns file");
    }
    iconFile = "AppIcon.icns";
  } else if (config.icon && options.mode === "production") {
    // Icon containers are regenerated per build, so development reloads skip the resize pass.
    generatedIcns = (await resolveIcons(config))?.icns;
    if (generatedIcns) iconFile = "AppIcon.icns";
  }
  const reservedResources = new Set<string>([
    ...(iconFile ? [iconFile] : []),
    ...(fonts.length > 0 ? ["fonts"] : []),
  ]);
  stageApplicationResources(config.resourceDir, config.resources, resources, reservedResources);
  if (config.macos.icon && iconFile) {
    cpSync(config.macos.icon, resolve(resources, iconFile));
  } else if (generatedIcns && iconFile) {
    writeFileSync(resolve(resources, iconFile), generatedIcns);
  }
  writeFileSync(
    resolve(contents, "Info.plist"),
    macInfoPlist({
      name: config.name,
      displayName,
      executableName: config.executableName,
      identifier,
      version: config.version,
      buildVersion: config.buildVersion,
      minimumSystemVersion: config.macos.minimumSystemVersion,
      category: config.macos.category,
      urlSchemes: config.protocols,
      documentTypes: config.documentTypes,
      ...(sparkle && config.updates?.publicKey
        ? { updater: updaterMetadata(config, options.target, options.mode) }
        : {}),
      ...(iconFile ? { iconFile } : {}),
    }),
  );
  writeFileSync(resolve(contents, "PkgInfo"), "APPL????");

  if (options.macAppStore) {
    const packagePath = await buildMacAppStorePackage(config, appPath, stagingRoot);
    return {
      artifactPath: appPath,
      executablePath,
      target: options.target,
      mode: options.mode,
      extraArtifacts: [packagePath],
      packagedArtifacts: [packagePath],
    };
  }

  if (sparkle) {
    for (const part of [
      join(sparkle, "Versions/B/Autoupdate"),
      join(sparkle, "Versions/B/Updater.app"),
      sparkle,
    ]) {
      await run(
        [
          "codesign",
          "--force",
          ...(identity === "-" ? [] : ["--options", "runtime", "--timestamp"]),
          "--sign",
          identity,
          part,
        ],
        config.projectRoot,
      );
    }
  }
  const signArguments = ["codesign", "--force", "--deep"];
  if (options.mode === "production" && identity !== "-") {
    signArguments.push("--options", "runtime", "--timestamp");
  }
  signArguments.push("--sign", identity);
  if (config.macos.entitlements) {
    signArguments.push("--entitlements", config.macos.entitlements);
  } else if (config.language === "typescript") {
    const entitlements = resolve(stagingRoot, "quickgui-bun.entitlements");
    writeFileSync(entitlements, bunEntitlements);
    signArguments.push("--entitlements", entitlements);
  }
  signArguments.push(appPath);
  await run(signArguments, config.projectRoot);
  await run(["codesign", "--verify", "--deep", "--strict", appPath], config.projectRoot);

  const dmgPath =
    options.mode === "production"
      ? await buildMacDmg(config, appPath, stagingRoot, identity, notarization)
      : undefined;

  return {
    artifactPath: appPath,
    executablePath,
    target: options.target,
    mode: options.mode,
    ...(dmgPath ? { dmgPath } : {}),
  };
}

// Bun's JavaScriptCore JIT and FFI trampolines need executable memory when re-signed.
// Keep library validation enabled: our shared libraries are signed with the app's identity.
export const bunEntitlements = `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>com.apple.security.cs.allow-jit</key><true/>
<key>com.apple.security.cs.allow-unsigned-executable-memory</key><true/>
</dict></plist>
`;

/**
 * Sign a bundle with the Mac App Store identity and wrap it in a signed installer package.
 *
 * The provisioning profile is embedded before signing because `codesign` seals
 * `Contents/embedded.provisionprofile` into the bundle signature.
 */
async function buildMacAppStorePackage(
  config: ResolvedQuickGuiConfig,
  appPath: string,
  stagingRoot: string,
): Promise<string> {
  const appStore = config.macos.appStore;
  if (!appStore) {
    throw new CliError(
      "`quickgui build --mas` needs `macos.appStore` with applicationIdentity, installerIdentity, " +
        "and provisioningProfile.",
    );
  }
  validateMasConfig(appStore);
  if (!existsSync(appStore.provisioningProfile)) {
    throw new CliError(`Provisioning profile not found: ${appStore.provisioningProfile}`);
  }
  cpSync(appStore.provisioningProfile, resolve(appPath, "Contents", "embedded.provisionprofile"));
  let entitlements = appStore.entitlements;
  if (!entitlements) {
    entitlements = resolve(stagingRoot, "quickgui-mas.entitlements");
    writeFileSync(
      entitlements,
      masEntitlementsTemplate(config.macos.teamIdentifier, config.identifier),
    );
  } else if (!existsSync(entitlements)) {
    throw new CliError(`Mac App Store entitlements not found: ${entitlements}`);
  }
  await run(
    masCodesignArguments(appPath, appStore.applicationIdentity, entitlements),
    config.projectRoot,
  );
  await run(["codesign", "--verify", "--deep", "--strict", appPath], config.projectRoot);
  const packagePath = resolve(stagingRoot, masPackageFilename(config.name, config.version));
  await run(
    productBuildArguments(appPath, appStore.installerIdentity, packagePath),
    config.projectRoot,
  );
  if (!existsSync(packagePath)) {
    throw new CliError(`productbuild did not produce the expected package: ${packagePath}`);
  }
  return packagePath;
}

async function buildMacDmg(
  config: ResolvedQuickGuiConfig,
  appPath: string,
  stagingRoot: string,
  identity: string,
  notarization?: MacOSNotarizationConfig,
): Promise<string> {
  const dmgPath = resolve(stagingRoot, macDmgFilename(config.name, config.version));
  const dmgTitle = config.macos.dmgTitle ?? config.name;
  const appFileName = basename(appPath);
  const volumeIcon = join(appPath, "Contents", "Resources", "AppIcon.icns");
  // create-dmg copies this folder, then adds the Applications drop link itself.
  const imageRoot = mkdtempSync(join(stagingRoot, ".dmg-"));
  cpSync(appPath, join(imageRoot, appFileName), { recursive: true, verbatimSymlinks: true });
  try {
    console.log(`[quickgui] Creating ${basename(dmgPath)}`);
    await run(
      createDmgArguments({
        dmgPath,
        sourceFolder: imageRoot,
        volumeName: dmgTitle,
        appFileName,
        ...(existsSync(volumeIcon) ? { volumeIcon } : {}),
      }),
      config.projectRoot,
    );
  } finally {
    rmSync(imageRoot, { recursive: true, force: true });
  }
  if (!existsSync(dmgPath) || !statSync(dmgPath).isFile()) {
    throw new CliError(`create-dmg did not produce the expected disk image: ${dmgPath}`);
  }

  if (identity !== "-") {
    await run(
      ["codesign", "--force", "--timestamp", "--sign", identity, dmgPath],
      config.projectRoot,
    );
    await run(["codesign", "--verify", "--strict", dmgPath], config.projectRoot);
  }

  if (notarization) {
    console.log(`[quickgui] Notarizing ${basename(dmgPath)}`);
    await run(macNotarytoolArguments(dmgPath, notarization), config.projectRoot);
    await run(["xcrun", "stapler", "staple", dmgPath], config.projectRoot);
    await run(["xcrun", "stapler", "validate", dmgPath], config.projectRoot);
  }

  return dmgPath;
}

export function macDmgFilename(name: string, version: string): string {
  const filename = `${name} ${version}.dmg`;
  if (filename.includes("\0") || basename(filename) !== filename) {
    throw new CliError("Application name and version cannot contain path separators on macOS");
  }
  return filename;
}

export function macNotarytoolArguments(
  dmgPath: string,
  notarization: MacOSNotarizationConfig,
): string[] {
  return [
    "xcrun",
    "notarytool",
    "submit",
    dmgPath,
    "--keychain-profile",
    notarization.keychainProfile,
    ...(notarization.keychain ? ["--keychain", notarization.keychain] : []),
    "--wait",
  ];
}

async function buildExecutable(
  config: ResolvedQuickGuiConfig,
  options: BuildProjectOptions,
  stagingRoot: string,
): Promise<StagedBuild> {
  const info = targetInfo(options.target);
  const suffix = info.platform === "windows" ? ".exe" : "";
  const executablePath = resolve(stagingRoot, `${config.executableName}${suffix}`);
  const fonts = stageFonts(config, resolve(stagingRoot, "fonts"));
  const libraries = await compileExecutable(config, options, executablePath, fonts);
  if (info.platform !== "windows") chmodSync(executablePath, 0o755);
  const payload = stageExecutableSidecars(config, stagingRoot, libraries, info.platform);
  const result: StagedBuild = {
    artifactPath: executablePath,
    extraArtifacts: payload,
    executablePath,
    target: options.target,
    mode: options.mode,
  };
  if (options.mode !== "production") return result;

  const icons = await resolveIcons(config, iconSource(config, info.platform));
  const packaged =
    info.platform === "linux"
      ? await packageLinux({
          config,
          libraries: payload,
          target: options.target,
          executablePath,
          stagingRoot,
          run: (command, cwd) => run(command, cwd),
          ...(icons ? { icons } : {}),
        })
      : await packageWindows({
          config,
          libraries: payload,
          executablePath,
          stagingRoot,
          run: (command, cwd) => run(command, cwd),
          ...(icons ? { icons } : {}),
        });
  return {
    ...result,
    extraArtifacts: [...payload, ...packaged.artifacts],
    packagedArtifacts: packaged.artifacts,
    ...(packaged.notes.length > 0 ? { notes: packaged.notes } : {}),
  };
}

async function compileExecutable(
  config: ResolvedQuickGuiConfig,
  options: BuildProjectOptions,
  executablePath: string,
  fonts: string[],
): Promise<string[]> {
  const started = performance.now();
  const libraries = await compileNativeApplication({
    config,
    mode: options.mode,
    target: options.target,
    executablePath,
    fonts,
  });
  console.log(
    `[quickgui] Compiled ${basename(executablePath)} in ${Math.round(performance.now() - started)} ms`,
  );
  return libraries;
}

/** Copy the configured fonts beside the executable and return their resource-relative names. */
function stageFonts(config: ResolvedQuickGuiConfig, destination: string): string[] {
  if (config.fonts.length === 0) return [];
  mkdirSync(destination, { recursive: true });
  const names: string[] = [];
  for (const font of config.fonts) {
    const name = basename(font);
    if (names.includes(`fonts/${name}`)) throw new CliError(`Duplicate font file name: ${name}`);
    cpSync(font, resolve(destination, name));
    names.push(`fonts/${name}`);
  }
  return names;
}

function replaceArtifacts(
  artifacts: Array<{ stagedPath: string; finalPath: string }>,
  stagingRoot: string,
): void {
  const backups: Array<{ finalPath: string; backupPath: string }> = [];
  const installed: Array<{ stagedPath: string; finalPath: string }> = [];
  try {
    for (const [index, artifact] of artifacts.entries()) {
      if (existsSync(artifact.finalPath)) {
        const backupPath = resolve(stagingRoot, `.quickgui-previous-artifact-${index}`);
        renameSync(artifact.finalPath, backupPath);
        backups.push({ finalPath: artifact.finalPath, backupPath });
      }
      renameSync(artifact.stagedPath, artifact.finalPath);
      installed.push(artifact);
    }
  } catch (error) {
    for (const artifact of installed.reverse()) {
      renameSync(artifact.finalPath, artifact.stagedPath);
    }
    for (const backup of backups.reverse()) {
      renameSync(backup.backupPath, backup.finalPath);
    }
    throw error;
  }
  for (const backup of backups) {
    rmSync(backup.backupPath, { recursive: true, force: true });
  }
}

/**
 * What one build publishes: installers and update archives as versioned artifacts, then the
 * files that always describe the newest release. Desktop entries, AppDirs, and NSIS scripts stay
 * local.
 */
export function releaseFiles(
  produced: readonly string[],
  manifestPath: string,
): { artifacts: string[]; pointers: string[] } {
  const artifact = /\.(?:AppImage|tar\.gz|deb|dmg|pkg|zip|exe)$/;
  const pointer = new Set(["install.sh", LATEST_LINUX_VERSION_FILE]);
  return {
    artifacts: [...new Set(produced.filter((path) => artifact.test(path)))],
    pointers: [...produced.filter((path) => pointer.has(basename(path))), manifestPath],
  };
}

/** The artifact the updater installs for this target, chosen from what the build produced. */
export function updateSource(
  options: Pick<BuildProjectOptions, "target">,
  artifactPath: string,
  packagePaths: readonly string[],
): string {
  const platform = targetInfo(options.target).platform;
  if (platform === "darwin") return artifactPath;
  if (platform === "windows") {
    const installer = packagePaths.find((path) => path.endsWith("-setup.exe"));
    if (!installer) {
      throw new CliError(
        "A Windows update artifact must be an .exe or .msi installer, but `makensis` did not " +
          "produce one. Install NSIS and re-run, or publish the update from a host that has it.",
      );
    }
    return installer;
  }
  const appImage = packagePaths.find((path) => path.endsWith(".AppImage"));
  return appImage ?? linuxTarball(packagePaths) ?? artifactPath;
}

/** Further artifacts published in the same appcast item as {@link updateSource}. */
export function updateAlternates(
  options: Pick<BuildProjectOptions, "target">,
  packagePaths: readonly string[],
): string[] {
  if (targetInfo(options.target).platform !== "linux") return [];
  const tarball = linuxTarball(packagePaths);
  return tarball && packagePaths.some((path) => path.endsWith(".AppImage")) ? [tarball] : [];
}

function linuxTarball(packagePaths: readonly string[]): string | undefined {
  return packagePaths.find((path) => path.endsWith(".tar.gz"));
}

function iconSource(
  config: ResolvedQuickGuiConfig,
  platform: "darwin" | "linux" | "windows",
): string | undefined {
  if (config.icon) return config.icon;
  return platform === "linux" ? config.linux.icon : undefined;
}

async function resolveIcons(
  config: ResolvedQuickGuiConfig,
  source = config.icon,
): Promise<IconBuildResult | undefined> {
  if (!source) return undefined;
  return await buildIcons(source);
}

/**
 * Names written into the Linux/Windows staging root after resources are copied.
 *
 * A configured resource whose basename matches one of these would be overwritten by icon
 * generation, the NSIS script, AppDir, or an installer, or would collide with `quickgui.json`.
 */
export function reservedSidecarNames(
  config: ResolvedQuickGuiConfig,
  platform: "darwin" | "linux" | "windows",
): string[] {
  const names = ["fonts", "quickgui.json", ".quickgui-icons", "release-notes.md"];
  if (platform === "linux") {
    const packageName = debianPackageName(config.executableName);
    names.push(
      `${config.executableName}.desktop`,
      `${config.executableName}.mime.xml`,
      `${config.executableName}.AppDir`,
      `${config.executableName}-${config.version}-x86_64.AppImage`,
      `${config.executableName}-${config.version}-aarch64.AppImage`,
      `${packageName}_${config.version}_amd64.deb`,
      `${packageName}_${config.version}_arm64.deb`,
      tarballName(config.executableName, config.version, "linux-x64"),
      tarballName(config.executableName, config.version, "linux-arm64"),
      "install.sh",
      LATEST_LINUX_VERSION_FILE,
    );
  }
  if (platform === "windows") {
    names.push(
      `${config.executableName}.ico`,
      `${config.executableName}.nsi`,
      `${config.executableName}-${config.version}-setup.exe`,
    );
  }
  return names;
}

/** Files and directories that must travel with the executable on every host. */
export function stageExecutableSidecars(
  config: ResolvedQuickGuiConfig,
  stagingRoot: string,
  alreadyStaged: readonly string[],
  platform: "darwin" | "linux" | "windows",
): string[] {
  const reserved = new Set([
    config.executableName,
    `${config.executableName}.exe`,
    ...reservedSidecarNames(config, platform),
    ...alreadyStaged.map((path) => basename(path)),
  ]);
  const staged = [...alreadyStaged];
  const names = new Set(staged.map((path) => basename(path)));
  const fontsDir = resolve(stagingRoot, "fonts");
  if (existsSync(fontsDir) && !names.has("fonts")) {
    staged.push(fontsDir);
    names.add("fonts");
  }
  staged.push(
    ...stageApplicationResources(config.resourceDir, config.resources, stagingRoot, reserved),
  );
  return staged;
}

function validateMacPackaging(config: ResolvedQuickGuiConfig, options: BuildProjectOptions): void {
  macDmgFilename(config.name, config.version);
  const dmgTitle = config.macos.dmgTitle ?? config.name;
  if (dmgTitle.length > 27) {
    throw new CliError(
      "The macOS DMG title cannot exceed 27 characters; set macos.dmgTitle to a shorter title",
    );
  }
  const identity = options.signingIdentity ?? config.macos.signingIdentity ?? "-";
  const notarization = options.notarization ?? config.macos.notarization;
  if (notarization && identity === "-") {
    throw new CliError(
      "macOS notarization requires a Developer ID signing identity; configure macos.signingIdentity or pass --sign",
    );
  }
}

function requireExistingFile(path: string, label: string): void {
  if (!existsSync(path) || !statSync(path).isFile()) {
    throw new CliError(`${label} not found: ${path}`);
  }
}

export function validateBuildInputs(
  config: ResolvedQuickGuiConfig,
  platform: "darwin" | "linux" | "windows",
): void {
  if (config.language === "rust") requireRustManifest(config);
  if (!existsSync(config.entry)) {
    throw new CliError(
      `${languageLabel(config.language)} application package not found: ${config.entry}`,
    );
  }
  if (
    config.resourceDir &&
    (!existsSync(config.resourceDir) || !statSync(config.resourceDir).isDirectory())
  ) {
    throw new CliError(`Resource directory not found: ${config.resourceDir}`);
  }
  for (const resource of config.resources) {
    if (!existsSync(resource)) throw new CliError(`Resource not found: ${resource}`);
  }
  for (const font of config.fonts) {
    if (!existsSync(font) || !statSync(font).isFile()) {
      throw new CliError(`Font file not found: ${font}`);
    }
  }
  if (platform === "darwin") {
    if (config.macos.icon) requireExistingFile(config.macos.icon, "macOS icon");
    if (config.macos.entitlements && !existsSync(config.macos.entitlements)) {
      throw new CliError(`macOS entitlements not found: ${config.macos.entitlements}`);
    }
  }
  if (platform === "windows" && config.windows.icon) {
    requireExistingFile(config.windows.icon, "Windows icon");
  }
  if (platform === "linux" && config.linux.icon) {
    requireExistingFile(config.linux.icon, "Linux icon");
  }
  if (config.icon) requireExistingFile(config.icon, "Icon");
  if (config.updates?.changelog && !existsSync(config.updates.changelog)) {
    throw new CliError(`Changelog not found: ${config.updates.changelog}`);
  }
}

async function run(command: string[], cwd: string): Promise<void> {
  const child = Bun.spawn(command, {
    cwd,
    stdin: "ignore",
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
    throw new CliError(`Command failed: ${command.join(" ")}${detail ? `\n${detail}` : ""}`);
  }
}

function windowsVersion(version: string): string {
  const parts = version
    .split(".")
    .slice(0, 4)
    .map((part) => (/^\d+$/.test(part) ? Number(part) : 0));
  while (parts.length < 4) parts.push(0);
  return parts.map((part) => Math.max(0, Math.min(65_535, part))).join(".");
}

interface MacInfoPlistOptions {
  name: string;
  displayName: string;
  executableName: string;
  identifier: string;
  version: string;
  buildVersion: string;
  minimumSystemVersion: string;
  category: string;
  urlSchemes?: readonly string[];
  documentTypes?: readonly ResolvedDocumentType[];
  iconFile?: string;
  updater?: ReturnType<typeof updaterMetadata>;
}

export function macInfoPlist(options: MacInfoPlistOptions): string {
  const updater = options.updater
    ? `
  <key>SUFeedURL</key><string>${xml(options.updater.feedUrl)}</string>
  <key>SUPublicEDKey</key><string>${xml(options.updater.publicKey)}</string>
  <key>SUEnableAutomaticChecks</key><${options.updater.automaticChecks ? "true" : "false"}/>
  <key>SUAutomaticallyUpdate</key><false/>
  <key>SUAllowsAutomaticUpdates</key><false/>
  <key>SUVerifyUpdateBeforeExtraction</key><true/>
  <key>SUEnableJavaScript</key><false/>
  <key>SUEnableSystemProfiling</key><false/>`
    : "";
  const icon = options.iconFile
    ? `\n  <key>CFBundleIconFile</key>\n  <string>${xml(options.iconFile)}</string>`
    : "";
  const urlTypes = options.urlSchemes?.length
    ? `
  <key>CFBundleURLTypes</key>
  <array>
    <dict>
      <key>CFBundleTypeRole</key>
      <string>Editor</string>
      <key>CFBundleURLName</key>
      <string>${xml(options.identifier)}</string>
      <key>CFBundleURLSchemes</key>
      <array>${options.urlSchemes
        .map((scheme) => `\n        <string>${xml(scheme)}</string>`)
        .join("")}
      </array>
    </dict>
  </array>`
    : "";
  const documents = options.documentTypes?.length
    ? macDocumentTypesPlist(options.documentTypes) + macTypeDeclarationsPlist(options.documentTypes)
    : "";
  return `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>en</string>
  <key>CFBundleDisplayName</key>
  <string>${xml(options.displayName)}</string>
  <key>CFBundleExecutable</key>
  <string>${xml(options.executableName)}</string>${icon}
  <key>CFBundleIdentifier</key>
  <string>${xml(options.identifier)}</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundleName</key>
  <string>${xml(options.name)}</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>${xml(options.version)}</string>
  <key>CFBundleVersion</key>
  <string>${xml(options.buildVersion)}</string>${urlTypes}${documents}${updater}
  <key>LSApplicationCategoryType</key>
  <string>${xml(options.category)}</string>
  <key>LSMinimumSystemVersion</key>
  <string>${xml(options.minimumSystemVersion)}</string>
  <key>NSHighResolutionCapable</key>
  <true/>
</dict>
</plist>
`;
}

function xml(value: string): string {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&apos;");
}

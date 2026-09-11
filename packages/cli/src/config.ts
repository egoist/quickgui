import { existsSync } from "node:fs";
import { extname, isAbsolute, resolve } from "node:path";
import { pathToFileURL } from "node:url";

import { CliError } from "./error.ts";
import {
  MAX_DOCUMENT_TYPES,
  MAX_DOCUMENT_TYPE_EXTENSIONS,
  type ResolvedDocumentType,
} from "./packaging/documents.ts";
import { parseTarget, type QuickGuiTarget } from "./targets.ts";

export type { QuickGuiTarget } from "./targets.ts";

export interface MacOSNotarizationConfig {
  /** Profile created with `xcrun notarytool store-credentials`. */
  keychainProfile: string;
  /** Optional non-default Keychain containing the profile. */
  keychain?: string;
}

export interface MacOSConfig {
  minimumSystemVersion?: string;
  category?: string;
  icon?: string;
  signingIdentity?: string;
  entitlements?: string;
  /** Mounted disk image title, up to 27 characters. */
  dmgTitle?: string;
  /** Submit the production DMG to Apple's notary service and staple its ticket. */
  notarization?: MacOSNotarizationConfig;
  /** Mac App Store submission inputs, used by `quickgui build --mas`. */
  appStore?: MacAppStoreConfig;
  /** Apple Team ID used by the generated App Sandbox entitlements template. */
  teamIdentifier?: string;
}

export interface WindowsConfig {
  icon?: string;
  publisher?: string;
  description?: string;
  copyright?: string;
  hideConsole?: boolean;
  /** NSIS installer layout. QuickGUI generates the script and runs `makensis` when available. */
  nsis?: WindowsNsisConfig;
  /** Authenticode signing, executed only when the build host is Windows. */
  signing?: WindowsSigningConfig;
}

/** One document type the packaged application declares to the operating system. */
export interface DocumentTypeConfig {
  /** Human-readable name, e.g. "QuickGUI Project". */
  name: string;
  /** Filename extensions without a leading dot. At most 64 per type. */
  extensions: string[];
  /** macOS `CFBundleTypeRole`. Defaults to "Editor". */
  role?: "Editor" | "Viewer";
  /** MIME types used by the Linux desktop entry and shared-mime-info package. */
  mimeTypes?: string[];
  /** Icon basename inside the application resources. */
  icon?: string;
  /** Uniform Type Identifier, e.g. "com.example.app.project". */
  utTypeIdentifier?: string;
  /** UTIs this type conforms to. Defaults to `["public.data"]`. */
  conformsTo?: string[];
  /** Declare the UTI as exported (owned by this app) instead of imported. Defaults to true. */
  exported?: boolean;
  /** Longer description used by the Linux shared-mime-info package. */
  description?: string;
}

/** Signed Sparkle-compatible appcasts for the optional updater extension. */
export interface UpdatesConfig {
  /** Appcast URL; {target} expands to e.g. darwin-arm64. Defaults to baseUrl/appcast-{target}.xml. */
  feedUrl?: string;
  /** Base64 raw Ed25519 public key shared by Sparkle and portable backends. */
  publicKey?: string;
  /** Default preference; user choices persist across launches. */
  automaticChecks?: boolean;
  /** Base64 Sparkle secret-key file, overridden by QUICKGUI_UPDATER_PRIVATE_KEY. */
  ed25519SecretKey?: string;
  /** Write a signed appcast during build. Legacy Minisign config still writes latest.json. */
  manifest?: boolean;
  /** Public base URL the artifact is published under, e.g. "https://dl.example.com/app". */
  baseUrl: string;
  /** Minisign secret key path. `QUICKGUI_MINISIGN_SECRET_KEY` overrides it. */
  minisignSecretKey?: string;
  /** Release-notes file copied into the manifest `notes` field. */
  notesFile?: string;
}

export interface LinuxConfig {
  /** Square PNG used for the desktop icon set. Falls back to the top-level `icon`. */
  icon?: string;
  /** Freedesktop main categories. Defaults to `["Utility"]`. */
  categories?: string[];
  /** `Comment=` line in the generated desktop entry. */
  comment?: string;
  /** Debian `Maintainer:` field. Required to build a `.deb`. */
  maintainer?: string;
  /** Debian `Section:` field. Defaults to "utils". */
  section?: string;
  /** Debian `Depends:` field entries. */
  depends?: string[];
  /** Build an AppDir and, when `appimagetool` is on PATH, an AppImage. Defaults to true. */
  appImage?: boolean;
  /** Build a `.deb` package. Defaults to true when `linux.maintainer` is set. */
  deb?: boolean;
}

/** Authenticode signing, executed only when building on Windows. */
export interface WindowsSigningConfig {
  /** PFX/P12 certificate path passed to `signtool /f`. */
  certificateFile?: string;
  /** Installed certificate subject name passed to `signtool /n`. */
  subjectName?: string;
  /** Environment variable holding the certificate password. */
  passwordEnvironmentVariable?: string;
  /** RFC 3161 timestamp URL. Defaults to DigiCert's public timestamp service. */
  timestampUrl?: string;
  /** File digest algorithm. Defaults to "sha256". */
  digest?: "sha256" | "sha384" | "sha512";
}

export interface WindowsNsisConfig {
  /** Default install directory expression. Defaults to `$LOCALAPPDATA\\<name>`. */
  installDirectory?: string;
  /** Create a desktop shortcut. Defaults to true. */
  createDesktopShortcut?: boolean;
  /** Create a Start Menu shortcut. Defaults to true. */
  createStartMenuShortcut?: boolean;
  /** Install for all users (writes to HKLM and Program Files). Defaults to false. */
  perMachine?: boolean;
}

/** Mac App Store submission inputs. */
export interface MacAppStoreConfig {
  /** `3rd Party Mac Developer Application: …` identity. */
  applicationIdentity: string;
  /** `3rd Party Mac Developer Installer: …` identity. */
  installerIdentity: string;
  /** `.provisionprofile` embedded as `Contents/embedded.provisionprofile`. */
  provisioningProfile: string;
  /** Sandbox entitlements. QuickGUI writes a default template when omitted. */
  entitlements?: string;
}

export type Language = "go" | "rust" | "typescript";
/** @deprecated Use {@link Language}. */
export type Frontend = Language;

export function parseLanguage(value: string): Language {
  if (value === "go" || value === "rust" || value === "typescript") return value;
  throw new CliError(
    `Unknown language ${JSON.stringify(value)}; expected go, rust, or typescript`,
  );
}

/** @deprecated Use {@link parseLanguage}. */
export function parseFrontend(value: string): Language {
  return parseLanguage(value);
}

export function languageLabel(language: Language): string {
  switch (language) {
    case "rust":
      return "Rust";
    case "typescript":
      return "TypeScript";
    default:
      return "Go";
  }
}

/** Application compilation and native shared-library options. */
export interface NativeConfig {
  /** Use an existing Rust shared library instead of the installed native package. */
  libraryPath?: string;
  /** Optional Go build tags. */
  tags?: string[];
}

export interface QuickGuiConfig {
  /** Application language. Existing projects default to Go. */
  language?: Language;
  /** @deprecated Use {@link language}. Still accepted in existing projects. */
  frontend?: Language;
  name: string;
  identifier: string;
  version?: string;
  buildVersion?: string;
  entry?: string;
  outDir?: string;
  target?: QuickGuiTarget;
  /** Native compilation options. */
  native?: NativeConfig;
  /** TypeScript extensions: npm packages, directories, or terminal/updater names. */
  extensions?: string[];
  resources?: string[];
  /** OpenType font files embedded in the executable and registered before app startup. */
  fonts?: string[];
  /** Custom URL schemes. Packaged macOS apps declare these in their signed Info.plist. */
  protocols?: string[];
  /** Square source PNG (>= 256x256) used to generate `.icns`, `.ico`, and Linux icon sizes. */
  icon?: string;
  /** File associations declared to every packaging backend. */
  documentTypes?: DocumentTypeConfig[];
  /** Signed updater manifest generation for `quickgui build --update-manifest`. */
  updates?: UpdatesConfig;
  macos?: MacOSConfig;
  windows?: WindowsConfig;
  linux?: LinuxConfig;
}

export interface ResolvedQuickGuiConfig {
  language: Language;
  name: string;
  executableName: string;
  identifier: string;
  version: string;
  buildVersion: string;
  entry: string;
  outDir: string;
  target?: QuickGuiTarget;
  native: { libraryPath?: string; tags: string[] };
  extensions: string[];
  resources: string[];
  fonts: string[];
  protocols: string[];
  icon?: string;
  documentTypes: ResolvedDocumentType[];
  updates?: Required<Pick<UpdatesConfig, "manifest" | "baseUrl">> & UpdatesConfig;
  macos: Required<Pick<MacOSConfig, "minimumSystemVersion" | "category">> & MacOSConfig;
  windows: Required<Pick<WindowsConfig, "hideConsole">> & WindowsConfig;
  linux: Required<Pick<LinuxConfig, "categories" | "section" | "depends" | "appImage" | "deb">> &
    LinuxConfig;
  projectRoot: string;
  configPath: string;
}

export function defineConfig(config: QuickGuiConfig): QuickGuiConfig {
  return config;
}

export async function loadConfig(
  projectRoot: string,
  configFile?: string,
): Promise<ResolvedQuickGuiConfig> {
  const root = resolve(projectRoot);
  const selected =
    configFile ??
    ["quickgui.toml", "quickgui.config.ts"].find((name) => existsSync(resolve(root, name)));
  if (selected === undefined) {
    throw new CliError(
      `QuickGUI config not found in ${root}: expected quickgui.toml or quickgui.config.ts`,
    );
  }
  const configPath = isAbsolute(selected) ? selected : resolve(root, selected);
  if (!existsSync(configPath)) {
    throw new CliError(`QuickGUI config not found: ${configPath}`);
  }
  let input: unknown;
  try {
    if (extname(configPath).toLowerCase() === ".toml") {
      // Read on each reload rather than using Bun's cached TOML module imports.
      input = Bun.TOML.parse(await Bun.file(configPath).text());
    } else {
      const url = pathToFileURL(configPath);
      url.searchParams.set("quickgui_reload", `${Date.now()}_${Math.random()}`);
      const module = (await import(url.href)) as { default?: unknown };
      input = module.default;
    }
  } catch (error) {
    throw new CliError(`Could not load ${configPath}`, { cause: error });
  }
  return resolveConfig(input, root, configPath);
}

export function resolveConfig(
  input: unknown,
  projectRoot: string,
  configPath = resolve(projectRoot, "quickgui.config.ts"),
): ResolvedQuickGuiConfig {
  if (!isRecord(input)) throw new CliError("QuickGUI config must be an object");
  const name = requiredString(input.name, "name", 128);
  const identifier = requiredString(input.identifier, "identifier", 255);
  if (!/^[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)+$/.test(identifier)) {
    throw new CliError(`Invalid application identifier \`${identifier}\``);
  }
  const version = optionalString(input.version, "version", 64) ?? "0.1.0";
  const buildVersion = optionalString(input.buildVersion, "buildVersion", 64) ?? version;
  const language = resolveLanguage(input);
  const entry = resolveRelative(
    projectRoot,
    optionalString(input.entry, "entry", 1_024) ?? (language === "typescript" ? "app.tsx" : "."),
  );
  const outDir = resolveRelative(
    projectRoot,
    optionalString(input.outDir, "outDir", 1_024) ?? "dist",
  );
  const target =
    input.target === undefined
      ? undefined
      : parseTarget(requiredString(input.target, "target", 64));
  const resources = stringArray(input.resources, "resources").map((path) =>
    resolveRelative(projectRoot, path),
  );
  const fonts = stringArray(input.fonts, "fonts").map((path) => resolveRelative(projectRoot, path));
  const protocols = protocolArray(input.protocols);
  const macos = objectOrEmpty(input.macos, "macos");
  const windows = objectOrEmpty(input.windows, "windows");
  const linux = objectOrEmpty(input.linux, "linux");
  const sourceIcon = optionalString(input.icon, "icon", 1_024);
  const documentTypes = resolveDocumentTypes(input.documentTypes);
  const updates = resolveUpdates(input.updates, projectRoot);
  const native = objectOrEmpty(input.native, "native");
  if (native.extensions !== undefined)
    throw new CliError("Use top-level extensions instead of native.extensions");
  if (language === "go" && input.extensions !== undefined)
    throw new CliError("extensions is for TypeScript; Go extensions are discovered from imports");
  if (language === "rust" && input.extensions !== undefined)
    throw new CliError("extensions is for TypeScript; Rust enables crate features in Cargo.toml");
  if (language === "typescript" && native.tags !== undefined)
    throw new CliError("native.tags contains Go build tags and is not supported by TypeScript");
  if (language === "rust" && native.tags !== undefined)
    throw new CliError("native.tags contains Go build tags; Rust uses Cargo features");
  if (language === "rust" && native.libraryPath !== undefined) {
    throw new CliError(
      "native.libraryPath is for Go and TypeScript shared libraries; Rust links the quickgui crate",
    );
  }
  const linuxIcon = optionalString(linux.icon, "linux.icon", 1_024);
  const linuxMaintainer = optionalString(linux.maintainer, "linux.maintainer", 255);
  const linuxComment = optionalString(linux.comment, "linux.comment", 512);
  const appStore = resolveMacAppStore(macos.appStore, projectRoot);
  const nsis = resolveWindowsNsis(windows.nsis);
  const windowsSigning = resolveWindowsSigning(windows.signing, projectRoot);
  const icon = optionalString(macos.icon, "macos.icon", 1_024);
  const entitlements = optionalString(macos.entitlements, "macos.entitlements", 1_024);
  const notarization = resolveMacOSNotarization(macos.notarization, projectRoot);
  const windowsIcon = optionalString(windows.icon, "windows.icon", 1_024);

  return {
    language,
    name,
    executableName: executableName(name),
    identifier,
    version,
    buildVersion,
    entry,
    outDir,
    ...(target ? { target } : {}),
    native: {
      ...(optionalString(native.libraryPath, "native.libraryPath", 1024)
        ? { libraryPath: resolveRelative(projectRoot, String(native.libraryPath)) }
        : {}),
      tags: stringArray(native.tags, "native.tags"),
    },
    extensions: stringArray(input.extensions, "extensions").map((path) =>
      path === "terminal" ||
      path === "updater" ||
      path.startsWith("@") ||
      path.startsWith("quickgui-extension-")
        ? path
        : resolveRelative(projectRoot, path),
    ),
    resources,
    fonts,
    protocols,
    ...(sourceIcon ? { icon: resolveRelative(projectRoot, sourceIcon) } : {}),
    documentTypes,
    ...(updates ? { updates } : {}),
    macos: {
      minimumSystemVersion:
        optionalString(macos.minimumSystemVersion, "macos.minimumSystemVersion", 32) ?? "14.0",
      category:
        optionalString(macos.category, "macos.category", 255) ??
        "public.app-category.developer-tools",
      ...(icon ? { icon: resolveRelative(projectRoot, icon) } : {}),
      ...(optionalString(macos.signingIdentity, "macos.signingIdentity", 512)
        ? { signingIdentity: String(macos.signingIdentity) }
        : {}),
      ...(entitlements ? { entitlements: resolveRelative(projectRoot, entitlements) } : {}),
      ...(optionalString(macos.dmgTitle, "macos.dmgTitle", 27)
        ? { dmgTitle: String(macos.dmgTitle) }
        : {}),
      ...(notarization ? { notarization } : {}),
      ...(appStore ? { appStore } : {}),
      ...(optionalString(macos.teamIdentifier, "macos.teamIdentifier", 64)
        ? { teamIdentifier: String(macos.teamIdentifier) }
        : {}),
    },
    windows: {
      hideConsole: optionalBoolean(windows.hideConsole, "windows.hideConsole") ?? true,
      ...(windowsIcon ? { icon: resolveRelative(projectRoot, windowsIcon) } : {}),
      ...(optionalString(windows.publisher, "windows.publisher", 255)
        ? { publisher: String(windows.publisher) }
        : {}),
      ...(optionalString(windows.description, "windows.description", 512)
        ? { description: String(windows.description) }
        : {}),
      ...(optionalString(windows.copyright, "windows.copyright", 512)
        ? { copyright: String(windows.copyright) }
        : {}),
      ...(nsis ? { nsis } : {}),
      ...(windowsSigning ? { signing: windowsSigning } : {}),
    },
    linux: {
      categories: linuxCategories(linux.categories),
      section: optionalString(linux.section, "linux.section", 64) ?? "utils",
      depends: stringArray(linux.depends, "linux.depends"),
      appImage: optionalBoolean(linux.appImage, "linux.appImage") ?? true,
      deb: optionalBoolean(linux.deb, "linux.deb") ?? linuxMaintainer !== undefined,
      ...(linuxIcon ? { icon: resolveRelative(projectRoot, linuxIcon) } : {}),
      ...(linuxMaintainer ? { maintainer: linuxMaintainer } : {}),
      ...(linuxComment ? { comment: linuxComment } : {}),
    },
    projectRoot: resolve(projectRoot),
    configPath: resolve(configPath),
  };
}

function resolveLanguage(input: Record<string, unknown>): Language {
  const language = optionalString(input.language, "language", 32);
  const frontend = optionalString(input.frontend, "frontend", 32);
  if (language && frontend && language !== frontend) {
    throw new CliError("`language` and `frontend` must be the same value");
  }
  return parseLanguage(language ?? frontend ?? "go");
}

function resolveMacOSNotarization(
  value: unknown,
  projectRoot: string,
): MacOSNotarizationConfig | undefined {
  if (value === undefined) return undefined;
  const notarization = objectOrEmpty(value, "macos.notarization");
  const keychainProfile = requiredString(
    notarization.keychainProfile,
    "macos.notarization.keychainProfile",
    512,
  );
  const keychain = optionalString(notarization.keychain, "macos.notarization.keychain", 1_024);
  return {
    keychainProfile,
    ...(keychain ? { keychain: resolveRelative(projectRoot, keychain) } : {}),
  };
}

function resolveDocumentTypes(value: unknown): ResolvedDocumentType[] {
  if (value === undefined) return [];
  if (!Array.isArray(value) || value.length > MAX_DOCUMENT_TYPES) {
    throw new CliError(
      `\`documentTypes\` must be an array with at most ${MAX_DOCUMENT_TYPES} entries`,
    );
  }
  const seen = new Set<string>();
  return value.map((item, index) => {
    const field = `documentTypes[${index}]`;
    const type = objectOrEmpty(item, field);
    const name = requiredString(type.name, `${field}.name`, 255);
    const extensions = stringArray(type.extensions, `${field}.extensions`).map((extension) =>
      extension.replace(/^\.+/, "").toLowerCase(),
    );
    if (extensions.length === 0 || extensions.length > MAX_DOCUMENT_TYPE_EXTENSIONS) {
      throw new CliError(
        `\`${field}.extensions\` must list 1 to ${MAX_DOCUMENT_TYPE_EXTENSIONS} extensions`,
      );
    }
    for (const extension of extensions) {
      if (!/^[a-z0-9][a-z0-9+.-]*$/.test(extension)) {
        throw new CliError(`Invalid file extension \`${extension}\` in \`${field}.extensions\``);
      }
      if (seen.has(extension)) {
        throw new CliError(`Duplicate file extension \`${extension}\` in \`documentTypes\``);
      }
      seen.add(extension);
    }
    const role =
      type.role === undefined ? "Editor" : requiredString(type.role, `${field}.role`, 16);
    if (role !== "Editor" && role !== "Viewer") {
      throw new CliError(`\`${field}.role\` must be "Editor" or "Viewer"`);
    }
    const mimeTypes = stringArray(type.mimeTypes, `${field}.mimeTypes`);
    for (const mimeType of mimeTypes) {
      if (!/^[a-z0-9][a-z0-9!#$&^_.+-]*\/[a-z0-9][a-z0-9!#$&^_.+-]*$/i.test(mimeType)) {
        throw new CliError(`Invalid MIME type \`${mimeType}\` in \`${field}.mimeTypes\``);
      }
    }
    const utTypeIdentifier = optionalString(
      type.utTypeIdentifier,
      `${field}.utTypeIdentifier`,
      255,
    );
    if (utTypeIdentifier && !/^[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)+$/.test(utTypeIdentifier)) {
      throw new CliError(`Invalid UTI \`${utTypeIdentifier}\` in \`${field}.utTypeIdentifier\``);
    }
    const conformsTo = stringArray(type.conformsTo, `${field}.conformsTo`);
    const icon = optionalString(type.icon, `${field}.icon`, 255);
    const description = optionalString(type.description, `${field}.description`, 1_024);
    return {
      name,
      extensions,
      role,
      mimeTypes,
      conformsTo: conformsTo.length > 0 ? conformsTo : ["public.data"],
      exported: optionalBoolean(type.exported, `${field}.exported`) ?? true,
      ...(icon ? { icon } : {}),
      ...(utTypeIdentifier ? { utTypeIdentifier } : {}),
      ...(description ? { description } : {}),
    } satisfies ResolvedDocumentType;
  });
}

function resolveUpdates(
  value: unknown,
  projectRoot: string,
): (Required<Pick<UpdatesConfig, "manifest" | "baseUrl">> & UpdatesConfig) | undefined {
  if (value === undefined) return undefined;
  const updates = objectOrEmpty(value, "updates");
  const baseUrl = requiredString(updates.baseUrl, "updates.baseUrl", 2_048);
  if (!/^https:\/\/[^\s"']+$/.test(baseUrl)) {
    throw new CliError("`updates.baseUrl` must be an HTTPS URL");
  }
  const publicKey = optionalString(updates.publicKey, "updates.publicKey", 128);
  if (
    publicKey &&
    (Buffer.from(publicKey, "base64").length !== 32 ||
      Buffer.from(publicKey, "base64").toString("base64") !== publicKey)
  )
    throw new CliError("updates.publicKey must be a base64 32-byte Ed25519 public key");
  const feedUrl = optionalString(updates.feedUrl, "updates.feedUrl", 2048);
  if (feedUrl) {
    const url = new URL(feedUrl.replaceAll("{target}", "darwin-arm64"));
    if (url.protocol !== "https:" || url.username || url.password || url.hash)
      throw new CliError("updates.feedUrl must use HTTPS without credentials or fragments");
  }
  const ed25519SecretKey = optionalString(
    updates.ed25519SecretKey,
    "updates.ed25519SecretKey",
    1024,
  );
  const automaticChecks = optionalBoolean(updates.automaticChecks, "updates.automaticChecks");
  const secretKey = optionalString(updates.minisignSecretKey, "updates.minisignSecretKey", 1_024);
  const notesFile = optionalString(updates.notesFile, "updates.notesFile", 1_024);
  return {
    manifest: optionalBoolean(updates.manifest, "updates.manifest") ?? false,
    baseUrl: baseUrl.replace(/\/+$/, ""),
    ...(publicKey ? { publicKey } : {}),
    ...(feedUrl ? { feedUrl } : {}),
    ...(ed25519SecretKey
      ? { ed25519SecretKey: resolveRelative(projectRoot, ed25519SecretKey) }
      : {}),
    ...(automaticChecks === undefined ? {} : { automaticChecks }),
    ...(secretKey ? { minisignSecretKey: resolveRelative(projectRoot, secretKey) } : {}),
    ...(notesFile ? { notesFile: resolveRelative(projectRoot, notesFile) } : {}),
  };
}

function resolveMacAppStore(value: unknown, projectRoot: string): MacAppStoreConfig | undefined {
  if (value === undefined) return undefined;
  const appStore = objectOrEmpty(value, "macos.appStore");
  const entitlements = optionalString(appStore.entitlements, "macos.appStore.entitlements", 1_024);
  return {
    applicationIdentity: requiredString(
      appStore.applicationIdentity,
      "macos.appStore.applicationIdentity",
      512,
    ),
    installerIdentity: requiredString(
      appStore.installerIdentity,
      "macos.appStore.installerIdentity",
      512,
    ),
    provisioningProfile: resolveRelative(
      projectRoot,
      requiredString(appStore.provisioningProfile, "macos.appStore.provisioningProfile", 1_024),
    ),
    ...(entitlements ? { entitlements: resolveRelative(projectRoot, entitlements) } : {}),
  };
}

function resolveWindowsNsis(value: unknown): WindowsNsisConfig | undefined {
  if (value === undefined) return undefined;
  const nsis = objectOrEmpty(value, "windows.nsis");
  const installDirectory = optionalString(
    nsis.installDirectory,
    "windows.nsis.installDirectory",
    1_024,
  );
  return {
    ...(installDirectory ? { installDirectory } : {}),
    ...(optionalBoolean(nsis.createDesktopShortcut, "windows.nsis.createDesktopShortcut") !==
    undefined
      ? { createDesktopShortcut: Boolean(nsis.createDesktopShortcut) }
      : {}),
    ...(optionalBoolean(nsis.createStartMenuShortcut, "windows.nsis.createStartMenuShortcut") !==
    undefined
      ? { createStartMenuShortcut: Boolean(nsis.createStartMenuShortcut) }
      : {}),
    ...(optionalBoolean(nsis.perMachine, "windows.nsis.perMachine") !== undefined
      ? { perMachine: Boolean(nsis.perMachine) }
      : {}),
  };
}

function resolveWindowsSigning(
  value: unknown,
  projectRoot: string,
): WindowsSigningConfig | undefined {
  if (value === undefined) return undefined;
  const signing = objectOrEmpty(value, "windows.signing");
  const certificateFile = optionalString(
    signing.certificateFile,
    "windows.signing.certificateFile",
    1_024,
  );
  const subjectName = optionalString(signing.subjectName, "windows.signing.subjectName", 512);
  if ((certificateFile === undefined) === (subjectName === undefined)) {
    throw new CliError("`windows.signing` needs exactly one of `certificateFile` or `subjectName`");
  }
  const passwordEnvironmentVariable = optionalString(
    signing.passwordEnvironmentVariable,
    "windows.signing.passwordEnvironmentVariable",
    255,
  );
  const timestampUrl = optionalString(signing.timestampUrl, "windows.signing.timestampUrl", 2_048);
  const digest = optionalString(signing.digest, "windows.signing.digest", 16);
  if (digest !== undefined && !["sha256", "sha384", "sha512"].includes(digest)) {
    throw new CliError("`windows.signing.digest` must be sha256, sha384, or sha512");
  }
  return {
    ...(certificateFile ? { certificateFile: resolveRelative(projectRoot, certificateFile) } : {}),
    ...(subjectName ? { subjectName } : {}),
    ...(passwordEnvironmentVariable ? { passwordEnvironmentVariable } : {}),
    ...(timestampUrl ? { timestampUrl } : {}),
    ...(digest ? { digest: digest as "sha256" | "sha384" | "sha512" } : {}),
  };
}

function linuxCategories(value: unknown): string[] {
  const categories = stringArray(value, "linux.categories");
  if (categories.length === 0) return ["Utility"];
  for (const category of categories) {
    if (!/^[A-Za-z][A-Za-z0-9-]*$/.test(category)) {
      throw new CliError(`Invalid desktop category \`${category}\` in \`linux.categories\``);
    }
  }
  return categories;
}

function protocolArray(value: unknown): string[] {
  if (value === undefined) return [];
  if (!Array.isArray(value) || value.length > 64) {
    throw new CliError("`protocols` must be an array with at most 64 URL schemes");
  }
  const protocols = value.map((item, index) =>
    requiredString(item, `protocols[${index}]`, 64).toLowerCase(),
  );
  for (const protocol of protocols) {
    if (!/^[a-z][a-z0-9+.-]*$/.test(protocol)) {
      throw new CliError(`Invalid URL scheme \`${protocol}\` in \`protocols\``);
    }
  }
  return [...new Set(protocols)];
}

function executableName(name: string): string {
  const value = name
    .normalize("NFKD")
    .replace(/[^A-Za-z0-9._-]+/g, "-")
    .replace(/^[._-]+|[._-]+$/g, "")
    .slice(0, 128);
  if (!value || value === "." || value === "..") {
    throw new CliError("Application name does not contain a usable executable name");
  }
  return value;
}

function resolveRelative(root: string, path: string): string {
  return isAbsolute(path) ? resolve(path) : resolve(root, path);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function objectOrEmpty(value: unknown, field: string): Record<string, unknown> {
  if (value === undefined) return {};
  if (!isRecord(value)) throw new CliError(`\`${field}\` must be an object`);
  return value;
}

function requiredString(value: unknown, field: string, maximum = 255): string {
  if (typeof value !== "string" || value.trim().length === 0 || value.length > maximum) {
    throw new CliError(`\`${field}\` must be a non-empty string of at most ${maximum} characters`);
  }
  return value.trim();
}

function optionalString(value: unknown, field: string, maximum: number): string | undefined {
  if (value === undefined) return undefined;
  return requiredString(value, field, maximum);
}

function optionalBoolean(value: unknown, field: string): boolean | undefined {
  if (value === undefined) return undefined;
  if (typeof value !== "boolean") throw new CliError(`\`${field}\` must be a boolean`);
  return value;
}

function stringArray(value: unknown, field: string): string[] {
  if (value === undefined) return [];
  if (!Array.isArray(value) || value.length > 1_024) {
    throw new CliError(`\`${field}\` must be an array with at most 1024 paths`);
  }
  return value.map((item, index) => requiredString(item, `${field}[${index}]`, 1_024));
}

/** Sparkle-compatible appcasts and raw Ed25519 signatures, shared by all updater backends. */
import { createPrivateKey, createPublicKey, generateKeyPairSync, sign } from "node:crypto";
import {
  chmodSync,
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  renameSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { basename, join, resolve } from "node:path";
import type { ResolvedQuickGuiConfig } from "../config.ts";
import { CliError } from "../error.ts";
import type { QuickGuiTarget } from "../targets.ts";
import { targetInfo } from "../targets.ts";
import { releaseNotes } from "./changelog.ts";
import { artifactUrl, feedUrl } from "./publish.ts";

const privatePrefix = Buffer.from("302e020100300506032b657004220420", "hex");
export function sparklePrivateKey(secret: string) {
  const bytes = Buffer.from(secret.trim(), "base64");
  if (bytes.length !== 32 || bytes.toString("base64") !== secret.trim())
    throw new CliError("Sparkle private key must be a base64 32-byte Ed25519 seed");
  const key = createPrivateKey({
    key: Buffer.concat([privatePrefix, bytes]),
    format: "der",
    type: "pkcs8",
  });
  return key;
}
export function sparklePublicKey(key: ReturnType<typeof createPrivateKey>): string {
  const jwk = createPublicKey(key).export({ format: "jwk" });
  if (!jwk.x) throw new CliError("Missing Ed25519 public key");
  return Buffer.from(jwk.x, "base64url").toString("base64");
}
export function generateUpdaterKeys(
  directory: string,
  force = false,
): { publicKeyPath: string; secretKeyPath: string } {
  const publicKeyPath = resolve(directory, "quickgui-update.pub"),
    secretKeyPath = resolve(directory, "quickgui-update.key");
  if (!force && [publicKeyPath, secretKeyPath].some(existsSync))
    throw new CliError("An updater key already exists; use --force to replace it");
  const { privateKey } = generateKeyPairSync("ed25519");
  const jwk = privateKey.export({ format: "jwk" });
  const publicKey = sparklePublicKey(privateKey);
  const secret = Buffer.from(jwk.d!, "base64url");
  mkdirSync(directory, { recursive: true });
  writeFileSync(secretKeyPath, secret.toString("base64") + "\n", {
    mode: 0o600,
    flag: force ? "w" : "wx",
  });
  chmodSync(secretKeyPath, 0o600);
  writeFileSync(publicKeyPath, publicKey + "\n", { flag: force ? "w" : "wx" });
  return { publicKeyPath, secretKeyPath };
}
export function updaterMetadata(
  config: ResolvedQuickGuiConfig,
  target: QuickGuiTarget,
  mode: "development" | "production",
) {
  return {
    feedUrl: config.updates ? feedUrl(config.updates.destination, target) : "",
    publicKey: config.updates?.publicKey ?? "",
    currentVersion: config.version,
    identifier: mode === "development" ? `${config.identifier}.dev` : config.identifier,
    automaticChecks: config.updates?.automaticChecks ?? true,
    development: mode === "development",
  };
}
const xml = (value: string): string =>
  value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&apos;");
export interface AppcastInput {
  name: string;
  version: string;
  buildVersion?: string;
  target: QuickGuiTarget;
  url: string;
  bytes: Uint8Array;
  privateKey: string;
  publicKey: string;
  notes?: string;
  minimumSystemVersion?: string;
  /** Further enclosures of the same release. The portable updater picks one by install kind. */
  alternates?: ReadonlyArray<{ url: string; bytes: Uint8Array }>;
}
export function renderAppcast(input: AppcastInput): string {
  if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(input.version))
    throw new CliError("Appcast versions must be semantic versions");
  if (Buffer.byteLength(input.notes ?? "") > 16 * 1024)
    throw new CliError("Appcast notes must be at most 16 KiB");
  const key = sparklePrivateKey(input.privateKey);
  if (sparklePublicKey(key) !== input.publicKey)
    throw new CliError("Update signing key does not match updates.publicKey");
  const platform = targetInfo(input.target).platform;
  const os = platform === "darwin" ? "macos" : platform;
  const enclosures = [{ url: input.url, bytes: input.bytes }, ...(input.alternates ?? [])]
    .map((artifact) => {
      const url = new URL(artifact.url);
      if (url.protocol !== "https:" || url.username || url.password || url.hash)
        throw new CliError("Appcast enclosure must use HTTPS without credentials or fragments");
      if (!artifact.bytes.byteLength || artifact.bytes.byteLength > 512 * 1024 * 1024)
        throw new CliError("Update artifacts must be between 1 byte and 512 MiB");
      const signature = sign(null, artifact.bytes, key).toString("base64");
      return `      <enclosure url="${xml(artifact.url)}" length="${artifact.bytes.byteLength}" type="application/octet-stream" sparkle:edSignature="${signature}" sparkle:os="${os}" />`;
    })
    .join("\n");
  return `<?xml version="1.0" encoding="utf-8"?>
<rss version="2.0" xmlns:sparkle="http://www.andymatuschak.org/xml-namespaces/sparkle">
  <channel>
    <title>${xml(input.name)} (${input.target})</title>
    <item>
      <title>${xml(input.version)}</title>
      <sparkle:version>${xml(platform === "darwin" ? (input.buildVersion ?? input.version) : input.version)}</sparkle:version>
      <sparkle:shortVersionString>${xml(input.version)}</sparkle:shortVersionString>
      <pubDate>${new Date().toUTCString()}</pubDate>
      <description sparkle:format="markdown">${xml(input.notes ?? "")}</description>
${platform === "darwin" && input.minimumSystemVersion ? `      <sparkle:minimumSystemVersion>${xml(input.minimumSystemVersion)}</sparkle:minimumSystemVersion>\n` : ""}${enclosures}
    </item>
  </channel>
</rss>
`;
}
export async function writeAppcast(input: {
  config: ResolvedQuickGuiConfig;
  target: QuickGuiTarget;
  outputDirectory: string;
  source: string;
  alternates?: string[];
  run: (argv: string[], cwd: string) => Promise<void>;
}) {
  const { config, target } = input;
  let artifactPath = input.source;
  if (targetInfo(target).platform === "darwin") {
    artifactPath = join(
      input.outputDirectory,
      `${config.executableName}-${config.version}-${target}.zip`,
    );
    await input.run(
      ["ditto", "-c", "-k", "--sequesterRsrc", "--keepParent", input.source, artifactPath],
      config.projectRoot,
    );
  } else if (
    targetInfo(target).platform === "linux" &&
    !artifactPath.endsWith(".AppImage") &&
    !artifactPath.endsWith(".tar.gz")
  )
    throw new CliError(
      "The updater extension requires an AppImage or the `linux.tarball` install on Linux",
    );
  else if (targetInfo(target).platform === "windows" && !artifactPath.endsWith(".exe"))
    throw new CliError("The updater extension requires a QuickGUI NSIS installer on Windows");
  if (targetInfo(target).platform !== "darwin" && !artifactPath.endsWith(".tar.gz")) {
    // Different architectures may share a release directory and base URL. The tarball is
    // already named after its target.
    const extension = targetInfo(target).platform === "windows" ? "exe" : "AppImage";
    const publishedPath = join(
      input.outputDirectory,
      `${config.executableName}-${config.version}-${target}.${extension}`,
    );
    if (resolve(artifactPath) !== resolve(publishedPath)) copyFileSync(artifactPath, publishedPath);
    artifactPath = publishedPath;
  }
  if (!config.updates)
    throw new CliError(
      "Writing an appcast needs a publishing target: set `updates.target` to \"github\" or \"s3\"",
    );
  const { destination } = config.updates;
  const secretPath = config.updates.ed25519SecretKey;
  if (secretPath && statSync(secretPath).size > 1024)
    throw new CliError("Update signing key file is too large");
  const secret =
    process.env.QUICKGUI_UPDATER_PRIVATE_KEY ??
    process.env.SPARKLE_PRIVATE_KEY ??
    (secretPath ? readFileSync(secretPath, "utf8") : undefined);
  if (!secret || !config.updates.publicKey)
    throw new CliError(
      "Set updates.publicKey and QUICKGUI_UPDATER_PRIVATE_KEY (or updates.ed25519SecretKey) to sign the appcast",
    );
  const published = (path: string): string =>
    artifactUrl(destination, config.version, basename(path));
  for (const path of [artifactPath, ...(input.alternates ?? [])])
    if (statSync(path).size > 512 * 1024 * 1024)
      throw new CliError("Update artifact exceeds 512 MiB");
  const url = published(artifactPath);
  const { changelog } = config.updates;
  if (changelog && statSync(changelog).size > 4 * 1024 * 1024)
    throw new CliError("The changelog exceeds 4 MiB");
  const notes = changelog
    ? releaseNotes(readFileSync(changelog, "utf8"), config.version, basename(changelog))
    : undefined;
  const manifest = renderAppcast({
    name: config.name,
    version: config.version,
    buildVersion: config.buildVersion,
    target,
    url,
    bytes: readFileSync(artifactPath),
    alternates: (input.alternates ?? []).map((path) => ({
      url: published(path),
      bytes: readFileSync(path),
    })),
    privateKey: secret,
    publicKey: config.updates.publicKey,
    ...(notes ? { notes } : {}),
    minimumSystemVersion: config.macos.minimumSystemVersion,
  });
  const manifestPath = join(input.outputDirectory, `appcast-${target}.xml`);
  const temporary = `${manifestPath}.tmp-${process.pid}`;
  writeFileSync(temporary, manifest);
  renameSync(temporary, manifestPath);
  // The same section becomes the GitHub release body.
  const notesPath = notes ? join(input.outputDirectory, "release-notes.md") : undefined;
  if (notesPath) writeFileSync(notesPath, `${notes}\n`);
  return { artifactPath, manifestPath, url, ...(notesPath ? { notesPath } : {}) };
}

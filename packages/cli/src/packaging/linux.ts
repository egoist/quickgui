/**
 * Linux desktop integration: `.desktop` entries, AppDir/AppImage layout, `.deb` packages, and the
 * self-updating per-user tarball with its install script.
 *
 * The `.deb` writer is pure TypeScript — an `ar` container holding two ustar+gzip tarballs — so it
 * runs on any host without `dpkg-deb`. AppImage creation still needs `appimagetool`; when that is
 * missing QuickGUI leaves the finished AppDir and prints the one command that completes it.
 */

import { CliError } from "../error.ts";
import type { QuickGuiTarget } from "../targets.ts";
import { pointerUrl, type UpdateDestination } from "./publish.ts";
import { createAr, createTar, type TarEntry } from "./archive.ts";
import { linuxMimeTypes, type ResolvedDocumentType } from "./documents.ts";
import { LINUX_ICON_SIZES } from "./icons.ts";

/** Debian architecture names for the Linux targets QuickGUI builds. */
export const DEBIAN_ARCHITECTURES: Readonly<Record<"arm64" | "x64", string>> = Object.freeze({
  arm64: "arm64",
  x64: "amd64",
});

export interface DesktopEntryOptions {
  name: string;
  executableName: string;
  identifier: string;
  comment?: string;
  categories: readonly string[];
  protocols: readonly string[];
  documentTypes: readonly ResolvedDocumentType[];
  /** `Exec=` command. Defaults to the executable name plus `%U` when the app handles URLs/files. */
  execPath?: string;
}

/** Freedesktop desktop entry text. Values are single-line and never contain a newline. */
export function desktopEntry(options: DesktopEntryOptions): string {
  const mimeTypes = [
    ...options.protocols.map((protocol) => `x-scheme-handler/${protocol}`),
    ...linuxMimeTypes(options.documentTypes),
  ];
  const exec = options.execPath ?? options.executableName;
  const acceptsArguments = mimeTypes.length > 0;
  const lines = [
    "[Desktop Entry]",
    "Type=Application",
    `Name=${desktopValue(options.name)}`,
    ...(options.comment ? [`Comment=${desktopValue(options.comment)}`] : []),
    `Exec=${desktopValue(exec)}${acceptsArguments ? " %U" : ""}`,
    `Icon=${desktopValue(options.executableName)}`,
    "Terminal=false",
    `Categories=${options.categories.map(desktopValue).join(";")};`,
    ...(mimeTypes.length > 0 ? [`MimeType=${mimeTypes.map(desktopValue).join(";")};`] : []),
    `StartupWMClass=${desktopValue(options.executableName)}`,
    `X-QuickGUI-Identifier=${desktopValue(options.identifier)}`,
  ];
  return `${lines.join("\n")}\n`;
}

function desktopValue(value: string): string {
  const single = value.replaceAll(/[\r\n]+/g, " ").trim();
  if (single.length === 0) throw new CliError("A desktop entry value cannot be empty");
  return single;
}

/** The `AppRun` script an AppDir needs when the payload is a plain executable. */
export function appRunScript(executableName: string): string {
  return `#!/bin/sh
HERE="$(dirname "$(readlink -f "$0")")"
exec "$HERE/usr/bin/${executableName}" "$@"
`;
}

/** `appimagetool` command line. */
export function appImageArguments(appDir: string, output: string): string[] {
  return ["appimagetool", "--no-appstream", appDir, output];
}

export interface DebianControlOptions {
  packageName: string;
  version: string;
  architecture: string;
  maintainer: string;
  description: string;
  section: string;
  depends: readonly string[];
  installedSizeKilobytes: number;
  homepage?: string;
}

/** Debian `control` file. The description body is indented per Debian policy. */
export function debianControl(options: DebianControlOptions): string {
  if (!/^[a-z0-9][a-z0-9+.-]+$/.test(options.packageName)) {
    throw new CliError(
      `Invalid Debian package name \`${options.packageName}\`: use lowercase letters, digits, +, -, and .`,
    );
  }
  if (!options.maintainer.includes("<") || !options.maintainer.includes(">")) {
    throw new CliError(
      "`linux.maintainer` must look like `Name <email@example.com>` to build a .deb",
    );
  }
  const [summary, ...rest] = options.description.split("\n");
  const body = rest
    .map((line) => (line.trim().length === 0 ? " ." : ` ${line.trim()}`))
    .join("\n");
  const lines = [
    `Package: ${options.packageName}`,
    `Version: ${options.version}`,
    `Architecture: ${options.architecture}`,
    `Maintainer: ${options.maintainer}`,
    `Installed-Size: ${Math.max(1, Math.round(options.installedSizeKilobytes))}`,
    `Section: ${options.section}`,
    "Priority: optional",
    ...(options.depends.length > 0 ? [`Depends: ${options.depends.join(", ")}`] : []),
    ...(options.homepage ? [`Homepage: ${options.homepage}`] : []),
    `Description: ${(summary ?? options.packageName).trim()}`,
    ...(body.length > 0 ? [body] : []),
  ];
  return `${lines.join("\n")}\n`;
}

/** `md5sums` control file body for the package payload. */
export function debianMd5Sums(files: ReadonlyArray<{ path: string; md5: string }>): string {
  return files.map((file) => `${file.md5}  ${file.path.replace(/^\/+/, "")}\n`).join("");
}

export interface DebianPackageInput {
  control: string;
  md5sums: string;
  /** Payload entries with paths relative to the filesystem root, e.g. `usr/bin/app`. */
  data: readonly TarEntry[];
  postinst?: string;
  gzip: (data: Uint8Array) => Uint8Array;
}

/** Assemble a `.deb` from already-built control text and payload entries. */
export function createDebianPackage(input: DebianPackageInput): Uint8Array {
  const encoder = new TextEncoder();
  const controlEntries: TarEntry[] = [
    { path: "./control", data: encoder.encode(input.control), mode: 0o644 },
    { path: "./md5sums", data: encoder.encode(input.md5sums), mode: 0o644 },
    ...(input.postinst
      ? [{ path: "./postinst", data: encoder.encode(input.postinst), mode: 0o755 }]
      : []),
  ].map((entry) => ({ ...entry, path: entry.path.replace(/^\.\//, "") }));
  return createAr([
    { name: "debian-binary", data: encoder.encode("2.0\n") },
    { name: "control.tar.gz", data: input.gzip(createTar(controlEntries)) },
    { name: "data.tar.gz", data: input.gzip(createTar(input.data)) },
  ]);
}

/** Standard payload paths for a QuickGUI Linux install under `prefix` (`usr` for a `.deb`). */
export function debianPayloadPaths(
  executableName: string,
  prefix = "usr",
): {
  executable: string;
  desktopEntry: string;
  mimePackage: string;
  icon: (size: number) => string;
} {
  return {
    executable: `${prefix}/bin/${executableName}`,
    desktopEntry: `${prefix}/share/applications/${executableName}.desktop`,
    mimePackage: `${prefix}/share/mime/packages/${executableName}.xml`,
    icon: (size: number) =>
      `${prefix}/share/icons/hicolor/${size}x${size}/apps/${executableName}.png`,
  };
}

/** Marker inside a tarball install. The updater only replaces a prefix that carries it. */
export const MANAGED_INSTALL_MARKER = "share/quickgui/install.json";
/** Icon size the install script and updater pin the desktop entry to. */
export const MANAGED_INSTALL_ICON_SIZE = 256;
/** Version pointer `install.sh` resolves "latest" from. */
export const LATEST_LINUX_VERSION_FILE = "latest-linux.txt";

/** Marker body. `install.sh` greps the identifier, so the serialization has no whitespace. */
export function managedInstallMarker(identifier: string, executableName: string): string {
  return `${JSON.stringify({ schema: 1, identifier, executable: executableName })}\n`;
}

/** Release tarball name. It carries the CLI target, so architectures can share a bucket. */
export function tarballName(
  executableName: string,
  version: string,
  target: QuickGuiTarget,
): string {
  return `${executableName}-${version}-${target}.tar.gz`;
}

export interface InstallScriptOptions {
  name: string;
  executableName: string;
  identifier: string;
  /** Lowercase command and directory name, as produced by `debianPackageName`. */
  packageName: string;
  /** Where releases are published. Without it the script only installs a local bundle. */
  destination?: UpdateDestination;
}

/** Environment variable prefix of the generated script, such as `MY_APP`. */
export function installScriptPrefix(packageName: string): string {
  const prefix = packageName.toUpperCase().replace(/[^A-Z0-9]+/g, "_");
  return /^[0-9]/.test(prefix) ? `APP_${prefix}` : prefix;
}

/**
 * POSIX `install.sh` for the release tarball: no root, no package manager. It unpacks into
 * `~/.local/<package>.app`, links the command into `~/.local/bin`, and registers the desktop
 * entry with absolute paths. The install keeps that path, so the updater can swap it in place.
 */
export function installScript(options: InstallScriptOptions): string {
  const prefix = installScriptPrefix(options.packageName);
  const executable = options.executableName;
  const { destination } = options;
  // GitHub keeps pointers under `releases/latest/download` and artifacts under their tag; a
  // bucket serves both from one directory.
  const releases =
    destination?.kind === "github"
      ? `https://github.com/${destination.repository}/releases`
      : destination
        ? pointerUrl(destination, "").replace(/\/+$/, "")
        : "";
  return `#!/usr/bin/env sh
set -eu

# Installs ${shellComment(options.name)} for Linux into ~/.local. Generated by \`quickgui build\`.
#
# Environment:
#   ${prefix}_VERSION        install this version instead of the latest
#   ${prefix}_BUNDLE_PATH    install a local tarball instead of downloading
#   ${prefix}_RELEASES_URL   download from here instead of the published location

app_name=${shellQuote(options.name)}
executable=${shellQuote(executable)}
identifier=${shellQuote(options.identifier)}
package=${shellQuote(options.packageName)}
default_releases=${shellQuote(releases)}
layout=${shellQuote(destination?.kind === "github" ? "github" : "flat")}
tag_prefix=${shellQuote(destination?.kind === "github" ? destination.tagPrefix : "")}

usage() {
    cat <<USAGE
Install $app_name for Linux into ~/.local.

Usage:
  sh install.sh
  sh install.sh --uninstall

Options:
  --uninstall   Remove $app_name, leaving its settings and data alone
  --help        Show this help
USAGE
}

main() {
    app_dir="$HOME/.local/$package.app"
    bin_link="$HOME/.local/bin/$package"
    data_home="\${XDG_DATA_HOME:-$HOME/.local/share}"
    desktop_file="$data_home/applications/$executable.desktop"
    mime_file="$data_home/mime/packages/$executable.xml"
    releases="\${${prefix}_RELEASES_URL:-$default_releases}"

    case "\${1:-}" in
        --uninstall) uninstall; return ;;
        --help | -h) usage; return ;;
        "") ;;
        *)
            echo "Unknown option: $1" >&2
            usage >&2
            exit 1
            ;;
    esac

    if [ "$(uname -s)" != "Linux" ]; then
        echo "This script installs the Linux build of $app_name." >&2
        exit 1
    fi
    if [ "$(id -u)" = "0" ]; then
        echo "Run this script as your own user; a root install cannot update itself." >&2
        exit 1
    fi
    # The desktop entry embeds this path, quoted for the Exec key and written through sed.
    case "$app_dir" in
        *[\\\\\\"\\\`\\$\\|\\&]* | *"
"*)
            echo "Your home directory path contains characters a desktop entry cannot hold." >&2
            exit 1
            ;;
    esac

    machine="$(uname -m)"
    case "$machine" in
        x86_64 | amd64) target="linux-x64" ;;
        aarch64 | arm64) target="linux-arm64" ;;
        *)
            echo "Unsupported architecture: $machine" >&2
            exit 1
            ;;
    esac

    if command -v curl >/dev/null 2>&1; then
        fetch() { command curl -fsSL "$1"; }
    elif command -v wget >/dev/null 2>&1; then
        fetch() { wget -qO- "$1"; }
    else
        fetch() {
            echo "Could not find 'curl' or 'wget' in your PATH." >&2
            return 1
        }
    fi

    temp="$(mktemp -d "\${TMPDIR:-/tmp}/$package-XXXXXX")"
    staging="$app_dir.new"
    trap 'rm -rf -- "$temp" "$staging"' EXIT INT TERM

    archive="$temp/bundle.tar.gz"
    if [ -n "\${${prefix}_BUNDLE_PATH:-}" ]; then
        cp "\$${prefix}_BUNDLE_PATH" "$archive"
    else
        if [ -z "$releases" ]; then
            echo "No download URL is configured. Set ${prefix}_RELEASES_URL or ${prefix}_BUNDLE_PATH." >&2
            exit 1
        fi
        if [ "$layout" = "github" ]; then
            pointers="$releases/latest/download"
        else
            pointers="$releases"
        fi
        version="\${${prefix}_VERSION:-}"
        if [ -z "$version" ]; then
            if ! version="$(fetch "$pointers/${LATEST_LINUX_VERSION_FILE}")"; then
                echo "Could not reach $pointers/${LATEST_LINUX_VERSION_FILE}." >&2
                echo "Set ${prefix}_VERSION to install a specific version." >&2
                exit 1
            fi
            version="$(printf '%s' "$version" | tr -d '[:space:]')"
        fi
        case "$version" in
            "" | *[!0-9A-Za-z.+-]*)
                echo "No usable $app_name version is published for Linux." >&2
                exit 1
                ;;
        esac
        if [ "$layout" = "github" ]; then
            bundle_url="$releases/download/$tag_prefix$version/$executable-$version-$target.tar.gz"
        else
            bundle_url="$releases/$executable-$version-$target.tar.gz"
        fi
        echo "Downloading $app_name $version for $machine"
        if ! fetch "$bundle_url" >"$archive"; then
            echo "Download failed: $bundle_url" >&2
            exit 1
        fi
    fi
    if ! tar -tzf "$archive" >/dev/null 2>&1; then
        echo "The bundle is not a readable tarball." >&2
        exit 1
    fi

    # Unpack beside the target and swap only once the contents check out, so a truncated
    # download cannot leave a working install in pieces. The tarball holds one versioned
    # top-level directory; stripping it keeps every install at the same path.
    echo "Installing to $app_dir"
    rm -rf "$staging"
    mkdir -p "$staging" "$(dirname "$bin_link")" "$(dirname "$desktop_file")"
    tar -xzf "$archive" --strip-components=1 -C "$staging"
    if [ ! -x "$staging/bin/$executable" ]; then
        echo "The bundle is missing bin/$executable." >&2
        exit 1
    fi
    if ! grep -qF '"identifier":"'"$identifier"'"' "$staging/${MANAGED_INSTALL_MARKER}" 2>/dev/null; then
        echo "The bundle is missing its managed-install marker." >&2
        exit 1
    fi
    # Replace rather than merge: a file dropped from a later layout must not survive the upgrade.
    rm -rf "$app_dir"
    mv "$staging" "$app_dir"
    ln -sf "$app_dir/bin/$executable" "$bin_link"

    # The packaged entry is relocatable (bare Exec/Icon names). Pin both to this install so the
    # launcher works without PATH or icon-theme setup.
    entry="$app_dir/share/applications/$executable.desktop"
    icon="$app_dir/share/icons/hicolor/${MANAGED_INSTALL_ICON_SIZE}x${MANAGED_INSTALL_ICON_SIZE}/apps/$executable.png"
    if [ -f "$entry" ]; then
        exec_value='"'"$app_dir/bin/$executable"'"'
        icon_rule=""
        if [ -f "$icon" ]; then
            icon_rule="s|^Icon=$executable$|Icon=$icon|"
        fi
        sed -e "s|^Exec=$executable|Exec=$exec_value|" -e "$icon_rule" "$entry" >"$desktop_file"
        if command -v update-desktop-database >/dev/null 2>&1; then
            update-desktop-database "$(dirname "$desktop_file")" 2>/dev/null || true
        fi
    fi
    if [ -f "$app_dir/share/mime/packages/$executable.xml" ]; then
        mkdir -p "$(dirname "$mime_file")"
        cp "$app_dir/share/mime/packages/$executable.xml" "$mime_file"
        if command -v update-mime-database >/dev/null 2>&1; then
            update-mime-database "$data_home/mime" 2>/dev/null || true
        fi
    fi

    echo "$app_name is installed."
    if [ -f "$desktop_file" ]; then
        echo "Open it from your applications menu."
    fi
    if [ "$(command -v "$package" || true)" = "$bin_link" ]; then
        echo "From a terminal: $package"
    else
        echo "From a terminal: $bin_link"
    fi
}

uninstall() {
    if [ ! -d "$app_dir" ] && [ ! -L "$bin_link" ]; then
        echo "$app_name is not installed at $app_dir." >&2
        exit 1
    fi
    # Only reclaim what this script created; a distro package's copies belong to its manager.
    if [ "$(readlink "$bin_link" 2>/dev/null || true)" = "$app_dir/bin/$executable" ]; then
        rm -f "$bin_link"
    fi
    if [ -f "$desktop_file" ] && grep -qF "$app_dir/bin/$executable" "$desktop_file"; then
        rm -f "$desktop_file"
        if [ -f "$mime_file" ]; then
            rm -f "$mime_file"
            if command -v update-mime-database >/dev/null 2>&1; then
                update-mime-database "$data_home/mime" 2>/dev/null || true
            fi
        fi
    fi
    rm -rf "$app_dir"
    echo "$app_name is uninstalled. Its settings and data were left alone."
}

main "$@"
`;
}

function shellQuote(value: string): string {
  return `'${value.replaceAll("'", `'\\''`)}'`;
}

function shellComment(value: string): string {
  return value.replaceAll(/[\r\n]+/g, " ");
}

export { LINUX_ICON_SIZES };

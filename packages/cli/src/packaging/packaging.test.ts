import { describe, expect, test } from "bun:test";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

import { resolveConfig } from "../config.ts";
import {
  macInfoPlist,
  reservedSidecarNames,
  stageExecutableSidecars,
  updateSource,
  validateBuildInputs,
} from "../build.ts";
import { createAr, createTar, normalizeArchivePath, splitUstarPath } from "./archive.ts";
import {
  linuxMimeTypes,
  macDocumentTypesPlist,
  macTypeDeclarationsPlist,
  nsisFileAssociationCommands,
  nsisProtocolCommands,
  nsisString,
  sharedMimeInfoXml,
  type ResolvedDocumentType,
} from "./documents.ts";
import {
  createIcns,
  createIco,
  ICNS_ENTRIES,
  iconsetEntryPath,
  pngDimensions,
  sipsResizeArguments,
} from "./icons.ts";
import {
  appImageArguments,
  appRunScript,
  createDebianPackage,
  debianControl,
  debianMd5Sums,
  debianPayloadPaths,
  desktopEntry,
} from "./linux.ts";
import {
  masCodesignArguments,
  masEntitlementsTemplate,
  masPackageFilename,
  productBuildArguments,
  validateMasConfig,
} from "./mas.ts";
import { debianPackageName, packageLinux, payloadMd5Sums } from "./pipeline.ts";
import { copyResources, extraPayloadEntries } from "./resources.ts";
import {
  buildUpdateManifest,
  joinUrl,
  minisignKeygenArguments,
  minisignSignArguments,
  rfc3339,
  serializeUpdateManifest,
  updateArchiveArguments,
  updateArtifactName,
  updateTarget,
  type UpdateManifest,
} from "./updates.ts";
import {
  makensisArguments,
  nsisScript,
  signToolArguments,
  windowsFileVersion,
} from "./windows.ts";

const documentTypes: ResolvedDocumentType[] = [
  {
    name: "Demo Project",
    extensions: ["demo", "demoproj"],
    role: "Editor",
    mimeTypes: ["application/x-demo-project"],
    conformsTo: ["public.data"],
    exported: true,
    utTypeIdentifier: "com.example.demo.project",
    description: "A Demo project bundle",
  },
  {
    name: "Demo Log",
    extensions: ["demolog"],
    role: "Viewer",
    mimeTypes: [],
    conformsTo: ["public.plain-text"],
    exported: false,
    utTypeIdentifier: "com.example.demo.log",
  },
];

/** A one-pixel-per-side PNG is enough: the writers only copy bytes and read `IHDR`. */
function fakePng(size: number): Uint8Array {
  const data = new Uint8Array(30 + size);
  data.set([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a], 0);
  const view = new DataView(data.buffer);
  view.setUint32(8, 13);
  view.setUint32(12, 0x49_48_44_52);
  view.setUint32(16, size);
  view.setUint32(20, size);
  return data;
}

describe("archive writers", () => {
  test("ustar blocks are 512-byte aligned with a valid checksum", () => {
    const payload = new TextEncoder().encode("hello");
    const tar = createTar([{ path: "usr/bin/demo", data: payload, mode: 0o755 }]);
    expect(tar.byteLength % 512).toBe(0);
    // header + one data block + two terminating blocks
    expect(tar.byteLength).toBe(512 * 4);
    expect(new TextDecoder().decode(tar.subarray(0, 12))).toBe("usr/bin/demo");
    expect(new TextDecoder().decode(tar.subarray(100, 107))).toBe("0000755");
    expect(new TextDecoder().decode(tar.subarray(257, 262))).toBe("ustar");
    expect(new TextDecoder().decode(tar.subarray(512, 517))).toBe("hello");
    let checksum = 0;
    for (const [index, byte] of tar.subarray(0, 512).entries()) {
      checksum += index >= 148 && index < 156 ? 0x20 : byte;
    }
    expect(Number.parseInt(new TextDecoder().decode(tar.subarray(148, 154)), 8)).toBe(checksum);
  });

  test("ustar rejects escaping and absolute paths", () => {
    expect(() => normalizeArchivePath("../escape")).toThrow();
    expect(() => normalizeArchivePath("/absolute")).toThrow();
    expect(() => normalizeArchivePath("usr/./bin")).toThrow();
    expect(normalizeArchivePath("./usr/bin/demo")).toBe("usr/bin/demo");
  });

  test("long paths move into the ustar prefix field", () => {
    const long = `${"directory/".repeat(11)}demo.png`;
    expect(long.length).toBeGreaterThan(100);
    const { name, prefix } = splitUstarPath(long);
    expect(name).toBe("demo.png");
    expect(prefix).toBe("directory/".repeat(11).slice(0, -1));
    expect(() => splitUstarPath("x".repeat(200))).toThrow();
  });

  test("ar members are padded to an even length", () => {
    const archive = createAr([
      { name: "debian-binary", data: new TextEncoder().encode("2.0\n") },
      { name: "control.tar.gz", data: new Uint8Array([1, 2, 3]) },
    ]);
    expect(new TextDecoder().decode(archive.subarray(0, 8))).toBe("!<arch>\n");
    expect(new TextDecoder().decode(archive.subarray(8, 24))).toBe("debian-binary   ");
    // 8 magic + 60 header + 4 data + 60 header + 3 data + 1 pad
    expect(archive.byteLength).toBe(8 + 60 + 4 + 60 + 3 + 1);
    expect(() => createAr([{ name: "../evil", data: new Uint8Array() }])).toThrow();
  });
});

describe("icon containers", () => {
  test("reads PNG dimensions and rejects non-PNG input", () => {
    expect(pngDimensions(fakePng(512))).toEqual({ width: 512, height: 512 });
    expect(() => pngDimensions(new Uint8Array(64))).toThrow();
  });

  test("icns holds an 8-byte magic header plus typed PNG entries", () => {
    const sources = new Map([
      [128, fakePng(128)],
      [256, fakePng(256)],
    ]);
    const icns = createIcns(sources);
    expect(new TextDecoder().decode(icns.subarray(0, 4))).toBe("icns");
    const view = new DataView(icns.buffer, icns.byteOffset, icns.byteLength);
    expect(view.getUint32(4)).toBe(icns.byteLength);
    // ic07 (128) comes first in ICNS_ENTRIES order, then ic13 and ic08 for 256.
    expect(new TextDecoder().decode(icns.subarray(8, 12))).toBe("ic07");
    expect(view.getUint32(12)).toBe(sources.get(128)!.byteLength + 8);
    const types = ICNS_ENTRIES.map((entry) => entry.type);
    expect(types).toEqual(["ic11", "ic12", "ic07", "ic13", "ic08", "ic09", "ic10"]);
    expect(() => createIcns(new Map())).toThrow();
  });

  test("ico directory entries record 256 as zero", () => {
    const sources = new Map([
      [32, fakePng(32)],
      [256, fakePng(256)],
    ]);
    const ico = createIco(sources);
    const view = new DataView(ico.buffer, ico.byteOffset, ico.byteLength);
    expect(view.getUint16(0, true)).toBe(0);
    expect(view.getUint16(2, true)).toBe(1);
    expect(view.getUint16(4, true)).toBe(2);
    expect(ico[6]).toBe(32);
    expect(ico[6 + 16]).toBe(0);
    expect(view.getUint32(6 + 12, true)).toBe(6 + 32);
    expect(view.getUint32(6 + 16 + 8, true)).toBe(sources.get(256)!.byteLength);
  });

  test("names the sips resize and pre-sized iconset paths", () => {
    expect(sipsResizeArguments("/a/icon.png", "/b/icon-64.png", 64)).toEqual([
      "sips",
      "-z",
      "64",
      "64",
      "/a/icon.png",
      "--out",
      "/b/icon-64.png",
    ]);
    expect(iconsetEntryPath("/assets/icon.png", 512)).toBe(
      "/assets/icon.iconset/icon_512x512.png",
    );
  });
});

describe("application resources", () => {
  test("copies files and directories by basename and rejects reserved names", () => {
    const root = mkdtempSync(join(tmpdir(), "quickgui-resources-"));
    try {
      const assets = join(root, "assets");
      mkdirSync(assets);
      writeFileSync(join(assets, "logo.png"), "png");
      writeFileSync(join(root, "notes.txt"), "hello");
      const destination = join(root, "out");
      mkdirSync(destination);
      expect(copyResources([assets, join(root, "notes.txt")], destination)).toEqual([
        join(destination, "assets"),
        join(destination, "notes.txt"),
      ]);
      expect(readFileSync(join(destination, "assets", "logo.png"), "utf8")).toBe("png");
      expect(() => copyResources([assets], destination, new Set(["assets"]))).toThrow(
        "reserved or duplicated",
      );
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  test("expands directories into deterministic archive members", () => {
    const root = mkdtempSync(join(tmpdir(), "quickgui-payload-"));
    try {
      const assets = join(root, "assets");
      mkdirSync(join(assets, "copy"), { recursive: true });
      writeFileSync(join(assets, "logo.png"), "png");
      writeFileSync(join(assets, "copy", "template.txt"), "hi");
      const entries = extraPayloadEntries([assets], "usr/bin");
      expect(entries.map((entry) => entry.path)).toEqual([
        "usr/bin/assets",
        "usr/bin/assets/copy",
        "usr/bin/assets/copy/template.txt",
        "usr/bin/assets/logo.png",
      ]);
      expect(entries[0]).toMatchObject({ type: "directory" });
      expect(new TextDecoder().decode(entries[2]?.data)).toBe("hi");
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  test("Linux AppDir and Debian payloads keep resources beside the executable", async () => {
    const root = mkdtempSync(join(tmpdir(), "quickgui-linux-resources-"));
    try {
      writeFileSync(join(root, "demo"), "exe");
      const assets = join(root, "assets");
      mkdirSync(assets);
      writeFileSync(join(assets, "note.txt"), "bundled");
      const config = resolveConfig(
        {
          name: "Demo",
          identifier: "com.example.demo",
          language: "go",
          entry: ".",
          linux: { appImage: true, deb: true, maintainer: "Demo <demo@example.com>" },
        },
        root,
      );
      const result = await packageLinux({
        config,
        libraries: [assets],
        target: "linux-x64",
        executablePath: join(root, "demo"),
        stagingRoot: root,
        run: async () => {},
      });
      expect(existsSync(join(root, "Demo.AppDir", "usr", "bin", "assets", "note.txt"))).toBe(true);
      const deb = result.artifacts.find((path) => path.endsWith(".deb"));
      expect(deb).toBeDefined();
      expect(readFileSync(deb!).byteLength).toBeGreaterThan(0);
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  test("Debian md5sums omit directory members", () => {
    const root = mkdtempSync(join(tmpdir(), "quickgui-md5-"));
    try {
      const assets = join(root, "assets");
      mkdirSync(assets);
      writeFileSync(join(assets, "note.txt"), "bundled");
      const sums = payloadMd5Sums(extraPayloadEntries([assets], "usr/bin"));
      expect(sums).toContain("usr/bin/assets/note.txt");
      expect(sums).not.toContain("  usr/bin/assets\n");
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  test("reserves generated packaging names before copying resources", () => {
    const root = mkdtempSync(join(tmpdir(), "quickgui-reserved-"));
    try {
      writeFileSync(join(root, "main.go"), "package main\n");
      const config = resolveConfig(
        {
          name: "Demo",
          identifier: "com.example.demo",
          language: "go",
          entry: ".",
          version: "1.2.3",
        },
        root,
      );
      expect(reservedSidecarNames(config, "windows")).toEqual(
        expect.arrayContaining(["Demo.ico", "Demo.nsi", "Demo-1.2.3-setup.exe", "quickgui.json"]),
      );
      expect(reservedSidecarNames(config, "linux")).toEqual(
        expect.arrayContaining(["Demo.AppDir", "Demo-1.2.3-x86_64.AppImage", "demo_1.2.3_amd64.deb"]),
      );
      writeFileSync(join(root, "Demo.ico"), "icon");
      writeFileSync(join(root, "quickgui.json"), "{}");
      const colliding = resolveConfig(
        {
          name: "Demo",
          identifier: "com.example.demo",
          language: "go",
          entry: ".",
          resources: ["Demo.ico", "quickgui.json"],
        },
        root,
      );
      expect(() => stageExecutableSidecars(colliding, root, [], "windows")).toThrow(
        "reserved or duplicated",
      );
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  test("update artifacts come from packaged paths, not executable sidecars", () => {
    expect(updateSource({ target: "linux-x64" }, "/out/Demo", [])).toBe("/out/Demo");
    expect(
      updateSource({ target: "linux-x64" }, "/out/Demo", ["/out/Demo-1.0.0-x86_64.AppImage"]),
    ).toBe("/out/Demo-1.0.0-x86_64.AppImage");
    expect(
      updateSource({ target: "windows-x64" }, "/out/Demo.exe", ["/out/Demo-1.0.0-setup.exe"]),
    ).toBe("/out/Demo-1.0.0-setup.exe");
  });

  test("linux.icon must be a file", () => {
    const root = mkdtempSync(join(tmpdir(), "quickgui-linux-icon-"));
    try {
      writeFileSync(join(root, "main.go"), "package main\n");
      mkdirSync(join(root, "icon-dir"));
      const config = resolveConfig(
        {
          name: "Demo",
          identifier: "com.example.demo",
          language: "go",
          entry: ".",
          linux: { icon: "icon-dir" },
        },
        root,
      );
      expect(() => validateBuildInputs(config, "linux")).toThrow("Linux icon not found");
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });
});

describe("file associations", () => {
  test("macOS document types and UTI declarations are split by export", () => {
    const documents = macDocumentTypesPlist(documentTypes);
    expect(documents).toContain("<key>CFBundleDocumentTypes</key>");
    expect(documents).toContain("<string>Demo Project</string>");
    expect(documents).toContain("<string>demoproj</string>");
    expect(documents).toContain("<string>Editor</string>");
    expect(documents).toContain("<string>Viewer</string>");
    expect(documents).toContain("<string>application/x-demo-project</string>");

    const declarations = macTypeDeclarationsPlist(documentTypes);
    expect(declarations).toContain("<key>UTExportedTypeDeclarations</key>");
    expect(declarations).toContain("<key>UTImportedTypeDeclarations</key>");
    expect(declarations).toContain("<string>com.example.demo.project</string>");
    expect(declarations).toContain("<string>public.plain-text</string>");
    expect(macTypeDeclarationsPlist([])).toBe("");
  });

  test("Info.plist embeds document types beside URL schemes", () => {
    const plist = macInfoPlist({
      name: "Demo",
      displayName: "Demo",
      executableName: "Demo",
      identifier: "com.example.demo",
      version: "1.0.0",
      buildVersion: "1",
      minimumSystemVersion: "13.0",
      category: "public.app-category.developer-tools",
      urlSchemes: ["demo"],
      documentTypes,
    });
    expect(plist).toContain("<key>CFBundleURLSchemes</key>");
    expect(plist).toContain("<key>CFBundleDocumentTypes</key>");
    expect(plist).toContain("<key>UTExportedTypeDeclarations</key>");
    expect(plist.indexOf("<key>CFBundleDocumentTypes</key>")).toBeLessThan(
      plist.indexOf("<key>LSApplicationCategoryType</key>"),
    );
  });

  test("Linux advertises MIME types once and describes each glob", () => {
    expect(linuxMimeTypes(documentTypes)).toEqual(["application/x-demo-project"]);
    const xml = sharedMimeInfoXml(documentTypes);
    expect(xml).toContain('<mime-type type="application/x-demo-project">');
    expect(xml).toContain('<glob pattern="*.demo"/>');
    expect(xml).toContain('<glob pattern="*.demoproj"/>');
  });

  test("NSIS registry commands cover every extension and scheme", () => {
    const commands = nsisFileAssociationCommands("com.example.demo", "Demo.exe", documentTypes);
    expect(commands).toContain(
      'WriteRegStr SHCTX "Software\\Classes\\.demo" "" "com.example.demo.demo"',
    );
    expect(commands).toContain(
      `WriteRegStr SHCTX "Software\\Classes\\com.example.demo.demo\\shell\\open\\command" "" '"$INSTDIR\\Demo.exe" "%1"'`,
    );
    expect(commands).toHaveLength(3 * 4);

    const protocols = nsisProtocolCommands("Demo.exe", ["demo"]);
    expect(protocols).toContain('WriteRegStr SHCTX "Software\\Classes\\demo" "URL Protocol" ""');
  });

  test("NSIS strings escape the dollar sign and quotes", () => {
    expect(nsisString('a "b" $c')).toBe('a $\\"b$\\" $$c');
  });
});

describe("Linux packaging", () => {
  test("desktop entry lists schemes before document MIME types", () => {
    const entry = desktopEntry({
      name: "Demo",
      executableName: "Demo",
      identifier: "com.example.demo",
      comment: "A demo",
      categories: ["Utility", "Development"],
      protocols: ["demo"],
      documentTypes,
    });
    expect(entry).toContain("[Desktop Entry]\nType=Application\nName=Demo\n");
    expect(entry).toContain("Comment=A demo\n");
    expect(entry).toContain("Exec=Demo %U\n");
    expect(entry).toContain("Categories=Utility;Development;\n");
    expect(entry).toContain("MimeType=x-scheme-handler/demo;application/x-demo-project;\n");
    expect(entry).toContain("StartupWMClass=Demo\n");
  });

  test("desktop entry omits %U and MimeType without handlers", () => {
    const entry = desktopEntry({
      name: "Demo",
      executableName: "Demo",
      identifier: "com.example.demo",
      categories: ["Utility"],
      protocols: [],
      documentTypes: [],
    });
    expect(entry).toContain("Exec=Demo\n");
    expect(entry).not.toContain("MimeType=");
  });

  test("AppRun and appimagetool arguments", () => {
    expect(appRunScript("Demo")).toContain('exec "$HERE/usr/bin/Demo" "$@"');
    expect(appImageArguments("/build/Demo.AppDir", "/build/Demo.AppImage")).toEqual([
      "appimagetool",
      "--no-appstream",
      "/build/Demo.AppDir",
      "/build/Demo.AppImage",
    ]);
  });

  test("Debian control validates the package name and maintainer", () => {
    const control = debianControl({
      packageName: "demo",
      version: "1.2.3",
      architecture: "amd64",
      maintainer: "Demo Team <demo@example.com>",
      description: "A demo application",
      section: "utils",
      depends: ["libc6"],
      installedSizeKilobytes: 2048.4,
    });
    expect(control).toContain("Package: demo\n");
    expect(control).toContain("Architecture: amd64\n");
    expect(control).toContain("Installed-Size: 2048\n");
    expect(control).toContain("Depends: libc6\n");
    expect(control).toContain("Description: A demo application\n");
    expect(() =>
      debianControl({
        packageName: "Demo",
        version: "1.0.0",
        architecture: "amd64",
        maintainer: "Demo Team <demo@example.com>",
        description: "x",
        section: "utils",
        depends: [],
        installedSizeKilobytes: 1,
      }),
    ).toThrow();
    expect(() =>
      debianControl({
        packageName: "demo",
        version: "1.0.0",
        architecture: "amd64",
        maintainer: "Demo Team",
        description: "x",
        section: "utils",
        depends: [],
        installedSizeKilobytes: 1,
      }),
    ).toThrow();
  });

  test("derives a Debian package name from the executable name", () => {
    expect(debianPackageName("My App")).toBe("my-app");
    expect(debianPackageName("Demo")).toBe("demo");
    expect(() => debianPackageName("!")).toThrow();
  });

  test("a .deb is an ar archive of debian-binary, control, and data", () => {
    const deb = createDebianPackage({
      control: debianControl({
        packageName: "demo",
        version: "1.0.0",
        architecture: "amd64",
        maintainer: "Demo Team <demo@example.com>",
        description: "A demo application",
        section: "utils",
        depends: [],
        installedSizeKilobytes: 1,
      }),
      md5sums: debianMd5Sums([{ path: "usr/bin/demo", md5: "0".repeat(32) }]),
      data: [{ path: "usr/bin/demo", data: new TextEncoder().encode("binary"), mode: 0o755 }],
      gzip: (bytes) => new Uint8Array(Bun.gzipSync(Buffer.from(bytes))),
    });
    const text = new TextDecoder().decode(deb.subarray(0, 200));
    expect(text.startsWith("!<arch>\n")).toBe(true);
    expect(text).toContain("debian-binary");
    expect(text).toContain("control.tar.gz");
    // Both tarballs are real gzip members (0x1f 0x8b).
    let gzipMembers = 0;
    for (let index = 0; index + 1 < deb.byteLength; index += 1) {
      if (deb[index] === 0x1f && deb[index + 1] === 0x8b) gzipMembers += 1;
    }
    expect(gzipMembers).toBe(2);
    expect(debianMd5Sums([{ path: "/usr/bin/demo", md5: "abc" }])).toBe("abc  usr/bin/demo\n");
  });

  test("payload paths follow the filesystem hierarchy standard", () => {
    const paths = debianPayloadPaths("demo");
    expect(paths.executable).toBe("usr/bin/demo");
    expect(paths.desktopEntry).toBe("usr/share/applications/demo.desktop");
    expect(paths.mimePackage).toBe("usr/share/mime/packages/demo.xml");
    expect(paths.icon(256)).toBe("usr/share/icons/hicolor/256x256/apps/demo.png");
  });
});

describe("Windows packaging", () => {
  const script = nsisScript({
    name: "Demo",
    executableName: "Demo.exe",
    identifier: "com.example.demo",
    version: "1.2.3",
    publisher: "Example Inc",
    executablePath: "/build/Demo.exe",
    outputFile: "/build/Demo-1.2.3-setup.exe",
    protocols: ["demo"],
    documentTypes,
  });

  test("declares the install directory, uninstaller, and registry entries", () => {
    expect(script).toContain('OutFile "/build/Demo-1.2.3-setup.exe"');
    expect(script).toContain('InstallDir "$LOCALAPPDATA\\Demo"');
    expect(script).toContain('VIProductVersion "1.2.3.0"');
    expect(script).toContain("RequestExecutionLevel user");
    expect(script).toContain('WriteUninstaller "$INSTDIR\\Uninstall.exe"');
    expect(script).toContain(
      'WriteRegStr SHCTX "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\com.example.demo" "DisplayName" "Demo"',
    );
    expect(script).toContain('CreateShortCut "$DESKTOP\\Demo.lnk"');
    expect(script).toContain('CreateShortCut "$SMPROGRAMS\\Demo\\Demo.lnk"');
    expect(script).toContain('WriteRegStr SHCTX "Software\\Classes\\.demo"');
    expect(script).toContain('WriteRegStr SHCTX "Software\\Classes\\demo" "URL Protocol" ""');
    expect(script).toContain('DeleteRegKey SHCTX "Software\\Classes\\com.example.demo.demo"');
  });

  test("per-machine installs request admin rights and Program Files", () => {
    const perMachine = nsisScript({
      name: "Demo",
      executableName: "Demo.exe",
      identifier: "com.example.demo",
      version: "1.2.3",
      publisher: "Example Inc",
      executablePath: "/build/Demo.exe",
      outputFile: "/build/setup.exe",
      protocols: [],
      documentTypes: [],
      perMachine: true,
      createDesktopShortcut: false,
    });
    expect(perMachine).toContain("RequestExecutionLevel admin");
    expect(perMachine).toContain('InstallDir "$PROGRAMFILES64\\Demo"');
    expect(perMachine).toContain("SetShellVarContext all");
    expect(perMachine).not.toContain('CreateShortCut "$DESKTOP\\Demo.lnk"');
  });

  test("installs extra files and directories beside the executable", () => {
    const root = mkdtempSync(join(tmpdir(), "quickgui-nsis-resources-"));
    try {
      const assets = join(root, "assets");
      mkdirSync(assets);
      const withFiles = nsisScript({
        name: "Demo",
        executableName: "Demo.exe",
        identifier: "com.example.demo",
        version: "1.2.3",
        publisher: "Example Inc",
        executablePath: "/build/Demo.exe",
        outputFile: "/build/setup.exe",
        extraFiles: [
          ["/build/quickgui_host.dll", "quickgui_host.dll"],
          [assets, "assets"],
        ],
        protocols: [],
        documentTypes: [],
      });
      expect(withFiles).toContain('File "/oname=quickgui_host.dll" "/build/quickgui_host.dll"');
      expect(withFiles).toContain('CreateDirectory "$INSTDIR\\assets"');
      expect(withFiles).toContain(`File /r "${assets}\\*.*"`);
      expect(withFiles).toContain('RMDir /r "$INSTDIR\\assets"');
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  test("file versions are four numeric components", () => {
    expect(windowsFileVersion("1.2.3")).toBe("1.2.3.0");
    expect(windowsFileVersion("1.2.3-beta.4")).toBe("1.2.3.0");
    expect(windowsFileVersion("99999.0.0.0")).toBe("65535.0.0.0");
  });

  test("makensis and signtool argument construction", () => {
    expect(makensisArguments("/build/demo.nsi")).toEqual(["makensis", "-V2", "/build/demo.nsi"]);
    expect(
      signToolArguments({
        artifact: "C:\\build\\setup.exe",
        certificateFile: "C:\\keys\\demo.pfx",
        password: "secret",
      }),
    ).toEqual([
      "signtool",
      "sign",
      "/fd",
      "sha256",
      "/td",
      "sha256",
      "/tr",
      "http://timestamp.digicert.com",
      "/f",
      "C:\\keys\\demo.pfx",
      "/p",
      "secret",
      "C:\\build\\setup.exe",
    ]);
    expect(() => signToolArguments({ artifact: "setup.exe" })).toThrow();
    expect(() =>
      signToolArguments({ artifact: "setup.exe", certificateFile: "a.pfx", subjectName: "b" }),
    ).toThrow();
  });
});

describe("Mac App Store packaging", () => {
  test("validates both identities and the profile extension", () => {
    const valid = {
      applicationIdentity: "3rd Party Mac Developer Application: Example (TEAMID)",
      installerIdentity: "3rd Party Mac Developer Installer: Example (TEAMID)",
      provisioningProfile: "/keys/demo.provisionprofile",
    };
    expect(() => validateMasConfig(valid)).not.toThrow();
    expect(() =>
      validateMasConfig({ ...valid, applicationIdentity: "Developer ID Application: Example" }),
    ).toThrow();
    expect(() =>
      validateMasConfig({ ...valid, installerIdentity: "Developer ID Installer: Example" }),
    ).toThrow();
    expect(() => validateMasConfig({ ...valid, provisioningProfile: "/keys/demo.mobileprovision" })).toThrow();
  });

  test("builds codesign and productbuild command lines", () => {
    expect(
      masCodesignArguments("/build/Demo.app", "3rd Party Mac Developer Application: Example", "/build/mas.entitlements"),
    ).toEqual([
      "codesign",
      "--force",
      "--deep",
      "--timestamp",
      "--options",
      "runtime",
      "--sign",
      "3rd Party Mac Developer Application: Example",
      "--entitlements",
      "/build/mas.entitlements",
      "/build/Demo.app",
    ]);
    expect(
      productBuildArguments("/build/Demo.app", "3rd Party Mac Developer Installer: Example", "/build/Demo.pkg"),
    ).toEqual([
      "productbuild",
      "--component",
      "/build/Demo.app",
      "/Applications",
      "--sign",
      "3rd Party Mac Developer Installer: Example",
      "/build/Demo.pkg",
    ]);
    expect(masPackageFilename("Demo", "1.2.3")).toBe("Demo 1.2.3.pkg");
    expect(() => masPackageFilename("De/mo", "1.0.0")).toThrow();
  });

  test("the entitlements template enables the App Sandbox", () => {
    const template = masEntitlementsTemplate("TEAMID", "com.example.demo");
    expect(template).toContain("<key>com.apple.security.app-sandbox</key>");
    expect(template).toContain("<string>TEAMID.com.example.demo</string>");
    expect(masEntitlementsTemplate(undefined, "com.example.demo")).not.toContain(
      "application-groups",
    );
  });
});

describe("updater manifest", () => {
  test("maps CLI targets to the core's default_update_target() strings", () => {
    expect(updateTarget("darwin-arm64")).toBe("darwin-aarch64");
    expect(updateTarget("darwin-x64")).toBe("darwin-x86_64");
    expect(updateTarget("linux-arm64")).toBe("linux-aarch64");
    expect(updateTarget("windows-x64")).toBe("windows-x86_64");
  });

  test("names the artifact layout each platform's installer accepts", () => {
    expect(updateArtifactName("darwin-arm64", "Demo", "1.2.3")).toBe("Demo.app.tar.gz");
    expect(updateArtifactName("windows-x64", "Demo", "1.2.3")).toBe("Demo-1.2.3-setup.exe");
    expect(updateArtifactName("linux-x64", "Demo", "1.2.3")).toBe(
      "Demo-1.2.3-x86_64.AppImage.tar.gz",
    );
    expect(updateArtifactName("linux-arm64", "Demo", "1.2.3")).toBe(
      "Demo-1.2.3-aarch64.AppImage.tar.gz",
    );
    expect(updateArchiveArguments("/out", "Demo.app", "/out/Demo.app.tar.gz")).toEqual([
      "tar",
      "-czf",
      "/out/Demo.app.tar.gz",
      "-C",
      "/out",
      "Demo.app",
    ]);
  });

  test("builds a platforms manifest and preserves sibling targets at the same version", () => {
    const existing: UpdateManifest = {
      version: "1.4.0",
      platforms: {
        "linux-x86_64": { url: "https://dl.example.com/app/old.tar.gz", signature: "sig" },
      },
    };
    const manifest = buildUpdateManifest({
      version: "1.4.0",
      baseUrl: "https://dl.example.com/app/",
      target: "darwin-arm64",
      artifactName: "Demo.app.tar.gz",
      signature: "c2lnbmF0dXJl",
      notes: "Fixes things",
      publishedAt: new Date("2026-09-03T12:00:00.500Z"),
      existing,
    });
    expect(manifest).toEqual({
      version: "1.4.0",
      pub_date: "2026-09-03T12:00:00Z",
      notes: "Fixes things",
      platforms: {
        "linux-x86_64": { url: "https://dl.example.com/app/old.tar.gz", signature: "sig" },
        "darwin-aarch64": {
          url: "https://dl.example.com/app/Demo.app.tar.gz",
          signature: "c2lnbmF0dXJl",
        },
      },
    });
    expect(serializeUpdateManifest(manifest).endsWith("\n")).toBe(true);
  });

  test("supports the flat url/signature form", () => {
    const manifest = buildUpdateManifest({
      version: "2.0.0",
      baseUrl: "https://dl.example.com/app",
      target: "linux-x64",
      artifactName: "demo.AppImage.tar.gz",
      signature: "sig",
      flat: true,
      publishedAt: new Date("2026-01-01T00:00:00Z"),
    });
    expect(manifest.url).toBe("https://dl.example.com/app/demo.AppImage.tar.gz");
    expect(manifest.signature).toBe("sig");
    expect(manifest.platforms).toBeUndefined();
  });

  test("rejects non-semantic versions, plain HTTP, and empty signatures", () => {
    const base = {
      baseUrl: "https://dl.example.com/app",
      target: "darwin-arm64" as const,
      artifactName: "Demo.app.tar.gz",
      signature: "sig",
    };
    expect(() => buildUpdateManifest({ ...base, version: "nightly" })).toThrow();
    expect(() =>
      buildUpdateManifest({ ...base, version: "1.0.0", baseUrl: "http://dl.example.com" }),
    ).toThrow();
    expect(() => buildUpdateManifest({ ...base, version: "1.0.0", signature: "" })).toThrow();
    expect(joinUrl("https://dl.example.com/app/", "a b.tar.gz")).toBe(
      "https://dl.example.com/app/a%20b.tar.gz",
    );
    expect(rfc3339(new Date("2026-09-03T12:00:00.999Z"))).toBe("2026-09-03T12:00:00Z");
  });

  test("the shared fixture round-trips through the manifest shape", () => {
    const fixture = JSON.parse(
      readFileSync(resolve(import.meta.dir, "../../../../tests/fixtures/updater-manifest.json"), "utf8"),
    ) as UpdateManifest;
    expect(fixture.version).toBe("1.4.0");
    expect(fixture.pub_date).toBe("2026-09-03T12:00:00Z");
    expect(Object.keys(fixture.platforms ?? {}).sort()).toEqual([
      "darwin-aarch64",
      "darwin-x86_64",
      "linux-x86_64",
      "windows-x86_64",
    ]);
    for (const platform of Object.values(fixture.platforms ?? {})) {
      expect(platform.url.startsWith("https://")).toBe(true);
      expect(platform.signature.length).toBeGreaterThan(0);
    }
    const rebuilt = buildUpdateManifest({
      version: fixture.version,
      baseUrl: "https://downloads.example.com/quickgui-demo",
      target: "darwin-arm64",
      artifactName: "Demo.app.tar.gz",
      signature: fixture.platforms!["darwin-aarch64"]!.signature,
      notes: fixture.notes!,
      publishedAt: new Date(fixture.pub_date!),
    });
    expect(rebuilt.platforms!["darwin-aarch64"]).toEqual(fixture.platforms!["darwin-aarch64"]!);
    expect(rebuilt.pub_date).toBe(fixture.pub_date);
  });

  test("minisign and rsign share one signing contract", () => {
    expect(
      minisignSignArguments({
        tool: "minisign",
        secretKey: "/keys/demo.key",
        artifact: "/out/Demo.app.tar.gz",
        signaturePath: "/out/Demo.app.tar.gz.minisig",
        comment: "Demo 1.0.0",
      }),
    ).toEqual([
      "minisign",
      "-S",
      "-s",
      "/keys/demo.key",
      "-m",
      "/out/Demo.app.tar.gz",
      "-x",
      "/out/Demo.app.tar.gz.minisig",
      "-c",
      "Demo 1.0.0",
    ]);
    expect(
      minisignSignArguments({
        tool: "rsign",
        secretKey: "/keys/demo.key",
        artifact: "/out/Demo.app.tar.gz",
        signaturePath: "/out/Demo.app.tar.gz.minisig",
      }),
    ).toEqual([
      "rsign",
      "sign",
      "-s",
      "/keys/demo.key",
      "-x",
      "/out/Demo.app.tar.gz.minisig",
      "/out/Demo.app.tar.gz",
    ]);
    expect(minisignKeygenArguments("minisign", "/k/a.pub", "/k/a.key", true)).toEqual([
      "minisign",
      "-G",
      "-p",
      "/k/a.pub",
      "-s",
      "/k/a.key",
      "-W",
    ]);
    expect(minisignKeygenArguments("rsign", "/k/a.pub", "/k/a.key", false)).toEqual([
      "rsign",
      "generate",
      "-p",
      "/k/a.pub",
      "-s",
      "/k/a.key",
      "-f",
    ]);
  });
});

describe("configuration", () => {
  const base = { name: "Demo", identifier: "com.example.demo" };

  test("resolves document types with defaults and bounds", () => {
    const config = resolveConfig(
      {
        ...base,
        documentTypes: [
          { name: "Demo Project", extensions: [".DEMO"], utTypeIdentifier: "com.example.demo.p" },
        ],
      },
      "/project",
    );
    expect(config.documentTypes).toEqual([
      {
        name: "Demo Project",
        extensions: ["demo"],
        role: "Editor",
        mimeTypes: [],
        conformsTo: ["public.data"],
        exported: true,
        utTypeIdentifier: "com.example.demo.p",
      },
    ]);
  });

  test("rejects duplicate extensions, bad roles, and malformed MIME types", () => {
    expect(() =>
      resolveConfig(
        {
          ...base,
          documentTypes: [
            { name: "A", extensions: ["demo"] },
            { name: "B", extensions: ["demo"] },
          ],
        },
        "/project",
      ),
    ).toThrow();
    expect(() =>
      resolveConfig(
        { ...base, documentTypes: [{ name: "A", extensions: ["demo"], role: "Owner" }] },
        "/project",
      ),
    ).toThrow();
    expect(() =>
      resolveConfig(
        { ...base, documentTypes: [{ name: "A", extensions: ["demo"], mimeTypes: ["nope"] }] },
        "/project",
      ),
    ).toThrow();
    expect(() =>
      resolveConfig({ ...base, documentTypes: [{ name: "A", extensions: [] }] }, "/project"),
    ).toThrow();
  });

  test("resolves updater and Linux defaults", () => {
    const config = resolveConfig(
      {
        ...base,
        updates: { manifest: true, baseUrl: "https://dl.example.com/app/", minisignSecretKey: "keys/demo.key" },
        linux: { maintainer: "Demo Team <demo@example.com>" },
      },
      "/project",
    );
    expect(config.updates).toEqual({
      manifest: true,
      baseUrl: "https://dl.example.com/app",
      minisignSecretKey: "/project/keys/demo.key",
    });
    expect(config.linux).toEqual({
      categories: ["Utility"],
      section: "utils",
      depends: [],
      appImage: true,
      deb: true,
      maintainer: "Demo Team <demo@example.com>",
    });
    expect(
      resolveConfig(base, "/project").linux.deb,
    ).toBe(false);
    expect(() =>
      resolveConfig({ ...base, updates: { baseUrl: "http://dl.example.com" } }, "/project"),
    ).toThrow();
  });

  test("Windows signing needs exactly one certificate selector", () => {
    expect(() =>
      resolveConfig({ ...base, windows: { signing: {} } }, "/project"),
    ).toThrow();
    expect(() =>
      resolveConfig(
        { ...base, windows: { signing: { certificateFile: "a.pfx", subjectName: "b" } } },
        "/project",
      ),
    ).toThrow();
    const config = resolveConfig(
      { ...base, windows: { signing: { subjectName: "Example Inc", digest: "sha384" } } },
      "/project",
    );
    expect(config.windows.signing).toEqual({ subjectName: "Example Inc", digest: "sha384" });
  });
});

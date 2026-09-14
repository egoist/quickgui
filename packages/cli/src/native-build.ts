/** Application compilation. Go and TypeScript reuse the Rust shared library; Rust apps link the crate. */
import { chmodSync, copyFileSync, constants, existsSync, mkdirSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import type { ResolvedQuickGuiConfig } from "./config.ts";
import { CliError } from "./error.ts";
import { hostTarget, targetInfo, type QuickGuiTarget } from "./targets.ts";
import { updaterMetadata } from "./packaging/appcast.ts";
import { unpackResources } from "./extension-resources.ts";
import { discoverExtensions, extensionLibraryName, resolveExtension } from "./extensions.ts";
import { prepareGoWorkspace } from "./go-build.ts";
import { compileRustApplication } from "./rust-build.ts";
import { compileTypeScriptApplication, typescriptExtensions } from "./typescript-build.ts";

export interface NativeCompileOptions {
  config: ResolvedQuickGuiConfig;
  mode: "development" | "production";
  target: QuickGuiTarget;
  executablePath: string;
  fonts: string[];
}

export function sharedLibraryName(target: QuickGuiTarget): string {
  switch (targetInfo(target).platform) {
    case "darwin":
      return "libquickgui_host.dylib";
    case "windows":
      return "quickgui_host.dll";
    default:
      return "libquickgui_host.so";
  }
}

export function resolveHostLibrary(
  target: QuickGuiTarget,
  projectRoot: string,
  override?: string,
): string {
  const name = sharedLibraryName(target);
  const explicit = override ?? process.env.QUICKGUI_LIBRARY;
  if (explicit) {
    const path = resolve(projectRoot, explicit);
    if (!existsSync(path)) throw new CliError(`Native shared library not found: ${path}`);
    return path;
  }
  let nativeDir: string;
  try {
    nativeDir = dirname(Bun.resolveSync("@quickgui/native/package.json", projectRoot));
  } catch {
    nativeDir = resolve(import.meta.dir, "..", "..", "native");
  }
  const candidates = [join(nativeDir, "lib", target, name)];
  if (target === hostTarget())
    candidates.push(resolve(nativeDir, "..", "..", "target", "release", name));
  for (const path of candidates) if (existsSync(path)) return path;
  throw new CliError(
    `No QuickGUI shared library for ${target}. Run bun run build:native, or set native.libraryPath.`,
  );
}

/** Pure build plan, also used by tests to enforce CGO_ENABLED=0. */
export function goBuildPlan(options: NativeCompileOptions): {
  argv: string[];
  env: Record<string, string | undefined>;
} {
  const { config, target, mode } = options;
  const info = targetInfo(target);
  const metadata = Buffer.from(
    JSON.stringify({
      name: config.name,
      version: config.version,
      identifier: mode === "development" ? `${config.identifier}.dev` : config.identifier,
      fonts: options.fonts,
    }),
  ).toString("base64url");
  const ldflags = [
    ...(mode === "production" ? ["-s", "-w"] : []),
    ...(info.platform === "windows" && config.windows.hideConsole ? ["-H=windowsgui"] : []),
    "-X",
    `github.com/egoist/quickgui/go/native.buildMetadata=${metadata}`,
  ];
  let entry = relative(config.projectRoot, config.entry).replaceAll("\\", "/");
  if (!entry.startsWith(".")) entry = `./${entry}`;
  if (entry === "./") entry = ".";
  return {
    argv: [
      "go",
      "build",
      ...(mode === "production" ? ["-trimpath"] : []),
      ...(config.native.tags.length ? ["-tags", config.native.tags.join(",")] : []),
      "-ldflags",
      ldflags.join(" "),
      "-o",
      options.executablePath,
      entry,
    ],
    env: {
      ...process.env,
      CGO_ENABLED: "0",
      GOOS: info.platform === "windows" ? "windows" : info.platform,
      GOARCH: info.architecture === "x64" ? "amd64" : "arm64",
    },
  };
}

export async function compileNativeApplication(options: NativeCompileOptions): Promise<string[]> {
  const { config, target } = options;
  if (config.language === "rust") {
    return compileRustApplication(options);
  }
  const typescript = config.language === "typescript";
  if (!typescript && !Bun.which("go"))
    throw new CliError("Go 1.23 or later is required on PATH");
  const library = resolveHostLibrary(target, config.projectRoot, config.native.libraryPath);
  const plan = goBuildPlan(options);
  const extensions = typescript
    ? typescriptExtensions(config.projectRoot, config.extensions)
    : await discoverExtensions(config, plan.argv.at(-1)!, plan.env);
  if (
    extensions.some(
      (extension) =>
        extension.name === "updater" && extension.package === "@quickgui/extension-updater",
    )
  ) {
    if (options.mode === "production" && !config.updates?.publicKey)
      throw new CliError(
        "The updater extension requires [updates] with baseUrl and publicKey for production builds",
      );
    const metadata = Buffer.from(
      JSON.stringify(updaterMetadata(config, target, options.mode)),
    ).toString("base64url");
    if (!typescript) {
      const flagIndex = plan.argv.indexOf("-ldflags") + 1;
      plan.argv[flagIndex] += " -X github.com/egoist/quickgui/extensions/updater.buildMetadata=" + metadata;
    }
  }
  const destination =
    targetInfo(target).platform === "darwin"
      ? join(dirname(options.executablePath), "..", "Frameworks")
      : dirname(options.executablePath);
  mkdirSync(destination, { recursive: true });
  const libraries = [join(destination, sharedLibraryName(target))];
  copyFileSync(library, libraries[0]!, constants.COPYFILE_FICLONE);
  for (const extension of extensions) {
    const source = await resolveExtension(extension, target, config.projectRoot);
    const path = join(destination, extensionLibraryName(extension, target));
    copyFileSync(source, path, constants.COPYFILE_FICLONE);
    libraries.push(path);
  }
  // Stage all images first so resource extraction cannot claim another
  // extension's (or the core's) library path before that image is copied.
  for (const extension of extensions) {
    for (const resource of extension.resources?.[targetInfo(target).platform] ?? []) {
      const input = await resolveExtension(extension, target, config.projectRoot, resource);
      if (resource.endsWith(".qgr")) libraries.push(unpackResources(input, destination));
      else {
        const output = join(destination, resource);
        if (existsSync(output))
          throw new CliError("Native extension resource collision: " + resource);
        copyFileSync(input, output, constants.COPYFILE_FICLONE);
        chmodSync(output, 0o755);
        libraries.push(output);
      }
    }
  }
  if (typescript) {
    await compileTypeScriptApplication(options, extensions);
    return libraries;
  }
  const overlay = await prepareGoWorkspace(config.projectRoot, [plan.argv.at(-1)!], {
    tags: config.native.tags,
    env: plan.env,
  });
  if (overlay) plan.argv.splice(2, 0, "-overlay", overlay);
  const child = Bun.spawn(plan.argv, {
    cwd: config.projectRoot,
    stdin: "ignore",
    stdout: "pipe",
    stderr: "pipe",
    env: plan.env,
  });
  const [status, stdout, stderr] = await Promise.all([
    child.exited,
    new Response(child.stdout).text(),
    new Response(child.stderr).text(),
  ]);
  if (status !== 0) throw new CliError(`Go compilation failed\n${stderr.trim() || stdout.trim()}`);
  if (!existsSync(options.executablePath))
    throw new CliError(`Go did not write ${options.executablePath}`);
  return libraries;
}

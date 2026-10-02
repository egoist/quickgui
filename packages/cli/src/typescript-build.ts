import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import type { NativeCompileOptions } from "./native-build.ts";
import { hidesWindowsConsole } from "./config.ts";
import { CliError } from "./error.ts";
import { extensionLibraryName, extensionManifests, type ExtensionManifest } from "./extensions.ts";
import { updaterMetadata } from "./packaging/appcast.ts";

export function typescriptExtensions(project: string, names: string[]): ExtensionManifest[] {
  const directories = names.map((name) => {
    const pkg = name === "terminal" || name === "updater" ? `@quickgui/extension-${name}` : name;
    return pkg.startsWith("@") || pkg.startsWith("quickgui-extension-")
      ? dirname(Bun.resolveSync(`${pkg}/package.json`, project))
      : pkg;
  });
  for (const directory of directories)
    if (!existsSync(join(directory, "quickgui.extension.json")))
      throw new CliError(`Native extension manifest not found in ${directory}`);
  return extensionManifests(directories.map((Dir) => ({ Dir })));
}

export async function compileTypeScriptApplication(
  options: NativeCompileOptions,
  extensions: ExtensionManifest[] = [],
): Promise<void> {
  const { quickguiSolidPlugin } = await import("./typescript-compiler.ts");
  const { config, target, executablePath, mode } = options;
  const cache = resolve(config.projectRoot, ".quickgui", "typescript");
  mkdirSync(cache, { recursive: true });
  const temporary = mkdtempSync(join(cache, "build-"));
  try {
    const runtime = Bun.resolveSync("@quickgui/native/runtime", config.projectRoot);
    const host = join(temporary, "quickgui-host.ts");
    const worker = join(temporary, "quickgui-worker.ts");
    writeFileSync(
      host,
      `import { runHost } from ${JSON.stringify(runtime)};\nawait runHost("./quickgui-worker.ts");\n`,
    );
    writeFileSync(
      worker,
      `import { runApplication } from ${JSON.stringify(runtime)};\nawait runApplication(() => import(${JSON.stringify(config.entry)}), ${JSON.stringify(
        {
          name: config.name,
          version: config.version,
          identifier: mode === "development" ? `${config.identifier}.dev` : config.identifier,
          fonts: options.fonts,
          extensions: extensions.map((extension) => ({
            name: extension.name,
            version: extension.version,
            library: extensionLibraryName(extension, target),
          })),
          ...(extensions.some((extension) => extension.name === "updater")
            ? { updater: updaterMetadata(config, target, mode) }
            : {}),
        },
      )});\n`,
    );
    const result = await Bun.build({
      entrypoints: [host, worker],
      target: "bun",
      format: "esm",
      throw: false,
      plugins: [
        quickguiSolidPlugin({
          projectRoot: config.projectRoot,
          development: mode === "development",
        }),
      ],
      minify: mode === "production",
      sourcemap: mode === "development" ? "inline" : "none",
      env: "disable",
      define: { "process.env.NODE_ENV": JSON.stringify(mode) },
      compile: {
        target: `bun-${target}`,
        outfile: executablePath,
        autoloadDotenv: false,
        autoloadBunfig: false,
        autoloadPackageJson: false,
        autoloadTsconfig: false,
        ...(target.startsWith("windows-")
          ? {
              windows: {
                hideConsole: hidesWindowsConsole(config, target, mode),
                title: config.name,
                version: config.version,
              },
            }
          : {}),
      },
    });
    if (!result.success)
      throw new CliError(`TypeScript compilation failed\n${result.logs.join("\n")}`);
    if (!existsSync(executablePath)) throw new CliError(`Bun did not write ${executablePath}`);
  } finally {
    rmSync(temporary, { recursive: true, force: true });
  }
}

/** Tests use the same Solid compiler and client-runtime resolution as packaged applications. */
export async function runTypeScript(
  project: string,
  command: "check" | "test" | "fmt",
  check = false,
): Promise<number> {
  let argv: string[];
  if (command === "check") {
    argv = [
      process.execPath,
      join(dirname(Bun.resolveSync("typescript/package.json", project)), "bin", "tsc"),
      "--noEmit",
    ];
  } else if (command === "fmt") {
    argv = ["bun", "run", "oxfmt", ...(check ? ["--check"] : ["--write"]), "."];
  } else {
    const cache = resolve(project, ".quickgui", "typescript");
    mkdirSync(cache, { recursive: true });
    const preload = join(cache, "test-preload.ts");
    writeFileSync(
      preload,
      `import { plugin } from "bun";\nimport { quickguiSolidPlugin } from ${JSON.stringify(resolve(import.meta.dir, "typescript-compiler.ts"))};\nplugin(quickguiSolidPlugin({ projectRoot: ${JSON.stringify(project)} }));\n`,
    );
    argv = [process.execPath, "test", "--preload", preload];
  }
  const child = Bun.spawn(argv, {
    cwd: project,
    stdin: "inherit",
    stdout: "inherit",
    stderr: "inherit",
  });
  return child.exited;
}

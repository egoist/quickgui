import { existsSync, mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { CliError } from "./error.ts";

export type ExtensionType = "go" | "zig" | "rust";

export interface InitExtensionOptions {
  directory: string;
  type: ExtensionType;
  install: boolean;
  name?: string;
  module?: string;
  npmPackage?: string;
}

export function parseExtensionType(value: string): ExtensionType {
  if (value === "go" || value === "zig" || value === "rust") return value;
  throw new CliError(`Unknown extension type: ${value}. Expected go, zig, or rust.`);
}

const templateRoot = fileURLToPath(new URL("../templates/extension/", import.meta.url));
const goKeywords = new Set([
  "break",
  "default",
  "func",
  "interface",
  "select",
  "case",
  "defer",
  "go",
  "map",
  "struct",
  "chan",
  "else",
  "goto",
  "package",
  "switch",
  "const",
  "fallthrough",
  "if",
  "range",
  "type",
  "continue",
  "for",
  "import",
  "return",
  "var",
  "main",
  "init",
]);

export async function initExtension(options: InitExtensionOptions): Promise<string> {
  const type = parseExtensionType(options.type);
  const destination = resolve(options.directory);
  if (existsSync(destination) && !statSync(destination).isDirectory())
    throw new CliError(`Project destination is not a directory: ${destination}`);
  if (existsSync(destination) && readdirSync(destination).length > 0)
    throw new CliError(`Project destination is not empty: ${destination}`);

  const name = options.name ?? defaultName(basename(destination));
  if (!/^[a-z][a-z0-9-]{0,63}$/.test(name) || name === "host" || name === "terminal")
    throw new CliError(
      "Extension name must start with a lowercase letter, contain only lowercase letters, digits, or hyphens, and be at most 64 characters. host and terminal are reserved.",
    );
  const module = options.module ?? `example.com/${name}`;
  // Keep replacements valid in Go imports, go.mod, JSON, and documentation.
  if (
    !/^[a-zA-Z0-9][a-zA-Z0-9.-]*(\/[a-zA-Z0-9][a-zA-Z0-9._~-]*)+$/.test(module) ||
    module.split("/").some((part) => part.endsWith("."))
  )
    throw new CliError("--module must be a Go import path such as github.com/acme/my-extension");
  if (type === "go" && options.npmPackage !== undefined)
    throw new CliError("--npm-package is only used by Zig and Rust extensions");
  const npmPackage = options.npmPackage ?? `quickgui-extension-${name}`;
  if (
    npmPackage.length > 214 ||
    !/^(?:@[a-z0-9][a-z0-9._-]*\/)?[a-z0-9][a-z0-9._-]*$/.test(npmPackage)
  )
    throw new CliError("--npm-package must be a lowercase npm package name, optionally scoped");

  let goPackage = name.replaceAll("-", "");
  if (goKeywords.has(goPackage)) goPackage += "ext";
  const { version } = JSON.parse(
    readFileSync(new URL("../package.json", import.meta.url), "utf8"),
  ) as { version: string };
  const replacements: Record<string, string> = {
    NAME: name,
    TYPE: type,
    GO_MODULE: module,
    GO_PACKAGE: goPackage,
    SDK_VERSION: version,
    NPM_PACKAGE: npmPackage,
    LIBRARY: `quickgui_${name.replaceAll("-", "_")}`,
    DEV_SCRIPT: type === "go" ? "quickgui dev" : "bun scripts/dev.ts",
    COMPILER_REQUIREMENT:
      type === "zig" ? "Zig 0.16.0" : "stable Rust (edition 2021 or newer)",
  };

  // Read and expand every template before creating the destination. A broken CLI
  // installation or invalid option must not leave a half-created project.
  const files = new Map<string, string>();
  for (const group of ["common", ...(type === "go" ? ["go"] : ["native", type])]) {
    collectTemplates(join(templateRoot, group), "", files, replacements);
  }
  for (const [relative, contents] of files) {
    const target = join(destination, templateOutputName(relative));
    mkdirSync(dirname(target), { recursive: true });
    writeFileSync(target, contents);
  }

  if (options.install) {
    for (const argv of [
      ["bun", "install"],
      ["go", "mod", "tidy"],
    ]) {
      try {
        const child = Bun.spawn(argv, {
          cwd: destination,
          env: { ...process.env, CGO_ENABLED: "0" },
          stdin: "inherit",
          stdout: "inherit",
          stderr: "inherit",
        });
        const status = await child.exited;
        if (status !== 0) throw new Error(`exit status ${status}`);
      } catch (error) {
        throw new CliError(
          `Extension created at ${destination}, but \`${argv.join(" ")}\` failed: ${error instanceof Error ? error.message : String(error)}. Run it again in the project directory.`,
        );
      }
    }
  }
  return destination;
}

/** Map scaffold names onto generated paths. `.tmpl` hides tool manifests from discovery. */
function templateOutputName(relative: string): string {
  if (relative === "gitignore") return ".gitignore";
  return relative.endsWith(".tmpl") ? relative.slice(0, -".tmpl".length) : relative;
}

function defaultName(directory: string): string {
  let name = directory
    .normalize("NFKD")
    .replace(/([a-z0-9])([A-Z])/g, "$1-$2")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
  if (!name) name = "quickgui-extension";
  if (!/^[a-z]/.test(name)) name = `extension-${name}`;
  return name.slice(0, 64);
}

function collectTemplates(
  directory: string,
  prefix: string,
  files: Map<string, string>,
  replacements: Record<string, string>,
): void {
  if (!existsSync(directory)) throw new CliError(`CLI extension template is missing: ${directory}`);
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const source = join(directory, entry.name);
    const relative = join(prefix, entry.name);
    if (entry.isDirectory()) {
      collectTemplates(source, relative, files, replacements);
    } else if (entry.isFile()) {
      const contents = readFileSync(source, "utf8").replace(
        /\{\{([A-Z_]+)\}\}/g,
        (_, token: string) => {
          const replacement = replacements[token];
          if (replacement === undefined)
            throw new CliError(`Unknown extension template token: ${token}`);
          return replacement;
        },
      );
      files.set(relative, contents);
    }
  }
}

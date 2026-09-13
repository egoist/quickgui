import { existsSync, mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { resolveConfig, type Language } from "./config.ts";
import { CliError } from "./error.ts";

export interface InitProjectOptions {
  language?: Language;
  directory: string;
  install: boolean;
  name?: string;
  identifier?: string;
}

// .tmpl keeps Go sources out of package discovery after bun install places the
// CLI under node_modules/@quickgui/cli. `@` is not a valid import-path character.
const goFiles = [
  ["package.json", "package.json"],
  ["quickgui.config.ts", "quickgui.config.ts"],
  ["go.mod.tmpl", "go.mod"],
  ["gitignore", ".gitignore"],
  ["README.md", "README.md"],
  ["main.go.tmpl", "main.go"],
] as const;
const typescriptFiles = [
  ["package.json", "package.json"],
  ["quickgui.config.ts", "quickgui.config.ts"],
  ["tsconfig.json", "tsconfig.json"],
  ["app.tsx", "app.tsx"],
  ["gitignore", ".gitignore"],
  ["README.md", "README.md"],
] as const;
const rustFiles = [
  ["package.json", "package.json"],
  ["quickgui.config.ts", "quickgui.config.ts"],
  ["Cargo.toml", "Cargo.toml"],
  ["src/main.rs", "src/main.rs"],
  ["gitignore", ".gitignore"],
  ["README.md", "README.md"],
] as const;

export async function initProject(options: InitProjectOptions): Promise<string> {
  const destination = resolve(options.directory);
  if (existsSync(destination) && !statSync(destination).isDirectory()) {
    throw new CliError(`Project destination is not a directory: ${destination}`);
  }
  if (existsSync(destination) && readdirSync(destination).length > 0) {
    throw new CliError(`Project destination is not empty: ${destination}`);
  }

  const defaultName = displayName(basename(destination));
  const name = options.name?.trim() || defaultName;
  const identifier = options.identifier?.trim() || `com.example.${identifierSegment(name)}`;
  resolveConfig({ name, identifier }, destination);

  const language = options.language ?? "go";
  const replacements: Record<string, string> = {
    "{{APP_NAME}}": JSON.stringify(name),
    "{{IDENTIFIER}}": JSON.stringify(identifier),
    "{{PACKAGE_NAME}}": JSON.stringify(packageName(name)),
    "{{CRATE_NAME}}": crateName(name),
    "{{GO_MODULE}}": `example.com/${packageName(name)}`,
    "{{README_TITLE}}": name.replaceAll("\n", " ").replaceAll("\r", " "),
  };

  mkdirSync(destination, { recursive: true });
  const templateRoot = fileURLToPath(
    new URL(`../templates/${templateDirectory(language)}/`, import.meta.url),
  );
  for (const [sourceName, targetName] of templateFiles(language)) {
    const source = join(templateRoot, sourceName);
    if (!existsSync(source)) throw new CliError(`CLI template is missing: ${source}`);
    const target = join(destination, targetName);
    mkdirSync(dirname(target), { recursive: true });
    let contents = readFileSync(source, "utf8");
    for (const [token, value] of Object.entries(replacements)) {
      contents = contents.replaceAll(token, value);
    }
    writeFileSync(target, contents);
  }

  if (options.install) {
    const child = Bun.spawn(["bun", "install"], {
      cwd: destination,
      stdin: "inherit",
      stdout: "inherit",
      stderr: "inherit",
    });
    const status = await child.exited;
    if (status !== 0) {
      throw new CliError(
        `Project created at ${destination}, but \`bun install\` failed with status ${status}`,
      );
    }
    const argv =
      language === "typescript"
        ? ["bun", "run", "check"]
        : language === "rust"
          ? ["cargo", "generate-lockfile"]
          : ["go", "mod", "tidy"];
    if (language === "rust" && !Bun.which("cargo")) {
      throw new CliError(
        `Project created at ${destination}, but cargo is required to finish a Rust project`,
      );
    }
    const prepare = Bun.spawn(argv, {
      cwd: destination,
      stdin: "inherit",
      stdout: "inherit",
      stderr: "inherit",
      env: { ...process.env, CGO_ENABLED: "0" },
    });
    if ((await prepare.exited) !== 0)
      throw new CliError(`Project created at ${destination}, but ${argv.join(" ")} failed`);
  }

  return destination;
}

function templateDirectory(language: Language): "native" | "typescript" | "rust" {
  if (language === "typescript") return "typescript";
  if (language === "rust") return "rust";
  return "native";
}

function templateFiles(language: Language): readonly (readonly [string, string])[] {
  if (language === "typescript") return typescriptFiles;
  if (language === "rust") return rustFiles;
  return goFiles;
}

function displayName(value: string): string {
  const words = value
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .split(/[^A-Za-z0-9]+/)
    .filter(Boolean);
  return words.length > 0
    ? words.map((word) => word[0]!.toUpperCase() + word.slice(1)).join(" ")
    : "QuickGUI App";
}

function packageName(value: string): string {
  const name = value
    .normalize("NFKD")
    .toLowerCase()
    .replace(/[^a-z0-9._-]+/g, "-")
    .replace(/^[._-]+|[._-]+$/g, "")
    .slice(0, 214);
  if (!name) throw new CliError(`Could not derive a package name from ${JSON.stringify(value)}`);
  return name;
}

function crateName(value: string): string {
  const name = packageName(value).replace(/[._]+/g, "-");
  return /^[A-Za-z]/.test(name) ? name : `app-${name}`;
}

function identifierSegment(value: string): string {
  return packageName(value).replace(/[._]+/g, "-");
}

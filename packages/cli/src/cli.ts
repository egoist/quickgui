#!/usr/bin/env bun

import { existsSync } from "node:fs";
import { resolve } from "node:path";

import { parseCliArgs, type HelpTopic, type ParsedCliCommand } from "./args.ts";
import { buildProject } from "./build.ts";
import { loadConfig, type Language } from "./config.ts";
import { runDev } from "./dev.ts";
import { CliError, errorMessage } from "./error.ts";
import { initProject } from "./init.ts";
import { initExtension } from "./init-extension.ts";
import { findMinisignTool } from "./packaging/pipeline.ts";
import { generateUpdaterKeys } from "./packaging/appcast.ts";
import { minisignKeygenArguments } from "./packaging/updates.ts";
import { hostTarget } from "./targets.ts";

export const CLI_VERSION = "0.1.3";

export async function runCli(argv: string[]): Promise<number> {
  const command = parseCliArgs(argv);
  switch (command.command) {
    case "help":
      console.log(helpText(command.topic));
      return 0;
    case "version":
      console.log(CLI_VERSION);
      return 0;
    case "init": {
      let language = command.language;
      if (language === undefined) {
        if (!process.stdin.isTTY || !process.stdout.isTTY) {
          throw new CliError(
            "Choosing a language requires an interactive terminal. Pass --language go, --language rust, or --language typescript.",
          );
        }
        const { cancel, select } = await import("@clack/prompts");
        const selected = await select<Language>({
          message: "Which language would you like to use?",
          initialValue: "go",
          options: [
            { value: "go", label: "Go" },
            { value: "typescript", label: "TypeScript", hint: "Bun and Solid" },
            { value: "rust", label: "Rust", hint: "native crate" },
          ],
        });
        if (typeof selected === "symbol") {
          cancel("Project creation cancelled.");
          return 130;
        }
        language = selected;
      }
      const destination = await initProject({ ...command, language });
      console.log(`\nCreated QuickGUI project at ${destination}`);
      console.log(`\n  cd ${relativeDisplayPath(destination)}`);
      if (!command.install) console.log("  bun install");
      console.log("  bun run dev");
      return 0;
    }
    case "init-extension": {
      const destination = await initExtension(command);
      console.log(
        `\nCreated ${command.type === "go" ? "pure Go" : command.type} extension at ${destination}`,
      );
      const path = relativeDisplayPath(destination).replaceAll("'", "'\"'\"'");
      console.log(`\n  cd '${path}'`);
      if (!command.install) {
        console.log("  bun install");
        console.log("  go mod tidy");
      }
      console.log("  bun run dev");
      return 0;
    }
    case "dev":
      return await runDev(command);
    case "build":
      return await runBuild(command);
    case "check":
    case "test": {
      const project = resolve(command.project);
      if (
        ["quickgui.toml", "quickgui.config.ts"].some((file) =>
          existsSync(resolve(project, file)),
        )
      ) {
        const config = await loadConfig(project);
        if (config.language === "typescript") {
          const { runTypeScript } = await import("./typescript-build.ts");
          return runTypeScript(project, command.command);
        }
        if (config.language === "rust") {
          const { runRust, rustManifestPath } = await import("./rust-build.ts");
          return runRust(project, command.command, {
            release: command.release,
            manifestPath: rustManifestPath(config),
          });
        }
      }
      const { runGo } = await import("./go-build.ts");
      return runGo(project, command.command, command.release);
    }
    case "fmt": {
      const project = resolve(command.project);
      const hasConfig = ["quickgui.toml", "quickgui.config.ts"].some((file) => existsSync(resolve(project, file)));
      const config = hasConfig ? await loadConfig(project) : undefined;
      const language = config?.language ?? "go";
      if (language === "typescript") {
        const { runTypeScript } = await import("./typescript-build.ts");
        return runTypeScript(project, "fmt", command.check);
      }
      if (language === "rust" && config) {
        const { runRust, rustManifestPath } = await import("./rust-build.ts");
        return runRust(project, "fmt", {
          check: command.check,
          manifestPath: rustManifestPath(config),
        });
      }
      const child = Bun.spawn(["go", "run", "github.com/egoist/quickgui/go/cmd/quickguifmt", command.check ? "-check" : "-w", "."], {
        cwd: project, env: { ...process.env, CGO_ENABLED: "0" }, stdin: "inherit", stdout: "inherit", stderr: "inherit",
      });
      return await child.exited;
    }
    case "keygen":
      return await runKeygen(command);
  }
}

async function runKeygen(
  command: Extract<ParsedCliCommand, { command: "keygen" }>,
): Promise<number> {
  if (command.sparkle) {
    if (!command.passwordless)
      throw new CliError(
        "Sparkle keys are raw Ed25519 keys; store the private key in your CI secret store",
      );
    const paths = generateUpdaterKeys(resolve(command.outDir), command.force);
    console.log("[quickgui] Wrote " + paths.publicKeyPath);
    console.log("[quickgui] Wrote " + paths.secretKeyPath);
    console.log(
      "Set updates.publicKey to the public key, and keep the secret key outside version control.",
    );
    return 0;
  }
  const tool = findMinisignTool();
  if (!tool) {
    throw new CliError(
      "Creating an update signing key needs `minisign` or `rsign` on PATH. Install one from " +
        "https://jedisct1.github.io/minisign/ and re-run `quickgui keygen`.",
    );
  }
  const outDir = resolve(command.outDir);
  const publicKeyPath = resolve(outDir, "quickgui-update.pub");
  const secretKeyPath = resolve(outDir, "quickgui-update.key");
  for (const path of [publicKeyPath, secretKeyPath]) {
    if (existsSync(path) && !command.force) {
      throw new CliError(`${path} already exists. Pass --force to overwrite it.`);
    }
  }
  const argv = minisignKeygenArguments(tool, publicKeyPath, secretKeyPath, command.passwordless);
  const child = Bun.spawn(argv, {
    cwd: outDir,
    stdin: "inherit",
    stdout: "inherit",
    stderr: "inherit",
  });
  const status = await child.exited;
  if (status !== 0) throw new CliError(`Command failed: ${argv.join(" ")}`);
  console.log(`\n[quickgui] Wrote ${publicKeyPath}`);
  console.log(`[quickgui] Wrote ${secretKeyPath}`);
  console.log(
    "\nKeep the secret key out of version control. Pass its path through " +
      "`updates.minisignSecretKey` or QUICKGUI_MINISIGN_SECRET_KEY, and embed the public key in " +
      "your application's UpdateClient.",
  );
  return 0;
}

async function runBuild(command: Extract<ParsedCliCommand, { command: "build" }>): Promise<number> {
  const projectRoot = resolve(command.project);
  const config = await loadConfig(projectRoot, command.configFile);
  const target = command.target ?? config.target ?? hostTarget();
  console.log(`[quickgui] Building ${config.name} for ${target}`);
  const result = await buildProject(config, {
    mode: "production",
    target,
    updateManifest: command.updateManifest || (config.updates?.manifest ?? false),
    macAppStore: command.macAppStore,
    ...(command.outDir ? { outDir: command.outDir } : {}),
    ...(command.signingIdentity ? { signingIdentity: command.signingIdentity } : {}),
    ...(command.updateBaseUrl ? { updateBaseUrl: command.updateBaseUrl } : {}),
    ...(command.notarizationProfile
      ? { notarization: { keychainProfile: command.notarizationProfile } }
      : {}),
  });
  console.log(`[quickgui] Created ${result.artifactPath}`);
  if (result.dmgPath) console.log(`[quickgui] Created ${result.dmgPath}`);
  for (const path of result.packagePaths ?? []) console.log(`[quickgui] Created ${path}`);
  if (result.updateArtifactPath) {
    console.log(`[quickgui] Signed ${result.updateArtifactPath}`);
  }
  if (result.manifestPath) console.log(`[quickgui] Created ${result.manifestPath}`);
  for (const note of result.notes ?? []) console.log(`[quickgui] ${note}`);
  return 0;
}

function helpText(topic?: HelpTopic): string {
  if (topic === "init-extension") {
    return `Usage: quickgui init-extension [directory] [options]

Create an extension with a runnable Go demo. Zig and Rust templates include a
standalone native service library, a Go wrapper, and Bun build scripts.

Options:
  --type <go|zig|rust> Extension language (default: go)
  --name <name>              Extension name (default: directory name)
  --module <path>            Go module path (default: example.com/<name>)
  --npm-package <name>       Native artifact package (default: quickgui-extension-<name>)
  --no-install               Skip bun install and go mod tidy
  -h, --help                 Show this help`;
  }
  if (topic === "keygen") {
    return `Usage: quickgui keygen [options]

Create an update signing key pair. Use --sparkle for the Go updater extension.

Options:
  --sparkle                  Generate a Sparkle-compatible Ed25519 key pair
  --out-dir <directory>      Where to write the key pair (default: .)
  --force                    Overwrite an existing key pair
  --password                 Encrypt the secret key with a password
  -h, --help                 Show this help`;
  }
  if (topic === "init") {
    return `Usage: quickgui init [directory] [options]

Create a QuickGUI project compiled to native code.

Options:
  --language <go|rust|typescript> Application language (prompts when omitted)
  --name <name>              Application display name
  --identifier <id>          Reverse-DNS bundle identifier
  --no-install               Skip bun install and language setup
  -h, --help                 Show this help`;
  }
  if (topic === "dev") {
    return `Usage: quickgui dev [options]

Compile the application to a native development app, run it, and rebuild and restart it on
source changes. On macOS the app is a signed .app bundle.

Options:
  --project <directory>      Project directory (default: .)
  --config <file>            Config file (auto: quickgui.toml, then quickgui.config.ts)
  --target <target>          Host target override
  --sign <identity>          macOS signing identity (default: ad-hoc)
  --once                     Run without watching
  --no-launch                Only create the development app
  -h, --help                 Show this help`;
  }
  if (topic === "build") {
    return `Usage: quickgui build [options]

Build a self-contained production application for a target platform.

Options:
  --project <directory>      Project directory (default: .)
  --config <file>            Config file (auto: quickgui.toml, then quickgui.config.ts)
  --target <target>          darwin-arm64, darwin-x64, linux-arm64,
                            linux-x64, windows-arm64, or windows-x64
  --out-dir <directory>      Output directory override
  --sign <identity>          macOS signing identity (default: ad-hoc)
  --notarize <profile>       Notary Keychain profile for the macOS DMG
  --mas                      Sign for the Mac App Store and build a .pkg
  --update-manifest          Sign the artifact and write an appcast (or legacy latest.json)
  --update-base-url <url>    Publication URL for the signed update artifact
  -h, --help                 Show this help`;
  }
  if (topic === "fmt") {
    return `Usage: quickgui fmt [options]

Format Go with the QuickGUI SDK formatter, TypeScript with the project formatter, or Rust with rustfmt.
The language is selected from the project's config.

Options:
  --project <directory>      Project directory (default: .)
  --check                    Report unformatted files without writing
  -h, --help                 Show this help`;
  }
  if (topic === "check" || topic === "test") {
    return `Usage: quickgui ${topic} [options]\n\n${topic === "check" ? "Type-check" : "Test"} Go, TypeScript, or Rust using the same build pipeline as dev and build.\n\nOptions:\n  --project <directory>      Project directory (default: .)\n  --release                  Use the release profile\n  -h, --help                 Show this help`;
  }
  return `QuickGUI CLI ${CLI_VERSION}

Usage: quickgui <command> [options]

Commands:
  init [directory]           Create a new project
  init-extension [directory] Create a Go, Zig, or Rust extension
  dev                        Run a native app with source reload
  build                      Package a production application
  fmt                        Format Go, TypeScript, or Rust source
  check                      Type-check compiled Go, TypeScript, or Rust views
  test                       Test compiled Go, TypeScript, or Rust views
  keygen                     Create a Minisign update signing key pair

Run \`quickgui help <command>\` for command-specific help.`;
}

function relativeDisplayPath(path: string): string {
  const current = process.cwd();
  return path.startsWith(`${current}/`) ? path.slice(current.length + 1) : path;
}

if (import.meta.main) {
  try {
    process.exitCode = await runCli(process.argv.slice(2));
  } catch (error) {
    console.error(`quickgui: ${errorMessage(error)}`);
    process.exitCode = error instanceof CliError ? error.exitCode : 1;
  }
}

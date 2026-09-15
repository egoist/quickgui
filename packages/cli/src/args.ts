import { CliError } from "./error.ts";
import { parseLanguage, type Language } from "./config.ts";
import { parseExtensionType, type InitExtensionOptions } from "./init-extension.ts";
import { parseTarget, type QuickGuiTarget } from "./targets.ts";

export type HelpTopic =
  | "pack-languages"
  | "init"
  | "init-extension"
  | "dev"
  | "build"
  | "keygen"
  | "fmt"
  | "check"
  | "test";

export type ParsedCliCommand =
  | { command: "pack-languages"; config: string; output: string; languages?: string[] }
  | { command: "help"; topic?: HelpTopic }
  | { command: "version" }
  | { command: "fmt"; project: string; check: boolean }
  | { command: "check" | "test"; project: string; release: boolean }
  | ({ command: "init-extension" } & InitExtensionOptions)
  | {
      command: "init";
      language?: Language;
      directory: string;
      install: boolean;
      name?: string;
      identifier?: string;
    }
  | {
      command: "dev";
      project: string;
      configFile?: string;
      once: boolean;
      launch: boolean;
      target?: QuickGuiTarget;
      signingIdentity?: string;
    }
  | {
      command: "build";
      project: string;
      configFile?: string;
      target?: QuickGuiTarget;
      outDir?: string;
      signingIdentity?: string;
      notarizationProfile?: string;
      updateManifest: boolean;
      updateBaseUrl?: string;
      macAppStore: boolean;
    }
  | {
      command: "keygen";
      sparkle?: boolean;
      outDir: string;
      force: boolean;
      passwordless: boolean;
    };

interface OptionSpec {
  key: string;
  value: boolean;
}

interface ParsedOptions {
  values: Map<string, string | true>;
  positionals: string[];
}

export function parseCliArgs(argv: string[]): ParsedCliCommand {
  if (argv.length === 0 || argv[0] === "--help" || argv[0] === "-h") {
    return { command: "help" };
  }
  if (argv[0] === "--version" || argv[0] === "-v") return { command: "version" };

  const command = argv[0];
  const rest = argv.slice(1);
  const helpTopics: readonly string[] = [
    "pack-languages",
    "init",
    "init-extension",
    "dev",
    "build",
    "keygen",
    "fmt",
    "check",
    "test",
  ];
  if (command === "help") {
    if (rest.length > 1 || (rest[0] && !helpTopics.includes(rest[0]))) {
      throw new CliError(
        "Usage: quickgui help [init|init-extension|dev|build|keygen|fmt|check|test|pack-languages]",
      );
    }
    return rest[0] ? { command: "help", topic: rest[0] as HelpTopic } : { command: "help" };
  }
  if (rest.includes("--help") || rest.includes("-h")) {
    if (command !== undefined && helpTopics.includes(command)) {
      return { command: "help", topic: command as HelpTopic };
    }
  }

  if (command === "check" || command === "test") {
    const parsed = parseOptions(rest, {
      "--project": { key: "project", value: true },
      "--release": { key: "release", value: false },
    });
    rejectPositionals(parsed, `quickgui ${command}`);
    return {
      command,
      project: stringOption(parsed, "project") ?? ".",
      release: parsed.values.has("release"),
    };
  }

  if (command === "fmt") {
    const parsed = parseOptions(rest, {
      "--project": { key: "project", value: true },
      "--check": { key: "check", value: false },
    });
    rejectPositionals(parsed, "quickgui fmt");
    return {
      command: "fmt",
      project: stringOption(parsed, "project") ?? ".",
      check: parsed.values.has("check"),
    };
  }

  if (command === "pack-languages") {
    const parsed = parseOptions(rest, {"--out":{key:"output",value:true},"--languages":{key:"languages",value:true}});
    if (parsed.positionals.length !== 1) throw new CliError("Usage: quickgui pack-languages <config.json> --out <languages.qglang>");
    const output = stringOption(parsed,"output");
    if (!output) throw new CliError("pack-languages requires --out");
    const selected = stringOption(parsed,"languages");
    return {command:"pack-languages",config:parsed.positionals[0]!,output,...(selected?{languages:selected.split(",").map(name=>name.trim())}:{})};
  }

  if (command === "init-extension") {
    const parsed = parseOptions(rest, {
      "--type": { key: "type", value: true },
      "--name": { key: "name", value: true },
      "--module": { key: "module", value: true },
      "--npm-package": { key: "npmPackage", value: true },
      "--no-install": { key: "noInstall", value: false },
    });
    if (parsed.positionals.length > 1)
      throw new CliError("Usage: quickgui init-extension [directory] [--type go|zig|rust]");
    const type = parseExtensionType(stringOption(parsed, "type") ?? "go");
    const name = stringOption(parsed, "name");
    const module = stringOption(parsed, "module");
    const npmPackage = stringOption(parsed, "npmPackage");
    if (type === "go" && npmPackage !== undefined)
      throw new CliError("--npm-package is only used by Zig and Rust extensions");
    return {
      command: "init-extension",
      directory: parsed.positionals[0] ?? "quickgui-extension",
      type,
      install: !parsed.values.has("noInstall"),
      ...(name ? { name } : {}),
      ...(module ? { module } : {}),
      ...(npmPackage ? { npmPackage } : {}),
    };
  }

  if (command === "init") {
    const parsed = parseOptions(rest, {
      "--language": { key: "language", value: true },
      "--frontend": { key: "language", value: true },
      "--name": { key: "name", value: true },
      "--identifier": { key: "identifier", value: true },
      "--no-install": { key: "noInstall", value: false },
    });
    if (parsed.positionals.length > 1) throw new CliError("Usage: quickgui init [directory]");
    const name = stringOption(parsed, "name");
    const identifier = stringOption(parsed, "identifier");
    const language = stringOption(parsed, "language");
    return {
      command: "init",
      ...(language ? { language: parseLanguage(language) } : {}),
      directory: parsed.positionals[0] ?? "quickgui-app",
      install: !parsed.values.has("noInstall"),
      ...(name ? { name } : {}),
      ...(identifier ? { identifier } : {}),
    };
  }

  if (command === "dev") {
    const parsed = parseOptions(rest, {
      "--project": { key: "project", value: true },
      "--config": { key: "configFile", value: true },
      "--target": { key: "target", value: true },
      "--sign": { key: "signingIdentity", value: true },
      "--once": { key: "once", value: false },
      "--no-launch": { key: "noLaunch", value: false },
    });
    rejectPositionals(parsed, "quickgui dev");
    const target = stringOption(parsed, "target");
    const signingIdentity = stringOption(parsed, "signingIdentity");
    const configFile = stringOption(parsed, "configFile");
    return {
      command: "dev",
      project: stringOption(parsed, "project") ?? ".",
      ...(configFile ? { configFile } : {}),
      once: parsed.values.has("once"),
      launch: !parsed.values.has("noLaunch"),
      ...(target ? { target: parseTarget(target) } : {}),
      ...(signingIdentity ? { signingIdentity } : {}),
    };
  }

  if (command === "build") {
    const parsed = parseOptions(rest, {
      "--project": { key: "project", value: true },
      "--config": { key: "configFile", value: true },
      "--target": { key: "target", value: true },
      "--out-dir": { key: "outDir", value: true },
      "--sign": { key: "signingIdentity", value: true },
      "--notarize": { key: "notarizationProfile", value: true },
      "--update-manifest": { key: "updateManifest", value: false },
      "--update-base-url": { key: "updateBaseUrl", value: true },
      "--mas": { key: "macAppStore", value: false },
    });
    rejectPositionals(parsed, "quickgui build");
    const target = stringOption(parsed, "target");
    const outDir = stringOption(parsed, "outDir");
    const configFile = stringOption(parsed, "configFile");
    const signingIdentity = stringOption(parsed, "signingIdentity");
    const notarizationProfile = stringOption(parsed, "notarizationProfile");
    const updateBaseUrl = stringOption(parsed, "updateBaseUrl");
    if (updateBaseUrl !== undefined && !/^https:\/\/[^\s"']+$/.test(updateBaseUrl)) {
      throw new CliError("--update-base-url must be an HTTPS URL");
    }
    return {
      command: "build",
      project: stringOption(parsed, "project") ?? ".",
      ...(configFile ? { configFile } : {}),
      updateManifest: parsed.values.has("updateManifest") || updateBaseUrl !== undefined,
      macAppStore: parsed.values.has("macAppStore"),
      ...(target ? { target: parseTarget(target) } : {}),
      ...(outDir ? { outDir } : {}),
      ...(signingIdentity ? { signingIdentity } : {}),
      ...(notarizationProfile ? { notarizationProfile } : {}),
      ...(updateBaseUrl ? { updateBaseUrl } : {}),
    };
  }

  if (command === "keygen") {
    const parsed = parseOptions(rest, {
      "--out-dir": { key: "outDir", value: true },
      "--force": { key: "force", value: false },
      "--password": { key: "password", value: false },
      "--sparkle": { key: "sparkle", value: false },
    });
    rejectPositionals(parsed, "quickgui keygen");
    return {
      command: "keygen",
      ...(parsed.values.has("sparkle") ? { sparkle: true } : {}),
      outDir: stringOption(parsed, "outDir") ?? ".",
      force: parsed.values.has("force"),
      passwordless: !parsed.values.has("password"),
    };
  }

  throw new CliError(`Unknown command: ${command}\nRun \`quickgui --help\` for usage.`);
}

function parseOptions(argv: string[], specs: Record<string, OptionSpec>): ParsedOptions {
  const values = new Map<string, string | true>();
  const positionals: string[] = [];
  let positionalOnly = false;

  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index]!;
    if (!positionalOnly && token === "--") {
      positionalOnly = true;
      continue;
    }
    if (positionalOnly || !token.startsWith("-")) {
      positionals.push(token);
      continue;
    }
    const equal = token.indexOf("=");
    const name = equal === -1 ? token : token.slice(0, equal);
    const inlineValue = equal === -1 ? undefined : token.slice(equal + 1);
    const spec = specs[name];
    if (!spec) throw new CliError(`Unknown option: ${name}`);
    if (values.has(spec.key)) throw new CliError(`Option may only be specified once: ${name}`);
    if (!spec.value) {
      if (inlineValue !== undefined) throw new CliError(`Option does not take a value: ${name}`);
      values.set(spec.key, true);
      continue;
    }
    const value = inlineValue ?? argv[++index];
    if (value === undefined || value.length === 0) {
      throw new CliError(`Option requires a value: ${name}`);
    }
    if (inlineValue === undefined && specs[value]) {
      throw new CliError(`Option requires a value: ${name}`);
    }
    values.set(spec.key, value);
  }

  return { values, positionals };
}

function stringOption(parsed: ParsedOptions, key: string): string | undefined {
  const value = parsed.values.get(key);
  return typeof value === "string" ? value : undefined;
}

function rejectPositionals(parsed: ParsedOptions, usage: string): void {
  if (parsed.positionals.length > 0) {
    throw new CliError(`${usage} does not accept positional arguments`);
  }
}

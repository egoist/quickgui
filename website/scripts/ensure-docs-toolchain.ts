import { readFileSync } from "node:fs";
import { homedir } from "node:os";
import { resolve } from "node:path";

const root = resolve(import.meta.dir, "../..");
export const WASM_TARGET = "wasm32-unknown-unknown";
export const RUSTUP_INSTALL_COMMAND = [
  "bash",
  "-euo",
  "pipefail",
  "-c",
  "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal --no-modify-path --target wasm32-unknown-unknown",
];

export function isCiEnvironment(env: Record<string, string | undefined> = process.env) {
  return [env.CI, env.WORKERS_CI, env.CF_PAGES].some(
    (value) => Boolean(value) && value !== "0" && value !== "false",
  );
}

export function docsWasmBindgenVersion(repoRoot = root) {
  const match = /(?:^|\n)wasm-bindgen\s*=\s*"=?([^"]+)"/.exec(
    readFileSync(resolve(repoRoot, "crates/quickgui-docs-demo/Cargo.toml"), "utf8"),
  );
  if (!match) throw new Error("wasm-bindgen version missing from crates/quickgui-docs-demo/Cargo.toml");
  return match[1];
}

export function docsMsrv(repoRoot = root) {
  const match = /^rust-version = "([^"]+)"/m.exec(readFileSync(resolve(repoRoot, "Cargo.toml"), "utf8"));
  if (!match) throw new Error("rust-version missing from Cargo.toml");
  return match[1];
}

function parseSemverPrefix(version: string) {
  const match = /(\d+)\.(\d+)(?:\.(\d+))?/.exec(version);
  return match ? [Number(match[1]), Number(match[2]), Number(match[3] ?? 0)] : null;
}

export function rustcMeetsMsrv(rustcVersionOutput: string, msrv: string) {
  const have = parseSemverPrefix(rustcVersionOutput);
  const need = parseSemverPrefix(msrv);
  if (!have || !need) return false;
  for (let i = 0; i < 3; i++) {
    if (have[i]! > need[i]!) return true;
    if (have[i]! < need[i]!) return false;
  }
  return true;
}

export function cargoBinDir(env: Record<string, string | undefined> = process.env) {
  return resolve(env.CARGO_HOME || resolve(env.HOME || env.USERPROFILE || homedir(), ".cargo"), "bin");
}

export function prependCargoBin(env: Record<string, string | undefined> = process.env) {
  const bin = cargoBinDir(env);
  const sep = process.platform === "win32" ? ";" : ":";
  const path = env.PATH ?? "";
  if (!path.split(sep).includes(bin)) env.PATH = path ? `${bin}${sep}${path}` : bin;
  return bin;
}

export type EnsureDocsToolchain = {
  env?: Record<string, string | undefined>;
  which?: (name: string) => string | null;
  run?: (args: string[]) => Promise<void>;
  output?: (args: string[]) => Promise<string>;
  version?: string;
};

async function spawn(args: string[], env: Record<string, string | undefined>, capture: boolean) {
  const child = Bun.spawn(args, {
    env,
    stdout: capture ? "pipe" : "inherit",
    stderr: capture ? "pipe" : "inherit",
  });
  const code = await child.exited;
  const stdout = capture ? await new Response(child.stdout).text() : "";
  const stderr = capture ? await new Response(child.stderr).text() : "";
  if (code) {
    throw new Error(`Command failed (${code}): ${args.join(" ")}${stderr ? `\n${stderr}` : ""}`);
  }
  return stdout;
}

export async function ensureDocsToolchain(options: EnsureDocsToolchain = {}) {
  const env = options.env ?? process.env;
  if (!isCiEnvironment(env)) return;
  prependCargoBin(env);
  const which = options.which ?? ((name) => Bun.which(name, { PATH: env.PATH }));
  const run = options.run ?? ((args) => spawn(args, env, false).then(() => undefined));
  const output = options.output ?? ((args) => spawn(args, env, true));
  const version = options.version ?? docsWasmBindgenVersion();
  if (!which("cargo") || !which("rustup")) {
    console.log("CI: installing Rust (rustup, cargo, wasm32-unknown-unknown)");
    await run(RUSTUP_INSTALL_COMMAND);
    prependCargoBin(env);
  } else {
    const rustc = which("rustc") ? await output(["rustc", "--version"]) : "";
    const msrv = docsMsrv();
    if (!rustcMeetsMsrv(rustc, msrv)) {
      console.log(`CI: updating Rust to satisfy MSRV ${msrv}`);
      await run([
        "rustup",
        "toolchain",
        "install",
        "stable",
        "--profile",
        "minimal",
        "--target",
        WASM_TARGET,
      ]);
      await run(["rustup", "default", "stable"]);
    }
  }
  await run(["rustup", "target", "add", WASM_TARGET]);
  const installed = which("wasm-bindgen") ? await output(["wasm-bindgen", "--version"]) : "";
  if (!installed.includes(version)) {
    console.log(`CI: installing wasm-bindgen-cli ${version}`);
    await run(["cargo", "install", "wasm-bindgen-cli", "--version", version, "--locked"]);
  }
}

if (import.meta.main) await ensureDocsToolchain();

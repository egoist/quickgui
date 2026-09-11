import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import {
  RUSTUP_INSTALL_COMMAND,
  WASM_TARGET,
  cargoBinDir,
  docsMsrv,
  docsWasmBindgenVersion,
  ensureDocsToolchain,
  isCiEnvironment,
  prependCargoBin,
  rustcMeetsMsrv,
} from "../scripts/ensure-docs-toolchain";

const website = resolve(import.meta.dir, "..");
const version = docsWasmBindgenVersion();
const msrv = docsMsrv();
const currentRustc = `rustc ${msrv}.0 (000000000 2026-01-01)`;

function commandOutput(args: string[], rustc = currentRustc, bindgen = "") {
  if (args[0] === "rustc") return rustc;
  if (args[0] === "wasm-bindgen") return bindgen;
  return "";
}

test("the wasm-bindgen CLI version is pinned next to the docs demo crate", () => {
  expect(version).toMatch(/^\d+\.\d+\.\d+$/);
  expect(msrv).toMatch(/^\d+\.\d+/);
  expect(rustcMeetsMsrv("rustc 1.83.0 (90b35a623 2024-11-26)", msrv)).toBe(false);
  expect(rustcMeetsMsrv(currentRustc, msrv)).toBe(true);
  expect(readFileSync(resolve(website, "README.md"), "utf8")).toContain(
    `cargo install wasm-bindgen-cli --version ${version} --locked`,
  );
});

test("CI includes Cloudflare Workers Builds and Pages", () => {
  expect(isCiEnvironment({})).toBe(false);
  expect(isCiEnvironment({ CI: "" })).toBe(false);
  expect(isCiEnvironment({ CI: "false" })).toBe(false);
  expect(isCiEnvironment({ CI: "true" })).toBe(true);
  expect(isCiEnvironment({ WORKERS_CI: "1" })).toBe(true);
  expect(isCiEnvironment({ CF_PAGES: "1" })).toBe(true);
});

test("deploy puts cargo bin on PATH before build and wrangler", () => {
  const pkg = JSON.parse(readFileSync(resolve(website, "package.json"), "utf8")) as {
    scripts: Record<string, string>;
  };
  expect(pkg.scripts.deploy).toBe("bun ./scripts/deploy.ts");
  const deploy = readFileSync(resolve(website, "scripts/deploy.ts"), "utf8");
  expect(deploy).toContain("ensureDocsToolchain");
  expect(deploy).toContain('["bun", "run", "build"]');
  expect(deploy).toContain("wrangler");
  expect(readFileSync(resolve(website, "scripts/build-demos.ts"), "utf8")).toContain(
    "ensureDocsToolchain",
  );
});

test("local runs skip toolchain installs", async () => {
  const commands: string[][] = [];
  const env = { PATH: "/usr/bin", HOME: "/tmp/local-home" };
  await ensureDocsToolchain({
    env,
    which: () => {
      throw new Error("which should not run outside CI");
    },
    run: async (args) => {
      commands.push(args);
    },
  });
  expect(commands).toEqual([]);
  expect(env.PATH).toBe("/usr/bin");
});

test("CI without Rust installs rustup, the wasm target, and wasm-bindgen-cli", async () => {
  const commands: string[][] = [];
  const bins: Record<string, string> = {};
  const env = { CI: "true", HOME: "/tmp/ci-home", PATH: "/usr/bin" };
  await ensureDocsToolchain({
    env,
    version,
    which: (name) => bins[name] ?? null,
    run: async (args) => {
      commands.push(args);
      if (args[0] === "bash") {
        bins.cargo = "/tmp/ci-home/.cargo/bin/cargo";
        bins.rustup = "/tmp/ci-home/.cargo/bin/rustup";
      }
      if (args[0] === "cargo" && args.includes("wasm-bindgen-cli")) {
        bins["wasm-bindgen"] = "/tmp/ci-home/.cargo/bin/wasm-bindgen";
      }
    },
    output: async (args) => commandOutput(args),
  });
  expect(env.PATH?.startsWith(`${cargoBinDir(env)}:`)).toBe(true);
  expect(commands[0]).toEqual(RUSTUP_INSTALL_COMMAND);
  expect(commands).toContainEqual(["rustup", "target", "add", WASM_TARGET]);
  expect(commands).toContainEqual([
    "cargo",
    "install",
    "wasm-bindgen-cli",
    "--version",
    version,
    "--locked",
  ]);
});

test("CI with cargo still installs a missing or mismatched wasm-bindgen CLI", async () => {
  const commands: string[][] = [];
  const env = { WORKERS_CI: "1", HOME: "/tmp/ci-home", PATH: "/usr/bin" };
  await ensureDocsToolchain({
    env,
    version,
    which: (name) => (name === "wasm-bindgen" ? null : `/bin/${name}`),
    run: async (args) => {
      commands.push(args);
    },
    output: async (args) => commandOutput(args),
  });
  expect(commands).not.toContainEqual(RUSTUP_INSTALL_COMMAND);
  expect(commands).toContainEqual(["rustup", "target", "add", WASM_TARGET]);
  expect(commands).toContainEqual([
    "cargo",
    "install",
    "wasm-bindgen-cli",
    "--version",
    version,
    "--locked",
  ]);
});

test("CI skips wasm-bindgen-cli when the pinned version is already on PATH", async () => {
  const commands: string[][] = [];
  const env = { CI: "1", HOME: "/tmp/ci-home", PATH: "/usr/bin" };
  await ensureDocsToolchain({
    env,
    version,
    which: (name) => `/bin/${name}`,
    run: async (args) => {
      commands.push(args);
    },
    output: async (args) => commandOutput(args, currentRustc, `wasm-bindgen ${version}`),
  });
  expect(commands).toEqual([["rustup", "target", "add", WASM_TARGET]]);
});

test("CI upgrades an old rustc before adding the wasm target", async () => {
  const commands: string[][] = [];
  await ensureDocsToolchain({
    env: { CI: "true", HOME: "/tmp/ci-home", PATH: "/usr/bin" },
    version,
    which: (name) => `/bin/${name}`,
    run: async (args) => {
      commands.push(args);
    },
    output: async (args) => commandOutput(args, "rustc 1.83.0 (90b35a623 2024-11-26)", `wasm-bindgen ${version}`),
  });
  expect(commands).toContainEqual([
    "rustup",
    "toolchain",
    "install",
    "stable",
    "--profile",
    "minimal",
    "--target",
    WASM_TARGET,
  ]);
  expect(commands).toContainEqual(["rustup", "default", "stable"]);
  expect(commands).toContainEqual(["rustup", "target", "add", WASM_TARGET]);
  expect(commands.some((args) => args.includes("wasm-bindgen-cli"))).toBe(false);
});

test("prependCargoBin is idempotent", () => {
  const env = { HOME: "/tmp/ci-home", PATH: "/usr/bin" };
  const bin = prependCargoBin(env);
  prependCargoBin(env);
  expect(env.PATH).toBe(`${bin}:/usr/bin`);
});

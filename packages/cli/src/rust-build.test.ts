import { expect, test } from "bun:test";
import {
  chmodSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { delimiter, join, resolve } from "node:path";

import { parseCliArgs } from "./args.ts";
import { loadConfig, resolveConfig } from "./config.ts";
import { initProject } from "./init.ts";
import {
  packagedAppMetadata,
  packagedResourceDir,
  parseCargoExecutable,
  parseCargoCore,
  rustBuildPlan,
  rustUpdaterResources,
  rustManifestPath,
  rustPackagedSidecars,
  rustTargetTriple,
} from "./rust-build.ts";
import { hostTarget } from "./targets.ts";

test("Rust is an application language and maps cargo triples", () => {
  expect(parseCliArgs(["init", "demo", "--language", "rust", "--no-install"])).toMatchObject({
    language: "rust",
    install: false,
  });
  expect(parseCliArgs(["init", "demo", "--frontend", "rust"])).toMatchObject({ language: "rust" });
  expect(
    resolveConfig({ name: "Demo", identifier: "com.example.demo", language: "rust" }, "/example")
      .language,
  ).toBe("rust");
  expect(rustTargetTriple("darwin-arm64")).toBe("aarch64-apple-darwin");
  expect(rustTargetTriple("darwin-x64")).toBe("x86_64-apple-darwin");
  expect(rustTargetTriple("linux-x64")).toBe("x86_64-unknown-linux-gnu");
  expect(rustTargetTriple("windows-arm64")).toBe("aarch64-pc-windows-msvc");
});

test("Rust builds invoke cargo without a shared host library", () => {
  const config = resolveConfig(
    { name: "Rust App", identifier: "dev.test.rust", language: "rust" },
    "/tmp/rust-app",
  );
  const plan = rustBuildPlan({
    config,
    mode: "production",
    target: "darwin-arm64",
    executablePath: "/tmp/Rust App",
    fonts: ["fonts/Test.ttf"],
  });
  expect(plan.argv.slice(0, 2)).toEqual(["cargo", "build"]);
  expect(plan.argv).toContain("--release");
  expect(plan.argv).toContain("--manifest-path");
  expect(plan.argv[0]).not.toBe("go");
  expect(plan.argv.join(" ")).not.toContain("cgo");
  if (hostTarget() === "darwin-arm64") {
    expect(plan.argv).not.toContain("--target");
  } else {
    expect(plan.argv).toContain("aarch64-apple-darwin");
  }
  expect(packagedAppMetadata(config, "development", ["fonts/Test.ttf"])).toEqual({
    name: "Rust App",
    version: "0.1.0",
    identifier: "dev.test.rust.dev",
    fonts: ["fonts/Test.ttf"],
  });
});

test("production Windows Rust builds hide the console", () => {
  const config = resolveConfig(
    { name: "Rust App", identifier: "dev.test.rust", language: "rust" },
    "/tmp/rust-app",
  );
  const plan = rustBuildPlan({
    config,
    mode: "production",
    target: "windows-x64",
    executablePath: "/tmp/app.exe",
    fonts: [],
  });
  expect(plan.env.RUSTFLAGS).toContain("SUBSYSTEM:WINDOWS");
});

test("Cargo artifact JSON reports the built executable", () => {
  expect(
    parseCargoExecutable(
      `${JSON.stringify({
        reason: "compiler-artifact",
        target: { kind: ["bin"], name: "demo" },
        executable: "/tmp/demo",
      })}\n`,
    ),
  ).toBe("/tmp/demo");
  expect(() => parseCargoExecutable("{}\n")).toThrow("did not produce an executable");
});

test("Rust development builds stream Cargo logs", async () => {
  const root = mkdtempSync(join(tmpdir(), "quickgui-rust-cargo-logs-"));
  try {
    mkdirSync(join(root, "src"));
    writeFileSync(
      join(root, "Cargo.toml"),
      '[package]\nname = "cargo-log-test"\nversion = "0.1.0"\nedition = "2024"\n',
    );
    writeFileSync(join(root, "src/main.rs"), "fn main() {}\n");
    writeFileSync(
      join(root, "quickgui.toml"),
      'name = "Cargo Log Test"\nidentifier = "dev.test.cargo-log"\nlanguage = "rust"\nentry = "."\n',
    );
    // Stub cargo so this assertion does not wait on a cold rustc compile.
    const artifact = join(root, "cargo-log-test");
    writeFileSync(artifact, "");
    const cargoHome = join(root, "cargo-bin");
    mkdirSync(cargoHome);
    const cargoShim = join(root, "cargo-shim.ts");
    writeFileSync(
      cargoShim,
      `console.error("   Compiling cargo-log-test v0.1.0");
console.log(${JSON.stringify(
        JSON.stringify({
          reason: "compiler-artifact",
          target: { kind: ["bin"], name: "cargo-log-test" },
          executable: artifact,
        }),
      )});
`,
    );
    const cargo = join(cargoHome, process.platform === "win32" ? "cargo.cmd" : "cargo");
    if (process.platform === "win32") {
      writeFileSync(cargo, `@echo off\r\n"${process.execPath}" "${cargoShim}" %*\r\n`);
    } else {
      writeFileSync(cargo, `#!/bin/sh\nexec "${process.execPath}" "${cargoShim}" "$@"\n`);
      chmodSync(cargo, 0o755);
    }
    const child = Bun.spawn(
      [process.execPath, join(import.meta.dir, "cli.ts"), "dev", "--project", root, "--no-launch"],
      {
        stdin: "ignore",
        stdout: "pipe",
        stderr: "pipe",
        env: {
          ...process.env,
          PATH: `${cargoHome}${delimiter}${process.env.PATH ?? ""}`,
          CARGO_BUILD_RUSTC_WRAPPER: "",
        },
      },
    );
    const [status, stdout, stderr] = await Promise.all([
      child.exited,
      new Response(child.stdout).text(),
      new Response(child.stderr).text(),
    ]);
    expect(status).toBe(0);
    expect(stdout).toContain("[quickgui] Development app:");
    expect(stderr).toContain("Compiling cargo-log-test");
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("Rust scaffold writes a crate and language config", async () => {
  const root = mkdtempSync(join(tmpdir(), "quickgui-rust-init-"));
  try {
    const project = join(root, "sample-app");
    await initProject({
      directory: project,
      language: "rust",
      install: false,
      name: "Sample App",
      identifier: "com.example.sample-app",
    });
    mkdirSync(join(project, "node_modules/@quickgui"), { recursive: true });
    symlinkSync(resolve(import.meta.dir, ".."), join(project, "node_modules/@quickgui/cli"), "dir");
    const config = await loadConfig(project);
    expect(config.language).toBe("rust");
    expect(readFileSync(join(project, "Cargo.toml"), "utf8")).toContain('name = "sample-app"');
    expect(readFileSync(join(project, "src/main.rs"), "utf8")).toContain("Application::new()");
    expect(readFileSync(join(project, "src/main.rs"), "utf8")).toContain("com.example.sample-app");
    expect(rustManifestPath(config)).toBe(join(project, "Cargo.toml"));
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("Rust rejects Go and TypeScript-only native options", () => {
  const input = { name: "Demo", identifier: "com.example.demo", language: "rust" as const };
  expect(() => resolveConfig({ ...input, native: { tags: ["demo"] } }, "/example")).toThrow(
    "Cargo features",
  );
  expect(() =>
    resolveConfig({ ...input, native: { libraryPath: "lib.dylib" } }, "/example"),
  ).toThrow("quickgui crate");
  expect(() => resolveConfig({ ...input, extensions: ["terminal"] }, "/example")).toThrow(
    "Cargo.toml",
  );
});

test("Rust packaging locates Cargo.toml from the project entry", () => {
  const root = mkdtempSync(join(tmpdir(), "quickgui-rust-manifest-"));
  try {
    writeFileSync(join(root, "Cargo.toml"), '[package]\nname = "demo"\nversion = "0.1.0"\n');
    mkdirSync(join(root, "src"));
    writeFileSync(join(root, "src/main.rs"), "fn main() {}");
    const config = resolveConfig(
      { name: "Demo", identifier: "com.example.demo", language: "rust", entry: "." },
      root,
    );
    expect(rustManifestPath(config)).toBe(join(root, "Cargo.toml"));
    expect(
      rustManifestPath(
        resolveConfig(
          { name: "Demo", identifier: "com.example.demo", language: "rust", entry: "src/main.rs" },
          root,
        ),
      ),
    ).toBe(join(root, "Cargo.toml"));
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("Rust sidecars stay inside the macOS bundle and beside other executables", () => {
  expect(packagedResourceDir("/tmp/App.app/Contents/MacOS/App", "darwin-arm64")).toBe(
    "/tmp/App.app/Contents/Resources",
  );
  expect(rustPackagedSidecars("/tmp/App.app/Contents/MacOS/App", "darwin-arm64", ["fonts/A.ttf"])).toEqual(
    [],
  );
  expect(rustPackagedSidecars("/tmp/app", "linux-x64", [])).toEqual(["/tmp/quickgui.json"]);
  expect(rustPackagedSidecars("/tmp/app.exe", "windows-x64", ["fonts/A.ttf"])).toEqual([
    "/tmp/quickgui.json",
    "/tmp/fonts",
  ]);
});

test("Rust builds embed updater settings and detect the core's updater feature", () => {
  const publicKey = Buffer.alloc(32, 7).toString("base64");
  const config = resolveConfig(
    {
      name: "Rust App",
      identifier: "dev.test.rust",
      version: "1.2.3",
      language: "rust",
      updates: { target: "s3", s3: { bucket: "releases", publicUrl: "https://dl.example.com/app" }, publicKey },
    },
    "/tmp/rust-app",
  );
  const plan = rustBuildPlan({
    config,
    mode: "production",
    target: "linux-x64",
    executablePath: "/tmp/app",
    fonts: [],
  });
  expect(
    JSON.parse(Buffer.from(plan.env.QUICKGUI_UPDATER_METADATA!, "base64url").toString("utf8")),
  ).toEqual({
    feedUrl: "https://dl.example.com/app/appcast-linux-x64.xml",
    publicKey,
    currentVersion: "1.2.3",
    identifier: "dev.test.rust",
    automaticChecks: true,
    development: false,
  });
  const artifact = (package_id: string, features: string[], kind = "lib", name = "quickgui") =>
    JSON.stringify({
      reason: "compiler-artifact",
      package_id,
      features,
      target: { kind: [kind], name },
    });
  const stdout = [
    artifact(
      "registry+https://github.com/rust-lang/crates.io-index#quickgui-system@0.1.5",
      [],
      "lib",
      "quickgui_system",
    ),
    "warning: not json",
    artifact("path+file:///work/quickgui#0.1.5", [], "custom-build", "build-script-build"),
    artifact("path+file:///work/quickgui#0.1.5", ["default", "updater"]),
    artifact("path+file:///project#demo@1.2.3", [], "bin", "demo"),
  ].join("\n");
  expect(parseCargoCore(stdout)).toEqual({ version: "0.1.5", features: ["default", "updater"] });
  expect(
    parseCargoCore(
      artifact("registry+https://github.com/rust-lang/crates.io-index#quickgui@0.2.0-beta.1", []),
    ),
  ).toEqual({ version: "0.2.0-beta.1", features: [] });
  expect(
    parseCargoCore(artifact("path+file:///project#demo@1.2.3", [], "bin", "demo")),
  ).toBeUndefined();
  // The CLI ships without the extension package, so it carries the resource list itself.
  const manifest = JSON.parse(
    readFileSync(
      join(import.meta.dir, "../../../extensions/updater/quickgui.extension.json"),
      "utf8",
    ),
  );
  expect(rustUpdaterResources(manifest.version)).toEqual(manifest);
});

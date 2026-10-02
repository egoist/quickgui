import { expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { resolveConfig } from "./config.ts";
import { compileNativeApplication, goBuildPlan, sharedLibraryName } from "./native-build.ts";
import { extensionLibraryName, type ExtensionManifest } from "./extensions.ts";
import { hostTarget, targetInfo } from "./targets.ts";

test("Go builds disable CGO and keep the Rust library out of the link command", () => {
  const config = resolveConfig(
    { name: "Go App", identifier: "dev.test.go", native: { tags: ["demo"] } },
    "/tmp/go-app",
  );
  const plan = goBuildPlan({
    config,
    mode: "development",
    target: "darwin-arm64",
    executablePath: "/tmp/My App",
    fonts: ["fonts/Test.ttf"],
  });
  expect(plan.argv.slice(0, 2)).toEqual(["go", "build"]);
  expect(plan.argv.at(-1)).toBe(".");
  expect(plan.env.CGO_ENABLED).toBe("0");
  expect(plan.env.GOOS).toBe("darwin");
  expect(plan.env.GOARCH).toBe("arm64");
  expect(plan.argv).toContain("demo");
  const flags = plan.argv[plan.argv.indexOf("-ldflags") + 1]!;
  const metadata = JSON.parse(
    Buffer.from(flags.split("buildMetadata=")[1]!, "base64url").toString(),
  );
  expect(metadata.identifier).toBe("dev.test.go.dev");
  expect(metadata.fonts).toEqual(["fonts/Test.ttf"]);
  expect(plan.argv.join(" ")).not.toContain("cargo");
  expect(plan.argv.join(" ")).not.toContain("clang");
});

test("production Go builds strip symbols and map x64 to amd64", () => {
  const config = resolveConfig(
    { name: "Go App", identifier: "dev.test.go", entry: "cmd/app" },
    "/tmp/go-app",
  );
  const plan = goBuildPlan({
    config,
    mode: "production",
    target: "darwin-x64",
    executablePath: "/tmp/app",
    fonts: [],
  });
  expect(plan.argv).toContain("-trimpath");
  expect(plan.argv.at(-1)).toBe("./cmd/app");
  expect(plan.env.GOARCH).toBe("amd64");
  expect(sharedLibraryName("darwin-x64")).toBe("libquickgui_host.dylib");
});

test("Windows Go builds hide the console only in production", () => {
  const config = resolveConfig({ name: "Go App", identifier: "dev.test.go" }, "/tmp/go-app");
  const ldflags = (mode: "development" | "production") => {
    const plan = goBuildPlan({
      config,
      mode,
      target: "windows-x64",
      executablePath: "/tmp/app.exe",
      fonts: [],
    });
    return plan.argv[plan.argv.indexOf("-ldflags") + 1]!;
  };
  // A GUI-subsystem child cannot write to the console `quickgui dev` hands it.
  expect(ldflags("development")).not.toContain("-H=windowsgui");
  expect(ldflags("production")).toContain("-H=windowsgui");
});

test("independent updater names do not require built-in updater configuration, and resources cannot replace another image", async () => {
  const root = mkdtempSync(join(tmpdir(), "quickgui-third-party-build-"));
  const previousDirectory = process.env.QUICKGUI_EXTENSION_DIR;
  delete process.env.QUICKGUI_EXTENSION_DIR;
  try {
    const target = hostTarget();
    const platform = targetInfo(target).platform;
    const core = join(root, sharedLibraryName(target));
    writeFileSync(core, "core artifact");
    writeFileSync(join(root, "go.mod"), "module example.test/extensions\n\ngo 1.23\n");
    const updater: ExtensionManifest = {
      schema: 1,
      abi: 1,
      name: "updater",
      package: "@acme/my-updater",
      version: "7.2.1",
      library: "quickgui_updater",
    };
    const other: ExtensionManifest = {
      ...updater,
      name: "another",
      package: "@acme/another",
      library: "quickgui_another",
      resources: { [platform]: [extensionLibraryName(updater, target)] },
    };
    for (const manifest of [updater, other]) {
      const source = join(root, manifest.name);
      mkdirSync(source);
      writeFileSync(join(source, "extension.go"), `package ${manifest.name}\n`);
      writeFileSync(join(source, "quickgui.extension.json"), JSON.stringify(manifest));
      const artifact = join(root, "node_modules", manifest.package);
      mkdirSync(join(artifact, "lib", target), { recursive: true });
      writeFileSync(
        join(artifact, "package.json"),
        JSON.stringify({
          name: manifest.package,
          version: manifest.version,
          exports: { "./package.json": "./package.json" },
        }),
      );
      writeFileSync(
        join(artifact, "lib", target, extensionLibraryName(manifest, target)),
        "extension: " + manifest.name,
      );
    }
    writeFileSync(
      join(
        root,
        "node_modules",
        other.package,
        "lib",
        target,
        extensionLibraryName(updater, target),
      ),
      "resource pretending to be another library",
    );
    writeFileSync(
      join(root, "main.go"),
      'package main\nimport _ "example.test/extensions/updater"\nfunc main() {}\n',
    );
    const executablePath =
      platform === "darwin"
        ? join(root, "Consumer.app/Contents/MacOS/Consumer")
        : join(root, "bundle", platform === "windows" ? "Consumer.exe" : "Consumer");
    mkdirSync(dirname(executablePath), { recursive: true });
    const options = {
      config: resolveConfig(
        { name: "Consumer", identifier: "test.extensions", native: { libraryPath: core } },
        root,
      ),
      mode: "production" as const,
      target,
      executablePath,
      fonts: [],
    };
    const libraries = await compileNativeApplication(options);
    expect(libraries).toHaveLength(2);
    writeFileSync(
      join(root, "main.go"),
      'package main\nimport (\n _ "example.test/extensions/updater"\n _ "example.test/extensions/another"\n)\nfunc main() {}\n',
    );
    await expect(compileNativeApplication(options)).rejects.toThrow("resource collision");
    expect(readFileSync(libraries[1]!, "utf8")).toBe("extension: updater");
  } finally {
    rmSync(root, { recursive: true, force: true });
    if (previousDirectory === undefined) delete process.env.QUICKGUI_EXTENSION_DIR;
    else process.env.QUICKGUI_EXTENSION_DIR = previousDirectory;
  }
}, 30_000);

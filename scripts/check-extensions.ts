#!/usr/bin/env bun
/** Headless integration proof against the real staged core and optional backend images. */
import {
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { basename, dirname, join, resolve } from "node:path";
import { compileNativeApplication, resolveHostLibrary } from "../packages/cli/src/native-build.ts";
import { resolveConfig } from "../packages/cli/src/config.ts";
import { hostTarget } from "../packages/cli/src/targets.ts";
import { buildExtension } from "../examples/native-extension/build-extension.ts";

const root = resolve(import.meta.dir, "..");
const target = hostTarget();
const version = JSON.parse(readFileSync(join(root, "package.json"), "utf8")).version as string;
async function run(
  argv: string[],
  cwd = root,
  extra: Record<string, string | undefined> = {},
): Promise<string> {
  const child = Bun.spawn(argv, {
    cwd,
    env: { ...process.env, CGO_ENABLED: "0", ...extra },
    stdin: "ignore",
    stdout: "pipe",
    stderr: "pipe",
  });
  const [status, stdout, stderr] = await Promise.all([
    child.exited,
    new Response(child.stdout).text(),
    new Response(child.stderr).text(),
  ]);
  if (status !== 0) throw new Error(`${argv.join(" ")}: ${stderr}\n${stdout}`);
  return stdout;
}
const coreGraph = await run([
  "cargo",
  "tree",
  "-p",
  "quickgui-host",
  "--edges",
  "normal",
  "--prefix",
  "none",
]);
if (
  /^(tree-sitter|imara-diff|pulldown-cmark|libghostty|portable-pty|quickgui-editor|quickgui-markdown|quickgui-terminal|quickgui-updater|ed25519-dalek|minisign-verify)\b/m.test(
    coreGraph,
  )
)
  throw new Error("Core library pulled in an optional backend dependency");
const extensionGraph = await run([
  "cargo",
  "tree",
  "-p",
  "quickgui-terminal",
  "-p", "quickgui-editor", "-p", "quickgui-markdown",
  "--edges",
  "normal",
  "--prefix",
  "none",
]);
if (/^(quickgui |quickgui-host|wgpu|taffy)\b/m.test(extensionGraph))
  throw new Error("Terminal backend pulled in the renderer or runtime");

const directory = mkdtempSync(join(tmpdir(), "quickgui-native-extensions-"));
console.log((await run(["bun", "scripts/check-language-packs.ts"])).trim());
try {
  const core = resolveHostLibrary(target, root);
  const independentComponent = join(directory, process.platform === "darwin" ? "libacme-counter.dylib" : process.platform === "win32" ? "acme-counter.dll" : "libacme-counter.so");
  await run(["cc", ...(process.platform === "darwin" ? ["-dynamiclib"] : ["-shared", "-fPIC"]), "-I", join(root, "include"), join(root, "tests/fixtures/native_component.c"), "-o", independentComponent]);
  await run(["go", "test", "./internal/ffi", "-run", "TestIndependentComponentLibrarySmoke", "-count=1"], join(root, "go"), { QUICKGUI_TEST_CORE: core, QUICKGUI_TEST_COMPONENT: independentComponent });
  console.log("[extensions] Independent C component: registered through purego without any host-specific component code");
  const backendManifest = JSON.parse(
    readFileSync(join(root, "extensions/terminal/quickgui.extension.json"), "utf8"),
  );
  const { extensionLibraryName } = await import("../packages/cli/src/extensions.ts");
  const backend = join(
    root,
    "extensions/terminal/lib",
    target,
    extensionLibraryName(backendManifest, target),
  );
  if (!existsSync(backend))
    throw new Error(
      "Build the terminal extension first: bun packages/native/build.ts --extension terminal",
    );
  await run(
    ["go", "test", "./internal/ffi", "-run", "TestExtensionLibrarySmoke", "-count=1"],
    join(root, "go"),
    { QUICKGUI_TEST_CORE: core, QUICKGUI_TEST_TERMINAL: backend },
  );
  for (const extensions of [[], ["editor"], ["markdown"], ["terminal"], ["updater"], ["editor", "markdown", "terminal", "updater"]]) {
    const project = join(directory, extensions.join("-") || "core");
    mkdirSync(project);
    writeFileSync(
      join(project, "go.mod"),
      `module example.test/extensions\n\ngo 1.23\n\nrequire github.com/egoist/quickgui/go v${version}\nreplace github.com/egoist/quickgui/go => ${JSON.stringify(join(root, "go"))}\n` + extensions.map(name => `require github.com/egoist/quickgui/extensions/${name} v${version}\nreplace github.com/egoist/quickgui/extensions/${name} => ${JSON.stringify(join(root, "extensions", name))}\n`).join(""),
    );
    writeFileSync(
      join(project, "main.go"),
      `package main
import (
  "github.com/egoist/quickgui/go/host"
  _ "github.com/egoist/quickgui/go/ui"
  ${extensions.map((name) => '_ "github.com/egoist/quickgui/extensions/' + name + '"').join("\n")}
)
func main() { if err := host.Load(); err != nil { panic(err) } }
`,
    );
    await run(["go", "mod", "tidy"], project, { GOWORK: "off" });
    const executablePath =
      process.platform === "darwin"
        ? join(project, "Test.app/Contents/MacOS/Test")
        : join(project, process.platform === "win32" ? "Test.exe" : "Test");
    mkdirSync(dirname(executablePath), { recursive: true });
    const libraries = await compileNativeApplication({
      config: resolveConfig(
        { name: "Test", identifier: "dev.quickgui.extension-test", native: { libraryPath: core } },
        project,
      ),
      mode: "development",
      target,
      executablePath,
      fonts: [],
    });
    const expectedImages = 1 + extensions.length;
    const expectedResources = extensions.includes("updater") ? 1 : 0;
    if (libraries.length !== expectedImages + expectedResources)
      throw new Error("Incorrect selected extension set");
    const actual = readdirSync(dirname(libraries[0]!)).filter((name) =>
      /\.(dylib|so|dll)$/.test(name),
    );
    if (
      actual.length !== expectedImages ||
      actual.some((name) => !libraries.some((path) => basename(path) === name))
    )
      throw new Error("Bundle contains unexpected libraries");
    await run([executablePath], directory, {
      QUICKGUI_LIBRARY: undefined,
      QUICKGUI_HOST_LIB: undefined,
      QUICKGUI_EXTENSION_DIR: undefined,
    });
    console.log(
      `[extensions] ${extensions.join(" + ") || "Core-only"}: ${actual.join(", ")} — loaded through purego`,
    );
  }

  // An independently versioned extension, built against only the public C header.
  // Copy its Go module and npm artifact into an isolated consumer so neither
  // dependency discovery nor artifact lookup can rely on checkout integration.
  const independent = join(directory, "independent");
  const extension = join(independent, "extension");
  cpSync(join(root, "examples/native-extension/echo"), extension, { recursive: true });
  writeFileSync(
    join(extension, "go.mod"),
    `module example.test/echo\n\ngo 1.23\n\nrequire github.com/egoist/quickgui/go v${version}\n`,
  );
  const artifactPackage = join(independent, "node_modules/@acme/extension-echo");
  const service = await buildExtension(join(artifactPackage, "lib", target));
  cpSync(
    join(root, "examples/native-extension/backend/package.json"),
    join(artifactPackage, "package.json"),
  );
  await run(
    [
      "go",
      "test",
      "./internal/ffi",
      "-run",
      "TestIndependentServiceLibrarySmoke",
      "-count=1",
      "-v",
    ],
    join(root, "go"),
    {
      QUICKGUI_TEST_CORE: core,
      QUICKGUI_TEST_SERVICE: service,
    },
  );
  writeFileSync(
    join(independent, "go.mod"),
    `module example.test/consumer\n\ngo 1.23\n\nrequire (\n github.com/egoist/quickgui/go v${version}\n example.test/echo v1.0.0\n)\nreplace github.com/egoist/quickgui/go => ${JSON.stringify(join(root, "go"))}\nreplace example.test/echo => ./extension\n`,
  );
  writeFileSync(
    join(independent, "main.go"),
    `package main\nimport (\n "github.com/egoist/quickgui/go/host"\n _ "example.test/echo"\n)\nfunc main() { if err := host.Load(); err != nil { panic(err) } }\n`,
  );
  await run(["go", "mod", "tidy"], independent, { GOWORK: "off" });
  const executablePath =
    process.platform === "darwin"
      ? join(independent, "Consumer.app/Contents/MacOS/Consumer")
      : join(independent, process.platform === "win32" ? "Consumer.exe" : "Consumer");
  mkdirSync(dirname(executablePath), { recursive: true });
  const libraries = await compileNativeApplication({
    config: resolveConfig(
      {
        name: "Consumer",
        identifier: "dev.quickgui.independent-extension-test",
        native: { libraryPath: core },
      },
      independent,
    ),
    mode: "development",
    target,
    executablePath,
    fonts: [],
  });
  if (libraries.length !== 2 || !libraries.some((path) => basename(path) === basename(service)))
    throw new Error("Independent consumer did not bundle exactly its selected extension and core");
  // Installed packages are a build-time input; the packaged app needs neither.
  rmSync(join(independent, "node_modules"), { recursive: true });
  rmSync(extension, { recursive: true });
  await run([executablePath], directory, {
    QUICKGUI_LIBRARY: undefined,
    QUICKGUI_HOST_LIB: undefined,
    QUICKGUI_EXTENSION_DIR: undefined,
  });
  console.log(
    "[extensions] Independent @acme/extension-echo@1.0.0: bundled and loaded through purego; JSON replies and errors verified",
  );
} finally {
  rmSync(directory, { recursive: true, force: true });
}
const updaterGraph = await run([
  "cargo",
  "tree",
  "-p",
  "quickgui-updater",
  "--edges",
  "normal",
  "--prefix",
  "none",
]);
if (/^(quickgui |quickgui-host|wgpu|taffy)\b/m.test(updaterGraph))
  throw new Error("Updater links another renderer/runtime");
console.log("Native extension dependency and bundle checks passed");

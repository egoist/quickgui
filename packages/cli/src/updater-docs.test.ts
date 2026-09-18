import { expect, test } from "bun:test";
import {
  copyFileSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { loadConfig } from "./config.ts";
import { generateUpdaterKeys, updaterMetadata } from "./packaging/appcast.ts";
import { quickguiSolidPlugin } from "./typescript-compiler.ts";
import { typescriptExtensions } from "./typescript-build.ts";

test("the TypeScript updater guide loads real configuration and compiles the extension import", async () => {
  const root = resolve(import.meta.dir, "../../..");
  const docs = readFileSync(
    join(root, "website/src/content/docs/typescript/en/updater.mdx"),
    "utf8",
  );
  const sources = [...docs.matchAll(/```(?:ts|tsx)\n([\s\S]*?)```/g)].map((match) => match[1]!);
  // Three destination examples (GitHub, S3, S3-compatible), then updates.ts and app.tsx.
  expect(sources).toHaveLength(5);
  const project = mkdtempSync(join(tmpdir(), "quickgui-updater-docs-"));
  try {
    const keys = generateUpdaterKeys(join(project, "keys"));
    copyFileSync(keys.publicKeyPath, join(project, "update-public-key.pub"));
    const packages = join(project, "node_modules/@quickgui");
    mkdirSync(packages, { recursive: true });
    for (const name of ["cli", "native", "solid"])
      symlinkSync(join(root, "packages", name), join(packages, name), "dir");
    symlinkSync(join(root, "extensions/updater"), join(packages, "extension-updater"), "dir");
    symlinkSync(
      dirname(Bun.resolveSync("solid-js/package.json", join(root, "packages/solid"))),
      join(project, "node_modules/solid-js"),
      "dir",
    );
    const destinations = [];
    for (const [index, file] of ["s3.config.ts", "r2.config.ts"].entries()) {
      writeFileSync(join(project, file), sources[index + 1]!);
      destinations.push((await loadConfig(project, file)).updates?.destination);
    }
    expect(destinations).toEqual([
      {
        kind: "s3",
        bucket: "my-app-releases",
        region: "us-east-1",
        publicUrl: "https://my-app-releases.s3.us-east-1.amazonaws.com",
        prefix: "",
      },
      {
        kind: "s3",
        bucket: "my-app-releases",
        endpoint: "https://ACCOUNT_ID.r2.cloudflarestorage.com",
        region: "auto",
        publicUrl: "https://downloads.example.com",
        prefix: "",
      },
    ]);
    for (const [file, index] of [
      ["quickgui.config.ts", 0],
      ["updates.ts", 3],
      ["app.tsx", 4],
    ] as const)
      writeFileSync(join(project, file), sources[index]!);
    const config = await loadConfig(project).catch((error) => {
      throw error.cause ?? error;
    });
    expect(config.language).toBe("typescript");
    expect(config.extensions).toEqual(["@quickgui/extension-updater"]);
    expect(typescriptExtensions(project, config.extensions)[0]?.package).toBe(
      "@quickgui/extension-updater",
    );
    expect(updaterMetadata(config, "darwin-arm64", "production")).toEqual({
      feedUrl:
        "https://github.com/example/my-app/releases/latest/download/appcast-darwin-arm64.xml",
      publicKey: readFileSync(keys.publicKeyPath, "utf8").trim(),
      currentVersion: "1.0.0",
      identifier: "com.example.my-app",
      automaticChecks: true,
      development: false,
    });
    const result = await Bun.build({
      entrypoints: [config.entry],
      target: "bun",
      plugins: [quickguiSolidPlugin({ projectRoot: project, development: false })],
    });
    expect(result.success).toBe(true);
  } finally {
    rmSync(project, { recursive: true, force: true });
  }
});

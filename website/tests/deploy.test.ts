import { expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { generatedWorkerConfig } from "../scripts/wrangler-output";

const website = resolve(import.meta.dir, "..");
const root = resolve(website, "..");

test("workspace root forwards build and deploy to the website", () => {
  const pkg = JSON.parse(readFileSync(resolve(website, "package.json"), "utf8")) as {
    scripts: Record<string, string>;
  };
  const workspace = JSON.parse(readFileSync(resolve(root, "package.json"), "utf8")) as {
    scripts: Record<string, string>;
  };
  expect(workspace.scripts.build).toBe("bun run --cwd website build");
  expect(workspace.scripts.deploy).toBe("bun run --cwd website deploy");
  expect(pkg.scripts.deploy).toContain("wrangler deploy -c build/server/wrangler.json");
  expect(pkg.scripts.build).not.toContain("wrangler-output.ts");
  expect(pkg.scripts["cf-typegen"]).toContain("wrangler.dev.jsonc");
  expect(pkg.scripts.typecheck).toContain("wrangler.dev.jsonc");
});

test("root Wrangler config deploys the generated worker and builds it first", () => {
  const config = readFileSync(resolve(root, "wrangler.jsonc"), "utf8");
  expect(config).toContain('"main": "website/build/server/index.js"');
  expect(config).toContain('"directory": "website/build/client"');
  expect(config).toContain('"no_bundle": true');
  expect(config).toContain("ensure-worker-build.ts");
  expect(config).not.toContain('"main": "./workers/app.ts"');
});

test("website Wrangler config uploads the generated worker instead of bundling workers/app.ts", () => {
  const config = readFileSync(resolve(website, "wrangler.jsonc"), "utf8");
  const dev = readFileSync(resolve(website, "wrangler.dev.jsonc"), "utf8");
  const vite = readFileSync(resolve(website, "vite.config.ts"), "utf8");
  expect(config).toContain('"main": "./build/server/index.js"');
  expect(config).toContain('"directory": "./build/client"');
  expect(config).toContain('"no_bundle": true');
  expect(config).toContain("ensure-worker-build.ts");
  expect(config).not.toContain('"main": "./workers/app.ts"');
  expect(dev).toContain('"main": "./workers/app.ts"');
  expect(vite).toContain("wrangler.dev.jsonc");
});

test("generated worker config errors without a Vite build", () => {
  expect(() => generatedWorkerConfig(resolve(website, "does-not-exist"))).toThrow(
    /Missing Vite\/Cloudflare worker output/,
  );
});

test("generated worker config prefers the Vite deploy redirect", () => {
  if (!existsSync(resolve(website, "build/server/wrangler.json"))) return;
  const config = generatedWorkerConfig(website);
  expect(config).toContain("build/server/wrangler.json");
  const generated = JSON.parse(readFileSync(config, "utf8")) as { main?: string; no_bundle?: boolean };
  expect(generated.main).toBe("index.js");
  expect(generated.no_bundle).toBe(true);
});

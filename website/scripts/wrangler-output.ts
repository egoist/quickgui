import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

export function generatedWorkerConfig(websiteRoot: string) {
  const redirectPath = resolve(websiteRoot, ".wrangler/deploy/config.json");
  if (existsSync(redirectPath)) {
    const redirect = JSON.parse(readFileSync(redirectPath, "utf8")) as { configPath?: string };
    if (redirect.configPath) {
      const generated = resolve(websiteRoot, ".wrangler/deploy", redirect.configPath);
      if (existsSync(generated)) return generated;
    }
  }
  const fallback = resolve(websiteRoot, "build/server/wrangler.json");
  if (existsSync(fallback)) return fallback;
  throw new Error(
    "Missing Vite/Cloudflare worker output at build/server/wrangler.json. Run `bun run build` before wrangler deploy. Wrangler cannot bundle workers/app.ts because it imports virtual:react-router/server-build.",
  );
}

if (import.meta.main) {
  process.stdout.write(`${generatedWorkerConfig(resolve(import.meta.dir, ".."))}\n`);
}

import { existsSync } from "node:fs";
import { resolve } from "node:path";

const website = resolve(import.meta.dir, "..");
const entry = resolve(website, "build/server/index.js");

if (existsSync(entry)) {
  process.stdout.write("Vite worker output already present at build/server/index.js\n");
  process.exit(0);
}

process.stdout.write("Building Vite worker output for Wrangler\n");
const result = Bun.spawnSync(["bun", "run", "build"], {
  cwd: website,
  stdout: "inherit",
  stderr: "inherit",
  stdin: "inherit",
});
process.exit(result.exitCode ?? 1);

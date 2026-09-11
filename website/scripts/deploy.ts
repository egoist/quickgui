import { resolve } from "node:path";
import { ensureDocsToolchain } from "./ensure-docs-toolchain";

const website = resolve(import.meta.dir, "..");

async function run(args: string[]) {
  const child = Bun.spawn(args, {
    cwd: website,
    env: process.env,
    stdout: "inherit",
    stderr: "inherit",
  });
  if (await child.exited) throw new Error(`Command failed: ${args.join(" ")}`);
}

await ensureDocsToolchain();
await run(["bun", "run", "build"]);
await run(["wrangler", "deploy", ...process.argv.slice(2)]);

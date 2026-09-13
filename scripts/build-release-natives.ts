#!/usr/bin/env bun
/** Build host, terminal, and updater images for each release target. */
import { resolve } from "node:path";

const root = resolve(import.meta.dir, "..");
const targets = process.argv.slice(2);
if (targets.length === 0)
  throw new Error("usage: bun scripts/build-release-natives.ts <target> [<target>...]");

function run(argv: string[]) {
  const child = Bun.spawnSync(argv, {
    cwd: root,
    stdin: "inherit",
    stdout: "inherit",
    stderr: "inherit",
  });
  if (child.exitCode !== 0) process.exit(child.exitCode ?? 1);
}

for (const target of targets) {
  run(["bun", "packages/native/build.ts", "--target", target]);
  run(["bun", "packages/native/build.ts", "--extension", "terminal", "--target", target]);
  run(["bun", "packages/native/build.ts", "--extension", "updater", "--target", target]);
}

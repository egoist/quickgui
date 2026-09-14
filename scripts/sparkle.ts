#!/usr/bin/env bun
/** Fetch the same pinned Sparkle distribution used by Waku, then stage a compact framework.
 * Only the native extension build runs this; ordinary Go builds reuse the verified artifact.
 */
import { createHash } from "node:crypto";
import {
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { join, resolve } from "node:path";
import { readBounded } from "../packages/cli/src/extensions.ts";
import { packResources } from "../packages/cli/src/extension-resources.ts";

export const sparkleVersion = "2.9.4";
const digest = "ce89daf967db1e1893ed3ebd67575ed82d3902563e3191ca92aaec9164fbdef9";
export async function stageSparkle(destination: string): Promise<void> {
  const cache = resolve(import.meta.dir, "../target/sparkle", sparkleVersion);
  mkdirSync(cache, { recursive: true });
  const archive = join(cache, "Sparkle.tar.xz");
  const valid = () =>
    existsSync(archive) &&
    createHash("sha256").update(readFileSync(archive)).digest("hex") === digest;
  if (!valid()) {
    const response = await fetch(
      `https://github.com/sparkle-project/Sparkle/releases/download/${sparkleVersion}/Sparkle-${sparkleVersion}.tar.xz`,
      { signal: AbortSignal.timeout(60_000) },
    );
    if (!response.ok || !response.body)
      throw new Error(`Sparkle download failed: ${response.status}`);
    const bytes = await readBounded(response.body, 64 * 1024 * 1024);
    if (createHash("sha256").update(bytes).digest("hex") !== digest)
      throw new Error("Sparkle archive checksum mismatch");
    writeFileSync(archive, bytes);
  }
  const temporary = mkdtempSync(join(cache, ".stage-"));
  try {
    const child = Bun.spawnSync(
      ["tar", "-xJf", archive, "-C", temporary, "./Sparkle.framework", "./bin", "./LICENSE"],
      { stderr: "pipe" },
    );
    if (child.exitCode !== 0) throw new Error(child.stderr.toString());
    const framework = join(temporary, "Sparkle.framework");
    for (const base of [framework, join(framework, "Versions/B")]) {
      for (const name of ["Headers", "PrivateHeaders", "Modules", "XPCServices"])
        rmSync(join(base, name), { recursive: true, force: true });
    }
    cpSync(join(temporary, "LICENSE"), join(framework, "Versions/B/Resources/Sparkle-LICENSE.txt"));
    mkdirSync(destination, { recursive: true });
    const packed = packResources(framework);
    writeFileSync(join(destination, "Sparkle.framework.qgr"), packed);
    // Keep a local unpacked copy for ABI tests; npm distributes only the bounded resource bundle.
    rmSync(join(destination, "Sparkle.framework"), { recursive: true, force: true });
    cpSync(framework, join(destination, "Sparkle.framework"), {
      recursive: true,
      verbatimSymlinks: true,
    });
    cpSync(join(temporary, "bin"), join(cache, "bin"), { recursive: true });
    console.log(`[native] Staged Sparkle ${sparkleVersion}`);
  } finally {
    rmSync(temporary, { recursive: true, force: true });
  }
}
if (import.meta.main)
  await stageSparkle(resolve(process.argv[2] ?? "extensions/updater/lib/darwin-arm64"));

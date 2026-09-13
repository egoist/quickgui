#!/usr/bin/env bun
import { createHash } from "node:crypto";
import { existsSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { npmPublishArgs, npmPublishTag } from "./npm-publish-tag.ts";

const mode = process.argv[2];
if (mode !== "--check" && mode !== "--publish")
  throw new Error("usage: bun scripts/release-npm.ts --check|--publish [archive-directory]");
const publish = mode === "--publish";
const root = resolve(import.meta.dir, "..");
const directory = resolve(root, process.argv[3] ?? "target/npm-release");
const version = JSON.parse(readFileSync(join(root, "package.json"), "utf8")).version as string;

function run(argv: string[]): { stdout: string; stderr: string; exitCode: number } {
  const child = Bun.spawnSync(argv, {
    cwd: root,
    stdin: "inherit",
    stdout: "pipe",
    stderr: "pipe",
  });
  return {
    stdout: child.stdout.toString().trim(),
    stderr: child.stderr.toString().trim(),
    exitCode: child.exitCode ?? 1,
  };
}

function mustRun(argv: string[]): string {
  const result = run(argv);
  if (result.exitCode !== 0) {
    if (result.stderr) console.error(result.stderr);
    throw new Error(`${argv.join(" ")} exited ${result.exitCode}`);
  }
  if (result.stderr) console.error(result.stderr);
  return result.stdout;
}

function alreadyPublished(stderr: string): boolean {
  return /EEXIST|EPUBLISHCONFLICT|previously published|cannot publish over|403 Forbidden/.test(stderr);
}

mustRun(["bun", join(import.meta.dir, "release-metadata.ts")]);

async function publishedIntegrity(name: string): Promise<string | undefined> {
  let response: Response | undefined;
  for (let attempt = 0; attempt < 6; attempt++) {
    try {
      response = await fetch(`https://registry.npmjs.org/${encodeURIComponent(name)}/${version}`, {
        signal: AbortSignal.timeout(60_000),
      });
      if (response.ok || response.status === 404) break;
      if (response.status < 500 && response.status !== 429)
        throw new Error(`Registry returned HTTP ${response.status} for ${name}`);
    } catch (error) {
      if (attempt === 5) throw error;
    }
    if (attempt < 5) await Bun.sleep(1000 * (attempt + 1));
  }
  if (response?.status === 404) return undefined;
  if (!response?.ok) throw new Error(`Registry request failed for ${name}`);
  const metadata = (await response.json()) as { dist?: { integrity?: string } };
  return metadata.dist?.integrity;
}

if (publish) {
  const [nodeMajor = 0, nodeMinor = 0] = mustRun(["node", "--version"])
    .replace(/^v/, "")
    .split(".")
    .map(Number);
  const [npmMajor = 0, npmMinor = 0, npmPatch = 0] = mustRun(["npm", "--version"])
    .split(".")
    .map(Number);
  if (nodeMajor < 22 || (nodeMajor === 22 && nodeMinor < 14))
    throw new Error("npm trusted publishing requires Node 22.14 or newer");
  if (npmMajor < 11 || (npmMajor === 11 && (npmMinor < 5 || (npmMinor === 5 && npmPatch < 1))))
    throw new Error("npm trusted publishing requires npm 11.5.1 or newer");
  if (!process.env.ACTIONS_ID_TOKEN_REQUEST_URL && !process.env.NODE_AUTH_TOKEN)
    throw new Error("GitHub OIDC or NODE_AUTH_TOKEN authentication is required");
}

for (const part of ["native", "extension-terminal", "extension-updater", "solid", "cli"]) {
  const name = `@quickgui/${part}`;
  const archive = join(directory, `quickgui-${part}-${version}.tgz`);
  if (!existsSync(archive)) throw new Error(`Missing archive: ${archive}`);
  const integrity = `sha512-${createHash("sha512").update(readFileSync(archive)).digest("base64")}`;
  const publicIntegrity = await publishedIntegrity(name);
  if (publicIntegrity) {
    console.log(
      publicIntegrity === integrity
        ? `${name}@${version} is already public with matching bytes; skipping`
        : `${name}@${version} is already public with different bytes; skipping immutable registry version`,
    );
    continue;
  }
  if (!publish) {
    console.log(`${name}@${version} is ready to publish with tag ${npmPublishTag}`);
    continue;
  }
  const published = run(npmPublishArgs(archive));
  if (published.exitCode !== 0) {
    if (alreadyPublished(published.stderr)) {
      console.log(`${name}@${version} is already on the registry; skipping`);
      continue;
    }
    if (published.stderr) console.error(published.stderr);
    throw new Error(
      `${name}@${version} publish failed. A 404 on an existing package usually means npm has no Trusted Publisher for GitHub owner egoist, repository quickgui, workflow release.yml, and no environment.`,
    );
  }
  if (published.stdout) console.log(published.stdout);
  if (published.stderr) console.error(published.stderr);
  console.log(`Published ${name}@${version}`);
}

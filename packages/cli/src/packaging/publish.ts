/**
 * Where releases are published, and the `--upload` step that puts them there.
 *
 * A destination fixes every URL the updater and `install.sh` read, so projects configure the
 * place once instead of spelling feed and artifact URLs. Each release has versioned artifacts
 * (installers, archives) and mutable pointers (`appcast-<target>.xml`, `latest-linux.txt`,
 * `install.sh`) that always describe the newest release.
 */

import { existsSync } from "node:fs";
import { basename } from "node:path";

import { CliError } from "../error.ts";

export interface GitHubDestination {
  kind: "github";
  /** `owner/name` on github.com. */
  repository: string;
  /** Release tags are this prefix plus the version. Defaults to "v". */
  tagPrefix: string;
}

export interface S3Destination {
  kind: "s3";
  bucket: string;
  /** Public HTTPS origin that serves the bucket, such as a CDN or custom domain. */
  publicUrl: string;
  /** S3 API endpoint for non-AWS providers (Cloudflare R2, MinIO, …). */
  endpoint?: string;
  region?: string;
  /** Key prefix inside the bucket, also appended to `publicUrl`. */
  prefix: string;
}

export type UpdateDestination = GitHubDestination | S3Destination;

function join(base: string, name: string): string {
  return `${base.replace(/\/+$/, "")}/${encodeURIComponent(name)}`;
}

function s3Base(destination: S3Destination): string {
  return destination.prefix
    ? `${destination.publicUrl}/${destination.prefix.split("/").map(encodeURIComponent).join("/")}`
    : destination.publicUrl;
}

export function releaseTag(destination: GitHubDestination, version: string): string {
  return `${destination.tagPrefix}${version}`;
}

/** Download URL of one versioned artifact. */
export function artifactUrl(destination: UpdateDestination, version: string, name: string): string {
  return destination.kind === "github"
    ? join(
        `https://github.com/${destination.repository}/releases/download/${encodeURIComponent(releaseTag(destination, version))}`,
        name,
      )
    : join(s3Base(destination), name);
}

/**
 * Stable URL of a mutable pointer. GitHub serves it from whichever release is marked latest,
 * which never includes drafts or pre-releases.
 */
export function pointerUrl(destination: UpdateDestination, name: string): string {
  return destination.kind === "github"
    ? join(`https://github.com/${destination.repository}/releases/latest/download`, name)
    : join(s3Base(destination), name);
}

export function feedUrl(destination: UpdateDestination, target: string): string {
  return pointerUrl(destination, `appcast-${target}.xml`);
}

export interface ReleaseUpload {
  destination: UpdateDestination;
  name: string;
  version: string;
  /** Versioned files, uploaded first so a pointer never names something that is not there yet. */
  artifacts: readonly string[];
  pointers: readonly string[];
  notesFile?: string;
  cwd: string;
}

/** Command lines for publishing to a GitHub release with the `gh` CLI. */
export function githubUploadCommands(upload: ReleaseUpload & { destination: GitHubDestination }): {
  view: string[];
  create: string[];
  upload: string[];
} {
  const tag = releaseTag(upload.destination, upload.version);
  const repository = ["--repo", upload.destination.repository];
  const files = [...upload.artifacts, ...upload.pointers];
  return {
    view: ["gh", "release", "view", tag, ...repository, "--json", "tagName"],
    create: [
      "gh",
      "release",
      "create",
      tag,
      ...repository,
      "--title",
      `${upload.name} ${upload.version}`,
      ...(upload.notesFile ? ["--notes-file", upload.notesFile] : ["--notes", ""]),
      ...files,
    ],
    // Other targets of the same version add their files to the release that already exists.
    upload: ["gh", "release", "upload", tag, ...repository, "--clobber", ...files],
  };
}

/** Object key of one uploaded file. */
export function s3Key(destination: S3Destination, path: string): string {
  return destination.prefix ? `${destination.prefix}/${basename(path)}` : basename(path);
}

const CONTENT_TYPES: Readonly<Record<string, string>> = {
  ".xml": "application/xml",
  ".txt": "text/plain; charset=utf-8",
  ".sh": "text/x-shellscript; charset=utf-8",
  ".gz": "application/gzip",
  ".zip": "application/zip",
};

async function spawn(command: string[], cwd: string): Promise<{ status: number; output: string }> {
  const child = Bun.spawn(command, { cwd, stdin: "ignore", stdout: "pipe", stderr: "pipe" });
  const [status, stdout, stderr] = await Promise.all([
    child.exited,
    new Response(child.stdout).text(),
    new Response(child.stderr).text(),
  ]);
  return { status, output: (stderr.trim() || stdout.trim()).slice(0, 4096) };
}

/** How `uploadRelease` reaches `gh`. Tests substitute it so they never touch GitHub. */
export interface UploadTools {
  which: (command: string) => string | null;
  spawn: (command: string[], cwd: string) => Promise<{ status: number; output: string }>;
}

/** Publish one build's release files. Returns the public URLs, pointers last. */
export async function uploadRelease(
  upload: ReleaseUpload,
  tools: UploadTools = { which: (command) => Bun.which(command), spawn },
): Promise<string[]> {
  const { destination } = upload;
  for (const path of [...upload.artifacts, ...upload.pointers])
    if (!existsSync(path)) throw new CliError(`Release file is missing: ${path}`);
  if (destination.kind === "github") {
    if (!tools.which("gh"))
      throw new CliError(
        "Uploading to GitHub needs the `gh` CLI on PATH, authenticated with GH_TOKEN or `gh auth login`",
      );
    const commands = githubUploadCommands({ ...upload, destination });
    let result = await tools.spawn(
      (await tools.spawn(commands.view, upload.cwd)).status === 0
        ? commands.upload
        : commands.create,
      upload.cwd,
    );
    // Parallel target builds race to create the release; the loser uploads into the winner's.
    if (result.status !== 0 && /already.exists/i.test(result.output))
      result = await tools.spawn(commands.upload, upload.cwd);
    if (result.status !== 0) throw new CliError(`GitHub upload failed\n${result.output}`);
  } else {
    // Credentials come from the standard S3_*/AWS_* environment variables.
    const client = new Bun.S3Client({
      bucket: destination.bucket,
      ...(destination.endpoint ? { endpoint: destination.endpoint } : {}),
      ...(destination.region ? { region: destination.region } : {}),
    });
    for (const path of [...upload.artifacts, ...upload.pointers]) {
      const extension = /\.[^.]+$/.exec(path)?.[0] ?? "";
      try {
        await client.write(s3Key(destination, path), Bun.file(path), {
          type: CONTENT_TYPES[extension] ?? "application/octet-stream",
        });
      } catch (error) {
        throw new CliError(
          `S3 upload failed for ${basename(path)}: ${error instanceof Error ? error.message : String(error)}`,
        );
      }
    }
  }
  return [
    ...upload.artifacts.map((path) => artifactUrl(destination, upload.version, basename(path))),
    ...upload.pointers.map((path) => pointerUrl(destination, basename(path))),
  ];
}

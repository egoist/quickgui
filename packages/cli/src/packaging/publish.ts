/**
 * Where releases are published, and the `--upload` step that puts them there.
 *
 * A destination fixes every URL the updater and `install.sh` read, so projects configure the
 * place once instead of spelling feed and artifact URLs. Each release has versioned artifacts
 * (installers, archives) and mutable pointers (`appcast-<target>.xml`, `latest-linux-<arch>.txt`,
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

/**
 * Command lines for publishing to a GitHub release with the `gh` CLI. The release is created as
 * a draft: nothing reaches users, and the update feed does not move, until someone publishes it
 * after every target has uploaded.
 */
export function githubUploadCommands(upload: ReleaseUpload & { destination: GitHubDestination }): {
  view: string[];
  create: string[];
  upload: string[];
  drafts: string[];
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
      "--draft",
      "--title",
      `${upload.name} ${upload.version}`,
      ...(upload.notesFile ? ["--notes-file", upload.notesFile] : ["--notes", ""]),
      ...files,
    ],
    // Other targets of the same version add their files to the release that already exists.
    upload: ["gh", "release", "upload", tag, ...repository, "--clobber", ...files],
    // GitHub lets several drafts share a tag, so parallel targets can each create one.
    drafts: [
      "gh",
      "api",
      `repos/${upload.destination.repository}/releases?per_page=100`,
      "--jq",
      `[.[] | select(.draft and .tag_name == ${JSON.stringify(tag)}) | {id, url: .html_url}] | sort_by(.id)`,
    ],
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
  // A failure explains itself on stderr; a success answers on stdout.
  const output = status === 0 ? stdout.trim() : stderr.trim() || stdout.trim();
  return { status, output: output.slice(0, 256 * 1024) };
}

/** How `uploadRelease` reaches `gh`. Tests substitute it so they never touch GitHub. */
export interface UploadTools {
  which: (command: string) => string | null;
  spawn: (command: string[], cwd: string) => Promise<{ status: number; output: string }>;
  /** Store one local file as an object. */
  putObject: (destination: S3Destination, key: string, path: string, type: string) => Promise<void>;
}

/** Credentials come from the standard `AWS_…` or `S3_…` environment variables. */
async function putObject(
  destination: S3Destination,
  key: string,
  path: string,
  type: string,
): Promise<void> {
  const client = new Bun.S3Client({
    bucket: destination.bucket,
    ...(destination.endpoint ? { endpoint: destination.endpoint } : {}),
    ...(destination.region ? { region: destination.region } : {}),
  });
  await client.write(key, Bun.file(path), { type });
}

export interface UploadResult {
  /** Public URLs of the uploaded files, pointers last. */
  urls: string[];
  /** Anything the user still has to do before the release is live. */
  notes: string[];
}

/** Publish one build's release files. */
export async function uploadRelease(
  upload: ReleaseUpload,
  tools: UploadTools = { which: (command) => Bun.which(command), spawn, putObject },
): Promise<UploadResult> {
  const notes: string[] = [];
  const { destination } = upload;
  for (const path of [...upload.artifacts, ...upload.pointers])
    if (!existsSync(path)) throw new CliError(`Release file is missing: ${path}`);
  if (destination.kind === "github") {
    if (!tools.which("gh"))
      throw new CliError(
        "Uploading to GitHub needs the `gh` CLI on PATH, authenticated with GH_TOKEN or `gh auth login`",
      );
    const commands = githubUploadCommands({ ...upload, destination });
    const exists = (await tools.spawn(commands.view, upload.cwd)).status === 0;
    let result = await tools.spawn(exists ? commands.upload : commands.create, upload.cwd);
    // A published release of this tag appeared between `view` and `create`.
    if (result.status !== 0 && /already.exists/i.test(result.output))
      result = await tools.spawn(commands.upload, upload.cwd);
    if (result.status !== 0) throw new CliError(`GitHub upload failed\n${result.output}`);
    if (!exists) {
      // Two targets that both saw no release each created a draft. The oldest one stays; a
      // later one is deleted and its files go into the oldest.
      const listed = await tools.spawn(commands.drafts, upload.cwd);
      let drafts: Array<{ id: number; url: string }> = [];
      try {
        drafts = listed.status === 0 ? JSON.parse(listed.output) : [];
      } catch {
        /* an unreadable listing leaves the draft that was just created */
      }
      const mine = drafts.find((draft) => result.output.includes(draft.url));
      if (mine && drafts[0] && mine.id !== drafts[0].id) {
        const removed = await tools.spawn(
          [
            "gh",
            "api",
            "-X",
            "DELETE",
            `repos/${destination.repository}/releases/${mine.id}`,
          ],
          upload.cwd,
        );
        if (removed.status !== 0)
          throw new CliError(`Could not remove a duplicate draft release\n${removed.output}`);
        result = await tools.spawn(commands.upload, upload.cwd);
        if (result.status !== 0) throw new CliError(`GitHub upload failed\n${result.output}`);
      }
    }
    const tag = releaseTag(destination, upload.version);
    notes.push(
      `GitHub release ${tag} is a draft. Once every target has uploaded, publish it to make the ` +
        `update live: gh release edit ${tag} --repo ${destination.repository} --draft=false`,
    );
  } else {
    for (const path of [...upload.artifacts, ...upload.pointers]) {
      const extension = /\.[^.]+$/.exec(path)?.[0] ?? "";
      try {
        await tools.putObject(
          destination,
          s3Key(destination, path),
          path,
          CONTENT_TYPES[extension] ?? "application/octet-stream",
        );
      } catch (error) {
        throw new CliError(
          `S3 upload failed for ${basename(path)}: ${error instanceof Error ? error.message : String(error)}`,
        );
      }
    }
  }
  return {
    urls: [
      ...upload.artifacts.map((path) => artifactUrl(destination, upload.version, basename(path))),
      ...upload.pointers.map((path) => pointerUrl(destination, basename(path))),
    ],
    notes,
  };
}

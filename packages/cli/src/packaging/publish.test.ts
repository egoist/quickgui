import { expect, test } from "bun:test";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { releaseFiles } from "../build.ts";
import { installScript } from "./linux.ts";
import {
  artifactUrl,
  feedUrl,
  githubUploadCommands,
  pointerUrl,
  s3Key,
  uploadRelease,
  type GitHubDestination,
  type S3Destination,
  type UploadTools,
} from "./publish.ts";

const github: GitHubDestination = { kind: "github", repository: "demo/app", tagPrefix: "v" };
const s3: S3Destination = {
  kind: "s3",
  bucket: "releases",
  publicUrl: "https://dl.example.com",
  prefix: "my app/stable",
};

test("a destination fixes the feed, pointer, and artifact URLs", () => {
  expect(feedUrl(github, "linux-x64")).toBe(
    "https://github.com/demo/app/releases/latest/download/appcast-linux-x64.xml",
  );
  expect(artifactUrl(github, "1.2.3", "Demo-1.2.3-linux-x64.tar.gz")).toBe(
    "https://github.com/demo/app/releases/download/v1.2.3/Demo-1.2.3-linux-x64.tar.gz",
  );
  expect(artifactUrl({ ...github, tagPrefix: "desktop/" }, "1.2.3", "a.zip")).toBe(
    "https://github.com/demo/app/releases/download/desktop%2F1.2.3/a.zip",
  );
  expect(feedUrl(s3, "darwin-arm64")).toBe(
    "https://dl.example.com/my%20app/stable/appcast-darwin-arm64.xml",
  );
  expect(artifactUrl(s3, "1.2.3", "Demo 1.dmg")).toBe(
    "https://dl.example.com/my%20app/stable/Demo%201.dmg",
  );
  expect(pointerUrl({ ...s3, prefix: "" }, "latest-linux-x64.txt")).toBe(
    "https://dl.example.com/latest-linux-x64.txt",
  );
  expect(s3Key(s3, "/out/linux-x64/install.sh")).toBe("my app/stable/install.sh");
  expect(s3Key({ ...s3, prefix: "" }, "/out/install.sh")).toBe("install.sh");
});

test("a build publishes installers and archives, then the files describing the newest release", () => {
  const out = (name: string) => `/out/linux-x64/${name}`;
  const release = releaseFiles(
    [
      "Demo.desktop",
      "Demo.AppDir",
      "Demo-1.2.3-x86_64.AppImage",
      "demo_1.2.3_amd64.deb",
      "Demo-1.2.3-linux-x64.tar.gz",
      "install.sh",
      "latest-linux-x64.txt",
      "Demo-1.2.3-linux-x64.tar.gz",
    ].map(out),
    out("appcast-linux-x64.xml"),
  );
  expect(release).toEqual({
    artifacts: [
      "Demo-1.2.3-x86_64.AppImage",
      "demo_1.2.3_amd64.deb",
      "Demo-1.2.3-linux-x64.tar.gz",
    ].map(out),
    pointers: ["install.sh", "latest-linux-x64.txt", "appcast-linux-x64.xml"].map(out),
  });
  const commands = githubUploadCommands({
    destination: github,
    name: "Demo",
    version: "1.2.3",
    ...release,
    notesFile: "/project/NOTES.md",
    cwd: "/project",
  });
  expect(commands.view.slice(0, 6)).toEqual(["gh", "release", "view", "v1.2.3", "--repo", "demo/app"]);
  expect(commands.create.slice(0, 10)).toEqual([
    "gh",
    "release",
    "create",
    "v1.2.3",
    "--repo",
    "demo/app",
    "--title",
    "Demo 1.2.3",
    "--notes-file",
    "/project/NOTES.md",
  ]);
  expect(commands.upload).toContain("--clobber");
  // Pointers go last in every command.
  expect(commands.create.at(-1)).toBe(out("appcast-linux-x64.xml"));
  expect(commands.upload.at(-1)).toBe(out("appcast-linux-x64.xml"));
});

test("install.sh follows the destination's layout", () => {
  const options = {
    name: "Demo",
    executableName: "Demo",
    identifier: "com.example.demo",
    packageName: "demo",
  };
  const onGitHub = installScript({ ...options, destination: { ...github, tagPrefix: "app-v" } });
  expect(onGitHub).toContain("default_releases='https://github.com/demo/app/releases'");
  expect(onGitHub).toContain("layout='github'");
  expect(onGitHub).toContain("tag_prefix='app-v'");
  const onS3 = installScript({ ...options, destination: s3 });
  expect(onS3).toContain("default_releases='https://dl.example.com/my%20app/stable'");
  expect(onS3).toContain("layout='flat'");
  const local = installScript(options);
  expect(local).toContain("default_releases=''");
  for (const script of [onGitHub, onS3, local]) {
    const check = Bun.spawnSync(["sh", "-n"], { stdin: new TextEncoder().encode(script) });
    expect(check.exitCode).toBe(0);
  }
});

test("GitHub uploads create the release once and add later targets to it", async () => {
  const root = mkdtempSync(join(tmpdir(), "quickgui-gh-"));
  try {
    const artifact = join(root, "Demo-1.2.3-linux-x64.tar.gz");
    const feed = join(root, "appcast-linux-x64.xml");
    writeFileSync(artifact, "archive");
    writeFileSync(feed, "<rss/>");
    const upload = {
      destination: github,
      name: "Demo",
      version: "1.2.3",
      artifacts: [artifact],
      pointers: [feed],
      cwd: root,
    };
    // A stand-in for gh: `view` fails until a release exists, and `create` fails once it does.
    const calls: string[] = [];
    let released = false;
    let viewSeesRelease = true;
    const tools: UploadTools = {
      putObject: async () => {
        throw new Error("GitHub uploads never touch S3");
      },
      which: () => "/usr/bin/gh",
      spawn: async (command) => {
        calls.push(`${command[2]} ${command[3]}`);
        if (command[2] === "view") return { status: released && viewSeesRelease ? 0 : 1, output: "" };
        if (command[2] === "create") {
          if (released)
            return { status: 1, output: "a release with the same tag name already exists" };
          released = true;
        }
        return { status: 0, output: "" };
      },
    };
    expect(await uploadRelease(upload, tools)).toEqual([
      "https://github.com/demo/app/releases/download/v1.2.3/Demo-1.2.3-linux-x64.tar.gz",
      "https://github.com/demo/app/releases/latest/download/appcast-linux-x64.xml",
    ]);
    await uploadRelease(upload, tools);
    expect(calls).toEqual(["view v1.2.3", "create v1.2.3", "view v1.2.3", "upload v1.2.3"]);
    // A parallel target created the release between `view` and `create`.
    viewSeesRelease = false;
    await uploadRelease(upload, tools);
    expect(calls.slice(-3)).toEqual(["view v1.2.3", "create v1.2.3", "upload v1.2.3"]);
    await expect(
      uploadRelease({ ...upload, artifacts: [join(root, "missing")] }, tools),
    ).rejects.toThrow("missing");
    await expect(uploadRelease(upload, { ...tools, which: () => null })).rejects.toThrow("gh");
    await expect(
      uploadRelease(upload, {
        ...tools,
        spawn: async () => ({ status: 1, output: "HTTP 403" }),
      }),
    ).rejects.toThrow("HTTP 403");
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("S3 uploads store versioned files before the pointers, under the prefix", async () => {
  const root = mkdtempSync(join(tmpdir(), "quickgui-s3-"));
  try {
    const files = ["Demo-1.2.3-linux-x64.tar.gz", "install.sh", "appcast-linux-x64.xml"].map(
      (name) => join(root, name),
    );
    for (const file of files) writeFileSync(file, "x");
    const stored: Array<[string, string]> = [];
    const tools: UploadTools = {
      which: () => null,
      spawn: async () => {
        throw new Error("S3 uploads run no commands");
      },
      putObject: async (destination, key, _path, type) => {
        expect(destination.bucket).toBe("releases");
        stored.push([key, type]);
      },
    };
    const upload = {
      destination: s3,
      name: "Demo",
      version: "1.2.3",
      artifacts: [files[0]!],
      pointers: [files[1]!, files[2]!],
      cwd: root,
    };
    expect(await uploadRelease(upload, tools)).toEqual([
      "https://dl.example.com/my%20app/stable/Demo-1.2.3-linux-x64.tar.gz",
      "https://dl.example.com/my%20app/stable/install.sh",
      "https://dl.example.com/my%20app/stable/appcast-linux-x64.xml",
    ]);
    expect(stored).toEqual([
      ["my app/stable/Demo-1.2.3-linux-x64.tar.gz", "application/gzip"],
      ["my app/stable/install.sh", "text/x-shellscript; charset=utf-8"],
      ["my app/stable/appcast-linux-x64.xml", "application/xml"],
    ]);
    await expect(
      uploadRelease(upload, {
        ...tools,
        putObject: async () => {
          throw new Error("AccessDenied");
        },
      }),
    ).rejects.toThrow("S3 upload failed for Demo-1.2.3-linux-x64.tar.gz: AccessDenied");
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

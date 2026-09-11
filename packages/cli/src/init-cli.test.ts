import { afterEach, expect, test } from "bun:test";
import { existsSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const roots: string[] = [];
const cli = join(import.meta.dir, "cli.ts");

afterEach(() => {
  for (const root of roots.splice(0)) rmSync(root, { recursive: true, force: true });
});

function destination(): string {
  const root = mkdtempSync(join(tmpdir(), "quickgui-init-cli-"));
  roots.push(root);
  return join(root, "my-app");
}

async function runInteractiveInit(directory: string, keys: string) {
  let output = "";
  let answered = false;
  const child = Bun.spawn([process.execPath, cli, "init", directory, "--no-install"], {
    env: { ...process.env, CI: "", TERM: "xterm-256color", PATH: "" },
    timeout: 3_000,
    terminal: {
      cols: 100,
      rows: 24,
      data(terminal, data) {
        output += new TextDecoder().decode(data);
        if (!answered && output.includes("TypeScript")) {
          answered = true;
          terminal.write(keys);
        }
      },
    },
  });
  try {
    const status = await child.exited;
    expect(output).toContain("Which language would you like to use?");
    expect(answered).toBe(true);
    return { status, output };
  } finally {
    child.terminal?.close();
  }
}

for (const [language, keys, entry, otherEntry] of [
  ["go", "\r", "main.go", "app.tsx"],
  ["typescript", "\x1b[B\r", "app.tsx", "main.go"],
  ["rust", "\x1b[B\x1b[B\r", "src/main.rs", "main.go"],
] as const) {
  test(`init prompts and scaffolds the selected ${language} language`, async () => {
    const directory = destination();
    const { status } = await runInteractiveInit(directory, keys);
    expect(status).toBe(0);
    expect(existsSync(join(directory, entry))).toBe(true);
    expect(existsSync(join(directory, otherEntry))).toBe(false);
  });

  test(`init accepts --language ${language} without a terminal`, async () => {
    const directory = destination();
    const child = Bun.spawn(
      [process.execPath, cli, "init", directory, "--language", language, "--no-install"],
      { stdin: "ignore", stdout: "pipe", stderr: "pipe", env: { ...process.env, PATH: "" } },
    );
    const [status, stdout, stderr] = await Promise.all([
      child.exited,
      new Response(child.stdout).text(),
      new Response(child.stderr).text(),
    ]);
    expect(status).toBe(0);
    expect(stderr).toBe("");
    expect(stdout).not.toContain("Which language");
    expect(existsSync(join(directory, entry))).toBe(true);
    expect(existsSync(join(directory, otherEntry))).toBe(false);
  });
}

for (const [key, keys] of [
  ["Ctrl+C", "\x03"],
  ["Escape", "\x1b"],
] as const) {
  test(`init cancels on ${key} without creating a project`, async () => {
    const directory = destination();
    const { status, output } = await runInteractiveInit(directory, keys);
    expect(status).toBe(130);
    expect(output).toContain("Project creation cancelled.");
    expect(existsSync(directory)).toBe(false);
  });
}

test("init requires an explicit language without a terminal", async () => {
  const directory = destination();
  const child = Bun.spawn([process.execPath, cli, "init", directory, "--no-install"], {
    stdin: "ignore",
    stdout: "pipe",
    stderr: "pipe",
  });
  const [status, stderr] = await Promise.all([child.exited, new Response(child.stderr).text()]);
  expect(status).toBe(1);
  expect(stderr).toContain("--language go, --language rust, or --language typescript");
  expect(existsSync(directory)).toBe(false);
});

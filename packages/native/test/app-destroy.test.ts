import { expect, test } from "bun:test";
import { fileURLToPath } from "node:url";

test("destroy rejects pending app services and ignores late popup events", () => {
  // Destroying the app must not invalidate the singleton shared by other test files.
  const result = Bun.spawnSync(
    [process.execPath, fileURLToPath(new URL("./fixtures/app-destroy.ts", import.meta.url))],
    { stdout: "pipe", stderr: "pipe", timeout: 5000 },
  );
  expect(result.stderr.toString()).toBe("");
  expect(result.exitCode).toBe(0);
});

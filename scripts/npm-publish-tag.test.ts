import { expect, test } from "bun:test";
import { npmDistTagAddArgs, npmPublishArgs, npmPublishTag } from "./npm-publish-tag.ts";

test("every version publishes onto latest", () => {
  expect(npmPublishTag).toBe("latest");
  expect(npmPublishArgs("pkg.tgz")).toEqual([
    "npm",
    "publish",
    "pkg.tgz",
    "--access",
    "public",
    "--tag",
    "latest",
  ]);
});

test("already-published versions are pointed at latest", () => {
  expect(npmDistTagAddArgs("@quickgui/native", "0.1.4-next.3")).toEqual([
    "npm",
    "dist-tag",
    "add",
    "@quickgui/native@0.1.4-next.3",
    "latest",
  ]);
});

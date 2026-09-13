import { expect, test } from "bun:test";
import { npmPublishArgs, npmPublishTag } from "./npm-publish-tag.ts";

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

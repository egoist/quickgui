import { expect, test } from "bun:test";
import { alreadyPublished, npmPublishArgs, npmPublishTag } from "./npm-publish-tag.ts";

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

test("staged and existing versions are treated as already published", () => {
  expect(
    alreadyPublished(
      'npm error code E409\nnpm error 409 Conflict - Cannot publish over previously staged version "0.1.4-next.4".',
    ),
  ).toBe(true);
  expect(alreadyPublished("npm error code EPUBLISHCONFLICT")).toBe(true);
  expect(alreadyPublished("npm error 404 Not Found")).toBe(false);
});

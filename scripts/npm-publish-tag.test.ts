import { expect, test } from "bun:test";
import { npmPublishArgs, npmPublishTag } from "./npm-publish-tag.ts";

test("stable versions leave the latest dist-tag to npm", () => {
  expect(npmPublishTag("0.1.4")).toBeUndefined();
  expect(npmPublishTag("1.0.0+build.9")).toBeUndefined();
  expect(npmPublishArgs("pkg.tgz", "0.1.4")).toEqual(["npm", "publish", "pkg.tgz", "--access", "public"]);
});

test("prereleases use the first identifier as the dist-tag", () => {
  expect(npmPublishTag("0.1.4-next.3")).toBe("next");
  expect(npmPublishTag("1.0.0-beta.1")).toBe("beta");
  expect(npmPublishTag("2.0.0-rc.0+sha.1")).toBe("rc");
  expect(npmPublishArgs("pkg.tgz", "0.1.4-next.3")).toEqual([
    "npm",
    "publish",
    "pkg.tgz",
    "--access",
    "public",
    "--tag",
    "next",
  ]);
});

test("rejects prereleases whose first identifier cannot be an npm tag", () => {
  expect(() => npmPublishTag("0.1.4-3")).toThrow(/letter-led identifier/);
  expect(() => npmPublishTag("0.1.4-0.3.7")).toThrow(/letter-led identifier/);
});

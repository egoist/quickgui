import { expect, test } from "bun:test";

import { extractReleaseNotes, headingVersion, releaseNotes } from "./changelog.ts";

const changelog = `# Changelog

Everything users should know about each release.

## 1.2.0 - 2026-09-19

Faster startup and a fix for lost drafts.

### Fixed

- Drafts survive a quit during a save.

## 1.1.0

- Added search.

## 1.0.0 - 2026-08-01
`;

test("a heading is ## x.y.z with an optional date", () => {
  expect(headingVersion("## 1.2.0")).toBe("1.2.0");
  expect(headingVersion("## 1.2.0 - 2026-09-19")).toBe("1.2.0");
  expect(headingVersion("## 2.0.0-beta.1  ")).toBe("2.0.0-beta.1");
  for (const line of ["# 1.2.0", "### 1.2.0", "## [1.2.0]x y", "## 1.2.0 (latest)", "## 1.2.0 - soon", "1.2.0"])
    expect(headingVersion(line)).toBeUndefined();
});

test("a release publishes only its own section, subsections included", () => {
  expect(extractReleaseNotes(changelog, "1.2.0")).toBe(
    "Faster startup and a fix for lost drafts.\n\n### Fixed\n\n- Drafts survive a quit during a save.",
  );
  expect(extractReleaseNotes(changelog, "1.1.0")).toBe("- Added search.");
  expect(extractReleaseNotes(changelog.replaceAll("\n", "\r\n"), "1.1.0")).toBe("- Added search.");
  // Present but empty, and absent, both mean there is nothing to publish.
  expect(extractReleaseNotes(changelog, "1.0.0")).toBeUndefined();
  expect(extractReleaseNotes(changelog, "1.2")).toBeUndefined();
});

test("a release without notes names the heading to add", () => {
  expect(releaseNotes(changelog, "1.2.0", "CHANGELOG.md")).toContain("Faster startup");
  expect(() => releaseNotes(changelog, "1.3.0", "CHANGELOG.md")).toThrow(
    "CHANGELOG.md has no notes for this release. Add a `## 1.3.0` section",
  );
  expect(() =>
    releaseNotes(`## 1.3.0\n\n${"x".repeat(16 * 1024 + 1)}\n`, "1.3.0", "CHANGELOG.md"),
  ).toThrow("16 KiB");
});

test("headings inside fenced code are part of the notes", () => {
  const fenced = [
    "## 2.0.0",
    "",
    "Changelogs now look like this:",
    "",
    "```md",
    "## 1.0.0",
    "- example",
    "```",
    "",
    "- Done",
    "",
    "## 1.0.0",
    "",
    "- First",
  ].join("\n");
  expect(extractReleaseNotes(fenced, "2.0.0")).toBe(
    "Changelogs now look like this:\n\n```md\n## 1.0.0\n- example\n```\n\n- Done",
  );
  expect(extractReleaseNotes(fenced, "1.0.0")).toBe("- First");
});

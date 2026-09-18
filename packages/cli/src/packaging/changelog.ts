/**
 * Release notes come from one changelog that covers every version. A release publishes the
 * section under the `## x.y.z` heading of the version being built; a ` - YYYY-MM-DD` date may follow.
 */

import { CliError } from "../error.ts";

/** Largest notes section an appcast carries; the updater enforces the same limit. */
export const MAX_RELEASE_NOTES_BYTES = 16 * 1024;

/**
 * The version a heading names, or undefined for any other line. A heading is `## x.y.z`,
 * optionally followed by a date: `## x.y.z - 2026-09-19`.
 */
export function headingVersion(line: string): string | undefined {
  return /^## (\S+)(?: - \d{4}-\d{2}-\d{2})?\s*$/.exec(line)?.[1];
}

/** Body of the section for `version`, without its heading. Undefined when there is none. */
export function extractReleaseNotes(changelog: string, version: string): string | undefined {
  const lines = changelog.split(/\r?\n/);
  // A `## ` line inside a fenced code block is sample text, not a heading.
  let fence: string | undefined;
  const headings = lines.map((line) => {
    const marker = /^ {0,3}(`{3,}|~{3,})/.exec(line)?.[1];
    if (marker && fence === undefined) fence = marker[0];
    else if (marker && marker[0] === fence && line.trim() === marker) fence = undefined;
    else if (fence === undefined && /^##\s/.test(line)) return true;
    return false;
  });
  const start = lines.findIndex((line, index) => headings[index] && headingVersion(line) === version);
  if (start === -1) return undefined;
  const end = headings.indexOf(true, start + 1);
  const body = lines
    .slice(start + 1, end === -1 ? undefined : end)
    .join("\n")
    .trim();
  return body.length > 0 ? body : undefined;
}

/** Notes for a release, or an error that says exactly which heading to add. */
export function releaseNotes(changelog: string, version: string, path: string): string {
  const notes = extractReleaseNotes(changelog, version);
  if (notes === undefined)
    throw new CliError(
      `${path} has no notes for this release. Add a \`## ${version}\` section with at least one line.`,
    );
  if (Buffer.byteLength(notes) > MAX_RELEASE_NOTES_BYTES)
    throw new CliError(
      `The \`## ${version}\` section of ${path} exceeds ${MAX_RELEASE_NOTES_BYTES / 1024} KiB`,
    );
  return notes;
}

/** QuickGUI publishes every version onto `latest`. npm 11 requires this flag for prereleases. */
export const npmPublishTag = "latest";

export function npmPublishArgs(archive: string): string[] {
  return ["npm", "publish", archive, "--access", "public", "--tag", npmPublishTag];
}

export function alreadyPublished(stderr: string): boolean {
  return /eexist|epublishconflict|e409|previously published|previously staged|cannot publish over|403 forbidden/i.test(
    stderr,
  );
}

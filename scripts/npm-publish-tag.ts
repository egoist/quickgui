/** Dist-tag for `npm publish`. Stable versions keep npm's default `latest`. */
export function npmPublishTag(version: string): string | undefined {
  const core = version.split("+", 1)[0] ?? version;
  const dash = core.indexOf("-");
  if (dash === -1) return undefined;
  const tag = core.slice(dash + 1).split(".", 1)[0];
  if (!tag || !/^[A-Za-z][\w.-]*$/.test(tag)) {
    throw new Error(
      `Prerelease ${version} needs a letter-led identifier for the npm dist-tag (for example 0.1.4-next.3)`,
    );
  }
  return tag;
}

export function npmPublishArgs(archive: string, version: string): string[] {
  const args = ["npm", "publish", archive, "--access", "public"];
  const tag = npmPublishTag(version);
  if (tag) args.push("--tag", tag);
  return args;
}

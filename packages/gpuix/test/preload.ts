import { plugin } from "bun";
import { dirname, resolve } from "node:path";
const runtime = resolve(
  dirname(Bun.resolveSync("solid-js/package.json", import.meta.dir)),
  "dist/solid.js",
);
plugin({
  name: "solid-client",
  setup(build) {
    build.module("solid-js", async () => ({ exports: await import(runtime), loader: "object" }));
  },
});

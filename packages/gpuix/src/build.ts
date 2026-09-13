import { dirname, resolve } from "node:path";
import type { BunPlugin } from "bun";
export function gpuixQuickGuiPlugin(): BunPlugin {
  return {
    name: "quickgui-gpuix",
    setup(build) {
      build.onResolve({ filter: /^@gpuix\/native$/ }, () => ({
        path: resolve(import.meta.dir, "native.ts"),
      }));
      build.onResolve({ filter: /use-window-size\.js$/ }, (args) =>
        args.importer.includes("@gpuix")
          ? { path: resolve(import.meta.dir, "window-size.ts") }
          : undefined,
      );
      // Select the client Solid runtime used only by QuickGUI's shared property encoder.
      build.onResolve({ filter: /^solid-js$/ }, () => ({
        path: resolve(
          dirname(Bun.resolveSync("solid-js/package.json", import.meta.dir)),
          "dist/solid.js",
        ),
      }));
    },
  };
}

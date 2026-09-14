#!/usr/bin/env bun
/** Compare the public renderer and gallery with the repository's component inventory. */
import { readFileSync } from "node:fs";
import { ALL_COMPONENT_DOCS } from "../website/src/lib/component-docs.ts";
import * as ui from "../packages/solid/src/index.ts";
import * as swift from "../packages/solid/src/swift-ui.ts";
import * as editor from "../extensions/editor/js/index.ts";
import * as markdown from "../extensions/markdown/js/index.ts";
import * as terminal from "../extensions/terminal/js/index.ts";

const extensionComponents = { ...editor, ...markdown, ...terminal } as Record<string, unknown>;

for (const component of ALL_COMPONENT_DOCS) {
  const name = component.slug === "svg" ? "Svg" : component.name;
  const module = (component.kind === "ui" ? ui : swift) as Record<string, unknown>;
  const value = extensionComponents[name] ?? module[name];
  if (typeof value !== "function" && typeof value !== "object")
    throw new Error(`Missing TypeScript component ${component.kind}/${name}`);
}
const go = readFileSync(new URL("../examples/components/main.go", import.meta.url), "utf8");
const solid = readFileSync(
  new URL("../examples/components-typescript/app.tsx", import.meta.url),
  "utf8",
);
const goIds = [...go.matchAll(/\{"([a-z-]+)", "[^"]+",/g)].map((match) => match[1]!);
const catalog = solid.match(/const DEMOS:[\s\S]*?= \[([\s\S]*?)\n\];/)?.[1];
if (!catalog) throw new Error("TypeScript gallery inventory missing");
const ids = new Set([...catalog.matchAll(/id: "([a-z-]+)"/g)].map((match) => match[1]!));
for (const id of goIds)
  if (!ids.has(id)) throw new Error(`TypeScript gallery is missing the Go demo ${id}`);
console.log(
  `TypeScript coverage: ${ALL_COMPONENT_DOCS.length} documented components; ${ids.size} demos cover all ${goIds.length} Go gallery entries`,
);

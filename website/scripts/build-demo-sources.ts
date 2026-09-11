import { existsSync, readFileSync, mkdirSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { createHighlighter } from "shiki";
import { getComponentApi } from "../src/lib/component-api.server";
import type { DemoSource } from "../src/lib/demo-source";
import { DOCS_FRONTENDS } from "../src/lib/docs";

const root = resolve(import.meta.dir, "../..");
const entryPath = "crates/quickgui-docs-demo/src/lib.rs";
const destination = resolve(root, "website/src/lib/generated/demo-sources.json");

export async function buildDemoSources(check = false) {
  const entry = readFileSync(resolve(root, entryPath), "utf8");
  const ids = [
    ...entry.match(/pub const COMPONENTS:[\s\S]*?= &\[([\s\S]*?)\];/)![1].matchAll(/"([a-z-]+)"/g),
  ].map((match) => match[1]);
  const highlighter = await createHighlighter({
    langs: ["go", "tsx", "rust"],
    themes: ["github-light-high-contrast", "github-dark-high-contrast"],
  });
  const result: Record<string, DemoSource> = {};
  for (const component of ids) {
    for (const frontend of DOCS_FRONTENDS) {
      const { example: code, language } = getComponentApi(frontend, "ui", component);
      if (!code.trim()) throw new Error(`Missing preview example: ${frontend}/${component}`);
      result[`${frontend}/${component}`] = {
        path: `website/src/content/docs/${frontend}/components/ui/${component}.mdx`,
        language,
        code,
        html: highlighter
          .codeToHtml(code, {
            lang: language,
            themes: { light: "github-light-high-contrast", dark: "github-dark-high-contrast" },
            defaultColor: false,
          })
          .replace("<pre ", '<pre tabindex="0" '),
      };
    }
  }
  highlighter.dispose();
  const output = JSON.stringify(result) + "\n";
  if (check) {
    if (readFileSync(destination, "utf8") !== output)
      throw new Error("Preview source is stale. Run bun run docs:api && bun ./scripts/build-demo-sources.ts.");
  } else {
    mkdirSync(resolve(destination, ".."), { recursive: true });
    if (!existsSync(destination) || readFileSync(destination, "utf8") !== output)
      writeFileSync(destination, output);
  }
  console.log(`Preview source: ${ids.length} demos in ${DOCS_FRONTENDS.length} frontend languages.`);
}
if (import.meta.main) await buildDemoSources(process.argv.includes("--check"));

import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { DOCS_GUIDE_ORDER, docsOutline } from "../src/lib/docs-structure";
import type { Locale } from "../src/i18n";

const root = resolve(import.meta.dir, "../src/content/docs/rust");
const notes: Record<Exclude<Locale, "en">, string> = {
  ja: "Rust ガイド本文は現在英語です。コンポーネントの API リファレンスには日本語の説明があります。",
  zh: "Rust 指南正文目前以英文提供，组件 API 参考包含中文说明。",
};

for (const locale of ["ja", "zh"] as const) {
  mkdirSync(resolve(root, locale), { recursive: true });
  for (const slug of DOCS_GUIDE_ORDER) {
    const english = readFileSync(resolve(root, "en", `${slug}.mdx`), "utf8");
    const titles = docsOutline(slug, locale).map((item) => item.title);
    const headings = [...english.matchAll(/^## (.+)$/gm)].map((match) => match[1]!);
    if (headings.length !== titles.length) {
      throw new Error(`${slug}: ${headings.length} English H2s, ${titles.length} ${locale} titles`);
    }
    let index = 0;
    const body = english.replace(/^## .+$/gm, () => `## ${titles[index++]}`);
    writeFileSync(resolve(root, locale, `${slug}.mdx`), `${notes[locale]}\n\n${body}`);
  }
}
console.log("Wrote localized Rust guide MDX files");

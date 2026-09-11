import type { HighlightedSnippets, SnippetKey } from "../lib/snippets";

let cached: Promise<HighlightedSnippets> | null = null;

async function highlightAll(): Promise<HighlightedSnippets> {
  const [
    { snippets },
    { createHighlighterCore },
    { createJavaScriptRegexEngine },
    go,
    bash,
    tsx,
    rust,
    githubLight,
    githubDark,
  ] = await Promise.all([
    import("../lib/snippets"),
    import("shiki/core"),
    import("shiki/engine/javascript"),
    import("shiki/langs/go.mjs"),
    import("shiki/langs/bash.mjs"),
    import("shiki/langs/tsx.mjs"),
    import("shiki/langs/rust.mjs"),
    import("shiki/themes/github-light.mjs"),
    import("shiki/themes/github-dark.mjs"),
  ]);

  const highlighter = await createHighlighterCore({
    themes: [githubLight.default, githubDark.default],
    langs: [go.default, bash.default, tsx.default, rust.default],
    engine: createJavaScriptRegexEngine({ forgiving: true }),
  });

  const out = {} as HighlightedSnippets;
  for (const key of Object.keys(snippets) as Array<SnippetKey>) {
    const { lang, code } = snippets[key];
    out[key] = highlighter.codeToHtml(code, {
      lang,
      themes: { light: "github-light", dark: "github-dark" },
      defaultColor: false,
    });
  }
  return out;
}

export function getHighlightedSnippets(): Promise<HighlightedSnippets> {
  cached ??= highlightAll();
  return cached;
}

import { useTranslation } from "react-i18next";
import type { HighlightedSnippets, SnippetKey } from "../../lib/snippets";
import { CodeBlock } from "../code-block";
import type { DocsFrontend } from "../../lib/docs";
import { FrontendPicker } from "../frontend-picker";

const examples: Record<DocsFrontend, { file: string; snippet: SnippetKey }> = {
  go: { file: "swiftui.go", snippet: "swiftUi" },
  typescript: { file: "swiftui.tsx", snippet: "typescriptSwiftUi" },
  rust: { file: "swiftui.rs", snippet: "rustSwiftUi" },
};

export function SwiftUi({
  highlighted,
  frontend,
  onFrontendChange,
}: {
  highlighted: HighlightedSnippets;
  frontend: DocsFrontend;
  onFrontendChange: (frontend: DocsFrontend) => void;
}) {
  const { t } = useTranslation();

  return (
    <section id="swift-ui" className="border-b border-border">
      <div className="grid gap-px bg-border lg:grid-cols-[2fr_3fr]">
        <div className="bg-background px-6 py-16 sm:px-12 sm:py-20">
          <div className="flex items-center gap-2 font-mono text-xs text-peach">
            <span className="i-simple-icons-swift size-4" aria-hidden />
            Native SwiftUI · macOS
          </div>
          <h2 className="mt-6 text-3xl font-semibold tracking-tight text-balance sm:text-4xl">
            {t("swiftUi.title")}
          </h2>
          <p className="mt-4 max-w-lg text-base leading-relaxed text-muted-foreground">
            {t("swiftUi.lead")}
          </p>
        </div>

        <div className="min-w-0 bg-card-2 p-6 sm:p-12">
          <div className="overflow-hidden border border-border bg-background">
            <div className="flex flex-wrap items-center justify-between gap-3 border-b border-border bg-card-2 px-5 py-2.5">
              <span className="font-mono text-xs">{examples[frontend].file}</span>
              <FrontendPicker value={frontend} onChange={onFrontendChange} />
            </div>
            <CodeBlock html={highlighted[examples[frontend].snippet]} />
          </div>
        </div>
      </div>
    </section>
  );
}

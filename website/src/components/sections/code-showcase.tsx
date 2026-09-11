import { useTranslation } from "react-i18next";
import { cn } from "@/lib/utils";
import { CodeBlock } from "../code-block";
import { SectionHeading } from "../section-heading";
import type { HighlightedSnippets } from "../../lib/snippets";
import type { DocsFrontend } from "../../lib/docs";

const PANES = [
  {
    key: "go",
    snippet: "counter",
    file: "counter.go",
    name: "Go",
  },
  { key: "typescript", snippet: "typescriptCounter", file: "counter.tsx", name: "TypeScript" },
  { key: "rust", snippet: "rustCounter", file: "main.rs", name: "Rust" },
] as const;

export function CodeShowcase({
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
    <section id="code" className="border-b border-border">
      <SectionHeading title={t("code.title")} lead={t("code.lead")} />

      {/* Mobile: tab bar switches panes; desktop shows all three side by side. */}
      <div className="flex border-b border-border lg:hidden">
        {PANES.map((pane) => (
          <button
            key={pane.key}
            type="button"
            aria-pressed={frontend === pane.key}
            onClick={() => onFrontendChange(pane.key)}
            className={cn(
              "flex-1 border-r border-border py-3 font-mono text-xs transition-colors last:border-r-0",
              frontend === pane.key
                ? "bg-background text-foreground"
                : "bg-card-2 text-muted-foreground",
            )}
          >
            {pane.file}
          </button>
        ))}
      </div>

      <div className="grid gap-px bg-border lg:grid-cols-3">
        {PANES.map((pane) => (
          <div
            key={pane.key}
            className={cn("min-w-0 bg-background", frontend !== pane.key && "hidden lg:block")}
          >
            <div className="hidden items-center justify-between border-b border-border bg-card-2 px-5 py-2.5 lg:flex">
              <span className="font-mono text-xs">{pane.file}</span>
              <span className="flex items-center gap-1.5 font-mono text-[11px] text-muted-foreground">
                {pane.name} · QuickGUI
              </span>
            </div>
            <CodeBlock html={highlighted[pane.snippet]} />
          </div>
        ))}
      </div>
    </section>
  );
}

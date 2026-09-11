import { useTranslation } from "react-i18next";
import { Button } from "./ui/button";
import { DOCS_FRONTENDS, docsPath, frontendLabel, type DocsFrontend } from "../lib/docs";

const icons: Record<DocsFrontend, string> = {
  go: "i-simple-icons-go size-6 text-[#00add8]",
  typescript: "i-simple-icons-typescript size-5 text-[#3178c6]",
  rust: "i-simple-icons-rust size-5 text-[#dea584]",
};

export function FrontendDocsLinks({ externalArrow = false }: { externalArrow?: boolean }) {
  const { t, i18n } = useTranslation();
  const prefix = i18n.language === "en" ? "" : `/${i18n.language}`;
  return (
    <>
      {DOCS_FRONTENDS.map((frontend) => (
        <Button key={frontend} asChild variant="outline" className="h-10 gap-2 px-5 text-sm">
          <a href={`${prefix}${docsPath(frontend)}`}>
            <span className={icons[frontend]} aria-hidden />
            {t("common.docsFor", { language: frontendLabel(frontend) })}
            <span
              className={
                externalArrow ? "i-lucide-arrow-up-right size-4" : "i-lucide-arrow-right size-4"
              }
              aria-hidden
            />
          </a>
        </Button>
      ))}
    </>
  );
}

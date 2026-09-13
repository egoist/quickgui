import { expect, test } from "bun:test";
import { renderToString } from "react-dom/server";
import { I18nextProvider } from "react-i18next";
import { MemoryRouter } from "react-router";
import { Hero } from "../src/components/sections/hero";
import { CodeShowcase } from "../src/components/sections/code-showcase";
import { SwiftUi } from "../src/components/sections/swift-ui";
import { Quickstart } from "../src/components/sections/quickstart";
import { FinalCta } from "../src/components/sections/final-cta";
import { DocsShell } from "../src/components/docs/docs-shell";
import { DOCS_FRONTENDS, docsPath, frontendLabel } from "../src/lib/docs";
import { createI18n, SUPPORTED_LOCALES } from "../src/i18n";
import { getCounterTokens, getHighlightedSnippets, getSwiftUiTokens } from "../src/server/highlight.server";

const plain = (html: string) => html.replace(/<[^>]*>/g, "");

test("homepage frontends share the init command and select their examples and docs links", async () => {
  const highlighted = await getHighlightedSnippets();
  const counters = await getCounterTokens();
  const swiftUi = await getSwiftUiTokens();
  for (const locale of SUPPORTED_LOCALES) {
    const i18n = createI18n(locale);
    const prefix = locale === "en" ? "" : `/${locale}`;
    for (const frontend of DOCS_FRONTENDS) {
      const render = (children: React.ReactNode) =>
        renderToString(<I18nextProvider i18n={i18n}>{children}</I18nextProvider>);
      const hero = render(<Hero />);
      const command = "bunx @quickgui/cli init my-app";
      expect(plain(hero)).toContain(i18n.t("hero.badge"));
      expect(plain(hero)).toContain(i18n.t("hero.badgeHint"));
      expect(plain(hero)).toContain(command);
      expect(hero).not.toContain("--frontend");
      expect(hero).not.toContain('role="group"');
      const cta = render(<FinalCta />);
      const showcase = render(
        <CodeShowcase tokens={counters} frontend={frontend} onFrontendChange={() => {}} />,
      );
      expect(showcase.match(/role="tab"/g)).toHaveLength(3);
      expect(showcase.match(/role="tabpanel"/g)).toHaveLength(1);
      expect(showcase.match(/aria-selected="true"/g)).toHaveLength(1);
      expect(showcase).toContain(`href="${prefix}${docsPath(frontend)}"`);
      expect(plain(showcase)).toContain(i18n.t(`code.frontends.${frontend}`));
      const signatures = {
        go: "func Counter()",
        typescript: "function Counter()",
        rust: "impl View for Counter",
      };
      for (const target of DOCS_FRONTENDS) {
        expect(plain(showcase).includes(signatures[target])).toBe(target === frontend);
      }
      for (const target of DOCS_FRONTENDS) {
        for (const html of [hero, cta]) {
          expect(html).toContain(`href="${prefix}${docsPath(target)}"`);
          expect(plain(html)).toContain(
            i18n.t("common.docsFor", { language: frontendLabel(target) }),
          );
        }
      }
      const swift = plain(
        render(
          <SwiftUi tokens={swiftUi} frontend={frontend} onFrontendChange={() => {}} />,
        ),
      );
      const extension = { go: "go", typescript: "tsx", rust: "rs" }[frontend];
      expect(swift).toContain(`swiftui.${extension}`);
      expect(swift).toContain(
        {
          go: "ui.SwiftUI.Host",
          typescript: "Host matchContents",
          rust: "MacSwiftUiHost",
        }[frontend],
      );
      const quickstart = plain(
        render(
          <Quickstart highlighted={highlighted} frontend={frontend} />,
        ),
      );
      expect(quickstart.replace(/\\\s*\n\s*/g, " ").replace(/\s+/g, " ")).toContain(command);
      expect(quickstart).not.toContain("--frontend");
    }
  }
});

test("the docs picker includes and selects every frontend in all locales", () => {
  for (const locale of SUPPORTED_LOCALES) {
    for (const frontend of DOCS_FRONTENDS) {
      const html = renderToString(
        <MemoryRouter>
          <DocsShell
            frontend={frontend}
            locale={locale}
            page={{
              title: "Getting started",
              description: "",
              outline: [],
              path: docsPath(frontend),
              area: "guide",
            }}
          >
            <p>Guide content</p>
          </DocsShell>
        </MemoryRouter>,
      );
      for (const option of DOCS_FRONTENDS) {
        expect(html).toContain(`<option value="${option}"`);
      }
      expect(html).toContain(`<option value="${frontend}" selected=""`);
      const docsPrefix = locale === "en" ? "" : `/${locale}`;
      expect(html).toContain(`href="${docsPrefix}${docsPath(frontend, "reactivity")}"`);
      expect(html).not.toContain(`href="${docsPrefix}${docsPath(frontend, "swift-ui-hosting")}"`);
      expect(html).not.toContain(`/docs/${frontend}/components/`);
      expect(html).not.toContain(`/docs/${frontend}/swift-ui/`);
    }
  }
});

test("components and SwiftUI docs pages render their own sidebars", () => {
  for (const frontend of DOCS_FRONTENDS) {
    const prefix = `/docs/${frontend}`;
    const components = renderToString(
      <MemoryRouter>
        <DocsShell
          frontend={frontend}
          locale="en"
          page={{
            title: "Components",
            description: "",
            outline: [],
            path: docsPath(frontend, "components"),
            area: "components",
          }}
        >
          <p>Components content</p>
        </DocsShell>
      </MemoryRouter>,
    );
    expect(components).toContain(`href="${prefix}/components"`);
    expect(components).toContain(`href="${prefix}/components/button"`);
    expect(components).not.toContain(`href="${prefix}/reactivity"`);
    expect(components).not.toContain(`href="${prefix}/swift-ui-hosting"`);
    expect(components).not.toContain(`href="${prefix}/swift-ui/button"`);

    const swiftUi = renderToString(
      <MemoryRouter>
        <DocsShell
          frontend={frontend}
          locale="en"
          page={{
            title: "SwiftUI",
            description: "",
            outline: [],
            path: docsPath(frontend, "swift-ui"),
            area: "swift-ui",
          }}
        >
          <p>SwiftUI content</p>
        </DocsShell>
      </MemoryRouter>,
    );
    expect(swiftUi).toContain(`href="${prefix}/swift-ui"`);
    expect(swiftUi).toContain(`href="${prefix}/swift-ui-hosting"`);
    expect(swiftUi).toContain(`href="${prefix}/swift-ui/button"`);
    expect(swiftUi).not.toContain(`href="${prefix}/reactivity"`);
    expect(swiftUi).not.toContain(`href="${prefix}/components/button"`);
  }
});

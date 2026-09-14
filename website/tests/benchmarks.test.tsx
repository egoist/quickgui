import { expect, spyOn, test } from "bun:test";
import { renderToReadableStream, renderToString } from "react-dom/server";
import { I18nextProvider } from "react-i18next";
import { Benchmarks } from "../src/components/sections/benchmarks";
import measured from "../src/data/desktop-benchmarks.json";
import { createI18n, SUPPORTED_LOCALES } from "../src/i18n";

test("server-rendered benchmark charts include every localized SVG title", async () => {
  const errors = spyOn(console, "error").mockImplementation(() => {});
  try {
    for (const locale of SUPPORTED_LOCALES) {
      const i18n = createI18n(locale);
      const tree = (
        <I18nextProvider i18n={i18n}>
          <Benchmarks />
        </I18nextProvider>
      );
      const streamed = await new Response(await renderToReadableStream(tree)).text();
      for (const html of [renderToString(tree), streamed]) {
        const titles = [...html.matchAll(/<title\b[^>]*>(.*?)<\/title>/gs)].map(
          (match) => match[1],
        );
        const expected: string[] = [];
        const number = (value: number) =>
          new Intl.NumberFormat(locale, {
            maximumFractionDigits: 1,
          }).format(value / 1_000_000);
        for (const metric of ["memory", "bundle"] as const) {
          expected.push(i18n.t(`benchmarks.${metric}`));
          for (const row of measured.results) {
            const value = number(metric === "memory" ? row.memoryBytes : row.bundleBytes);
            const range =
              metric === "memory"
                ? ` · ${i18n.t("benchmarks.range", {
                    min: number(row.memoryMinBytes),
                    max: number(row.memoryMaxBytes),
                  })}`
                : "";
            expected.push(`${row.name} ${row.version}: ${value} MB${range}`);
          }
        }
        expect(titles).toEqual(expected);
      }
    }
    expect(errors).not.toHaveBeenCalled();
  } finally {
    errors.mockRestore();
  }
});

test("all measured apps fit inside the charts and appear in the preview picker", () => {
  expect(measured.results.map((row) => row.id)).toEqual([
    "quickgui-go",
    "quickgui-typescript",
    "quickgui-rust",
    "gpui",
    "tauri",
    "electron",
  ]);
  const html = renderToString(
    <I18nextProvider i18n={createI18n("en")}>
      <Benchmarks />
    </I18nextProvider>,
  );
  const charts = [...html.matchAll(/<svg\b[^>]*viewBox="0 0 [\d.]+ ([\d.]+)"[^>]*>(.*?)<\/svg>/gs)];
  expect(charts).toHaveLength(2);
  for (const chart of charts) {
    const height = Number(chart[1]);
    const bars = [
      ...chart[2]!.matchAll(/<rect\b[^>]*y="([\d.]+)"[^>]*height="([\d.]+)"[^>]*fill="([^"]+)"/g),
    ];
    expect(bars).toHaveLength(measured.results.length);
    for (const bar of bars) {
      expect(Number(bar[1]) + Number(bar[2])).toBeLessThan(height - 34);
      expect(bar[3]).toBeTruthy();
    }
  }
  const previews = [
    ...html.matchAll(/<button\b[^>]*aria-pressed="(?:true|false)"[^>]*>(.*?)<\/button>/gs),
  ];
  expect(previews.map((match) => match[1])).toEqual(measured.results.map((row) => row.name));
});

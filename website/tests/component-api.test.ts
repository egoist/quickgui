import { getDemoSource } from "../src/lib/demo-source.server";
import { expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { ALL_COMPONENT_DOCS } from "../src/lib/component-docs";
import { getComponentApi } from "../src/lib/component-api.server";
import { DEMO_COMPONENTS } from "../src/lib/component-demos";
import { DOCS_FRONTENDS } from "../src/lib/docs";

const root = resolve(import.meta.dir, "../..");
test("each frontend reference uses real declarations, examples, and source targets", () => {
  const sourceLengths = new Map<string, number>();
  for (const component of ALL_COMPONENT_DOCS)
    for (const frontend of DOCS_FRONTENDS) {
      const api = getComponentApi(frontend, component.kind, component.slug);
      expect(api.example.length).toBeGreaterThan(0);
      expect(api.sections.length).toBeGreaterThan(0);
      for (const section of api.sections) {
        expect(section.signature).not.toContain("undefined");
        for (const entry of [section, ...section.entries]) {
          const [file, line] = entry.source.split("#L");
          if (!sourceLengths.has(file)) {
            expect(existsSync(resolve(root, file))).toBe(true);
            sourceLengths.set(file, readFileSync(resolve(root, file), "utf8").split("\n").length);
          }
          expect(Number(line)).toBeGreaterThan(0);
          expect(sourceLengths.get(file)!).toBeGreaterThanOrEqual(Number(line));
        }
        expect(new Set(section.entries.map((entry) => entry.name)).size).toBe(
          section.entries.length,
        );
      }
    }
});

test("compound props preserve callback types, inherited fields, and namespaces", () => {
  const slider = getComponentApi("go", "ui", "slider").sections.find(
    (section) => section.name === "Slider.Root",
  )!;
  expect(slider.signature).toStartWith("ui.Slider.Root(");
  expect(slider.entries.find((entry) => entry.name === "OnValueChange")?.type).toBe(
    "func([]float64, *native.Event)",
  );
  expect(slider.entries.some((entry) => entry.name === "Disabled")).toBe(true);
  const terminal = getComponentApi("go", "ui", "terminal").sections[0];
  expect(terminal.signature).toStartWith("terminal.View(");
  expect(terminal.entries.some((entry) => entry.name === "Program")).toBe(true);
  const view = getComponentApi("typescript", "ui", "view").sections[0];
  for (const name of ["rounded-lg", "flex-col", "p-3", "text-lg", "background-color", "border-radius"]) {
    expect(view.entries.find((entry) => entry.name === name)?.source).toStartWith(
      "packages/solid/src/style-helpers.generated.ts#L",
    );
  }
  expect(view.entries.some((entry) => entry.name === "rounded")).toBe(false);
});

test("the browser catalog matches the Rust demo dispatcher", () => {
  const source = readFileSync(resolve(root, "crates/quickgui-docs-demo/src/lib.rs"), "utf8");
  const declared = [
    ...source.match(/pub const COMPONENTS:[\s\S]*?= &\[([\s\S]*?)\];/)![1].matchAll(/"([a-z-]+)"/g),
  ].map((match) => match[1]);
  expect(DEMO_COMPONENTS).toEqual(declared);
  expect(new Set(declared).size).toBe(declared.length);
  for (const name of declared)
    expect(
      ALL_COMPONENT_DOCS.some((component) => component.kind === "ui" && component.slug === name),
    ).toBe(true);
  expect(declared).not.toContain("terminal");
  expect(declared).not.toContain("system-popover");
});

test("every preview uses the selected frontend's documented example and syntax highlighting", () => {
  for (const component of DEMO_COMPONENTS) {
    for (const frontend of DOCS_FRONTENDS) {
      const demo = getDemoSource(frontend, component)!;
      expect(demo).toBeDefined();
      expect(demo.language).toBe(frontend === "typescript" ? "tsx" : frontend);
      expect(demo.path).toBe(`website/src/content/docs/${frontend}/components/ui/${component}.mdx`);
      const source = readFileSync(resolve(root, demo.path), "utf8");
      expect(demo.code.trim().length).toBeGreaterThan(0);
      expect(source).toContain(`\`\`\`${demo.language}\n${demo.code}\n\`\`\``);
      expect(demo.code).toBe(getComponentApi(frontend, "ui", component).example);
      expect(demo.html).toContain('class="shiki ');
      expect(demo.html).toContain("--shiki-light:");
    }
  }
  expect(getDemoSource("go", "button")!.code).toContain("ui.Button(");
  expect(getDemoSource("typescript", "button")!.code).toContain("<Button");
  expect(getDemoSource("rust", "button")!.code).toContain("button()");
  for (const frontend of DOCS_FRONTENDS) {
    expect(getDemoSource(frontend, "terminal")).toBeUndefined();
    expect(getDemoSource(frontend, "missing")).toBeUndefined();
  }
});

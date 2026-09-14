import { expect, test } from "bun:test";
import { NativeNodeTag, PropertyCode } from "@quickgui/native";
import { Markdown } from "@quickgui/extension-markdown";

test("QuickGUI and Solid runtimes are peer-only extension dependencies", async () => {
  const manifest = await Bun.file(new URL("../../package.json", import.meta.url)).json();
  expect(manifest.dependencies?.["@quickgui/native"]).toBeUndefined();
  expect(manifest.dependencies?.["@quickgui/solid"]).toBeUndefined();
  expect(manifest.dependencies?.["solid-js"]).toBeUndefined();
  expect(manifest.peerDependencies).toMatchObject({
    "@quickgui/native": "workspace:*",
    "@quickgui/solid": "workspace:*",
    "solid-js": "2.0.0-rc.8",
  });
  expect(manifest.devDependencies).toMatchObject(manifest.peerDependencies);
});

test("Markdown declares streaming and retained CodeBlock rendering", () => {
  const markdown = Markdown({
    content: "# Hello\n\n```rust\nfn main() {}\n```\n",
    streaming: true,
    codeBlockComponent: { package: "third-party", name: "custom-code", props: { theme: "dark" } },
    codeBlockMaxHeight: 280,
    style: { fontSize: 16 },
  });

  expect(markdown.tag).toBe(NativeNodeTag.Extension);
  expect(markdown.properties.get(PropertyCode.ExtensionPackage)).toBe("markdown");
  const props = JSON.parse(markdown.properties.get(PropertyCode.ExtensionProps) as string);
  expect(props.value).toContain("fn main");
  expect(props.streaming).toBe(true);
  expect(props.codeBlockComponent).toEqual({ package: "third-party", name: "custom-code", props: { theme: "dark" } });
  expect(props.codeBlockMaxHeight).toBe(280);
});

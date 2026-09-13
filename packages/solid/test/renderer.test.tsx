import { expect, test } from "bun:test";
import { createMemo, createSignal, flush, For, onCleanup, Show } from "solid-js";
import { app, Window, NativeNodeTag, PropertyCode, type NativeNode } from "@quickgui/native";
import { Button, Text, View, createRenderer } from "../src/index.ts";

await app.whenReady();

function text(node: NativeNode): string {
  return (node.text ?? "") + node.children.map(text).join("");
}

test("textColor reaches native paint states using the canonical color field", () => {
  const host = new Window({
    renderer: createRenderer(() => (
      <Button style={{ hover: { textColor: "#ff0000" } }}>Test</Button>
    )),
  });
  const style = JSON.parse(
    host.root.children[0]!.properties.get(PropertyCode.HoverStyle) as string,
  );
  expect(style.color).toBe(0xff0000ff);
  host.close();
});

test("memo-derived lists mount synchronously and custom component prop spreads preserve children", () => {
  const host = new Window({ renderer: () => () => {} });
  const Caption = (props: { children: string }) => <Text {...props} style={{ fontSize: 12 }} />;
  const dispose = createRenderer(() => {
    const [records] = createSignal(Array.from({ length: 1000 }, (_, i) => String(i)));
    const matching = createMemo(() => records());
    const visible = createMemo(() => matching().slice(0, 100));
    return (
      <View>
        <For each={visible()}>{(item) => <Caption>{item}</Caption>}</For>
      </View>
    );
  })(host);
  expect(host.root.children).toHaveLength(1);
  expect(
    host.root.children[0]!.children.filter((node) => node.tag === NativeNodeTag.View),
  ).toHaveLength(100);
  expect(text(host.root)).toStartWith("012345");
  dispose();
  host.close();
});

test("compiled Solid 2 JSX updates retained nodes and disposes effects without reconstructing components", () => {
  const [count, setCount] = createSignal(0);
  let constructions = 0,
    cleaned = 0;
  const host = new Window({ renderer: () => () => {} });
  function Counter() {
    constructions++;
    onCleanup(() => cleaned++);
    return (
      <view style={{ padding: 12 }}>
        <Text style={{ fontSize: count() + 10 }}>Count: {count()}</Text>
        <Button onClick={() => setCount(count() + 1)}>Increment</Button>
        <Show when={count() > 0}>
          <Text>Visible</Text>
        </Show>
      </view>
    );
  }
  const dispose = createRenderer(Counter)(host);
  const root = host.root.children[0]!;
  const label = root.children[0]!;
  // Rust's Text tag renders its own string, so only leaf nodes may carry that tag.
  for (const node of host.nodes.values())
    if (node.tag === NativeNodeTag.Text) expect(node.children).toHaveLength(0);
  expect(root.properties.get(PropertyCode.Padding)).toBe(12);
  expect(text(root)).toContain("Count: 0");
  const button = root.children.find((node) => node.tag === NativeNodeTag.Button)!;
  host._dispatchEvent("click", button.id);
  flush();
  expect(root.children[0]).toBe(label);
  expect(label.properties.get(PropertyCode.FontSize)).toBe(11);
  expect(text(root)).toContain("Count: 1");
  expect(text(root)).toContain("Visible");
  expect(constructions).toBe(1);
  setCount(0);
  flush();
  expect(text(root)).not.toContain("Visible");
  dispose();
  expect(cleaned).toBe(1);
  expect(host.nodes.size).toBe(1);
  setCount(4);
  flush();
  expect(host.nodes.size).toBe(1);
  host.close();
});

test("keyed lists preserve native identities and independent windows own their own reactive roots", () => {
  const [items, setItems] = createSignal(["a", "b", "c"]);
  const first = new Window({ renderer: () => () => {} }),
    second = new Window({ renderer: () => () => {} });
  const render = createRenderer(() => (
    <View>
      <For each={items()}>{(item) => <Text>{item}</Text>}</For>
    </View>
  ));
  const disposeFirst = render(first),
    disposeSecond = render(second);
  const rows = first.root.children[0]!;
  const before = new Map(
    rows.children
      .filter((node) => node.tag === NativeNodeTag.View)
      .map((node) => [text(node), node]),
  );
  setItems(["c", "a", "b"]);
  flush();
  expect(rows.children.filter((node) => node.tag === NativeNodeTag.View)).toEqual([
    before.get("c")!,
    before.get("a")!,
    before.get("b")!,
  ]);
  disposeFirst();
  setItems(["b"]);
  flush();
  expect(text(second.root)).toBe("b");
  expect(first.nodes.size).toBe(1);
  disposeSecond();
  first.close();
  second.close();
});

test("View and intrinsic div render direct text and update reactive text in place", async () => {
  const [value, setValue] = createSignal("one");
  const view = new Window({ renderer: createRenderer(() => <View>some text {value()}</View>) });
  const div = new Window({ renderer: createRenderer(() => <div>some text {value()}</div>) });
  const roots = [view.root.children[0]!, div.root.children[0]!];
  for (const root of roots) {
    expect(root.tag).toBe(NativeNodeTag.View);
    expect(text(root)).toBe("some text one");
    expect(root.children.every((child) => child.tag === NativeNodeTag.Text)).toBe(true);
  }
  setValue("two");
  flush();
  for (const [index, root] of roots.entries()) {
    expect([view, div][index]!.root.children[0]).toBe(root);
    expect(text(root)).toBe("some text two");
  }
  view.close();
  div.close();
  await Promise.resolve();
  setValue("three");
  flush();
  expect(view.nodes.size).toBe(0);
  expect(div.nodes.size).toBe(0);
});

test("a TSX module with no imports can render an intrinsic div", async () => {
  const { IntrinsicDiv } = await import("./fixtures/intrinsic-div.tsx");
  const host = new Window({ renderer: createRenderer(IntrinsicDiv) });
  const root = host.root.children[0]!;
  expect(root.tag).toBe(NativeNodeTag.View);
  expect(root.properties.get(PropertyCode.FontSize)).toBe(18);
  expect(text(root)).toBe("some text");
  const span = root.children[1]!;
  expect(span.tag).toBe(NativeNodeTag.View);
  expect(span.properties.get(PropertyCode.Color)).toBe(0xff563412);
  host.close();
});

test("span uses the same native text container and reactive props as Text", () => {
  const [value, setValue] = createSignal("one");
  const [size, setSize] = createSignal(14);
  const host = new Window({
    renderer: createRenderer(() => (
      <div>
        <Text style={{ fontSize: size() }}>{value()}</Text>
        <span style={{ fontSize: size() }}>{value()}</span>
      </div>
    )),
  });
  const [textNode, span] = host.root.children[0]!.children;
  expect(span!.tag).toBe(textNode!.tag);
  expect([...span!.properties]).toEqual([...textNode!.properties]);
  setValue("two");
  setSize(20);
  flush();
  expect(host.root.children[0]!.children[1]).toBe(span!);
  expect(text(span!)).toBe("two");
  expect(span!.properties.get(PropertyCode.FontSize)).toBe(20);
  host.close();
});

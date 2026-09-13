import { expect, mock, test } from "bun:test";
import { fakeBinding, calls } from "../../native/test/fake-binding.ts";
mock.module("../../native/src/binding.ts", () => fakeBinding);
const { app, PropertyCode } = await import("@quickgui/native");
const { GpuixRenderer } = await import("../src/native.ts");
await app.whenReady();
function host() {
  const renderer = new GpuixRenderer();
  renderer.init();
  return renderer;
}

test("keyed moves retain native identity and equivalent writes submit no work", () => {
  const renderer = host();
  renderer.applyBatch(
    JSON.stringify([
      ["createElement", 1, "div"],
      ["createElement", 2, "text"],
      ["createElement", 3, "text"],
      ["setText", 2, "one"],
      ["setText", 3, "two"],
      ["appendChild", 1, 2],
      ["appendChild", 1, 3],
      ["setRoot", 1],
    ]),
  );
  const second = renderer.records.get(3)!.node;
  renderer.applyBatch(JSON.stringify([["insertBefore", 1, 3, 2]]));
  expect(renderer.records.get(1)!.node.children[0]).toBe(second);
  renderer.applyBatch(JSON.stringify([["setStyle", 2, { color: "#ffffff" }]]));
  renderer.window!.flush();
  const before = calls.length;
  renderer.applyBatch(JSON.stringify([["setStyle", 2, { color: "#ffffff" }]]));
  expect(calls.length).toBe(before);
  renderer.window!.close();
});

test("unsupported styling rejects the whole batch before creating nodes", () => {
  const renderer = host();
  expect(() =>
    renderer.applyBatch(
      JSON.stringify([
        ["createElement", 1, "div"],
        ["setStyle", 1, { madeUp: 42 }],
      ]),
    ),
  ).toThrow("parity gap");
  expect(renderer.records.size).toBe(0);
  renderer.window!.close();
});

test("style withdrawal clears properties and subtree destruction releases every ID", () => {
  const renderer = host();
  renderer.applyBatch(
    JSON.stringify([
      ["createElement", 1, "div"],
      ["createElement", 2, "text"],
      ["appendChild", 1, 2],
      ["setStyle", 2, { color: "#ffffff", padding: 8 }],
      ["setRoot", 1],
    ]),
  );
  renderer.applyBatch(JSON.stringify([["setStyle", 2, { padding: 8 }]]));
  expect(renderer.records.get(2)!.node.properties.has(PropertyCode.Color)).toBe(false);
  expect(renderer.applyBatch(JSON.stringify([["destroyElement", 1]]))).toEqual([1, 2]);
  expect(renderer.records.size).toBe(0);
  renderer.window!.close();
});

test("motion reaches the Rust host as one declaration without JavaScript frame polling", () => {
  const renderer = host();
  const motion = {
    initial: { opacity: 0 },
    animate: { opacity: 1 },
    transition: { duration: 0.3, ease: "easeOut" },
  };
  renderer.applyBatch(
    JSON.stringify([
      ["createElement", 1, "div"],
      ["setCustomProp", 1, "motion", motion],
      ["setRoot", 1],
    ]),
  );
  expect(renderer.records.get(1)!.node.properties.get(PropertyCode.Motion)).toBe(
    JSON.stringify(motion),
  );
  expect(renderer.requiresTick()).toBe(false);
  renderer.window!.close();
});

test("a late invalid insertion cannot leave a partially mutated tree", () => {
  const renderer = host();
  expect(() =>
    renderer.applyBatch(
      JSON.stringify([
        ["createElement", 1, "div"],
        ["appendChild", 1, 99],
      ]),
    ),
  ).toThrow("Unknown GPUIX node 99");
  expect(renderer.records.size).toBe(0);
  renderer.applyBatch(
    JSON.stringify([
      ["createElement", 1, "div"],
      ["createElement", 2, "div"],
      ["appendChild", 1, 2],
      ["setRoot", 1],
    ]),
  );
  expect(() => renderer.applyBatch(JSON.stringify([["appendChild", 2, 1]]))).toThrow("cycle");
  expect(renderer.records.get(2)!.node.parent).toBe(renderer.records.get(1)!.node);
  renderer.window!.close();
});

test("input text stays text and keys retain GPUIX modifier names", () => {
  const received: Record<string, unknown>[] = [];
  const renderer = new GpuixRenderer((_error, event) => received.push(event));
  renderer.init();
  renderer.applyBatch(
    JSON.stringify([
      ["createElement", 1, "input"],
      ["setEventListener", 1, "change", true],
      ["setEventListener", 1, "keyDown", true],
      ["setRoot", 1],
    ]),
  );
  const id = renderer.records.get(1)!.node.id;
  renderer.window!._dispatchEvent("input", id, "123");
  renderer.window!._dispatchEvent(
    "keydown",
    id,
    JSON.stringify({ key: "ArrowDown", meta: true, control: false, shift: true, alt: false }),
  );
  expect(received[0]!.value).toBe("123");
  expect(received[1]!.key).toBe("down");
  expect(received[1]!.modifiers).toEqual({ cmd: true, ctrl: false, shift: true, alt: false });
  renderer.window!.close();
});

test("custom title bars drag through empty chrome while controls opt out", () => {
  const renderer = new GpuixRenderer();
  renderer.init({ titlebarTransparent: true, titlebarHeight: 51 });
  const band = renderer.window!.root.children.find(
    (node) => node.properties.get(PropertyCode.AppRegion) === "drag",
  );
  expect(band).toBeDefined();
  renderer.applyBatch(
    JSON.stringify([
      ["createElement", 1, "div"],
      ["createElement", 2, "div"],
      ["appendChild", 1, 2],
      ["setRoot", 1],
      ["setEventListener", 2, "click", true],
      ["setStyle", 2, { width: 24, height: 24 }],
    ]),
  );
  expect(renderer.records.get(2)!.node.properties.get(PropertyCode.AppRegion)).toBe("no-drag");
  renderer.applyBatch(JSON.stringify([["setEventListener", 2, "click", false]]));
  expect(renderer.records.get(2)!.node.properties.has(PropertyCode.AppRegion)).toBe(false);
  renderer.window!.close();
});

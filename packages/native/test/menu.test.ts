import { describe, expect, mock, test } from "bun:test";
import { callsNamed, fakeBinding, queueEvents } from "./fake-binding.ts";

mock.module("../src/binding.ts", () => fakeBinding);

const { Clipboard, Menu, Window, app } = await import("../src/index.ts");
const { nativeImageSource, serializeNativeMenu } = await import("../src/system.ts");
await app.whenReady();

function parsedMenu(name: "setApplicationMenu"): unknown {
  const last = callsNamed(name).at(-1);
  return JSON.parse(String(last?.args[1]));
}

describe("native menu declarations", () => {
  test("accelerators, hidden flags, and new roles reach the native JSON verbatim", () => {
    Menu.setApplicationMenu([
      {
        label: "File",
        items: [
          {
            label: "Save As…",
            accelerator: "CmdOrCtrl+Shift+S",
            click: () => {},
          },
          { label: "Debug", hidden: true },
          { type: "role", label: "Delete", role: "delete" },
          {
            type: "role",
            label: "Paste and Match Style",
            role: "paste-and-match-style",
            accelerator: "CmdOrCtrl+Alt+Shift+V",
          },
          { type: "system-menu", label: "Open Recent", menu: "recent-documents" },
        ],
      },
    ]);

    const menus = parsedMenu("setApplicationMenu") as {
      items: Record<string, unknown>[];
    }[];
    const items = menus[0]!.items;
    expect(items[0]!.accelerator).toBe("CmdOrCtrl+Shift+S");
    expect(items[0]!.hidden).toBeUndefined();
    expect(items[1]!.hidden).toBe(true);
    expect(items[2]).toMatchObject({ type: "role", role: "delete" });
    expect(items[3]).toMatchObject({
      role: "paste-and-match-style",
      accelerator: "CmdOrCtrl+Alt+Shift+V",
    });
    expect(items[4]).toMatchObject({
      type: "system-menu",
      menu: "recent-documents",
    });
  });

  test("serialization keeps one callback id per action item", () => {
    const clicked: string[] = [];
    const serialized = serializeNativeMenu([
      {
        label: "Edit",
        items: [
          { label: "One", click: () => clicked.push("one") },
          { type: "separator" },
          { label: "Two", accelerator: "Cmd+2", click: () => clicked.push("two") },
        ],
      },
    ]);
    expect(serialized.actionIds).toHaveLength(2);
    const parsed = JSON.parse(serialized.json) as {
      items: Record<string, unknown>[];
    }[];
    expect(parsed[0]!.items[2]!.accelerator).toBe("Cmd+2");
    expect(parsed[0]!.items[2]!.id).toBe(serialized.actionIds[1]);
  });

  test("image sources pass an explicit template flag to the host", () => {
    expect(nativeImageSource("/icons/statusTemplate.png")).toEqual({
      path: "/icons/statusTemplate.png",
    });
    expect(nativeImageSource({ path: "/icons/plain.png", template: false })).toEqual({
      path: "/icons/plain.png",
      template: false,
    });
    expect(
      nativeImageSource({ data: Uint8Array.from([0, 0, 0, 255]), width: 1, height: 1, template: true }),
    ).toMatchObject({ width: 1, height: 1, template: true });
  });

  test("menu icons carry an explicit template flag", () => {
    const serialized = serializeNativeMenu([
      {
        label: "App",
        items: [
          { label: "Status", icon: { path: "/icons/statusTemplate.png" }, click: () => {} },
          {
            label: "Plain",
            icon: { path: "/icons/plain.png", template: false },
            click: () => {},
          },
        ],
      },
    ]);
    const parsed = JSON.parse(serialized.json) as {
      items: { icon?: Record<string, unknown> }[];
    }[];
    expect(parsed[0]!.items[0]!.icon).toEqual({ path: "/icons/statusTemplate.png" });
    expect(parsed[0]!.items[1]!.icon).toEqual({
      path: "/icons/plain.png",
      template: false,
    });
  });
});

describe("window close interception", () => {
  test("interception is declared while listeners exist and withdrawn afterwards", () => {
    const window = new Window({ renderer: () => () => {}, title: "Interception" });
    const before = callsNamed("performWindowAction").length;

    const received: unknown[] = [];
    const dispose = window.onCloseRequested((event) => received.push(event));
    const declared = callsNamed("performWindowAction").slice(before);
    expect(declared).toHaveLength(1);
    expect(declared[0]!.args.slice(2)).toEqual(["set-close-interception", "true"]);

    // A held native close reaches the listener instead of closing the window.
    queueEvents({
      kind: "close-requested",
      window: window.nativeId,
      target: 0,
    });
    app.dispatchEvents();
    expect(received).toEqual([{ window }]);
    expect(window.closed).toBe(false);

    // A second listener does not re-declare the same flag.
    const second = window.onCloseRequested(() => {});
    expect(callsNamed("performWindowAction").slice(before)).toHaveLength(1);
    second();
    expect(callsNamed("performWindowAction").slice(before)).toHaveLength(1);

    dispose();
    const withdrawn = callsNamed("performWindowAction").slice(before);
    expect(withdrawn).toHaveLength(2);
    expect(withdrawn[1]!.args.slice(2)).toEqual(["set-close-interception", "false"]);
    window.destroy();
  });

  test("close and destroy both complete a held request", () => {
    const window = new Window({ renderer: () => () => {}, title: "Completion" });
    window.onCloseRequested(() => window.close());
    const closes = callsNamed("closeWindow").length;

    queueEvents({
      kind: "close-requested",
      window: window.nativeId,
      target: 0,
    });
    app.dispatchEvents();
    expect(callsNamed("closeWindow")).toHaveLength(closes + 1);

    const forced = new Window({ renderer: () => () => {}, title: "Forced" });
    const before = callsNamed("performWindowAction").length;
    forced.onCloseRequested(() => {
      throw new Error("destroy must bypass every close listener");
    });
    forced.destroy();
    const declared = callsNamed("performWindowAction").slice(before);
    expect(declared.at(-1)!.args.slice(2)).toEqual(["set-close-interception", "false"]);
  });

  test('on("closed") mirrors onClose and fires once', () => {
    const window = new Window({ renderer: () => () => {}, title: "Closed" });
    const seen: unknown[] = [];
    window.on("closed", (payload) => seen.push(payload));
    queueEvents({ kind: "close", window: window.nativeId, target: 0 });
    app.dispatchEvents();
    expect(seen).toEqual([{ window }]);
    expect(window.closed).toBe(true);
  });
});

describe("application quit interception", () => {
  test("a beforeQuit listener declares interception for its lifetime", async () => {
    const before = callsNamed("setQuitInterception").length;
    const reasons: string[] = [];
    const dispose = app.on("beforeQuit", (event) => reasons.push(event.reason));
    expect(callsNamed("setQuitInterception").slice(before).at(-1)!.args[1]).toBe(true);

    // A second listener keeps the single declaration.
    const second = app.on("beforeQuit", () => {});
    expect(callsNamed("setQuitInterception").slice(before)).toHaveLength(1);
    second();

    queueEvents({
      kind: "before-quit",
      window: 0,
      target: 0,
      value: "operating-system",
    });
    queueEvents({
      kind: "will-quit",
      window: 0,
      target: 0,
      value: "explicit",
    });
    const willQuit: string[] = [];
    const disposeWill = app.on("willQuit", (event) => willQuit.push(event.reason));
    app.dispatchEvents();
    expect(reasons).toEqual(["operating-system"]);
    expect(willQuit).toEqual(["explicit"]);

    // Completing the held quit bypasses interception.
    const exits = callsNamed("exitApp").length;
    await app.quit({ force: true });
    expect(callsNamed("exitApp")).toHaveLength(exits + 1);

    // The ordinary request still runs the preventable phases.
    const requests = callsNamed("requestAppQuit").length;
    await app.quit();
    expect(callsNamed("requestAppQuit")).toHaveLength(requests + 1);

    disposeWill();
    dispose();
    expect(callsNamed("setQuitInterception").at(-1)!.args[1]).toBe(false);
  });

  test("exit rejects out-of-range codes and force-quits otherwise", async () => {
    await expect(app.exit(-1)).rejects.toBeInstanceOf(RangeError);
    await expect(app.exit(1.5)).rejects.toBeInstanceOf(RangeError);
    const exits = callsNamed("exitAppWithCode").length;
    expect(await app.exit(3)).toBe(true);
    const recorded = callsNamed("exitAppWithCode");
    expect(recorded).toHaveLength(exits + 1);
    // The core carries the status through its own teardown instead of the host inventing one.
    expect(recorded.at(-1)?.args[1]).toBe(3);
  });
});

describe("clipboard format helpers", () => {
  test("formats, presence, and buffers map onto the core's MIME entries", async () => {
    await Clipboard.writeBuffer("application/x-quickgui", new Uint8Array([1, 2, 3]));
    expect(await Clipboard.availableFormats()).toEqual(["application/x-quickgui"]);
    expect(await Clipboard.has("application/x-quickgui")).toBe(true);
    expect(await Clipboard.has("text/plain")).toBe(false);
    expect(await Clipboard.readBuffer("application/x-quickgui")).toEqual(new Uint8Array([1, 2, 3]));
    expect(await Clipboard.readBuffer("text/plain")).toBeUndefined();

    await Clipboard.writeBuffer("text/plain", new TextEncoder().encode("hello"));
    expect(await Clipboard.readText()).toBe("hello");
    expect(await Clipboard.availableFormats()).toEqual(["text/plain"]);
  });

  test("the Find pasteboard round-trips one bounded text entry", async () => {
    expect(await Clipboard.readFindText()).toBe("");
    await Clipboard.writeFindText("needle");
    expect(callsNamed("writeFindClipboard").at(-1)!.args[0]).toEqual({
      entries: [{ kind: "text", text: "needle" }],
    });
    expect(await Clipboard.readFindText()).toBe("needle");

    await Clipboard.writeFindText("");
    expect(callsNamed("writeFindClipboard").at(-1)!.args[0]).toEqual({
      entries: [],
    });
    expect(await Clipboard.readFindText()).toBe("");
  });
});

describe("window tab and character palette commands", () => {
  test("each command is one fire-and-forget native window action", () => {
    const window = new Window({ renderer: () => () => {}, title: "Tabs" });
    const before = callsNamed("performWindowAction").length;
    window.setTabbingIdentifier("documents");
    window.selectNextTab();
    window.selectPreviousTab();
    window.selectTab(2);
    window.mergeAllWindows();
    window.moveTabToNewWindow();
    window.toggleTabBar();
    window.toggleTabOverview();
    window.showCharacterPalette();
    window.setTabbingIdentifier();

    expect(
      callsNamed("performWindowAction")
        .slice(before)
        .map((call) => call.args.slice(2)),
    ).toEqual([
      ["set-tabbing-identifier", "documents"],
      ["select-next-tab", undefined],
      ["select-previous-tab", undefined],
      ["select-tab", "2"],
      ["merge-all-windows", undefined],
      ["move-tab-to-new-window", undefined],
      ["toggle-tab-bar", undefined],
      ["toggle-tab-overview", undefined],
      ["show-character-palette", undefined],
      ["set-tabbing-identifier", ""],
    ]);
    expect(() => window.selectTab(-1)).toThrow(RangeError);
    window.destroy();
  });
});

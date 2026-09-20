import { afterAll, describe, expect, mock, test } from "bun:test";
import {
  callsNamed,
  fakeBinding,
  lastCall,
  queueEvents,
  setFrameMetricsReply,
} from "./fake-binding.ts";

/** Window actions carry their payload in the fourth binding argument. */
function windowActions(action: string): unknown[] {
  return callsNamed("performWindowAction")
    .filter((call) => call.args[2] === action)
    .map((call) => call.args[3]);
}

mock.module("../src/binding.ts", () => fakeBinding);

const { Menu, Metrics, Shell, SpellChecker, Window, app } = await import("../src/index.ts");
await app.whenReady();

const window = new Window({ renderer: () => () => {} });
const other = new Window({ renderer: () => () => {} });

// The host app is a module singleton shared with every other test file, so these windows are
// released instead of lingering in `app.windows`.
afterAll(() => {
  window.destroy();
  other.destroy();
});

/** Deliver one native event batch through the ordinary host dispatch path. */
function deliver(...events: Record<string, unknown>[]): void {
  queueEvents(...events);
  app.dispatchEvents();
}

function windowEvent(kind: string, value?: string): Record<string, unknown> {
  return { kind, window: window.nativeId, target: 0, value };
}

describe("window lifecycle events", () => {
  test("core notifications become the documented paired JavaScript events", () => {
    const seen: string[] = [];
    const disposers = [
      window.on("minimize", () => seen.push("minimize")),
      window.on("restore", () => seen.push("restore")),
      window.on("maximize", () => seen.push("maximize")),
      window.on("unmaximize", () => seen.push("unmaximize")),
      window.on("enterFullScreen", () => seen.push("enterFullScreen")),
      window.on("leaveFullScreen", () => seen.push("leaveFullScreen")),
      window.on("readyToShow", () => seen.push("readyToShow")),
      window.on("focus", () => seen.push("focus")),
      window.on("blur", () => seen.push("blur")),
    ];

    deliver(
      windowEvent("window-minimize", "true"),
      windowEvent("window-minimize", "false"),
      windowEvent("window-maximize", "true"),
      windowEvent("window-maximize", "false"),
      windowEvent("window-fullscreen", "true"),
      windowEvent("window-fullscreen", "false"),
      windowEvent("window-ready-to-show"),
      windowEvent("window-focus", "true"),
      windowEvent("window-focus", "false"),
    );

    expect(seen).toEqual([
      "minimize",
      "restore",
      "maximize",
      "unmaximize",
      "enterFullScreen",
      "leaveFullScreen",
      "readyToShow",
      "focus",
      "blur",
    ]);
    for (const dispose of disposers) dispose();
  });

  test("occlusion, level, appearance, geometry, and will-events carry their payloads", () => {
    const payloads: unknown[] = [];
    const disposers = [
      window.on("occlusionChange", (event) => payloads.push(event.occluded)),
      window.on("levelChange", (event) => payloads.push(event.level)),
      window.on("appearanceChange", (event) => payloads.push(event.appearance)),
      window.on("resize", (event) => payloads.push(event.size)),
      window.on("move", (event) => payloads.push(event.position)),
      window.on("willResize", (event) => payloads.push(event.size)),
      window.on("willMove", (event) => payloads.push(event.position)),
    ];

    deliver(
      windowEvent("window-occlusion", "true"),
      windowEvent("window-level", "screen-saver"),
      windowEvent("window-appearance", "dark"),
      windowEvent("window-resize", JSON.stringify({ width: 800, height: 600 })),
      windowEvent("window-move", JSON.stringify({ x: 12, y: 34 })),
      windowEvent("window-will-resize", JSON.stringify({ width: 640, height: 480 })),
      windowEvent("window-will-move", JSON.stringify({ x: -5, y: 7 })),
    );

    expect(payloads).toEqual([
      true,
      "screen-saver",
      "dark",
      { width: 800, height: 600 },
      { x: 12, y: 34 },
      { width: 640, height: 480 },
      { x: -5, y: 7 },
    ]);
    for (const dispose of disposers) dispose();
  });

  test("malformed geometry payloads are dropped instead of reaching listeners", () => {
    let delivered = 0;
    const dispose = window.on("resize", () => {
      delivered += 1;
    });
    deliver(
      windowEvent("window-resize", "not json"),
      windowEvent("window-resize", JSON.stringify({ width: 10 })),
      windowEvent("window-resize"),
    );
    expect(delivered).toBe(0);
    dispose();
  });

  test("a disposed listener stops receiving further notifications", () => {
    let delivered = 0;
    const dispose = window.on("minimize", () => {
      delivered += 1;
    });
    deliver(windowEvent("window-minimize", "true"));
    dispose();
    deliver(windowEvent("window-minimize", "true"));
    expect(delivered).toBe(1);
  });

  test("the application observes activation and deactivation", () => {
    const seen: string[] = [];
    const disposers = [
      app.on("activate", () => seen.push("activate")),
      app.on("deactivate", () => seen.push("deactivate")),
    ];
    deliver(
      { kind: "app-activate", window: 0, target: 0 },
      { kind: "app-deactivate", window: 0, target: 0 },
    );
    expect(seen).toEqual(["activate", "deactivate"]);
    for (const dispose of disposers) dispose();
  });
});

describe("declared-ahead resize and move policies", () => {
  test("policies travel as one bounded declaration and withdraw with null", () => {
    window.setResizePolicy({
      aspectRatio: 16 / 9,
      minimum: { width: 320, height: 180 },
      snap: { width: 8, height: 8 },
    });
    expect(JSON.parse(String(windowActions("set-resize-policy").at(-1)))).toEqual({
      aspectRatio: 16 / 9,
      minimum: { width: 320, height: 180 },
      snap: { width: 8, height: 8 },
    });

    window.setResizePolicy(null);
    expect(windowActions("set-resize-policy").at(-1)).toBeUndefined();

    window.setMovePolicy({ keepOnScreen: true });
    expect(JSON.parse(String(windowActions("set-move-policy").at(-1)))).toEqual({
      keepOnScreen: true,
    });
    window.setMovePolicy(null);
    expect(windowActions("set-move-policy").at(-1)).toBeUndefined();
  });
});

describe("window stacking, input, and state commands", () => {
  test("frame metrics are read on demand for the requested window", async () => {
    setFrameMetricsReply({
      frameNumber: 0,
      cpuMilliseconds: 0,
      smoothedCpuMilliseconds: 0,
      frameMilliseconds: 0,
      smoothedFrameMilliseconds: 0,
    });
    expect(await Metrics.getFrameMetrics(window)).toBeNull();

    setFrameMetricsReply({
      frameNumber: 42,
      cpuMilliseconds: 1.25,
      smoothedCpuMilliseconds: 1.5,
      frameMilliseconds: 8,
      smoothedFrameMilliseconds: 10,
    });
    expect(await Metrics.getFrameMetrics(window)).toEqual({
      frameNumber: 42,
      cpuMilliseconds: 1.25,
      smoothedCpuMilliseconds: 1.5,
      frameMilliseconds: 8,
      smoothedFrameMilliseconds: 10,
    });
    expect(lastCall("getFrameMetrics").args).toEqual([1, window.nativeId]);
  });

  test("always-on-top carries the Electron level name it was given", () => {
    window.setAlwaysOnTop(true, "screenSaver");
    expect(JSON.parse(String(windowActions("set-always-on-top").at(-1)))).toEqual({
      flag: true,
      level: "screenSaver",
    });
    window.setAlwaysOnTop(false);
    expect(JSON.parse(String(windowActions("set-always-on-top").at(-1)))).toEqual({
      flag: false,
    });
  });

  test("ordering, pass-through, enablement, aspect ratio, buttons, and shadow", () => {
    window.moveTop();
    expect(windowActions("move-top")).toHaveLength(1);

    window.moveAbove(other);
    expect(windowActions("move-above").at(-1)).toBe(String(other.nativeId));
    expect(() => window.moveAbove(window)).toThrow(RangeError);

    window.setIgnoreMouseEvents(true, { forward: true });
    expect(JSON.parse(String(windowActions("set-ignore-mouse-events").at(-1)))).toEqual({
      ignore: true,
      forward: true,
    });
    window.setIgnoreMouseEvents(false);
    expect(JSON.parse(String(windowActions("set-ignore-mouse-events").at(-1)))).toEqual({
      ignore: false,
      forward: false,
    });

    window.setEnabled(false);
    expect(windowActions("set-enabled").at(-1)).toBe("false");

    window.setAspectRatio({ width: 16, height: 9 });
    expect(JSON.parse(String(windowActions("set-aspect-ratio").at(-1)))).toEqual({
      width: 16,
      height: 9,
    });
    window.setAspectRatio(null);
    expect(windowActions("set-aspect-ratio").at(-1)).toBeUndefined();

    window.setWindowButtonVisibility(false);
    expect(windowActions("set-window-button-visibility").at(-1)).toBe("false");

    window.setHasShadow(true);
    expect(windowActions("set-shadow").at(-1)).toBe("true");
  });

  test("restore state round-trips from the native snapshot into window options", async () => {
    const state = await window.getRestoreState();
    expect(state).toEqual({
      x: 32,
      y: 64,
      width: 900,
      height: 600,
      maximized: false,
      fullscreen: true,
      displayId: "3",
      displayUuid: "00112233-4455-6677-8899-aabbccddeeff",
      scaleFactor: 2,
    });
    expect(lastCall("getWindowRestoreState").args[1]).toBe(window.nativeId);

    const restored = new Window({ renderer: () => () => {}, restoreState: state });
    const options = lastCall("createWindow").args[1] as {
      restoreState?: Record<string, unknown>;
    };
    expect(options.restoreState).toEqual({
      x: 32,
      y: 64,
      width: 900,
      height: 600,
      maximized: false,
      fullscreen: true,
      displayId: "3",
      displayUuid: "00112233-4455-6677-8899-aabbccddeeff",
      scaleFactor: 2,
    });
    restored.destroy();
  });
});

describe("application shell", () => {
  test("fire-and-forget mutations never wait on the native main thread", () => {
    app.focus({ steal: true });
    expect(lastCall("performAppMutation").args.slice(1)).toEqual(["activate", "true"]);
    app.focus();
    expect(lastCall("performAppMutation").args.slice(1)).toEqual(["activate", "false"]);
    app.hide();
    expect(lastCall("performAppMutation").args.slice(1)).toEqual(["hide", undefined]);
    app.show();
    expect(lastCall("performAppMutation").args.slice(1)).toEqual(["unhide", undefined]);
    app.setSecureKeyboardEntryEnabled(true);
    expect(lastCall("performAppMutation").args.slice(1)).toEqual([
      "set-secure-keyboard-entry",
      "true",
    ]);
    Shell.beep();
    expect(lastCall("performAppMutation").args.slice(1)).toEqual(["beep", undefined]);
  });

  test("application-level dictionary calls are bounded before they reach the core", () => {
    SpellChecker.learnWord("quickgui");
    expect(lastCall("performAppMutation").args.slice(1)).toEqual(["learn-word", "quickgui"]);
    SpellChecker.ignoreWord("quickgui");
    expect(lastCall("performAppMutation").args.slice(1)).toEqual(["ignore-word", "quickgui"]);
    expect(() => SpellChecker.learnWord("")).toThrow(TypeError);
    expect(() => SpellChecker.ignoreWord("a".repeat(257))).toThrow(TypeError);
  });

  test("activation policy resolves only when the operating system answers", async () => {
    const pending = app.setActivationPolicy("accessory");
    const call = lastCall("performAppService");
    expect(call.args.slice(2)).toEqual(["set-activation-policy", "accessory"]);
    const request = call.args[1] as number;
    deliver({ kind: "app-service", window: 0, target: request });
    await pending;
  });

  test("a rejected application service surfaces the native error", async () => {
    const pending = app.setActivationPolicy("prohibited");
    const request = lastCall("performAppService").args[1] as number;
    deliver({
      kind: "app-service",
      window: 0,
      target: request,
      error: "this platform service is not implemented",
    });
    await expect(pending).rejects.toThrow("this platform service is not implemented");
  });

  test("dock bounces resolve with the identifier that cancels them", async () => {
    const pending = app.dock.bounce("critical");
    const call = lastCall("performAppService");
    expect(call.args.slice(2)).toEqual(["request-dock-attention", "critical"]);
    deliver({
      kind: "app-service",
      window: 0,
      target: call.args[1] as number,
      value: "17",
    });
    expect(await pending).toBe(17);

    app.dock.cancelBounce(17);
    expect(lastCall("performAppMutation").args.slice(1)).toEqual(["cancel-dock-attention", "17"]);
    expect(() => app.dock.cancelBounce(1.5)).toThrow(TypeError);
  });

  test("dock visibility tracks the last request this process made", async () => {
    const hidden = app.dock.hide();
    const call = lastCall("performAppService");
    expect(call.args.slice(2)).toEqual(["set-dock-visible", "false"]);
    deliver({ kind: "app-service", window: 0, target: call.args[1] as number });
    await hidden;
    expect(app.dock.isVisible()).toBe(false);

    const shown = app.dock.show();
    deliver({
      kind: "app-service",
      window: 0,
      target: lastCall("performAppService").args[1] as number,
    });
    await shown;
    expect(app.dock.isVisible()).toBe(true);
  });

  test("packaging and the applications folder read through the core", async () => {
    expect(app.isPackaged).toBe(true);
    expect(await app.isInApplicationsFolder()).toBe(false);

    const pending = app.moveToApplicationsFolder();
    deliver({
      kind: "app-service",
      window: 0,
      target: lastCall("performAppService").args[1] as number,
      value: "true",
    });
    expect(await pending).toBe(true);
  });
});

describe("native popup and per-window menus", () => {
  test("a popup resolves after it closes and its click callbacks run", async () => {
    let clicked = 0;
    const pending = Menu.popup(
      [{ label: "Copy", click: () => (clicked += 1) }, { type: "separator" }, { label: "Paste" }],
      { window, x: 24, y: 48 },
    );
    const call = lastCall("showWindowPopupMenu");
    expect(call.args[2]).toBe(window.nativeId);
    expect(call.args.slice(4)).toEqual([24, 48]);
    const menu = JSON.parse(String(call.args[3])) as {
      items: { type: string; id?: number }[];
    }[];
    expect(menu).toHaveLength(1);
    const action = menu[0]?.items[0];
    expect(action?.type).toBe("action");

    deliver({
      kind: "menu-action",
      window: window.nativeId,
      target: action?.id ?? 0,
    });
    expect(clicked).toBe(1);

    deliver({
      kind: "popup-menu",
      window: 0,
      target: call.args[1] as number,
    });
    await pending;

    // The callbacks are released with the popup, so a stale action id does nothing.
    deliver({
      kind: "menu-action",
      window: window.nativeId,
      target: action?.id ?? 0,
    });
    expect(clicked).toBe(1);
  });

  test("a popup position requires both coordinates and finite values", async () => {
    await expect(Menu.popup([{ label: "One" }], { window, x: 4 })).rejects.toThrow(TypeError);
    await expect(Menu.popup([{ label: "One" }], { window, x: Number.NaN, y: 0 })).rejects.toThrow(
      TypeError,
    );
  });

  test("a dismissed popup still resolves and releases its callbacks", async () => {
    const pending = Menu.popup([{ label: "Only" }], { window });
    const call = lastCall("showWindowPopupMenu");
    expect(call.args.slice(4)).toEqual([undefined, undefined]);
    deliver({ kind: "popup-menu", window: 0, target: call.args[1] as number });
    await pending;
  });

  test("per-window menus replace and then inherit the application menu", () => {
    window.setMenu([{ label: "File", items: [{ label: "Open" }] }]);
    const declared = JSON.parse(String(windowActions("set-menu").at(-1))) as {
      label: string;
    }[];
    expect(declared[0]?.label).toBe("File");

    window.setMenu(null);
    expect(windowActions("set-menu").at(-1)).toBeUndefined();
  });
});

import { expect, test } from "bun:test";
import * as native from "@quickgui/native";
import { existsSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import Updater, {
  acknowledgeStartup,
  Updater as NamedUpdater,
  type UpdateEvent,
} from "@quickgui/extension-updater";
import { callsNamed, emitExtensionEvent, lastCall } from "../../../../packages/native/test/fake-binding.ts";

await native.app.whenReady();

test("the native runtime is a peer rather than an installed updater dependency", async () => {
  const manifest = await Bun.file(new URL("../../package.json", import.meta.url)).json();
  expect(manifest.dependencies?.["@quickgui/native"]).toBeUndefined();
  expect(manifest.peerDependencies?.["@quickgui/native"]).toBe("workspace:*");
  expect(manifest.devDependencies?.["@quickgui/native"]).toBe("workspace:*");
});

test("the optional updater package exposes its API independently of the native core", () => {
  expect(Updater).toBe(NamedUpdater);
  expect("Updater" in native).toBe(false);
});

test("updaters inherit CLI metadata, observe native state, and route commands through one session", async () => {
  const runtime = globalThis as typeof globalThis & { __QUICKGUI_UPDATER_OPTIONS__?: object };
  const previous = runtime.__QUICKGUI_UPDATER_OPTIONS__;
  const defaults = {
    feedUrl: "https://downloads.example.com/my-app/appcast-darwin-arm64.xml",
    publicKey: "GX9rI+FsU4YtjNSAyOdwmHw28GPA59ORGo2SvUhRXvY=",
    currentVersion: "1.0.0",
    identifier: "com.example.my-app",
    automaticChecks: true,
    development: true,
  };
  runtime.__QUICKGUI_UPDATER_OPTIONS__ = defaults;
  const events: UpdateEvent[] = [];
  const updater = new Updater({ automaticChecks: false }, (event) => events.push(event));
  try {
    await updater.ready;
    const started = lastCall("startExtension");
    expect(started.args.slice(0, 2)).toEqual(["updater", { ...defaults, automaticChecks: false }]);
    const session = started.args[2] as number;
    const disabled: UpdateEvent = {
      kind: "state",
      status: "disabled",
      automaticChecks: false,
      quitRequired: false,
    };
    emitExtensionEvent(session, disabled);
    expect(events).toEqual([disabled]);
    const before = callsNamed("invoke").length;
    expect(updater.state).toEqual(disabled);
    expect(updater.state.status).toBe("disabled");
    expect(callsNamed("invoke")).toHaveLength(before);

    await updater.check();
    await updater.install();
    await updater.setAutomaticChecks(true);
    expect(
      callsNamed("invoke")
        .slice(before)
        .map((call) => call.args),
    ).toEqual([
      ["extension/updater/check", { session, value: true }],
      ["extension/updater/install", { session, value: null }],
      ["extension/updater/automatic", { session, value: true }],
    ]);
    const available: UpdateEvent = { ...disabled, status: "available", version: "1.1.0" };
    emitExtensionEvent(session, available);
    expect(updater.state).toEqual(available);
    const closes = callsNamed("stopExtension").length;
    await updater.close();
    await updater.close();
    expect(
      callsNamed("stopExtension")
        .slice(closes)
        .map((call) => call.args),
    ).toEqual([["updater", session]]);
    emitExtensionEvent(session, { ...available, status: "installing" });
    expect(updater.state).toEqual(available);
    await expect(updater.check()).rejects.toThrow("closed");
  } finally {
    await updater.close();
    if (previous === undefined) delete runtime.__QUICKGUI_UPDATER_OPTIONS__;
    else runtime.__QUICKGUI_UPDATER_OPTIONS__ = previous;
  }
});

test("an updated app acknowledges startup only through the helper's private file", () => {
  const stage = mkdtempSync(join(tmpdir(), "quickgui-updater-ready-"));
  try {
    const ready = join(stage, "application-ready");
    process.env.QUICKGUI_UPDATE_READY_FILE = ready;
    acknowledgeStartup();
    expect(readFileSync(ready, "utf8")).toBe("ready\n");
    expect(process.env.QUICKGUI_UPDATE_READY_FILE).toBeUndefined();
    for (const path of [join(stage, "other-name"), "application-ready"]) {
      process.env.QUICKGUI_UPDATE_READY_FILE = path;
      acknowledgeStartup();
      expect(existsSync(path)).toBe(false);
      expect(process.env.QUICKGUI_UPDATE_READY_FILE).toBeUndefined();
    }
  } finally {
    delete process.env.QUICKGUI_UPDATE_READY_FILE;
    rmSync(stage, { recursive: true, force: true });
  }
});

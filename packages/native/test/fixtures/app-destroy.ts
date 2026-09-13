import { mock } from "bun:test";
import assert from "node:assert/strict";
import { fakeBinding, lastCall } from "../fake-binding.ts";

mock.module("../../src/binding.ts", () => fakeBinding);
const { app, Menu, Window } = await import("../../src/index.ts");
const { dispatchSystemEvent } = await import("../../src/system.ts");
await app.whenReady();
const window = new Window({ renderer: () => () => {} });
let clicked = 0;
const requests = [
  app.dock.bounce(),
  app.dock.hide(),
  app.setActivationPolicy("accessory"),
  app.moveToApplicationsFolder(),
  Menu.popup([{ label: "Action", click: () => clicked++ }], { window }),
];
const service = lastCall("performAppService").args[1] as number;
const popup = lastCall("showWindowPopupMenu");
const menu = JSON.parse(String(popup.args[3])) as { items: { id: number }[] }[];
const outcomes = Promise.allSettled(requests);
const deadline = setTimeout(() => {
  console.error("application-service promises remained pending after app.destroy()");
  process.exit(1);
}, 1000);
try {
  app.destroy();
  app.destroy();
  for (const outcome of await outcomes) {
    assert.equal(outcome.status, "rejected");
    if (outcome.status === "rejected") {
      assert.match(String(outcome.reason), /the QuickGUI app was destroyed/);
    }
  }
  for (const event of [
    { kind: "app-service", window: 0, target: service },
    { kind: "popup-menu", window: 0, target: popup.args[1] as number },
    { kind: "menu-action", window: window.nativeId, target: menu[0]!.items[0]!.id },
  ]) {
    dispatchSystemEvent(event, () => undefined);
  }
  assert.equal(clicked, 0);
} finally {
  clearTimeout(deadline);
}

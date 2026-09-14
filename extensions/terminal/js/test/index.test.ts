import { expect, test } from "bun:test";
import { createSignal, flush } from "solid-js";
import { NativeNodeTag, PropertyCode, QuickGuiEvent } from "@quickgui/native";
import { createComponent } from "@quickgui/solid";
import { Terminal, terminalStatusFromEvent } from "@quickgui/extension-terminal";

test("terminal configuration and status use the generic component boundary", () => {
  const [directory, setDirectory] = createSignal("/tmp");
  let status = "";
  const terminal = createComponent(Terminal, {
    program: "/bin/zsh", arguments: ["-l"],
    get workingDirectory() { return directory(); },
    environment: { TERM_TEST: "1" }, scrollback: 20_000,
    style: { height: 300 },
    onStatus(event) { status = terminalStatusFromEvent(event).status; },
  });
  expect(terminal.tag).toBe(NativeNodeTag.Extension);
  expect(terminal.properties.get(PropertyCode.ExtensionPackage)).toBe("terminal");
  const props = () => JSON.parse(terminal.properties.get(PropertyCode.ExtensionProps) as string);
  expect(props()).toMatchObject({ program: "/bin/zsh", arguments: ["-l"], workingDirectory: "/tmp", scrollback: 20_000 });
  setDirectory("/var/tmp"); flush();
  expect(props().workingDirectory).toBe("/var/tmp");
  terminal.listeners.get("componentchange")!(new QuickGuiEvent("componentchange", terminal, JSON.stringify({ kind: "status", value: { status: "running", title: "zsh", workingDirectory: "/var/tmp" } })));
  expect(status).toBe("running");
});

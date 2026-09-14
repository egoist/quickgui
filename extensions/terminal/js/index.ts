import { QuickGuiEvent, type NativeNode, type ColorValue } from "@quickgui/native";
import { ExtensionComponent, type NativeProps } from "@quickgui/solid";

export type TerminalPalette = readonly [ColorValue, ColorValue, ColorValue, ColorValue, ColorValue, ColorValue, ColorValue, ColorValue, ColorValue, ColorValue, ColorValue, ColorValue, ColorValue, ColorValue, ColorValue, ColorValue];
export interface TerminalProps extends Omit<NativeProps, "children"> {
  program?: string;
  arguments?: readonly string[];
  workingDirectory?: string;
  environment?: Readonly<Record<string, string>>;
  scrollback?: number;
  palette?: TerminalPalette;
  cursorColor?: ColorValue;
  paddingColor?: "background" | "extend";
  fontThicken?: boolean;
  onStatus?: (event: QuickGuiEvent) => void;
}
export type TerminalStatusKind = "starting" | "running" | "exited" | "failed";
export interface TerminalStatusEvent {
  status: TerminalStatusKind;
  title: string;
  workingDirectory: string | null;
  processId?: number;
  exitCode?: number | null;
  signal?: string | null;
  message?: string;
  agent?: string;
  agentStatus?: "idle" | "working" | "blocked";
  agentProcessId?: number;
}
export function terminalStatusFromEvent(event: QuickGuiEvent): TerminalStatusEvent {
  if (!event.value) throw new TypeError("Terminal status event has no payload");
  return JSON.parse(event.value) as TerminalStatusEvent;
}

export function Terminal(props: TerminalProps): NativeNode {
  return ExtensionComponent({
    package: "terminal", component: "terminal",
    get properties() {
      return { program: props.program, arguments: props.arguments, workingDirectory: props.workingDirectory,
        environment: props.environment, scrollback: props.scrollback, palette: props.palette,
        cursorColor: props.cursorColor, paddingColor: props.paddingColor, fontThicken: props.fontThicken,
        style: props.style };
    },
    get style() { return props.style; },
    onEvent(kind, value, event) {
      if (kind === "status") props.onStatus?.(new QuickGuiEvent("componentchange", event.target, JSON.stringify(value)));
    },
  });
}

export default Terminal;

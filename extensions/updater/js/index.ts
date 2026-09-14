import { ExtensionSession } from "@quickgui/native";

export interface UpdaterOptions {
  feedUrl?: string;
  publicKey?: string;
  currentVersion?: string;
  identifier?: string;
  automaticChecks?: boolean;
  allowDevelopment?: boolean;
}
export interface UpdateEvent {
  kind: string;
  status: "idle" | "checking" | "available" | "downloading" | "installing" | "disabled";
  version?: string;
  notes?: string;
  downloadedBytes?: number;
  totalBytes?: number;
  automaticChecks: boolean;
  error?: string;
  quitRequired: boolean;
}
/** Optional Rust updater extension: Sparkle on macOS and verified native handoff elsewhere. */
export class Updater {
  readonly #session: ExtensionSession<UpdateEvent>;
  #state: UpdateEvent = {
    kind: "state",
    status: "idle",
    automaticChecks: false,
    quitRequired: false,
  };
  readonly ready: Promise<void>;
  constructor(options: UpdaterOptions = {}, changed?: (event: UpdateEvent) => void) {
    const defaults = (globalThis as typeof globalThis & { __QUICKGUI_UPDATER_OPTIONS__?: object })
      .__QUICKGUI_UPDATER_OPTIONS__;
    this.#session = new ExtensionSession("updater", { ...defaults, ...options }, (event) => {
      this.#state = event;
      changed?.(event);
    });
    this.ready = this.#session.ready;
  }
  get state(): UpdateEvent {
    return this.#state;
  }
  async check(): Promise<void> {
    await this.#session.request("check", true);
  }
  async install(): Promise<void> {
    await this.#session.request("install");
  }
  async setAutomaticChecks(enabled: boolean): Promise<void> {
    await this.#session.request("automatic", enabled);
  }
  close(): Promise<void> {
    return this.#session.close();
  }
}

export default Updater;

import { writeFileSync } from "node:fs";
import { basename, isAbsolute } from "node:path";
import { app, ExtensionSession } from "@quickgui/native";

/**
 * Tell the install helper that the updated application started. Without it the helper restores
 * the previous version. This runs when the app becomes ready, whether or not an `Updater` is
 * created, and children never inherit the helper's private path.
 */
export function acknowledgeStartup(): void {
  const path = process.env.QUICKGUI_UPDATE_READY_FILE;
  if (!path) return;
  delete process.env.QUICKGUI_UPDATE_READY_FILE;
  if (!isAbsolute(path) || basename(path) !== "application-ready" || path.length >= 4096) return;
  try {
    writeFileSync(path, "ready\n", { flag: "wx", mode: 0o600 });
  } catch {
    // The helper treats a missing acknowledgement as a failed start and rolls back.
  }
}
void app.whenReady().then(acknowledgeStartup, () => {});

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

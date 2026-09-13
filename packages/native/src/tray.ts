import { Buffer } from "node:buffer";
import { resolve as resolvePath } from "node:path";
import * as binding from "./binding.ts";

export type TrayIconSource =
  | string
  | {
      path: string;
      /** macOS template-image flag. When omitted, `*Template.png` paths are inferred. */
      template?: boolean;
    }
  | {
      /** Encoded image bytes, or RGBA8 when width and height are supplied. */
      data: Uint8Array;
      width?: number;
      height?: number;
      /** macOS template-image flag. Omitted values stay inferred or false. */
      template?: boolean;
    };

export interface TrayMenuActionItem {
  type?: "action";
  label: string;
  enabled?: boolean;
  checked?: boolean;
  click?: () => void;
}

export interface TrayMenuSeparatorItem {
  type: "separator";
}

export interface TrayMenuSubmenuItem {
  type: "submenu";
  label: string;
  enabled?: boolean;
  items: readonly TrayMenuItem[];
}

export type TrayMenuItem = TrayMenuActionItem | TrayMenuSeparatorItem | TrayMenuSubmenuItem;

export interface TrayIconOptions {
  icon: TrayIconSource;
  tooltip?: string | undefined;
  /** Status-bar title on macOS and compatible Linux hosts; unsupported on Windows. */
  title?: string | undefined;
  /** Treat the image as a monochrome template on macOS. */
  iconIsTemplate?: boolean;
  /** Show the menu for a primary click on macOS and Windows. */
  menuOnLeftClick?: boolean;
  visible?: boolean;
  menu?: readonly TrayMenuItem[];
}

export type TrayEventType =
  | "click"
  | "double-click"
  | "enter"
  | "move"
  | "leave"
  | "menu-item"
  | "scroll";

export interface TrayEvent {
  kind: TrayEventType;
  button?: "left" | "right" | "middle";
  position?: { x: number; y: number };
  pressed?: boolean;
  scrollDelta?: number;
  horizontal?: boolean;
}

type AppContext = {
  appId: number;
  hosted: boolean;
};

let resolveContext: (() => AppContext) | undefined;
const pendingRequests = new Map<number, { resolve: () => void; reject: (error: Error) => void }>();
const icons = new Map<number, TrayIcon>();
let nextRequest = 1;
let nextIcon = 1;
let nextMenuAction = 1;

export function configureTrayContext(context: () => AppContext): void {
  resolveContext = context;
}

function context(): AppContext {
  if (!resolveContext) throw new Error("QuickGUI's native tray context is not configured");
  return resolveContext();
}

function allocateBoundedId(next: number, used: ReadonlyMap<number, unknown>): number {
  let candidate = next;
  for (let attempt = 0; attempt <= used.size; attempt += 1) {
    if (!used.has(candidate)) return candidate;
    candidate = candidate >= 0xffff_ffff ? 1 : candidate + 1;
  }
  throw new Error("the native tray id space is exhausted");
}

function operation(
  action: "set" | "remove" | "show-menu",
  id: number,
  options?: binding.NativeTrayIconOptions,
): Promise<void> {
  try {
    const current = context();
    const request = allocateBoundedId(nextRequest, pendingRequests);
    nextRequest = request >= 0xffff_ffff ? 1 : request + 1;
    return new Promise<void>((resolve, reject) => {
      pendingRequests.set(request, { resolve, reject });
      try {
        let acceptance: Promise<void> | undefined;
        if (action === "set") {
          if (!options) throw new Error("setting a tray icon requires options");
          {
            acceptance = binding.setHostedTrayIcon(current.appId, request, options);
          }
        } else if (action === "remove") {
          {
            acceptance = binding.removeHostedTrayIcon(current.appId, request, id);
          }
        } else {
          acceptance = binding.showHostedTrayMenu(current.appId, request, id);
        }
        void acceptance?.catch((error) => {
          if (!pendingRequests.delete(request)) return;
          reject(error instanceof Error ? error : new Error(String(error)));
        });
      } catch (error) {
        pendingRequests.delete(request);
        reject(error instanceof Error ? error : new Error(String(error)));
      }
    });
  } catch (error) {
    return Promise.reject(error instanceof Error ? error : new Error(String(error)));
  }
}

function nativeMenu(items: readonly TrayMenuItem[], callbacks: Map<number, () => void>): unknown[] {
  return items.map((item) => {
    if (item.type === "separator") return { type: "separator" };
    if (item.type === "submenu") {
      return {
        type: "submenu",
        label: item.label,
        enabled: item.enabled ?? true,
        items: nativeMenu(item.items, callbacks),
      };
    }
    const id = allocateBoundedId(nextMenuAction, callbacks);
    nextMenuAction = id >= 0xffff_ffff ? 1 : id + 1;
    callbacks.set(id, item.click ?? (() => {}));
    return {
      type: "action",
      id,
      label: item.label,
      enabled: item.enabled ?? true,
      checked: item.checked ?? false,
    };
  });
}

function nativeOptions(
  id: number,
  options: TrayIconOptions,
  callbacks: Map<number, () => void>,
): binding.NativeTrayIconOptions {
  const native: binding.NativeTrayIconOptions = {
    id,
    menu: JSON.stringify(nativeMenu(options.menu ?? [], callbacks)),
  };
  if (typeof options.icon === "string") {
    native.iconPath = resolvePath(options.icon);
  } else if ("path" in options.icon) {
    native.iconPath = resolvePath(options.icon.path);
    if (options.icon.template !== undefined) native.iconIsTemplate = options.icon.template;
  } else {
    native.iconData = Buffer.from(
      options.icon.data.buffer,
      options.icon.data.byteOffset,
      options.icon.data.byteLength,
    );
    if (options.icon.width !== undefined) native.width = options.icon.width;
    if (options.icon.height !== undefined) native.height = options.icon.height;
    if (options.icon.template !== undefined) native.iconIsTemplate = options.icon.template;
  }
  if (options.tooltip !== undefined) native.tooltip = options.tooltip;
  if (options.title !== undefined) native.title = options.title;
  if (options.iconIsTemplate !== undefined) native.iconIsTemplate = options.iconIsTemplate;
  native.menuOnLeftClick = options.menuOnLeftClick ?? true;
  native.visible = options.visible ?? true;
  return native;
}

/** One application-owned native tray/status icon. */
export class TrayIcon {
  readonly id: number;
  #options: TrayIconOptions;
  #menuCallbacks = new Map<number, () => void>();
  #listeners = new Map<TrayEventType, Set<(event: TrayEvent) => void>>();
  #operations = Promise.resolve();
  #destroyed = false;

  private constructor(id: number, options: TrayIconOptions) {
    this.id = id;
    this.#options = { ...options, menu: [...(options.menu ?? [])] };
  }

  static async create(id: number, options: TrayIconOptions): Promise<TrayIcon> {
    const icon = new TrayIcon(id, options);
    await icon.#sync();
    return icon;
  }

  get destroyed(): boolean {
    return this.#destroyed;
  }

  on(type: TrayEventType, listener: (event: TrayEvent) => void): () => void {
    if (this.#destroyed) return () => {};
    const listeners = this.#listeners.get(type) ?? new Set();
    listeners.add(listener);
    this.#listeners.set(type, listeners);
    return () => {
      listeners.delete(listener);
      if (listeners.size === 0) this.#listeners.delete(type);
    };
  }

  async setIcon(icon: TrayIconSource): Promise<void> {
    await this.update({ icon });
  }

  async setMenu(menu: readonly TrayMenuItem[]): Promise<void> {
    await this.update({ menu });
  }

  async setTooltip(tooltip?: string): Promise<void> {
    await this.update({ tooltip });
  }

  async setTitle(title?: string): Promise<void> {
    await this.update({ title });
  }

  async setVisible(visible: boolean): Promise<void> {
    await this.update({ visible });
  }

  update(options: Partial<TrayIconOptions>): Promise<void> {
    return this.#enqueue(async () => {
      this.#assertAlive();
      const next = { ...this.#options, ...options };
      if (options.menu) next.menu = [...options.menu];
      const previous = this.#options;
      this.#options = next;
      try {
        await this.#sync();
      } catch (error) {
        this.#options = previous;
        throw error;
      }
    });
  }

  showMenu(): Promise<void> {
    return this.#enqueue(async () => {
      this.#assertAlive();
      await operation("show-menu", this.id);
    });
  }

  destroy(): Promise<void> {
    return this.#enqueue(async () => {
      if (this.#destroyed) return;
      await operation("remove", this.id);
      this.#destroyed = true;
      this.#menuCallbacks.clear();
      this.#listeners.clear();
      icons.delete(this.id);
    });
  }

  _dispatch(event: TrayEvent & { menuItemId?: number }): void {
    if (this.#destroyed) return;
    if (event.kind === "menu-item" && event.menuItemId !== undefined) {
      this.#menuCallbacks.get(event.menuItemId)?.();
    }
    for (const listener of this.#listeners.get(event.kind) ?? []) listener(event);
  }

  _didDestroy(): void {
    if (this.#destroyed) return;
    this.#destroyed = true;
    this.#menuCallbacks.clear();
    this.#listeners.clear();
  }

  async #sync(): Promise<void> {
    const callbacks = new Map<number, () => void>();
    await operation("set", this.id, nativeOptions(this.id, this.#options, callbacks));
    this.#menuCallbacks = callbacks;
  }

  #enqueue(task: () => Promise<void>): Promise<void> {
    const pending = this.#operations.then(task, task);
    this.#operations = pending.catch(() => {});
    return pending;
  }

  #assertAlive(): void {
    if (this.#destroyed) throw new Error("this native tray icon has been destroyed");
  }
}

/** Native tray/status icons. Linux support uses StatusNotifierItem over D-Bus. */
export const Tray = Object.freeze({
  isSupported(): boolean {
    return (
      process.platform === "darwin" || process.platform === "win32" || process.platform === "linux"
    );
  },

  async create(options: TrayIconOptions): Promise<TrayIcon> {
    const id = allocateBoundedId(nextIcon, icons);
    nextIcon = id >= 0xffff_ffff ? 1 : id + 1;
    const icon = await TrayIcon.create(id, options);
    icons.set(id, icon);
    return icon;
  },
});

export function rejectPendingTrayRequests(error: Error): void {
  for (const [request, pending] of pendingRequests) {
    pendingRequests.delete(request);
    pending.reject(error);
  }
  for (const icon of icons.values()) icon._didDestroy();
  icons.clear();
}

export function dispatchTrayEvent(event: binding.NativeEvent): boolean {
  if (event.kind === "tray-operation") {
    const pending = pendingRequests.get(event.target);
    if (!pending) return true;
    pendingRequests.delete(event.target);
    if (event.error !== undefined) pending.reject(new Error(event.error));
    else pending.resolve();
    return true;
  }
  if (event.kind !== "tray-event") return false;
  const tray = icons.get(event.target);
  if (!tray) return true;
  try {
    const value = JSON.parse(event.value ?? "{}") as {
      kind?: unknown;
      menuItemId?: unknown;
      button?: unknown;
      position?: unknown;
      pressed?: unknown;
      scrollDelta?: unknown;
      horizontal?: unknown;
    };
    const kinds: readonly TrayEventType[] = [
      "click",
      "double-click",
      "enter",
      "move",
      "leave",
      "menu-item",
      "scroll",
    ];
    if (typeof value.kind !== "string" || !kinds.includes(value.kind as TrayEventType)) {
      return true;
    }
    const trayEvent: TrayEvent & { menuItemId?: number } = {
      kind: value.kind as TrayEventType,
    };
    if (typeof value.menuItemId === "number") trayEvent.menuItemId = value.menuItemId;
    if (value.button === "left" || value.button === "right" || value.button === "middle") {
      trayEvent.button = value.button;
    }
    if (
      typeof value.position === "object" &&
      value.position !== null &&
      "x" in value.position &&
      "y" in value.position &&
      typeof value.position.x === "number" &&
      typeof value.position.y === "number"
    ) {
      trayEvent.position = { x: value.position.x, y: value.position.y };
    }
    if (typeof value.pressed === "boolean") trayEvent.pressed = value.pressed;
    if (typeof value.scrollDelta === "number") trayEvent.scrollDelta = value.scrollDelta;
    if (typeof value.horizontal === "boolean") trayEvent.horizontal = value.horizontal;
    tray._dispatch(trayEvent);
  } catch {
    // Invalid native events fail closed.
  }
  return true;
}

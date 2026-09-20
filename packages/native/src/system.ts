import { Buffer } from "node:buffer";
import { resolve as resolvePath } from "node:path";
import * as binding from "./binding.ts";
import type { Window } from "./index.ts";
import { configureTrayContext, dispatchTrayEvent, rejectPendingTrayRequests } from "./tray.ts";

export { Tray, TrayIcon } from "./tray.ts";
export type {
  TrayEvent,
  TrayEventType,
  TrayIconOptions,
  TrayIconSource,
  TrayMenuActionItem,
  TrayMenuItem,
  TrayMenuSeparatorItem,
  TrayMenuSubmenuItem,
} from "./tray.ts";

export interface Rectangle {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface Display {
  /** Process-stable display identifier. Persist `uuid`, when available, across launches. */
  id: string;
  uuid?: string;
  name: string;
  bounds: Rectangle;
  workArea: Rectangle;
  scaleFactor: number;
  refreshRate?: number;
  primary: boolean;
}

export interface DesktopIntegrationSupport {
  systemNotifications: boolean;
  scheduledNotifications: boolean;
  notificationReplies: boolean;
  nativeApplicationMenus: boolean;
  nativePopupMenus: boolean;
  trayIcons: boolean;
  programmableTrayPopup: boolean;
  globalShortcuts: boolean;
  singleInstance: boolean;
  dynamicProtocolRegistration: boolean;
  autostart: boolean;
  windowIcons: boolean;
  windowFocusability: boolean;
  windowOpacity: boolean;
  skipTaskbar: boolean;
  visibleOnAllWorkspaces: boolean;
  cursorControl: boolean;
  cursorScreenPosition: boolean;
  taskbarProgress: boolean;
  taskbarOverlayIcons: boolean;
  dockBadges: boolean;
  dockIcons: boolean;
  dockMenus: boolean;
  recentDocuments: boolean;
  fileIcons: boolean;
  nativeAboutPanel: boolean;
  userTasks: boolean;
}

export interface AboutPanelOptions {
  applicationName?: string;
  applicationVersion?: string;
  version?: string;
  copyright?: string;
  credits?: string;
  icon?: ImageSource;
}

export interface UserTask {
  title: string;
  /** Platform command-line argument string retained by the Windows Jump List. */
  arguments: string;
  program?: string;
  description?: string;
  workingDirectory?: string;
  icon?: { path: string; index: number };
}

export interface NativeImage {
  data: Uint8Array;
  width: number;
  height: number;
}

export interface KeyboardLayout {
  id: string;
  name: string;
}

export type AppearanceMode = "light" | "dark";
export type AppearancePreference = AppearanceMode | "system";
export type WindowKind = "normal" | "popover" | "system-popover" | "floating" | "dialog";
export type WindowLevel =
  | "always-on-bottom"
  | "normal"
  | "always-on-top"
  | "floating"
  | "modal-panel"
  | "main-menu"
  | "status"
  | "pop-up-menu"
  | "screen-saver";
/**
 * A stacking level named the Electron way.
 *
 * `window.setAlwaysOnTop` accepts these alongside QuickGUI's kebab-case names; both resolve to the
 * same core `WindowLevel`.
 */
export type ElectronWindowLevel =
  | "normal"
  | "floating"
  | "torn-off-menu"
  | "tornOffMenu"
  | "modal-panel"
  | "modalPanel"
  | "main-menu"
  | "mainMenu"
  | "status"
  | "pop-up-menu"
  | "popUpMenu"
  | "screen-saver"
  | "screenSaver";
export type CursorGrabMode = "none" | "confined" | "locked";
export type TaskbarProgressState = "none" | "normal" | "indeterminate" | "paused" | "error";
export type WindowBackgroundAppearance = "opaque" | "transparent" | "blurred";
export type MacOSVibrancy =
  | "appearance-based"
  | "titlebar"
  | "selection"
  | "menu"
  | "popover"
  | "sidebar"
  | "header"
  | "sheet"
  | "window"
  | "hud"
  | "fullscreen-ui"
  | "tooltip"
  | "content"
  | "under-window"
  | "under-page";
export type MacOSVisualEffectState = "followWindow" | "active" | "inactive";
export type ImageSource =
  | string
  | {
      path: string;
      /** macOS template-image flag. When omitted, `*Template.png` paths are inferred. */
      template?: boolean;
    }
  | {
      /** Encoded image bytes, or tightly packed RGBA8 when both dimensions are supplied. */
      data: Uint8Array;
      width?: number;
      height?: number;
      /** macOS template-image flag. Omitted values stay false for byte sources. */
      template?: boolean;
    };

export interface WindowState {
  displayId?: string;
  kind: WindowKind;
  bounds: Rectangle;
  viewportSize: { width: number; height: number };
  minimumSize?: { width: number; height: number };
  maximumSize?: { width: number; height: number };
  scaleFactor: number;
  appearance: AppearanceMode;
  backgroundAppearance: WindowBackgroundAppearance;
  vibrancy?: MacOSVibrancy;
  visualEffectState: MacOSVisualEffectState;
  focused: boolean;
  focusable: boolean;
  visible: boolean;
  minimized: boolean;
  maximized: boolean;
  fullscreen: boolean;
  occluded: boolean;
  movable: boolean;
  resizable: boolean;
  minimizable: boolean;
  maximizable: boolean;
  closable: boolean;
  decorated: boolean;
  shadow: boolean;
  contentProtected: boolean;
  windowLevel: WindowLevel;
  skipTaskbar: boolean;
  visibleOnAllWorkspaces: boolean;
  opacity: number;
  hasIcon: boolean;
  taskbarProgressState: TaskbarProgressState;
  taskbarProgress: number;
  hasTaskbarOverlayIcon: boolean;
  cursorVisible: boolean;
  cursorGrab: CursorGrabMode;
  cursorHitTest: boolean;
  cursorPosition?: { x: number; y: number };
  representedFile: boolean;
  documentEdited: boolean;
  nativeTabbing: boolean;
  nativeTabs: {
    count: number;
    selectedIndex?: number;
    tabBarVisible: boolean;
    overviewVisible: boolean;
    truncated: boolean;
  };
}

/** CPU-side timing for the last frame a window completed. */
export interface FrameMetrics {
  /** Monotonic count of completed frames. */
  frameNumber: number;
  /** Application-thread CPU time spent preparing and submitting the last frame. */
  cpuMilliseconds: number;
  /** Exponentially smoothed application-thread CPU time. */
  smoothedCpuMilliseconds: number;
  /** Wall time spent preparing and submitting the last frame, including any surface wait. */
  frameMilliseconds: number;
  /** Exponentially smoothed wall time. Derive FPS as `1000 / smoothedFrameMilliseconds`. */
  smoothedFrameMilliseconds: number;
}

export interface ClipboardTextEntry {
  type: "text";
  text: string;
  /** Application-owned JSON or string metadata. */
  metadata?: string;
}

export interface ClipboardImageEntry {
  type: "image";
  mimeType:
    | "image/png"
    | "image/jpeg"
    | "image/webp"
    | "image/gif"
    | "image/svg+xml"
    | "image/bmp"
    | "image/tiff"
    | "image/x-icon"
    | "image/x-portable-anymap";
  data: Uint8Array;
}

export interface ClipboardFilesEntry {
  type: "files";
  paths: readonly string[];
}

export interface ClipboardDataEntry {
  type: "data";
  mimeType: string;
  data: Uint8Array;
}

export interface ClipboardBookmarkEntry {
  type: "bookmark";
  title: string;
  url: string;
}

export type ClipboardEntry =
  | ClipboardTextEntry
  | ClipboardImageEntry
  | ClipboardFilesEntry
  | ClipboardDataEntry
  | ClipboardBookmarkEntry;

export interface ClipboardItem {
  /** Representations are committed atomically by one native clipboard write. */
  entries: readonly ClipboardEntry[];
}

export interface NotificationAction {
  id: string;
  label: string;
  type?: "button" | "text-input";
  placeholder?: string;
}

export interface NotificationAttachment {
  id: string;
  path: string;
}

export interface NotificationOptions {
  /** Stable identity used to replace or later dismiss this notification. */
  tag: string;
  title: string;
  body?: string;
  subtitle?: string;
  actions?: readonly NotificationAction[];
  /** `default`, `silent`, or a platform-recognized sound name. */
  sound?: string;
  icon?: string;
  attachments?: readonly NotificationAttachment[];
  deliverAt?: Date;
}

export interface NotificationResponse {
  tag: string;
  /** Omitted when the notification body, rather than an action button, was activated. */
  actionId?: string;
  /** Inline text returned by a `text-input` action. */
  reply?: string;
}

export type NotificationPermissionStatus = "not-determined" | "granted" | "denied" | "unsupported";

export type MenuRole =
  | "cut"
  | "copy"
  | "paste"
  | "select-all"
  | "undo"
  | "redo"
  | "about"
  | "hide-application"
  | "hide-other-applications"
  | "show-all-applications"
  | "quit"
  | "close-window"
  | "minimize-window"
  | "zoom-window"
  | "toggle-fullscreen"
  | "bring-all-to-front"
  | "show-help"
  | "paste-and-match-style"
  | "delete"
  | "start-speaking"
  | "stop-speaking"
  | "select-next-tab"
  | "select-previous-tab"
  | "merge-all-windows"
  | "move-tab-to-new-window"
  | "toggle-tab-bar"
  | "toggle-tab-overview";
export type MenuItemMark = "none" | "check" | "radio";

export interface MenuActionItem {
  type?: "action";
  label: string;
  enabled?: boolean;
  checked?: boolean;
  mark?: MenuItemMark;
  icon?: ImageSource;
  role?: MenuRole;
  /** Electron accelerator syntax, for example `CmdOrCtrl+Shift+S`. */
  accelerator?: string;
  /** Keep the item out of the presented native menu without removing its callback. */
  hidden?: boolean;
  click?: () => void;
}

export interface MenuRoleItem {
  type: "role";
  label: string;
  role: MenuRole;
  enabled?: boolean;
  checked?: boolean;
  mark?: MenuItemMark;
  icon?: ImageSource;
  /** Electron accelerator syntax, for example `CmdOrCtrl+Shift+S`. */
  accelerator?: string;
  hidden?: boolean;
}

export interface MenuSeparatorItem {
  type: "separator";
}

export interface MenuSubmenuItem {
  type: "submenu";
  label: string;
  enabled?: boolean;
  items: readonly MenuItem[];
}

export interface MenuServicesItem {
  /** macOS Services. Other platforms omit this operating-system-owned submenu. */
  type: "services";
  label?: string;
}

export interface MenuSystemItem {
  type: "system-menu";
  label: string;
  menu: "services" | "window" | "help" | "recent-documents";
}

export type MenuItem =
  | MenuActionItem
  | MenuRoleItem
  | MenuSeparatorItem
  | MenuSubmenuItem
  | MenuServicesItem
  | MenuSystemItem;

export interface MenuDefinition {
  label: string;
  enabled?: boolean;
  items: readonly MenuItem[];
}

export type GlobalShortcutListener = () => void;
export type PowerEventType =
  | "suspend"
  | "resume"
  | "lock-screen"
  | "unlock-screen"
  | "shutdown-requested"
  | "power-source-changed"
  | "thermal-state-changed"
  | "low-power-mode-changed"
  | "cpu-speed-limit-changed";

export type PowerEvent =
  | { type: "suspend" | "resume" | "lock-screen" | "unlock-screen" | "shutdown-requested" }
  | { type: "power-source-changed"; source: "ac" | "battery" | "unknown" }
  | {
      type: "thermal-state-changed";
      state: "unknown" | "nominal" | "fair" | "serious" | "critical";
    }
  | { type: "low-power-mode-changed"; enabled: boolean }
  | { type: "cpu-speed-limit-changed"; percent: number };

export type PowerSource = "ac" | "battery" | "unknown";
export type BatteryStatus = "charging" | "discharging" | "full" | "not-charging" | "unknown";
export type ThermalState = "unknown" | "nominal" | "fair" | "serious" | "critical";
export type SessionState = "active" | "inactive" | "locked" | "unknown";
export type IdleState = "active" | "idle" | "locked" | "unknown";
export type PowerAssertionKind = "prevent-application-suspension" | "prevent-display-sleep";

export interface PowerState {
  source: PowerSource;
  battery?: { chargePercent?: number; status: BatteryStatus };
  thermalState: ThermalState;
  lowPowerMode?: boolean;
  cpuSpeedLimitPercent?: number;
}

export interface SystemColor {
  red: number;
  green: number;
  blue: number;
  alpha: number;
}

export interface SystemPreferencesSnapshot {
  colorScheme: "light" | "dark" | "unknown";
  reduceMotion?: boolean;
  reduceTransparency?: boolean;
  increaseContrast?: boolean;
  differentiateWithoutColor?: boolean;
  invertColors?: boolean;
  forcedColors?: boolean;
  screenReader?: boolean;
  switchControl?: boolean;
  colors: Partial<
    Record<
      | "accent"
      | "highlight"
      | "highlightText"
      | "windowBackground"
      | "windowText"
      | "controlBackground"
      | "controlText"
      | "link",
      SystemColor
    >
  >;
}

export type PermissionKind = "camera" | "microphone" | "screen-recording" | "accessibility";
export type PermissionStatus = "not-determined" | "granted" | "denied" | "restricted" | "unknown";

/** RAII native sleep assertion. Releasing or dropping this object removes the OS assertion. */
export class PowerAssertion {
  readonly #native: binding.NativePowerAssertion;

  constructor(kind: PowerAssertionKind, reason: string) {
    this.#native = new binding.NativePowerAssertion(kind, reason);
  }

  get kind(): PowerAssertionKind {
    return this.#native.kind as PowerAssertionKind;
  }

  get reason(): string {
    return this.#native.reason;
  }

  get active(): boolean {
    return this.#native.active;
  }

  release(): boolean {
    return this.#native.release();
  }
}

type AppContext = {
  appId: number;
  hosted: boolean;
};

type ContextResolver = () => AppContext;
type WindowResolver = (window?: Window) => { context: AppContext; window: Window };

let resolveContext: ContextResolver | undefined;
let resolveWindow: WindowResolver | undefined;

export function configureSystemContext(context: ContextResolver, window: WindowResolver): void {
  resolveContext = context;
  resolveWindow = window;
  configureTrayContext(context);
}

function context(): AppContext {
  if (!resolveContext) throw new Error("QuickGUI's native system context is not configured");
  return resolveContext();
}

function windowContext(window?: Window): { context: AppContext; window: Window } {
  if (!resolveWindow) throw new Error("QuickGUI's native window context is not configured");
  return resolveWindow(window);
}

async function nativeDisplays(): Promise<binding.NativeDisplay[]> {
  const current = context();
  return await binding.getHostedDisplays(current.appId);
}

function normalizeDisplay(display: binding.NativeDisplay): Display {
  const normalized: Display = {
    id: display.id,
    name: display.name,
    bounds: { ...display.bounds },
    workArea: { ...display.workArea },
    scaleFactor: display.scaleFactor,
    primary: display.primary,
  };
  if (display.uuid !== undefined) normalized.uuid = display.uuid;
  if (display.refreshRate !== undefined) normalized.refreshRate = display.refreshRate;
  return normalized;
}

async function nativeKeyboardLayout(): Promise<KeyboardLayout> {
  const current = context();
  const layout = await binding.getHostedKeyboardLayout(current.appId);
  return { id: layout.id, name: layout.name };
}

function normalizeWindowState(state: binding.NativeWindowState): WindowState {
  const normalized: WindowState = {
    kind: state.kind as WindowKind,
    bounds: { x: state.x, y: state.y, width: state.width, height: state.height },
    viewportSize: { width: state.viewportWidth, height: state.viewportHeight },
    scaleFactor: state.scaleFactor,
    appearance: state.appearance === "dark" ? "dark" : "light",
    backgroundAppearance: state.backgroundAppearance as WindowBackgroundAppearance,
    visualEffectState: state.visualEffectState as MacOSVisualEffectState,
    focused: state.focused,
    focusable: state.focusable,
    visible: state.visible,
    minimized: state.minimized,
    maximized: state.maximized,
    fullscreen: state.fullscreen,
    occluded: state.occluded,
    movable: state.movable,
    resizable: state.resizable,
    minimizable: state.minimizable,
    maximizable: state.maximizable,
    closable: state.closable,
    decorated: state.decorated,
    shadow: state.shadow,
    contentProtected: state.contentProtected,
    windowLevel: state.windowLevel as WindowLevel,
    skipTaskbar: state.skipTaskbar,
    visibleOnAllWorkspaces: state.visibleOnAllWorkspaces,
    opacity: state.opacity,
    hasIcon: state.hasIcon,
    taskbarProgressState: state.taskbarProgressState as TaskbarProgressState,
    taskbarProgress: state.taskbarProgress,
    hasTaskbarOverlayIcon: state.hasTaskbarOverlayIcon,
    cursorVisible: state.cursorVisible,
    cursorGrab: state.cursorGrab as CursorGrabMode,
    cursorHitTest: state.cursorHitTest,
    representedFile: state.representedFile,
    documentEdited: state.documentEdited,
    nativeTabbing: state.nativeTabbing,
    nativeTabs: {
      count: state.nativeTabCount,
      tabBarVisible: state.nativeTabBarVisible,
      overviewVisible: state.nativeTabOverviewVisible,
      truncated: state.nativeTabsTruncated,
    },
  };
  if (state.vibrancy !== undefined) {
    normalized.vibrancy = state.vibrancy as MacOSVibrancy;
  }
  if (state.displayId !== undefined) normalized.displayId = state.displayId;
  if (state.minimumWidth !== undefined && state.minimumHeight !== undefined) {
    normalized.minimumSize = { width: state.minimumWidth, height: state.minimumHeight };
  }
  if (state.maximumWidth !== undefined && state.maximumHeight !== undefined) {
    normalized.maximumSize = { width: state.maximumWidth, height: state.maximumHeight };
  }
  if (state.cursorX !== undefined && state.cursorY !== undefined) {
    normalized.cursorPosition = { x: state.cursorX, y: state.cursorY };
  }
  if (state.nativeSelectedTab !== undefined) {
    normalized.nativeTabs.selectedIndex = state.nativeSelectedTab;
  }
  return normalized;
}

export async function getNativeWindowState(window: Window): Promise<WindowState> {
  const { context: current, window: resolved } = windowContext(window);
  const state = await binding.getHostedWindowState(current.appId, resolved.nativeId);
  return normalizeWindowState(state);
}

/** Read the latest completed frame without requesting another frame. */
export async function getNativeFrameMetrics(window: Window): Promise<FrameMetrics> {
  const { context: current, window: resolved } = windowContext(window);
  return await binding.getHostedFrameMetrics(current.appId, resolved.nativeId);
}

/** Declare whether the hosted application intercepts the preventable before-quit phase. */
export function setNativeQuitInterception(intercepting: boolean): void {
  const current = context();
  binding.setHostedQuitInterception(current.appId, intercepting);
}

/** Ask for a preventable native quit that runs the before-quit and will-quit phases. */
export async function requestNativeQuit(): Promise<boolean> {
  const current = context();
  return await binding.requestHostedAppQuit(current.appId);
}

export function performNativeWindowAction(window: Window, action: string, value?: string): void {
  const { context: current, window: resolved } = windowContext(window);
  {
    binding.performHostedWindowAction(current.appId, resolved.nativeId, action, value);
  }
}

export function performNativeWindowImageAction(
  window: Window,
  action: "set-icon" | "clear-icon" | "set-taskbar-overlay-icon",
  image?: ImageSource,
  description?: string,
): void {
  const { context: current, window: resolved } = windowContext(window);
  const native = image === undefined ? undefined : nativeImageSource(image);
  {
    binding.performHostedWindowImageAction(
      current.appId,
      resolved.nativeId,
      action,
      native,
      description,
    );
  }
}

export function nativeImageSource(source: ImageSource): binding.NativeImageSource {
  if (typeof source === "string") return { path: resolvePath(source) };
  if ("path" in source) {
    const native: binding.NativeImageSource = { path: resolvePath(source.path) };
    if (source.template !== undefined) native.template = source.template;
    return native;
  }
  const native: binding.NativeImageSource = {
    data: Buffer.from(source.data.buffer, source.data.byteOffset, source.data.byteLength),
  };
  if (source.width !== undefined) native.width = source.width;
  if (source.height !== undefined) native.height = source.height;
  if (source.template !== undefined) native.template = source.template;
  return native;
}

function nativeClipboardItem(item: ClipboardItem): binding.NativeClipboardItem {
  return {
    entries: item.entries.map((entry): binding.NativeClipboardEntry => {
      if (entry.type === "text") {
        const native: binding.NativeClipboardEntry = { kind: "text", text: entry.text };
        if (entry.metadata !== undefined) native.metadata = entry.metadata;
        return native;
      }
      if (entry.type === "image") {
        return {
          kind: "image",
          format: entry.mimeType,
          data: Buffer.from(entry.data.buffer, entry.data.byteOffset, entry.data.byteLength),
        };
      }
      if (entry.type === "files") return { kind: "files", paths: [...entry.paths] };
      if (entry.type === "data") {
        return {
          kind: "data",
          format: entry.mimeType,
          data: Buffer.from(entry.data.buffer, entry.data.byteOffset, entry.data.byteLength),
        };
      }
      return { kind: "bookmark", text: entry.title, url: entry.url };
    }),
  };
}

function clipboardItem(item: binding.NativeClipboardItem): ClipboardItem {
  return {
    entries: item.entries.map((entry): ClipboardEntry => {
      if (entry.kind === "text" && entry.text !== undefined) {
        const text: ClipboardTextEntry = { type: "text", text: entry.text };
        if (entry.metadata !== undefined) text.metadata = entry.metadata;
        return text;
      }
      if (entry.kind === "image" && entry.format && entry.data) {
        return {
          type: "image",
          mimeType: entry.format as ClipboardImageEntry["mimeType"],
          data: Uint8Array.from(entry.data),
        };
      }
      if (entry.kind === "files" && entry.paths) {
        return { type: "files", paths: entry.paths };
      }
      if (entry.kind === "data" && entry.format && entry.data) {
        return { type: "data", mimeType: entry.format, data: Uint8Array.from(entry.data) };
      }
      if (entry.kind === "bookmark" && entry.text !== undefined && entry.url !== undefined) {
        return { type: "bookmark", title: entry.text, url: entry.url };
      }
      throw new Error(`the native clipboard returned an invalid ${entry.kind} entry`);
    }),
  };
}

const screenListeners = new Set<(displays: readonly Display[]) => void>();
const appearanceListeners = new Set<(appearance: AppearanceMode, window: Window) => void>();
const keyboardListeners = new Set<(layout: KeyboardLayout) => void>();
const powerListeners = new Map<PowerEventType, Set<(event: PowerEvent) => void>>();
const notificationListeners = new Set<(response: NotificationResponse) => void>();
const systemPreferencesListeners = new Set<(preferences: SystemPreferencesSnapshot) => void>();
let menuCallbacks = new Map<number, () => void>();
let applicationMenuActionIds = new Set<number>();
let nextMenuAction = 1;
const windowStateListeners = new Map<number, Set<(state: WindowState) => void>>();
const pendingShellRequests = new Map<
  number,
  { resolve: () => void; reject: (error: Error) => void }
>();
let nextShellRequest = 1;
const pendingNotificationPermissionRequests = new Map<
  number,
  { resolve: (status: NotificationPermissionStatus) => void; reject: (error: Error) => void }
>();
let nextNotificationPermissionRequest = 1;
const pendingFileIconRequests = new Map<
  number,
  { resolve: (icon: NativeImage) => void; reject: (error: Error) => void }
>();
const pendingUserTaskRequests = new Map<
  number,
  { resolve: () => void; reject: (error: Error) => void }
>();
let nextFileIconRequest = 1;
let nextUserTaskRequest = 1;
let disposeDockMenu: (() => void) | undefined;
const globalShortcutRegistrations = new Map<
  number,
  { accelerator: string; listener: GlobalShortcutListener }
>();
const globalShortcutIds = new Map<string, number>();
const pendingGlobalShortcutRequests = new Map<
  number,
  { resolve: () => void; reject: (error: Error) => void }
>();
let nextGlobalShortcutRequest = 1;
let nextGlobalShortcutRegistration = 1;

function watchHostedRequestAcceptance<T extends { reject: (error: Error) => void }>(
  operation: Promise<void>,
  request: number,
  pending: Map<number, T>,
): void {
  void operation.catch((error) => {
    const requestState = pending.get(request);
    if (!requestState) return;
    pending.delete(request);
    requestState.reject(error instanceof Error ? error : new Error(String(error)));
  });
}

export const Clipboard = Object.freeze({
  async read(): Promise<ClipboardItem | undefined> {
    const current = context();
    const item = await binding.readHostedClipboard(current.appId);
    return item ? clipboardItem(item) : undefined;
  },

  async write(item: ClipboardItem): Promise<void> {
    const current = context();
    const native = nativeClipboardItem(item);
    await binding.writeHostedClipboard(current.appId, native);
  },

  async readText(): Promise<string> {
    const item = await this.read();
    if (!item) return "";
    const text = item.entries
      .filter((entry): entry is ClipboardTextEntry => entry.type === "text")
      .map((entry) => entry.text)
      .join("");
    if (text) return text;
    return item.entries
      .filter((entry): entry is ClipboardFilesEntry => entry.type === "files")
      .flatMap((entry) => entry.paths)
      .join("\n");
  },

  async writeText(text: string): Promise<void> {
    await this.write({ entries: [{ type: "text", text }] });
  },

  async clear(): Promise<void> {
    await this.write({ entries: [] });
  },

  /** Every MIME type the current clipboard item can supply. */
  async availableFormats(): Promise<readonly string[]> {
    const item = await this.read();
    if (!item) return [];
    return item.entries.map(clipboardEntryFormat);
  },

  /** Whether the current clipboard item carries this MIME type. */
  async has(format: string): Promise<boolean> {
    const formats = await this.availableFormats();
    return formats.includes(format);
  },

  /** Read one arbitrary representation by MIME type. */
  async readBuffer(format: string): Promise<Uint8Array | undefined> {
    const item = await this.read();
    if (!item) return undefined;
    for (const entry of item.entries) {
      if (clipboardEntryFormat(entry) !== format) continue;
      if (entry.type === "data" || entry.type === "image") return entry.data;
      if (entry.type === "text") return new TextEncoder().encode(entry.text);
    }
    return undefined;
  },

  /** Atomically replace the clipboard with one arbitrary representation. */
  async writeBuffer(format: string, data: Uint8Array): Promise<void> {
    if (format === "text/plain") {
      await this.writeText(new TextDecoder().decode(data));
      return;
    }
    await this.write({ entries: [{ type: "data", mimeType: format, data }] });
  },

  /**
   * Read macOS's shared Find pasteboard. Other platforms report an empty search string.
   */
  async readFindText(): Promise<string> {
    const current = context();
    const item = await binding.readHostedFindClipboard(current.appId);
    if (!item) return "";
    return clipboardItem(item)
      .entries.filter((entry): entry is ClipboardTextEntry => entry.type === "text")
      .map((entry) => entry.text)
      .join("");
  },

  /** Replace macOS's shared Find pasteboard. An empty string clears it. */
  async writeFindText(text: string): Promise<void> {
    const current = context();
    const native = nativeClipboardItem({
      entries: text ? [{ type: "text", text }] : [],
    });
    await binding.writeHostedFindClipboard(current.appId, native);
  },
});

function clipboardEntryFormat(entry: ClipboardEntry): string {
  if (entry.type === "text") return "text/plain";
  if (entry.type === "files") return "text/uri-list";
  if (entry.type === "bookmark") return "text/x-moz-url";
  return entry.mimeType;
}

function allocateShellRequest(): number {
  for (let attempt = 0; attempt <= pendingShellRequests.size; attempt += 1) {
    const request = nextShellRequest;
    nextShellRequest = request >= 0xffff_ffff ? 1 : request + 1;
    if (!pendingShellRequests.has(request)) return request;
  }
  throw new Error("the native shell request id space is exhausted");
}

function shellRequest(action: string, value: string): Promise<void> {
  try {
    const current = context();
    const request = allocateShellRequest();
    return new Promise<void>((resolve, reject) => {
      pendingShellRequests.set(request, {
        resolve,
        reject: (error) => reject(error),
      });
      try {
        {
          watchHostedRequestAcceptance(
            binding.performHostedShellAction(current.appId, request, action, value),
            request,
            pendingShellRequests,
          );
        }
      } catch (error) {
        pendingShellRequests.delete(request);
        reject(error instanceof Error ? error : new Error(String(error)));
      }
    });
  } catch (error) {
    return Promise.reject(error instanceof Error ? error : new Error(String(error)));
  }
}

function externalUrl(value: string | URL): string {
  const url = value instanceof URL ? value : new URL(value);
  if (url.protocol === "file:") {
    throw new TypeError("Shell.openExternal does not accept file URLs; use Shell.openPath");
  }
  return url.href;
}

/** Cross-platform operations delegated to the user's registered system applications. */
export const Shell = Object.freeze({
  openExternal(url: string | URL): Promise<void> {
    try {
      return shellRequest("open-external", externalUrl(url));
    } catch (error) {
      return Promise.reject(error instanceof Error ? error : new Error(String(error)));
    }
  },

  openPath(path: string): Promise<void> {
    return shellRequest("open-path", resolvePath(path));
  },

  showItemInFolder(path: string): Promise<void> {
    return shellRequest("reveal-path", resolvePath(path));
  },

  trashItem(path: string): Promise<void> {
    return shellRequest("trash-path", resolvePath(path));
  },

  /** Play the operating system's alert sound. */
  beep(): void {
    appMutation("beep");
  },
});

/**
 * Application-level control of the installed spell-check provider.
 *
 * Per-input `spellcheck` and `autocorrect` behavior is a component property; these two calls are
 * the application-wide dictionary operations a context menu performs.
 */
export const SpellChecker = Object.freeze({
  /** Add one word to the user dictionary. */
  learnWord(word: string): void {
    appMutation("learn-word", validateSpellWord(word));
  },

  /** Ignore one word for the remainder of the shared checking session. */
  ignoreWord(word: string): void {
    appMutation("ignore-word", validateSpellWord(word));
  },
});

/** Longest word accepted by the application-level dictionary calls. */
const MAX_SPELL_WORD_BYTES = 256;

function validateSpellWord(word: string): string {
  if (
    typeof word !== "string" ||
    word.length === 0 ||
    word.includes("\0") ||
    new TextEncoder().encode(word).length > MAX_SPELL_WORD_BYTES
  ) {
    throw new TypeError(
      `a spell-check word must be nonempty, NUL-free, and at most ${MAX_SPELL_WORD_BYTES} UTF-8 bytes`,
    );
  }
  return word;
}

/** Operating-system notifications delivered outside QuickGUI windows. */
export const Notifications = Object.freeze({
  isSupported(): boolean {
    return (
      process.platform === "darwin" || process.platform === "win32" || process.platform === "linux"
    );
  },

  async show(options: NotificationOptions): Promise<void> {
    const current = context();
    const native: binding.NativeNotificationOptions = {
      tag: options.tag,
      title: options.title,
      body: options.body ?? "",
      actions: (options.actions ?? []).map((action) => {
        const nativeAction: binding.NativeNotificationAction = {
          id: action.id,
          label: action.label,
        };
        if (action.type !== undefined) nativeAction.kind = action.type;
        if (action.placeholder !== undefined) nativeAction.placeholder = action.placeholder;
        return nativeAction;
      }),
    };
    if (options.subtitle !== undefined) native.subtitle = options.subtitle;
    if (options.sound !== undefined) native.sound = options.sound;
    if (options.icon !== undefined) native.iconPath = resolvePath(options.icon);
    if (options.attachments !== undefined) {
      native.attachments = options.attachments.map((attachment) => ({
        id: attachment.id,
        path: resolvePath(attachment.path),
      }));
    }
    if (options.deliverAt !== undefined) native.deliveryAtMs = options.deliverAt.getTime();
    await binding.showHostedNotification(current.appId, native);
  },

  async dismiss(tag: string): Promise<void> {
    const current = context();
    await binding.dismissHostedNotification(current.appId, tag);
  },

  onResponse(listener: (response: NotificationResponse) => void): () => void {
    notificationListeners.add(listener);
    return () => notificationListeners.delete(listener);
  },

  getPermissionStatus(): Promise<NotificationPermissionStatus> {
    return notificationPermissionRequest(false);
  },

  requestPermission(): Promise<NotificationPermissionStatus> {
    return notificationPermissionRequest(true);
  },
});

function notificationPermissionRequest(prompt: boolean): Promise<NotificationPermissionStatus> {
  try {
    const current = context();
    const request = allocateBoundedId(
      nextNotificationPermissionRequest,
      pendingNotificationPermissionRequests,
    );
    nextNotificationPermissionRequest = request >= 0xffff_ffff ? 1 : request + 1;
    return new Promise((resolve, reject) => {
      pendingNotificationPermissionRequests.set(request, { resolve, reject });
      try {
        {
          watchHostedRequestAcceptance(
            binding.performHostedNotificationPermissionRequest(current.appId, request, prompt),
            request,
            pendingNotificationPermissionRequests,
          );
        }
      } catch (error) {
        pendingNotificationPermissionRequests.delete(request);
        reject(error instanceof Error ? error : new Error(String(error)));
      }
    });
  } catch (error) {
    return Promise.reject(error instanceof Error ? error : new Error(String(error)));
  }
}

function allocateMenuAction(callbacks: Map<number, () => void>): number {
  for (let attempt = 0; attempt <= callbacks.size + menuCallbacks.size; attempt += 1) {
    const id = nextMenuAction;
    nextMenuAction = id >= 0xffff_ffff ? 1 : id + 1;
    if (!callbacks.has(id) && !menuCallbacks.has(id)) return id;
  }
  throw new Error("the native menu action id space is exhausted");
}

export function serializeNativeMenu(definitions: readonly MenuDefinition[]): {
  json: string;
  actionIds: readonly number[];
  install: () => () => void;
} {
  const callbacks = new Map<number, () => void>();
  const native = definitions.map((menu) => ({
    label: menu.label,
    enabled: menu.enabled ?? true,
    items: nativeMenuItems(menu.items, callbacks),
  }));
  return {
    json: JSON.stringify(native),
    actionIds: [...callbacks.keys()],
    install: () => {
      for (const [id, callback] of callbacks) menuCallbacks.set(id, callback);
      return () => {
        for (const id of callbacks.keys()) menuCallbacks.delete(id);
      };
    },
  };
}

function nativeMenuItems(
  items: readonly MenuItem[],
  callbacks: Map<number, () => void>,
): unknown[] {
  return items.map((item) => {
    if (item.type === "separator") return { type: "separator" };
    if (item.type === "services") {
      return { type: "services", label: item.label ?? "Services" };
    }
    if (item.type === "system-menu") {
      return { type: "system-menu", label: item.label, menu: item.menu };
    }
    if (item.type === "submenu") {
      return {
        type: "submenu",
        label: item.label,
        enabled: item.enabled ?? true,
        items: nativeMenuItems(item.items, callbacks),
      };
    }
    if (item.type === "role") {
      return {
        type: "role",
        label: item.label,
        enabled: item.enabled ?? true,
        checked: item.checked ?? false,
        ...(item.mark ? { mark: item.mark } : {}),
        role: item.role,
        ...(item.icon ? { icon: jsonImageSource(item.icon) } : {}),
        ...(item.accelerator ? { accelerator: item.accelerator } : {}),
        ...(item.hidden ? { hidden: true } : {}),
      };
    }
    const id = allocateMenuAction(callbacks);
    callbacks.set(id, item.click ?? (() => {}));
    return {
      type: "action",
      id,
      label: item.label,
      enabled: item.enabled ?? true,
      checked: item.checked ?? false,
      ...(item.mark ? { mark: item.mark } : {}),
      ...(item.role ? { role: item.role } : {}),
      ...(item.icon ? { icon: jsonImageSource(item.icon) } : {}),
      ...(item.accelerator ? { accelerator: item.accelerator } : {}),
      ...(item.hidden ? { hidden: true } : {}),
    };
  });
}

function jsonImageSource(source: ImageSource): unknown {
  if (typeof source === "string") return { path: resolvePath(source) };
  if ("path" in source) {
    return {
      path: resolvePath(source.path),
      ...(source.template !== undefined ? { template: source.template } : {}),
    };
  }
  return {
    dataBase64: Buffer.from(
      source.data.buffer,
      source.data.byteOffset,
      source.data.byteLength,
    ).toString("base64"),
    ...(source.width !== undefined ? { width: source.width } : {}),
    ...(source.height !== undefined ? { height: source.height } : {}),
    ...(source.template !== undefined ? { template: source.template } : {}),
  };
}

/** Declarative platform-native application menus. */
export const Menu = Object.freeze({
  setApplicationMenu(definitions: readonly MenuDefinition[] | null): void {
    const current = context();
    const serialized = serializeNativeMenu(definitions ?? []);
    binding.setHostedApplicationMenu(current.appId, serialized.json);
    for (const id of applicationMenuActionIds) menuCallbacks.delete(id);
    serialized.install();
    applicationMenuActionIds = new Set(serialized.actionIds);
    // Application menu actions live until the next replacement; keep the disposer represented
    // by their exact ids so window and Dock menu callbacks remain independently owned.
  },

  /**
   * Present a platform-native popup menu owned by one window.
   *
   * `x` and `y` are window-local logical pixels from the top-left; omit both to use the current
   * cursor position. The promise resolves once the menu closes, whether an item was chosen or the
   * user dismissed it, and the item callbacks are released with it.
   */
  popup(items: readonly MenuItem[], options: PopupMenuOptions = {}): Promise<void> {
    return showNativePopupMenu(items, options);
  },
});

function allocateBoundedId(next: number, used: ReadonlyMap<number, unknown>): number {
  let candidate = next;
  for (let attempt = 0; attempt <= used.size; attempt += 1) {
    if (!used.has(candidate)) return candidate;
    candidate = candidate >= 0xffff_ffff ? 1 : candidate + 1;
  }
  throw new Error("the native system id space is exhausted");
}

function globalShortcutOperation(
  action: "register" | "unregister" | "unregister-all",
  registration?: number,
  accelerator?: string,
): Promise<void> {
  try {
    const current = context();
    const request = allocateBoundedId(nextGlobalShortcutRequest, pendingGlobalShortcutRequests);
    nextGlobalShortcutRequest = request >= 0xffff_ffff ? 1 : request + 1;
    return new Promise<void>((resolve, reject) => {
      pendingGlobalShortcutRequests.set(request, { resolve, reject });
      try {
        {
          watchHostedRequestAcceptance(
            binding.performHostedGlobalShortcutAction(
              current.appId,
              request,
              action,
              registration,
              accelerator,
            ),
            request,
            pendingGlobalShortcutRequests,
          );
        }
      } catch (error) {
        pendingGlobalShortcutRequests.delete(request);
        reject(error instanceof Error ? error : new Error(String(error)));
      }
    });
  } catch (error) {
    return Promise.reject(error instanceof Error ? error : new Error(String(error)));
  }
}

/** System-wide keyboard shortcuts. Linux support requires an X11 session. */
export const GlobalShortcut = Object.freeze({
  isSupported(): boolean {
    return (
      process.platform === "darwin" || process.platform === "win32" || process.platform === "linux"
    );
  },

  async register(
    accelerator: string,
    listener: GlobalShortcutListener,
  ): Promise<() => Promise<void>> {
    if (globalShortcutIds.has(accelerator)) {
      throw new Error(`the global shortcut \`${accelerator}\` is already registered`);
    }
    const registration = allocateBoundedId(
      nextGlobalShortcutRegistration,
      globalShortcutRegistrations,
    );
    nextGlobalShortcutRegistration = registration >= 0xffff_ffff ? 1 : registration + 1;
    globalShortcutIds.set(accelerator, registration);
    globalShortcutRegistrations.set(registration, { accelerator, listener });
    try {
      await globalShortcutOperation("register", registration, accelerator);
    } catch (error) {
      globalShortcutIds.delete(accelerator);
      globalShortcutRegistrations.delete(registration);
      throw error;
    }
    let disposed = false;
    return async () => {
      if (disposed) return;
      disposed = true;
      if (globalShortcutRegistrations.get(registration)?.accelerator !== accelerator) return;
      await globalShortcutOperation("unregister", registration);
      globalShortcutRegistrations.delete(registration);
      globalShortcutIds.delete(accelerator);
    };
  },

  async unregister(accelerator: string): Promise<void> {
    const registration = globalShortcutIds.get(accelerator);
    if (registration === undefined) return;
    await globalShortcutOperation("unregister", registration);
    globalShortcutRegistrations.delete(registration);
    globalShortcutIds.delete(accelerator);
  },

  async unregisterAll(): Promise<void> {
    await globalShortcutOperation("unregister-all");
    globalShortcutRegistrations.clear();
    globalShortcutIds.clear();
  },

  isRegistered(accelerator: string): boolean {
    return globalShortcutIds.has(accelerator);
  },
});

export const Screen = Object.freeze({
  async getAllDisplays(): Promise<Display[]> {
    return (await nativeDisplays()).map(normalizeDisplay);
  },

  async getPrimaryDisplay(): Promise<Display | undefined> {
    return (await this.getAllDisplays()).find((display) => display.primary);
  },

  async getDisplayNearestPoint(point: { x: number; y: number }): Promise<Display | undefined> {
    let nearest: Display | undefined;
    let nearestDistance = Number.POSITIVE_INFINITY;
    for (const display of await this.getAllDisplays()) {
      const right = display.bounds.x + display.bounds.width;
      const bottom = display.bounds.y + display.bounds.height;
      const dx =
        point.x < display.bounds.x ? display.bounds.x - point.x : Math.max(0, point.x - right);
      const dy =
        point.y < display.bounds.y ? display.bounds.y - point.y : Math.max(0, point.y - bottom);
      const distance = dx * dx + dy * dy;
      if (distance < nearestDistance) {
        nearest = display;
        nearestDistance = distance;
      }
    }
    return nearest;
  },

  async getCursorScreenPoint(): Promise<{ x: number; y: number }> {
    const current = context();
    const point = await binding.getHostedCursorScreenPosition(current.appId);
    return { x: point.x, y: point.y };
  },

  onChange(listener: (displays: readonly Display[]) => void): () => void {
    screenListeners.add(listener);
    return () => screenListeners.delete(listener);
  },
});

/** Compile-time native integration availability for the current target. */
export const Desktop = Object.freeze({
  async getSupport(): Promise<DesktopIntegrationSupport> {
    const current = context();
    const support = await binding.getHostedDesktopIntegrationSupport(current.appId);
    return { ...support };
  },

  setDockBadge(value?: string): void {
    const current = context();
    binding.setHostedDockBadge(current.appId, value);
  },

  setDockIcon(icon?: ImageSource): void {
    const current = context();
    const native = icon === undefined ? undefined : nativeImageSource(icon);
    binding.setHostedDockIcon(current.appId, native);
  },

  setDockMenu(menu?: MenuDefinition): void {
    const current = context();
    const serialized = menu === undefined ? undefined : serializeNativeMenu([menu]);
    binding.setHostedDockMenu(current.appId, serialized?.json);
    disposeDockMenu?.();
    disposeDockMenu = serialized?.install();
  },

  addRecentDocument(path: string): void {
    const current = context();
    const native = resolvePath(path);
    binding.addHostedRecentDocument(current.appId, native);
  },

  clearRecentDocuments(): void {
    const current = context();
    binding.clearHostedRecentDocuments(current.appId);
  },

  showAboutPanel(options: AboutPanelOptions = {}): void {
    const current = context();
    const native: binding.NativeAboutPanelOptions = {};
    if (options.applicationName !== undefined) native.applicationName = options.applicationName;
    if (options.applicationVersion !== undefined) {
      native.applicationVersion = options.applicationVersion;
    }
    if (options.version !== undefined) native.version = options.version;
    if (options.copyright !== undefined) native.copyright = options.copyright;
    if (options.credits !== undefined) native.credits = options.credits;
    if (options.icon !== undefined) native.icon = nativeImageSource(options.icon);
    binding.showHostedAboutPanel(current.appId, native);
  },

  getFileIcon(path: string, size: "small" | "normal" | "large" = "normal"): Promise<NativeImage> {
    try {
      const current = context();
      const request = allocateBoundedId(nextFileIconRequest, pendingFileIconRequests);
      nextFileIconRequest = request >= 0xffff_ffff ? 1 : request + 1;
      return new Promise((resolve, reject) => {
        pendingFileIconRequests.set(request, { resolve, reject });
        try {
          {
            watchHostedRequestAcceptance(
              binding.requestHostedFileIcon(current.appId, request, resolvePath(path), size),
              request,
              pendingFileIconRequests,
            );
          }
        } catch (error) {
          pendingFileIconRequests.delete(request);
          reject(error instanceof Error ? error : new Error(String(error)));
        }
      });
    } catch (error) {
      return Promise.reject(error instanceof Error ? error : new Error(String(error)));
    }
  },

  setUserTasks(tasks: readonly UserTask[]): Promise<void> {
    try {
      const current = context();
      const request = allocateBoundedId(nextUserTaskRequest, pendingUserTaskRequests);
      nextUserTaskRequest = request >= 0xffff_ffff ? 1 : request + 1;
      const native = tasks.map((task): binding.NativeUserTask => {
        const value: binding.NativeUserTask = {
          title: task.title,
          arguments: task.arguments,
        };
        if (task.program !== undefined) value.program = resolvePath(task.program);
        if (task.description !== undefined) value.description = task.description;
        if (task.workingDirectory !== undefined) {
          value.workingDirectory = resolvePath(task.workingDirectory);
        }
        if (task.icon !== undefined) {
          value.iconPath = resolvePath(task.icon.path);
          value.iconIndex = task.icon.index;
        }
        return value;
      });
      return new Promise((resolve, reject) => {
        pendingUserTaskRequests.set(request, { resolve, reject });
        try {
          {
            watchHostedRequestAcceptance(
              binding.setHostedUserTasks(current.appId, request, native),
              request,
              pendingUserTaskRequests,
            );
          }
        } catch (error) {
          pendingUserTaskRequests.delete(request);
          reject(error instanceof Error ? error : new Error(String(error)));
        }
      });
    } catch (error) {
      return Promise.reject(error instanceof Error ? error : new Error(String(error)));
    }
  },
});

export const Appearance = Object.freeze({
  async getCurrent(window?: Window): Promise<AppearanceMode> {
    return (await getNativeWindowState(windowContext(window).window)).appearance;
  },

  onChange(listener: (appearance: AppearanceMode, window: Window) => void): () => void {
    appearanceListeners.add(listener);
    return () => appearanceListeners.delete(listener);
  },
});

export const Keyboard = Object.freeze({
  getLayout(): Promise<KeyboardLayout> {
    return nativeKeyboardLayout();
  },

  onLayoutChange(listener: (layout: KeyboardLayout) => void): () => void {
    keyboardListeners.add(listener);
    return () => keyboardListeners.delete(listener);
  },
});

/** Native operating-system sleep and user-session transitions. */
export const PowerMonitor = Object.freeze({
  getState(): PowerState {
    const state = binding.getPowerState();
    const result: PowerState = {
      source: state.source as PowerSource,
      thermalState: state.thermalState as ThermalState,
    };
    if (state.battery !== undefined) {
      const battery: NonNullable<PowerState["battery"]> = {
        status: state.battery.status as BatteryStatus,
      };
      if (state.battery.chargePercent !== undefined) {
        battery.chargePercent = state.battery.chargePercent;
      }
      result.battery = battery;
    }
    if (state.lowPowerMode !== undefined) result.lowPowerMode = state.lowPowerMode;
    if (state.cpuSpeedLimitPercent !== undefined) {
      result.cpuSpeedLimitPercent = state.cpuSpeedLimitPercent;
    }
    return result;
  },

  isOnBatteryPower(): boolean {
    return this.getState().source === "battery";
  },

  getCurrentThermalState(): ThermalState {
    return this.getState().thermalState;
  },

  getSystemIdleTime(): number {
    return binding.getSystemIdleTime();
  },

  getSystemIdleState(thresholdSeconds: number): IdleState {
    return binding.getSystemIdleState(thresholdSeconds) as IdleState;
  },

  getSessionState(): SessionState {
    return binding.getSessionState() as SessionState;
  },

  on<T extends PowerEventType>(
    event: T,
    listener: (event: Extract<PowerEvent, { type: T }>) => void,
  ): () => void {
    const listeners = powerListeners.get(event) ?? new Set();
    const wrapped = listener as (event: PowerEvent) => void;
    listeners.add(wrapped);
    powerListeners.set(event, listeners);
    return () => {
      listeners.delete(wrapped);
      if (listeners.size === 0) powerListeners.delete(event);
    };
  },
});

async function nativeSystemPreferences(): Promise<binding.NativeSystemPreferences> {
  const current = context();
  return await binding.getHostedSystemPreferences(current.appId);
}

function normalizeSystemPreferences(
  preferences: binding.NativeSystemPreferences,
): SystemPreferencesSnapshot {
  const result: SystemPreferencesSnapshot = {
    colorScheme: preferences.colorScheme as SystemPreferencesSnapshot["colorScheme"],
    colors: {},
  };
  const optionalBooleans = [
    "reduceMotion",
    "reduceTransparency",
    "increaseContrast",
    "differentiateWithoutColor",
    "invertColors",
    "forcedColors",
    "screenReader",
    "switchControl",
  ] as const;
  for (const key of optionalBooleans) {
    const value = preferences[key];
    if (value !== undefined) result[key] = value;
  }
  const colors = [
    ["accent", "accentColor"],
    ["highlight", "highlightColor"],
    ["highlightText", "highlightTextColor"],
    ["windowBackground", "windowBackgroundColor"],
    ["windowText", "windowTextColor"],
    ["controlBackground", "controlBackgroundColor"],
    ["controlText", "controlTextColor"],
    ["link", "linkColor"],
  ] as const;
  for (const [key, nativeKey] of colors) {
    const value = preferences[nativeKey];
    if (value !== undefined) result.colors[key] = { ...value };
  }
  return result;
}

/** Core-retained appearance and accessibility settings. */
export const SystemPreferences = Object.freeze({
  async getCurrent(): Promise<SystemPreferencesSnapshot> {
    return normalizeSystemPreferences(await nativeSystemPreferences());
  },

  onChange(listener: (preferences: SystemPreferencesSnapshot) => void): () => void {
    systemPreferencesListeners.add(listener);
    return () => systemPreferencesListeners.delete(listener);
  },
});

/** Explicit native privacy authorization checks and prompts. */
export const Permissions = Object.freeze({
  status(kind: PermissionKind): PermissionStatus {
    return binding.getPermissionStatus(kind) as PermissionStatus;
  },

  async request(kind: PermissionKind): Promise<PermissionStatus> {
    return (await binding.requestPermission(kind)) as PermissionStatus;
  },
});

export function onNativeWindowStateChange(
  window: Window,
  listener: (state: WindowState) => void,
): () => void {
  const listeners = windowStateListeners.get(window.nativeId) ?? new Set();
  listeners.add(listener);
  windowStateListeners.set(window.nativeId, listeners);
  return () => {
    listeners.delete(listener);
    if (listeners.size === 0) windowStateListeners.delete(window.nativeId);
  };
}

export function removeNativeWindowStateListeners(window: Window): void {
  windowStateListeners.delete(window.nativeId);
}

export function rejectPendingSystemRequests(error: Error): void {
  for (const [request, pending] of pendingShellRequests) {
    pendingShellRequests.delete(request);
    pending.reject(error);
  }
  for (const [request, pending] of pendingNotificationPermissionRequests) {
    pendingNotificationPermissionRequests.delete(request);
    pending.reject(error);
  }
  for (const [request, pending] of pendingFileIconRequests) {
    pendingFileIconRequests.delete(request);
    pending.reject(error);
  }
  for (const [request, pending] of pendingUserTaskRequests) {
    pendingUserTaskRequests.delete(request);
    pending.reject(error);
  }
  for (const [request, pending] of pendingGlobalShortcutRequests) {
    pendingGlobalShortcutRequests.delete(request);
    pending.reject(error);
  }
  for (const [request, pending] of pendingAppServiceRequests) {
    pendingAppServiceRequests.delete(request);
    pending.reject(error);
  }
  globalShortcutRegistrations.clear();
  globalShortcutIds.clear();
  rejectPendingTrayRequests(error);
  screenListeners.clear();
  appearanceListeners.clear();
  keyboardListeners.clear();
  powerListeners.clear();
  notificationListeners.clear();
  systemPreferencesListeners.clear();
  menuCallbacks.clear();
  applicationMenuActionIds.clear();
  disposeDockMenu?.();
  disposeDockMenu = undefined;
  windowStateListeners.clear();
}

export function dispatchSystemEvent(
  event: binding.NativeEvent,
  findWindow: (id: number) => Window | undefined,
): boolean {
  if (dispatchTrayEvent(event)) return true;
  if (event.kind === "shell") {
    const pending = pendingShellRequests.get(event.target);
    if (!pending) return true;
    pendingShellRequests.delete(event.target);
    if (event.error !== undefined) pending.reject(new Error(event.error));
    else pending.resolve();
    return true;
  }
  if (event.kind === "global-shortcut-operation") {
    const pending = pendingGlobalShortcutRequests.get(event.target);
    if (!pending) return true;
    pendingGlobalShortcutRequests.delete(event.target);
    if (event.error !== undefined) pending.reject(new Error(event.error));
    else pending.resolve();
    return true;
  }
  if (event.kind === "global-shortcut") {
    globalShortcutRegistrations.get(event.target)?.listener();
    return true;
  }
  if (event.kind === "power-event") {
    const value = parsePowerEvent(event.value);
    if (!value) return true;
    for (const listener of powerListeners.get(value.type) ?? []) listener(value);
    return true;
  }
  if (event.kind === "system-preferences-change") {
    void SystemPreferences.getCurrent()
      .then((preferences) => {
        for (const listener of systemPreferencesListeners) listener(preferences);
      })
      .catch(() => {});
    return true;
  }
  if (event.kind === "notification-response") {
    try {
      const value = JSON.parse(event.value ?? "{}") as {
        tag?: unknown;
        actionId?: unknown;
        reply?: unknown;
      };
      if (typeof value.tag !== "string") return true;
      const response: NotificationResponse = { tag: value.tag };
      if (typeof value.actionId === "string") response.actionId = value.actionId;
      if (typeof value.reply === "string") response.reply = value.reply;
      for (const listener of notificationListeners) listener(response);
    } catch {
      // The app-level dispatcher follows the same fail-closed parsing contract.
    }
    return notificationListeners.size > 0;
  }
  if (event.kind === "notification-permission") {
    const pending = pendingNotificationPermissionRequests.get(event.target);
    if (!pending) return true;
    pendingNotificationPermissionRequests.delete(event.target);
    if (event.error !== undefined) pending.reject(new Error(event.error));
    else if (
      event.value === "not-determined" ||
      event.value === "granted" ||
      event.value === "denied" ||
      event.value === "unsupported"
    ) {
      pending.resolve(event.value);
    } else {
      pending.reject(new Error("the native notification permission response was invalid"));
    }
    return true;
  }
  if (event.kind === "file-icon") {
    const pending = pendingFileIconRequests.get(event.target);
    if (!pending) return true;
    pendingFileIconRequests.delete(event.target);
    if (event.error !== undefined) pending.reject(new Error(event.error));
    else if (event.data !== undefined && event.width !== undefined && event.height !== undefined) {
      pending.resolve({
        data: Uint8Array.from(event.data),
        width: event.width,
        height: event.height,
      });
    } else {
      pending.reject(new Error("the native file-icon response was invalid"));
    }
    return true;
  }
  if (event.kind === "user-tasks") {
    const pending = pendingUserTaskRequests.get(event.target);
    if (!pending) return true;
    pendingUserTaskRequests.delete(event.target);
    if (event.error !== undefined) pending.reject(new Error(event.error));
    else pending.resolve();
    return true;
  }
  if (event.kind === "menu-action") {
    menuCallbacks.get(event.target)?.();
    return true;
  }
  if (event.kind === "app-service" || event.kind === "popup-menu") {
    const pending = pendingAppServiceRequests.get(event.target);
    if (!pending) return true;
    pendingAppServiceRequests.delete(event.target);
    if (event.error !== undefined) pending.reject(new Error(event.error));
    else pending.resolve(event.value);
    return true;
  }
  if (event.kind === "screen-change") {
    void Screen.getAllDisplays()
      .then((displays) => {
        for (const listener of screenListeners) listener(displays);
      })
      .catch(() => {});
    return true;
  }
  if (event.kind === "keyboard-layout-change") {
    void Keyboard.getLayout()
      .then((layout) => {
        for (const listener of keyboardListeners) listener(layout);
      })
      .catch(() => {});
    return true;
  }
  if (event.kind === "appearance-change") {
    const window = findWindow(event.window);
    if (!window) return true;
    const appearance = event.value === "dark" ? "dark" : "light";
    for (const listener of appearanceListeners) listener(appearance, window);
    return true;
  }
  if (event.kind === "window-state-change") {
    const window = findWindow(event.window);
    if (!window) return true;
    const listeners = windowStateListeners.get(event.window);
    if (!listeners?.size) return true;
    void getNativeWindowState(window)
      .then((state) => {
        for (const listener of listeners) listener(state);
      })
      .catch(() => {});
    return true;
  }
  return false;
}

function parsePowerEvent(value: string | undefined): PowerEvent | undefined {
  try {
    const event = JSON.parse(value ?? "{}") as Record<string, unknown>;
    switch (event.type) {
      case "suspend":
      case "resume":
      case "lock-screen":
      case "unlock-screen":
      case "shutdown-requested":
        return { type: event.type };
      case "power-source-changed":
        if (event.source === "ac" || event.source === "battery" || event.source === "unknown") {
          return { type: event.type, source: event.source };
        }
        return undefined;
      case "thermal-state-changed":
        if (
          event.state === "unknown" ||
          event.state === "nominal" ||
          event.state === "fair" ||
          event.state === "serious" ||
          event.state === "critical"
        ) {
          return { type: event.type, state: event.state };
        }
        return undefined;
      case "low-power-mode-changed":
        return typeof event.enabled === "boolean"
          ? { type: event.type, enabled: event.enabled }
          : undefined;
      case "cpu-speed-limit-changed":
        return typeof event.percent === "number" && Number.isFinite(event.percent)
          ? { type: event.type, percent: event.percent }
          : undefined;
      default:
        return undefined;
    }
  } catch {
    return undefined;
  }
}

/** Persistable window geometry and display identity. */
export interface WindowRestoreState {
  x: number;
  y: number;
  width: number;
  height: number;
  maximized: boolean;
  fullscreen: boolean;
  /** Process-level display identifier captured with the bounds, when one was known. */
  displayId?: string;
  /** Stable physical display identity, when the platform exposes one. */
  displayUuid?: string;
  scaleFactor: number;
}

/** How the application appears in the Dock and application switcher. */
export type ActivationPolicy = "regular" | "accessory" | "prohibited";
/** Urgency of a request for the user's attention. */
export type DockAttentionType = "critical" | "informational";

const pendingAppServiceRequests = new Map<
  number,
  { resolve: (value: string | undefined) => void; reject: (error: Error) => void }
>();
let nextAppServiceRequest = 1;
/** The last Dock visibility JavaScript asked for; the platform exposes no query. */
let dockVisible = true;

/** Read one window's persistable geometry and display identity. */
export async function getNativeWindowRestoreState(window: Window): Promise<WindowRestoreState> {
  const { context: current, window: resolved } = windowContext(window);
  const state = await binding.getHostedWindowRestoreState(current.appId, resolved.nativeId);
  const restore: WindowRestoreState = {
    x: state.x,
    y: state.y,
    width: state.width,
    height: state.height,
    maximized: state.maximized,
    fullscreen: state.fullscreen,
    scaleFactor: state.scaleFactor,
  };
  if (state.displayId !== undefined) restore.displayId = state.displayId;
  if (state.displayUuid !== undefined) restore.displayUuid = state.displayUuid;
  return restore;
}

/** Convert a persisted restore state into the shape the native window options accept. */
export function nativeWindowRestoreState(
  state: WindowRestoreState,
): binding.NativeWindowRestoreState {
  const native: binding.NativeWindowRestoreState = {
    x: state.x,
    y: state.y,
    width: state.width,
    height: state.height,
    maximized: state.maximized,
    fullscreen: state.fullscreen,
    scaleFactor: state.scaleFactor,
  };
  if (state.displayId !== undefined) native.displayId = state.displayId;
  if (state.displayUuid !== undefined) native.displayUuid = state.displayUuid;
  return native;
}

/**
 * Start one application-shell service and resolve when the operating system answers.
 *
 * The native side never blocks the application thread; the outcome returns as an `app-service`
 * event carrying this request id.
 */
function appServiceRequest(action: string, value?: string): Promise<string | undefined> {
  try {
    const current = context();
    const request = allocateBoundedId(nextAppServiceRequest, pendingAppServiceRequests);
    nextAppServiceRequest = request >= 0xffff_ffff ? 1 : request + 1;
    return new Promise<string | undefined>((resolve, reject) => {
      pendingAppServiceRequests.set(request, { resolve, reject });
      try {
        {
          watchHostedRequestAcceptance(
            binding.performHostedAppService(current.appId, request, action, value),
            request,
            pendingAppServiceRequests,
          );
        }
      } catch (error) {
        pendingAppServiceRequests.delete(request);
        reject(error instanceof Error ? error : new Error(String(error)));
      }
    });
  } catch (error) {
    return Promise.reject(error instanceof Error ? error : new Error(String(error)));
  }
}

/** Apply one fire-and-forget application-shell mutation. */
function appMutation(action: string, value?: string): void {
  const current = context();
  binding.performHostedAppMutation(current.appId, action, value);
}

/** Change how the application appears in the Dock and application switcher. */
export async function setNativeActivationPolicy(policy: ActivationPolicy): Promise<void> {
  await appServiceRequest("set-activation-policy", policy);
  if (policy !== "regular") dockVisible = false;
}

/** Bring the application forward, optionally stealing focus from the frontmost application. */
export function activateNativeApplication(steal: boolean): void {
  appMutation("activate", steal ? "true" : "false");
}

export function hideNativeApplication(): void {
  appMutation("hide");
}

export function unhideNativeApplication(): void {
  appMutation("unhide");
}

/** Route keystrokes straight to this process, bypassing input monitoring. */
export function setNativeSecureKeyboardEntry(enabled: boolean): void {
  appMutation("set-secure-keyboard-entry", enabled ? "true" : "false");
}

/** Bounce the Dock tile and resolve with the identifier that cancels a critical bounce. */
export async function requestNativeDockAttention(
  type: DockAttentionType = "informational",
): Promise<number> {
  const value = await appServiceRequest("request-dock-attention", type);
  const id = Number(value);
  return Number.isFinite(id) ? id : 0;
}

export function cancelNativeDockAttention(id: number): void {
  if (!Number.isInteger(id)) {
    throw new TypeError("a dock attention request id must be an integer");
  }
  appMutation("cancel-dock-attention", String(id));
}

export async function setNativeDockVisible(visible: boolean): Promise<void> {
  await appServiceRequest("set-dock-visible", visible ? "true" : "false");
  dockVisible = visible;
}

/** The last Dock visibility this process asked for. */
export function nativeDockVisible(): boolean {
  return dockVisible;
}

/** Whether this process can relocate its bundle into an `/Applications` directory. */
export async function getNativeApplicationsFolderSupport(): Promise<{
  supported: boolean;
  alreadyInstalled: boolean;
}> {
  const current = context();
  const support = await binding.getHostedApplicationsFolderSupport(current.appId);
  return { supported: support.supported, alreadyInstalled: support.alreadyInstalled };
}

/** Move the running application bundle into `/Applications`. */
export async function moveNativeApplicationToApplicationsFolder(): Promise<boolean> {
  return (await appServiceRequest("move-to-applications-folder")) === "true";
}

/** Whether this process is running from an installed application bundle. */
export function nativeApplicationPackaged(): boolean {
  return binding.isApplicationPackaged();
}

/** Exit the application with an explicit process exit code. */
export async function exitNativeAppWithCode(code: number): Promise<boolean> {
  if (!Number.isInteger(code)) {
    throw new TypeError("a process exit code must be an integer");
  }
  const current = context();
  return await binding.exitHostedAppWithCode(current.appId, code);
}

const windowMenuDisposers = new Map<number, () => void>();

/** Replace or clear one window's native menu declaration. */
export function setNativeWindowMenu(
  window: Window,
  definitions: readonly MenuDefinition[] | null,
): void {
  const { context: current, window: resolved } = windowContext(window);
  const serialized = definitions === null ? undefined : serializeNativeMenu(definitions);
  {
    binding.performHostedWindowAction(
      current.appId,
      resolved.nativeId,
      "set-menu",
      serialized?.json,
    );
  }
  windowMenuDisposers.get(resolved.nativeId)?.();
  const dispose = serialized?.install();
  if (dispose) windowMenuDisposers.set(resolved.nativeId, dispose);
  else windowMenuDisposers.delete(resolved.nativeId);
}

/** @internal Release the menu callbacks a closed window owned. */
export function releaseNativeWindowMenu(nativeId: number): void {
  windowMenuDisposers.get(nativeId)?.();
  windowMenuDisposers.delete(nativeId);
}

/** Where a native popup menu opens, in window-local logical pixels from the top-left. */
export interface PopupMenuOptions {
  window?: Window;
  x?: number;
  y?: number;
}

/** Present a native popup menu and resolve once it closes. */
function showNativePopupMenu(
  items: readonly MenuItem[],
  options: PopupMenuOptions = {},
): Promise<void> {
  try {
    const { context: current, window: resolved } = windowContext(options.window);
    if ((options.x === undefined) !== (options.y === undefined)) {
      throw new TypeError("a native popup menu position requires both x and y");
    }
    if (
      (options.x !== undefined && !Number.isFinite(options.x)) ||
      (options.y !== undefined && !Number.isFinite(options.y))
    ) {
      throw new TypeError("a native popup menu position must be finite");
    }
    const serialized = serializeNativeMenu([{ label: "Popup", items: [...items] }]);
    const request = allocateBoundedId(nextAppServiceRequest, pendingAppServiceRequests);
    nextAppServiceRequest = request >= 0xffff_ffff ? 1 : request + 1;
    return new Promise<void>((resolve, reject) => {
      const dispose = serialized.install();
      pendingAppServiceRequests.set(request, {
        resolve: () => {
          dispose();
          resolve();
        },
        reject: (error) => {
          dispose();
          reject(error);
        },
      });
      try {
        {
          watchHostedRequestAcceptance(
            binding.showHostedWindowPopupMenu(
              current.appId,
              request,
              resolved.nativeId,
              serialized.json,
              options.x,
              options.y,
            ),
            request,
            pendingAppServiceRequests,
          );
        }
      } catch (error) {
        pendingAppServiceRequests.delete(request);
        dispose();
        reject(error instanceof Error ? error : new Error(String(error)));
      }
    });
  } catch (error) {
    return Promise.reject(error instanceof Error ? error : new Error(String(error)));
  }
}

import * as binding from "./binding.ts";
import { Buffer } from "node:buffer";
import {
  type AlertDialogOptions,
  type OpenDialogOptions,
  type OpenDialogResult,
  type SaveDialogOptions,
  type SaveDialogResult,
  normalizeAlertDialogOptions,
  normalizeOpenDialogOptions,
  normalizeSaveDialogOptions,
} from "./dialog.ts";
import {
  MutationBatch,
  NativeNodeTag,
  PropertyCode,
  PROTOCOL_VERSION,
  ROOT_NODE_ID,
  type NativePropertyValue,
} from "./protocol.ts";
import {
  NativeNode,
  QuickGuiEvent,
  cleanupNativeNodes,
  createNativeElement,
  createNativeSentinel,
  createNativeText,
  getNativeFirstChild,
  getNativeNextSibling,
  getNativeParent,
  insertNativeNode,
  isNativeText,
  parseColor,
  removeNativeNode,
  replaceNativeText,
  setNativeEventListener,
  setNativeProperty,
  type ColorValue,
  type NativeElementName,
  type NativeEventListener,
  type NativeEventType,
} from "./native-tree.ts";
import {
  activateNativeApplication,
  cancelNativeDockAttention,
  configureSystemContext,
  Desktop,
  dispatchSystemEvent,
  exitNativeAppWithCode,
  getNativeApplicationsFolderSupport,
  getNativeWindowRestoreState,
  getNativeWindowState,
  hideNativeApplication,
  moveNativeApplicationToApplicationsFolder,
  nativeApplicationPackaged,
  nativeDockVisible,
  nativeImageSource,
  nativeWindowRestoreState,
  onNativeWindowStateChange,
  performNativeWindowAction,
  performNativeWindowImageAction,
  rejectPendingSystemRequests,
  releaseNativeWindowMenu,
  removeNativeWindowStateListeners,
  requestNativeDockAttention,
  requestNativeQuit,
  serializeNativeMenu,
  setNativeActivationPolicy,
  setNativeDockVisible,
  setNativeQuitInterception,
  setNativeSecureKeyboardEntry,
  setNativeWindowMenu,
  unhideNativeApplication,
  type ActivationPolicy,
  type DockAttentionType,
  type WindowRestoreState,
} from "./system.ts";
import { type SecondInstanceEvent, urlsFromArguments } from "./single-instance.ts";

export { NativeNodeTag, PropertyCode } from "./protocol.ts";
export {
  MAX_COLLECTION_JSON_BYTES,
  MAX_COMPONENT_ITEMS,
  MAX_COMPONENT_JSON_BYTES,
  MAX_COMPONENT_VALUE_BYTES,
  MAX_COMPONENT_VALUES,
  MAX_DECLARED_OPTIONS,
  MAX_DECLARED_TREE_NODES,
  MAX_DRAG_JSON_BYTES,
  MAX_FILTERS_PER_ELEMENT,
  MAX_GRADIENT_STOPS,
  MAX_KEYMAP_JSON_BYTES,
  MAX_MENU_JSON_BYTES,
  MAX_MENU_HOVER_DELAY_MS,
  MAX_MENU_ITEMS,
  MAX_MENU_LINK_BYTES,
  MAX_AVATAR_FALLBACK_DELAY_MS,
  MAX_CHECKBOX_GROUP_VALUES,
  MAX_DRAWER_SNAP_POINTS,
  MAX_MENUBAR_MENUS,
  MAX_NAVIGATION_MENU_DELAY_MS,
  MAX_NAVIGATION_MENU_ITEMS,
  MAX_OPTIONS_JSON_BYTES,
  MAX_OTP_LENGTH,
  MAX_PREVIEW_CARD_DELAY_MS,
  MAX_SCROLL_AREA_OVERFLOW_THRESHOLD,
  MAX_SELECT_VALUES,
  MAX_COMBOBOX_VALUES,
  MAX_GROUP_STYLES_PER_ELEMENT,
  MAX_HOVER_GROUP_NAME_BYTES,
  MAX_STATE_STYLE_JSON_BYTES,
  MAX_STYLE_DECLARATION_BYTES,
  MAX_TABLE_COLUMNS,
  MAX_TABLE_ROWS,
  MAX_TOASTS,
  MAX_TOOLTIP_TEXT_BYTES,
  NativePart,
  type NativePartName,
} from "./protocol.ts";
export {
  NativeNode,
  QuickGuiEvent,
  cleanupNativeNodes,
  createNativeElement,
  createNativeSentinel,
  createNativeText,
  getNativeFirstChild,
  getNativeNextSibling,
  getNativeParent,
  insertNativeNode,
  isNativeText,
  parseColor,
  removeNativeNode,
  replaceNativeText,
  setNativeEventListener,
  setNativeProperty,
} from "./native-tree.ts";
export type {
  ColorValue,
  NativeElementName,
  NativeEventListener,
  NativeEventType,
} from "./native-tree.ts";
export type {
  AlertDialogButton,
  AlertDialogButtonRole,
  AlertDialogLevel,
  AlertDialogOptions,
  FileDialogFilter,
  OpenDialogOptions,
  OpenDialogProperty,
  OpenDialogResult,
  SaveDialogOptions,
  SaveDialogResult,
} from "./dialog.ts";
export {
  Appearance,
  Clipboard,
  Desktop,
  GlobalShortcut,
  Keyboard,
  Menu,
  Notifications,
  Permissions,
  PowerMonitor,
  PowerAssertion,
  Screen,
  SpellChecker,
  SystemPreferences,
  Shell,
  Tray,
  TrayIcon,
} from "./system.ts";
export { AutoStart, SecureStorage } from "./integrations.ts";
export { CrashReporter, Metrics } from "./integrations.ts";
export type {
  CrashBacktracePolicy,
  CrashKind,
  CrashLocation,
  CrashReport,
  CrashReporterOptions,
  CrashUploadSummary,
  CpuSampler,
  CpuUsage,
  ProcessMetrics,
  SystemMemory,
} from "./integrations.ts";
export { DeepLink } from "./single-instance.ts";
export type { SecondInstanceEvent } from "./single-instance.ts";
export { Router } from "./routing.ts";
export type {
  RouteDefinition,
  RouteLocation,
  RouteMatch,
  RouteValue,
  RouterState,
} from "./routing.ts";
export type {
  AutoStartMode,
  AutoStartOptions,
  ProtocolRegistrationOptions,
} from "./integrations.ts";
export type {
  AppearanceMode,
  AppearancePreference,
  ClipboardEntry,
  ClipboardBookmarkEntry,
  ClipboardDataEntry,
  ClipboardFilesEntry,
  ClipboardImageEntry,
  ClipboardItem,
  ClipboardTextEntry,
  Display,
  DesktopIntegrationSupport,
  AboutPanelOptions,
  UserTask,
  NativeImage,
  GlobalShortcutListener,
  KeyboardLayout,
  MenuActionItem,
  MenuDefinition,
  MenuItem,
  MenuRole,
  MenuItemMark,
  MenuRoleItem,
  MenuSystemItem,
  MenuSeparatorItem,
  MenuServicesItem,
  MenuSubmenuItem,
  NotificationAction,
  NotificationOptions,
  NotificationAttachment,
  NotificationPermissionStatus,
  NotificationResponse,
  PermissionKind,
  PermissionStatus,
  PowerEvent,
  PowerEventType,
  PowerAssertionKind,
  PowerSource,
  PowerState,
  BatteryStatus,
  ThermalState,
  SessionState,
  IdleState,
  SystemColor,
  SystemPreferencesSnapshot,
  Rectangle,
  FrameMetrics,
  TrayEvent,
  TrayEventType,
  TrayIconOptions,
  TrayIconSource,
  TrayMenuActionItem,
  TrayMenuItem,
  TrayMenuSeparatorItem,
  TrayMenuSubmenuItem,
  WindowState,
  WindowRestoreState,
  WindowBackgroundAppearance,
  ElectronWindowLevel,
  ActivationPolicy,
  DockAttentionType,
  PopupMenuOptions,
  MacOSVibrancy,
  MacOSVisualEffectState,
  WindowKind,
  WindowLevel,
  CursorGrabMode,
  TaskbarProgressState,
  ImageSource,
} from "./system.ts";
import type {
  AppearancePreference,
  CursorGrabMode,
  ElectronWindowLevel,
  ImageSource,
  KeyboardLayout,
  MenuDefinition,
  NotificationResponse,
  TaskbarProgressState,
  WindowBackgroundAppearance,
  MacOSVibrancy,
  MacOSVisualEffectState,
  WindowKind,
  WindowLevel,
  WindowState,
} from "./system.ts";

export type WindowCloseListener = (window: Window) => void;
export type WindowCloseRequestListener = (event: { window: Window }) => void;

export interface WindowEventMap {
  /** The window has closed and released its retained tree. */
  closed: { window: Window };
  /**
   * A native close was requested and held.
   *
   * Complete it with `window.close()` or `window.destroy()`, or ignore it to keep the window.
   */
  closeRequested: { window: Window };
  /** The window was minimized to the Dock or taskbar. */
  minimize: { window: Window };
  /** The window returned from the Dock or taskbar. */
  restore: { window: Window };
  /** The window entered the platform's maximized/zoomed state. */
  maximize: { window: Window };
  /** The window left the platform's maximized/zoomed state. */
  unmaximize: { window: Window };
  enterFullScreen: { window: Window };
  leaveFullScreen: { window: Window };
  /**
   * The first frame reached the screen.
   *
   * A window created with `visible: false` can be shown here without a flash of empty chrome.
   */
  readyToShow: { window: Window };
  /** The compositor started or stopped hiding this window's contents. */
  occlusionChange: { window: Window; occluded: boolean };
  /** The effective native stacking level changed. */
  levelChange: { window: Window; level: WindowLevel };
  /**
   * The window manager proposed a new inner size.
   *
   * This is a notification: the narrowing itself is declared ahead with
   * `window.setResizePolicy()`, because the core must answer the platform synchronously.
   */
  willResize: { window: Window; size: Size };
  /** The window manager proposed a new position. See `window.setMovePolicy()`. */
  willMove: { window: Window; position: Point };
  resize: { window: Window; size: Size };
  move: { window: Window; position: Point };
  focus: { window: Window };
  blur: { window: Window };
  appearanceChange: { window: Window; appearance: "light" | "dark" };
}
export type WindowRenderer = (window: Window) => () => void;
export type PopoverPlacement =
  | "top-start"
  | "top"
  | "top-end"
  | "bottom-start"
  | "bottom"
  | "bottom-end"
  | "left-start"
  | "left"
  | "left-end"
  | "right-start"
  | "right"
  | "right-end";

export type PerformanceProfile = "low-power" | "balanced" | "high-performance";
export type InitialWindowState = "normal" | "maximized" | "fullscreen";

/**
 * The narrowing applied when the window manager proposes a new inner size.
 *
 * The constraint is declared ahead because the core answers the platform synchronously; the
 * `willResize` event is only a notification of what was proposed.
 */
export interface WindowResizePolicy {
  /** Content `width / height` the resize is snapped to. */
  aspectRatio?: number;
  minimum?: Size;
  maximum?: Size;
  /** Grid step applied to the proposed inner size before the ratio and bounds. */
  snap?: Size;
}

/** The narrowing applied when the window manager proposes a new position. */
export interface WindowMovePolicy {
  /** Keep the window's origin inside the work area of the display that contains it. */
  keepOnScreen?: boolean;
}
export interface Size {
  width: number;
  height: number;
}
export interface Point {
  x: number;
  y: number;
}
export interface WindowBounds extends Point, Size {
  state?: InitialWindowState;
}

export interface WindowOptions {
  renderer: WindowRenderer;
  title?: string;
  width?: number;
  height?: number;
  position?: Point;
  initialState?: InitialWindowState;
  displayId?: string;
  /** Pass `null` to remove the core's default minimum window size. */
  minimumSize?: Size | null;
  minimumWidth?: number;
  minimumHeight?: number;
  maximumSize?: Size;
  maximumWidth?: number;
  maximumHeight?: number;
  representedFile?: string;
  documentEdited?: boolean;
  tabbingIdentifier?: string;
  background?: ColorValue;
  backgroundAppearance?: WindowBackgroundAppearance;
  /** Electron-compatible macOS `NSVisualEffectView` semantic material. */
  vibrancy?: MacOSVibrancy;
  /** Defaults to `followWindow`. Used when `vibrancy` is enabled. */
  visualEffectState?: MacOSVisualEffectState;
  performanceProfile?: PerformanceProfile;
  appearance?: AppearancePreference;
  titleBarStyle?: "default" | "hidden" | "hiddenInset";
  kind?: WindowKind;
  focus?: boolean;
  focusable?: boolean;
  visible?: boolean;
  movable?: boolean;
  resizable?: boolean;
  minimizable?: boolean;
  maximizable?: boolean;
  closable?: boolean;
  decorated?: boolean;
  shadow?: boolean;
  contentProtected?: boolean;
  windowLevel?: WindowLevel | "automatic";
  skipTaskbar?: boolean;
  visibleOnAllWorkspaces?: boolean;
  opacity?: number;
  icon?: ImageSource;
  taskbarProgress?: { state: TaskbarProgressState; progress: number };
  taskbarOverlay?: { icon: ImageSource; description: string };
  cursorVisible?: boolean;
  cursorGrab?: CursorGrabMode;
  cursorHitTest?: boolean;
  cursorPosition?: Point;
  menu?: readonly MenuDefinition[];
  /**
   * Persisted geometry and display identity captured with `window.getRestoreState()`.
   *
   * The core re-validates every field, so a stale or hostile value can never place a window off
   * every connected display.
   */
  restoreState?: WindowRestoreState;
  lineScrollPixels?: number;
  keySequenceTimeoutMs?: number;
  reduceMotion?: boolean;
  trafficLightPosition?: { x: number; y: number };
  transparent?: boolean;
  blur?: boolean;
  /** Open this window as a system popover anchored to the mounted node. */
  anchor?: NativeNode;
  placement?: PopoverPlacement;
  gap?: number;
  offset?: { x: number; y: number };
  viewportMargin?: number;
  dismissOnEscape?: boolean;
  dismissOnPointerOutside?: boolean;
  grab?: boolean;
  acceptsKeyFocus?: boolean;
}

export type QuitMode = "default" | "last-window-closed" | "explicit";

export interface AppPathOverrides {
  resourceDir?: string;
  configDir?: string;
  dataDir?: string;
  localDataDir?: string;
  cacheDir?: string;
  logDir?: string;
  runtimeDir?: string;
  tempDir?: string;
}

export interface AppOptions {
  name?: string;
  version?: string;
  identifier?: string;
  paths?: AppPathOverrides;
  quitMode?: QuitMode;
  /** OpenType font files registered by the Rust core before the first window is created. */
  fonts?: readonly string[];
}

export interface RelaunchOptions {
  executable?: string;
  /** Omit to preserve current arguments; pass `null` to relaunch without arguments. */
  arguments?: readonly string[] | null;
  workingDirectory?: string;
}

export interface AppInfo {
  name: string;
  version: string;
  identifier: string;
}

export interface AppPaths {
  executable: string;
  executableDir: string;
  resourceDir: string;
  homeDir?: string;
  configDir?: string;
  dataDir?: string;
  localDataDir?: string;
  cacheDir?: string;
  logDir?: string;
  runtimeDir?: string;
  tempDir: string;
  audioDir?: string;
  desktopDir?: string;
  documentDir?: string;
  downloadDir?: string;
  pictureDir?: string;
  videoDir?: string;
}

export interface SystemInfo {
  operatingSystem:
    | "macos"
    | "windows"
    | "linux"
    | "freebsd"
    | "dragonfly"
    | "netbsd"
    | "openbsd"
    | "android"
    | "ios"
    | "wasm"
    | "other";
  family: "unix" | "windows" | "wasm" | "other";
  name: string;
  version?: string;
  edition?: string;
  codename?: string;
  architecture: string;
  bitness: "32" | "64" | "unknown";
  hostname?: string;
  locale?: string;
  preferredLanguages: readonly string[];
  languagesTruncated: boolean;
}

/** Why the operating system or application began an orderly shutdown. */
export type QuitReason = "explicit" | "relaunch" | "last-window-closed" | "operating-system";

export interface AppEventMap {
  ready: undefined;
  quit: { exitCode: number };
  /**
   * The first preventable quit phase.
   *
   * Registering a listener declares quit interception, so the native shutdown is held and the
   * application stays alive until JavaScript completes it with `app.quit({ force: true })` or
   * `app.exit(code)`.
   */
  beforeQuit: { reason: QuitReason };
  /** The final quit phase. Purely a notification; the shutdown already proceeds. */
  willQuit: { reason: QuitReason };
  openUrls: readonly string[];
  reopen: { hasVisibleWindows: boolean };
  /** The application became the frontmost one. */
  activate: undefined;
  /** Another application became frontmost. */
  deactivate: undefined;
  systemWake: undefined;
  keyboardLayoutChange: KeyboardLayout;
  notificationResponse: NotificationResponse;
  secondInstance: SecondInstanceEvent;
}

type DialogEventKind = "alert-dialog" | "open-dialog" | "save-dialog";

type PendingDialog = {
  kind: DialogEventKind;
  window: Window | undefined;
  complete: (event: binding.NativeEvent) => void;
  reject: (reason: Error) => void;
};

let activeApp: App | undefined;
let currentWindow: Window | undefined;
const hostedRuntime = true;

function nativeAppOptions(options: AppOptions): binding.NativeAppOptions {
  const native: binding.NativeAppOptions = {};
  if (options.name !== undefined) native.name = options.name;
  if (options.version !== undefined) native.version = options.version;
  if (options.identifier !== undefined) native.identifier = options.identifier;
  if (options.quitMode !== undefined) native.quitMode = options.quitMode;
  if (options.fonts !== undefined) {
    native.fonts = [...options.fonts];
  }
  const paths = options.paths;
  if (paths?.resourceDir !== undefined) native.resourceDir = paths.resourceDir;
  if (paths?.configDir !== undefined) native.configDir = paths.configDir;
  if (paths?.dataDir !== undefined) native.dataDir = paths.dataDir;
  if (paths?.localDataDir !== undefined) native.localDataDir = paths.localDataDir;
  if (paths?.cacheDir !== undefined) native.cacheDir = paths.cacheDir;
  if (paths?.logDir !== undefined) native.logDir = paths.logDir;
  if (paths?.runtimeDir !== undefined) native.runtimeDir = paths.runtimeDir;
  if (paths?.tempDir !== undefined) native.tempDir = paths.tempDir;
  return native;
}

const embeddedAppOptions = (
  globalThis as typeof globalThis & { __QUICKGUI_APP_OPTIONS__?: AppOptions }
).__QUICKGUI_APP_OPTIONS__;

function withCurrentWindow<T>(window: Window, callback: () => T): T {
  const previous = currentWindow;
  currentWindow = window;
  try {
    return callback();
  } finally {
    currentWindow = previous;
  }
}

configureSystemContext(
  () => {
    const app = activeApp;
    if (!app) throw new Error("create a QuickGUI App before using a native system API");
    app._assertReady();
    return { appId: app.nativeId, hosted: hostedRuntime };
  },
  (window) => {
    const app = window?.app ?? activeApp;
    if (!app) throw new Error("create a QuickGUI App before using a native window API");
    const resolved = window ?? app.windows.values().next().value;
    if (!resolved || resolved.closed || app.windows.get(resolved.nativeId) !== resolved) {
      throw new Error("a native window API requires an open QuickGUI Window");
    }
    app._assertReady();
    return {
      context: { appId: app.nativeId, hosted: hostedRuntime },
      window: resolved,
    };
  },
);

const nativeProtocolVersion = binding.protocolVersion();
if (nativeProtocolVersion !== PROTOCOL_VERSION) {
  throw new Error(
    `QuickGUI native protocol mismatch: JavaScript uses ${PROTOCOL_VERSION}, binding uses ${nativeProtocolVersion}. Reinstall or rebuild @quickgui/native.`,
  );
}

class App {
  readonly nativeId: number;
  readonly windows = new Map<number, Window>();
  #running = false;
  #ready = false;
  #destroyed = false;
  readonly #readyPromise: Promise<void>;
  #nextDialogRequest = 1;
  readonly #pendingDialogs = new Map<number, PendingDialog>();
  #singleInstanceIdentifier: string | undefined;
  #quitIntercepting = false;
  #requestedExitCode: number | undefined;
  readonly #appEventListeners = new Map<keyof AppEventMap, Set<(payload: unknown) => unknown>>();

  constructor() {
    if (activeApp) {
      throw new Error("a QuickGUI App is already active in this JavaScript isolate");
    }
    const initialOptions = embeddedAppOptions ? nativeAppOptions(embeddedAppOptions) : undefined;
    this.nativeId = binding.createHostedApp(initialOptions);
    activeApp = this;
    this.#readyPromise = Promise.resolve().then(async () => {
      this.#assertAlive();
      await binding.prepareHostedApp(this.nativeId);

      this.#ready = true;
      this.#syncQuitInterception();
      this.#emitAppEvent("ready", undefined);
    });
    void this.#readyPromise.catch(() => {});
  }

  isReady(): boolean {
    this.#assertAlive();
    return this.#ready;
  }

  whenReady(): Promise<void> {
    return this.#readyPromise;
  }

  command<T = unknown>(value: Record<string, unknown>): Promise<T> {
    this._assertReady();
    return binding.command<T>(this.nativeId, value);
  }

  /** Patch core-owned identity, paths, and quit policy before the first readiness turn. */
  async configure(options: AppOptions): Promise<void> {
    this.#assertAlive();
    const native = nativeAppOptions(options);
    await binding.configureHostedApp(this.nativeId, native);
  }

  async getInfo(): Promise<AppInfo | undefined> {
    this._assertReady();
    const info = await binding.getHostedAppInfo(this.nativeId);
    return info ? { ...info } : undefined;
  }

  async getPaths(): Promise<AppPaths | undefined> {
    this._assertReady();
    const paths = await binding.getHostedAppPaths(this.nativeId);
    if (!paths) return undefined;
    const result: AppPaths = {
      executable: paths.executable,
      executableDir: paths.executableDir,
      resourceDir: paths.resourceDir,
      tempDir: paths.tempDir,
    };
    if (paths.homeDir !== undefined) result.homeDir = paths.homeDir;
    if (paths.configDir !== undefined) result.configDir = paths.configDir;
    if (paths.dataDir !== undefined) result.dataDir = paths.dataDir;
    if (paths.localDataDir !== undefined) result.localDataDir = paths.localDataDir;
    if (paths.cacheDir !== undefined) result.cacheDir = paths.cacheDir;
    if (paths.logDir !== undefined) result.logDir = paths.logDir;
    if (paths.runtimeDir !== undefined) result.runtimeDir = paths.runtimeDir;
    if (paths.audioDir !== undefined) result.audioDir = paths.audioDir;
    if (paths.desktopDir !== undefined) result.desktopDir = paths.desktopDir;
    if (paths.documentDir !== undefined) result.documentDir = paths.documentDir;
    if (paths.downloadDir !== undefined) result.downloadDir = paths.downloadDir;
    if (paths.pictureDir !== undefined) result.pictureDir = paths.pictureDir;
    if (paths.videoDir !== undefined) result.videoDir = paths.videoDir;
    return result;
  }

  async getSystemInfo(): Promise<SystemInfo> {
    this._assertReady();
    const info = await binding.getHostedSystemInfo(this.nativeId);
    const result: SystemInfo = {
      operatingSystem: info.operatingSystem as SystemInfo["operatingSystem"],
      family: info.family as SystemInfo["family"],
      name: info.name,
      architecture: info.architecture,
      bitness: info.bitness as SystemInfo["bitness"],
      preferredLanguages: [...info.preferredLanguages],
      languagesTruncated: info.languagesTruncated,
    };
    if (info.version !== undefined) result.version = info.version;
    if (info.edition !== undefined) result.edition = info.edition;
    if (info.codename !== undefined) result.codename = info.codename;
    if (info.hostname !== undefined) result.hostname = info.hostname;
    if (info.locale !== undefined) result.locale = info.locale;
    return result;
  }

  getWindows(): readonly Window[] {
    this._assertReady();
    return [...this.windows.values()];
  }

  async getActiveWindow(): Promise<Window | undefined> {
    this._assertReady();
    const registry = await binding.getHostedWindowRegistry(this.nativeId);
    return registry.activeWindow === undefined
      ? undefined
      : this.windows.get(registry.activeWindow);
  }

  flush(): void {
    this.#assertAlive();
    for (const window of this.windows.values()) window.flush();
  }

  dispatchEvents(): void {
    this.#dispatchNativeEvents(binding.takeEvents(this.nativeId));
  }

  #dispatchNativeEvents(events: binding.NativeEvent[]): void {
    for (const event of events) {
      if (
        event.kind === "alert-dialog" ||
        event.kind === "open-dialog" ||
        event.kind === "save-dialog"
      ) {
        this.#dispatchDialog(event);
        continue;
      }
      const systemEvent = dispatchSystemEvent(event, (id) => this.windows.get(id));
      const appEvent = this.#dispatchAppEvent(event);
      if (systemEvent || appEvent) continue;
      const window = this.windows.get(event.window);
      if (!window) continue;
      if (event.kind === "close") {
        this._didCloseWindow(window);
      } else if (event.kind === "close-requested") {
        window._didRequestClose();
      } else if (event.kind.startsWith("window-")) {
        window._didObserveLifecycle(event.kind, event.value);
      } else {
        window._dispatchEvent(event.kind as NativeEventType, event.target, event.value);
      }
    }
  }

  on<K extends keyof AppEventMap>(
    type: K,
    listener: (payload: AppEventMap[K]) => void,
  ): () => void {
    this.#assertAlive();
    const listeners = this.#appEventListeners.get(type) ?? new Set();
    const wrapped = (payload: unknown) => listener(payload as AppEventMap[K]);
    listeners.add(wrapped);
    this.#appEventListeners.set(type, listeners);
    this.#syncQuitInterception();
    return () => {
      listeners.delete(wrapped);
      if (listeners.size === 0) this.#appEventListeners.delete(type);
      this.#syncQuitInterception();
    };
  }

  /**
   * Declare quit interception to the core whenever a `beforeQuit` listener exists.
   *
   * A JavaScript listener can never veto a native decision synchronously, so interception is
   * declared ahead of time and the decision is completed later by an explicit quit or exit call.
   */
  #syncQuitInterception(): void {
    if (this.#destroyed || !this.#ready) return;
    const intercepting = (this.#appEventListeners.get("beforeQuit")?.size ?? 0) > 0;
    if (intercepting === this.#quitIntercepting) return;
    this.#quitIntercepting = intercepting;
    try {
      setNativeQuitInterception(intercepting);
    } catch {
      // The application is shutting down; the core no longer needs the declaration.
      this.#quitIntercepting = false;
    }
  }

  async run(): Promise<number> {
    if (this.#running) throw new Error("this QuickGUI app is already running");
    this.#running = true;
    let exitCode: number | undefined;
    try {
      await this.whenReady();
      {
        for (;;) {
          const update = await binding.waitForHostedEvents(this.nativeId);
          this.#dispatchNativeEvents(update.events);
          this.flush();
          if (update.exitCode !== undefined && update.exitCode !== null) {
            exitCode = update.exitCode;
            break;
          }
        }
      }
      if (exitCode === undefined) throw new Error("the QuickGUI app exited without a status code");
      if (this.#requestedExitCode !== undefined && exitCode === 0) {
        exitCode = this.#requestedExitCode;
      }
      await this.#emitAppEventAndWait("quit", { exitCode });
      return exitCode;
    } finally {
      this.#running = false;
      await this.releaseSingleInstanceLock();
    }
  }

  async requestSingleInstanceLock(identifier: string): Promise<boolean> {
    this.#assertAlive();
    if (this.#singleInstanceIdentifier) {
      if (this.#singleInstanceIdentifier !== identifier) {
        throw new Error("this QuickGUI app already owns a different single-instance lock");
      }
      return true;
    }
    this._assertReady();
    const acquired = await binding.requestHostedSingleInstanceLock(this.nativeId, identifier);
    if (acquired) this.#singleInstanceIdentifier = identifier;
    return acquired;
  }

  async releaseSingleInstanceLock(): Promise<boolean> {
    if (!this.#singleInstanceIdentifier || this.#destroyed) return false;
    const released = await binding.releaseHostedSingleInstanceLock(this.nativeId);
    if (released) this.#singleInstanceIdentifier = undefined;
    return released;
  }

  /**
   * Request an orderly native shutdown. Returns false after shutdown already began.
   *
   * The default runs the preventable `beforeQuit` and `willQuit` phases, so a registered
   * `beforeQuit` listener holds the application open. Pass `{ force: true }` to complete a held
   * quit, bypassing every interception.
   */
  async quit(options: { force?: boolean } = {}): Promise<boolean> {
    this.#assertAlive();
    this._assertReady();
    if (options.force !== true) return await requestNativeQuit();
    return await binding.exitHostedApp(this.nativeId);
  }

  /**
   * Complete a held quit and report `code` from `app.run()` and the `quit` event.
   *
   * The core's native event loop does not carry an application-chosen exit status, so QuickGUI
   * reports the requested code from the JavaScript host rather than inventing a native one.
   */
  async exit(code = 0): Promise<boolean> {
    if (!Number.isInteger(code) || code < 0 || code > 255) {
      throw new RangeError("an application exit code must be an integer between 0 and 255");
    }
    this.#assertAlive();
    this._assertReady();
    this.#requestedExitCode = code;
    // The core carries the status through its own teardown, so the process exits with `code`
    // even when the host is embedded in another runtime.
    return await exitNativeAppWithCode(code);
  }

  /** Whether this process is running from an installed application bundle. */
  get isPackaged(): boolean {
    return nativeApplicationPackaged();
  }

  /**
   * Change how the application appears in the Dock and application switcher.
   *
   * macOS applies `NSApplicationActivationPolicy`; other platforms reject with an unsupported
   * platform error.
   */
  async setActivationPolicy(policy: ActivationPolicy): Promise<void> {
    this.#assertAlive();
    this._assertReady();
    await setNativeActivationPolicy(policy);
  }

  /**
   * Bring the application forward.
   *
   * `steal` uses AppKit's ignore-other-apps activation, which takes focus from the frontmost
   * application. Prefer the default unless the user just asked for this application explicitly.
   */
  focus(options: { steal?: boolean } = {}): void {
    this.#assertAlive();
    this._assertReady();
    activateNativeApplication(options.steal ?? false);
  }

  /** Hide every window of this application. */
  hide(): void {
    this.#assertAlive();
    this._assertReady();
    hideNativeApplication();
  }

  /** Reveal an application hidden by `app.hide()`. */
  show(): void {
    this.#assertAlive();
    this._assertReady();
    unhideNativeApplication();
  }

  /** Route keystrokes straight to this process, bypassing input monitoring. */
  setSecureKeyboardEntryEnabled(enabled: boolean): void {
    this.#assertAlive();
    this._assertReady();
    setNativeSecureKeyboardEntry(enabled);
  }

  /** Whether the running bundle already lives in an `/Applications` directory. */
  async isInApplicationsFolder(): Promise<boolean> {
    this.#assertAlive();
    this._assertReady();
    return (await getNativeApplicationsFolderSupport()).alreadyInstalled;
  }

  /**
   * Move the running application bundle into `/Applications`.
   *
   * Resolves to `false` when the bundle is already installed there. QuickGUI never restarts the
   * process on its own; call `app.relaunch()` after a successful move.
   */
  async moveToApplicationsFolder(): Promise<boolean> {
    this.#assertAlive();
    this._assertReady();
    return await moveNativeApplicationToApplicationsFolder();
  }

  /** macOS Dock tile control, alongside the badge, icon, and menu on `Desktop`. */
  readonly dock = Object.freeze({
    setBadge: (value?: string): void => {
      Desktop.setDockBadge(value);
    },
    setIcon: (icon?: ImageSource): void => {
      Desktop.setDockIcon(icon);
    },
    setMenu: (menu?: MenuDefinition): void => {
      Desktop.setDockMenu(menu);
    },
    /**
     * Bounce the Dock tile and resolve with the identifier that cancels a critical bounce.
     *
     * `critical` keeps bouncing until the application is activated or the request is cancelled;
     * `informational` bounces once.
     */
    bounce: (type: DockAttentionType = "informational"): Promise<number> =>
      requestNativeDockAttention(type),
    /** Stop an in-flight critical bounce. */
    cancelBounce: (id: number): void => {
      cancelNativeDockAttention(id);
    },
    hide: (): Promise<void> => setNativeDockVisible(false),
    show: (): Promise<void> => setNativeDockVisible(true),
    /** The last Dock visibility this process asked for; the platform exposes no query. */
    isVisible: (): boolean => nativeDockVisible(),
  });

  /** Schedule a replacement process after ordinary child-first native teardown. */
  async relaunch(options: RelaunchOptions = {}): Promise<boolean> {
    this.#assertAlive();
    this._assertReady();
    const native: binding.NativeRelaunchOptions = {};
    if (options.executable !== undefined) native.executable = options.executable;
    if (options.arguments === null) native.clearArguments = true;
    else if (options.arguments !== undefined) native.arguments = [...options.arguments];
    if (options.workingDirectory !== undefined) {
      native.workingDirectory = options.workingDirectory;
    }
    return await binding.relaunchHostedApp(this.nativeId, native);
  }

  destroy(): void {
    if (this.#destroyed) return;
    const error = new Error("the QuickGUI app was destroyed");
    this.#rejectDialogs(undefined, error);
    rejectPendingSystemRequests(error);
    void this.releaseSingleInstanceLock().catch(() => {});
    binding.destroyHostedApp(this.nativeId);
    this.#destroyed = true;
    this.#appEventListeners.clear();
    if (activeApp === this) activeApp = undefined;
    for (const window of this.windows.values()) window._didDestroy();
    this.windows.clear();
  }

  #dispatchAppEvent(event: binding.NativeEvent): boolean {
    let type: keyof AppEventMap;
    let payload: AppEventMap[keyof AppEventMap];
    if (event.kind === "open-urls") {
      type = "openUrls";
      try {
        const parsed: unknown = JSON.parse(event.value ?? "[]");
        payload =
          Array.isArray(parsed) && parsed.every((url) => typeof url === "string") ? parsed : [];
      } catch {
        payload = [];
      }
    } else if (event.kind === "reopen") {
      type = "reopen";
      payload = { hasVisibleWindows: event.value === "true" };
    } else if (event.kind === "system-wake") {
      type = "systemWake";
      payload = undefined;
    } else if (event.kind === "app-activate" || event.kind === "app-deactivate") {
      type = event.kind === "app-activate" ? "activate" : "deactivate";
      payload = undefined;
    } else if (event.kind === "before-quit" || event.kind === "will-quit") {
      type = event.kind === "before-quit" ? "beforeQuit" : "willQuit";
      payload = { reason: quitReason(event.value) };
    } else if (event.kind === "keyboard-layout-change") {
      const layout = binding.getHostedKeyboardLayout(this.nativeId);
      void layout
        .then((value) => {
          if (this.#destroyed) return;
          this.#emitAppEvent("keyboardLayoutChange", {
            id: value.id,
            name: value.name,
          });
        })
        .catch(() => {});
      return true;
    } else if (event.kind === "notification-response") {
      type = "notificationResponse";
      try {
        const parsed = JSON.parse(event.value ?? "{}") as {
          tag?: unknown;
          actionId?: unknown;
          reply?: unknown;
        };
        if (typeof parsed.tag !== "string") return true;
        const response: NotificationResponse = { tag: parsed.tag };
        if (typeof parsed.actionId === "string") response.actionId = parsed.actionId;
        if (typeof parsed.reply === "string") response.reply = parsed.reply;
        payload = response;
      } catch {
        return true;
      }
    } else if (event.kind === "second-instance") {
      type = "secondInstance";
      try {
        const parsed = JSON.parse(event.value ?? "{}") as {
          argv?: unknown;
          cwd?: unknown;
        };
        if (
          !Array.isArray(parsed.argv) ||
          !parsed.argv.every((argument) => typeof argument === "string") ||
          typeof parsed.cwd !== "string"
        ) {
          return true;
        }
        payload = { argv: parsed.argv, cwd: parsed.cwd };
        const urls = urlsFromArguments(parsed.argv);
        if (urls.length > 0) this.#emitAppEvent("openUrls", urls);
      } catch {
        return true;
      }
    } else {
      return false;
    }
    this.#emitAppEvent(type, payload);
    return true;
  }

  #emitAppEvent<K extends keyof AppEventMap>(type: K, payload: AppEventMap[K]): void {
    for (const listener of this.#appEventListeners.get(type) ?? []) listener(payload);
  }

  async #emitAppEventAndWait<K extends keyof AppEventMap>(
    type: K,
    payload: AppEventMap[K],
  ): Promise<void> {
    await Promise.all(
      [...(this.#appEventListeners.get(type) ?? [])].map((listener) => listener(payload)),
    );
  }

  _registerWindow(window: Window): void {
    this.#assertAlive();
    this.windows.set(window.nativeId, window);
  }

  _assertReady(): void {
    this.#assertAlive();
    if (!this.isReady()) {
      throw new Error("await app.whenReady() before using the native QuickGUI application");
    }
  }

  _closeWindow(window: Window): void {
    window.flush();
    if (this.#destroyed || window.closed) return;
    {
      binding.closeHostedWindow(this.nativeId, window.nativeId);
    }
  }

  _didCloseWindow(window: Window): void {
    if (this.windows.get(window.nativeId) !== window) return;
    this.windows.delete(window.nativeId);
    this.#rejectDialogs(window, new Error("the native dialog's owner window closed"));
    window._didClose();
  }

  _showAlertDialog(window: Window | undefined, options: AlertDialogOptions): Promise<number> {
    try {
      const nativeOptions = normalizeAlertDialogOptions(options);
      return this.#requestDialog(
        window,
        "alert-dialog",
        (request) => {
          {
            binding.showHostedAlertDialog(this.nativeId, window?.nativeId, request, nativeOptions);
          }
        },
        (event) => {
          const response = Number(event.value);
          if (!Number.isSafeInteger(response) || response < 0) {
            throw new Error("the native dialog returned an invalid button index");
          }
          return response;
        },
      );
    } catch (error) {
      return Promise.reject(asError(error));
    }
  }

  _showOpenDialog(
    window: Window | undefined,
    options: OpenDialogOptions,
  ): Promise<OpenDialogResult> {
    try {
      const nativeOptions = normalizeOpenDialogOptions(options);
      return this.#requestDialog(
        window,
        "open-dialog",
        (request) => {
          {
            binding.showHostedOpenDialog(this.nativeId, window?.nativeId, request, nativeOptions);
          }
        },
        (event) => ({
          canceled: event.paths === undefined,
          filePaths: event.paths ?? [],
        }),
      );
    } catch (error) {
      return Promise.reject(asError(error));
    }
  }

  _showSaveDialog(
    window: Window | undefined,
    options: SaveDialogOptions,
  ): Promise<SaveDialogResult> {
    try {
      const nativeOptions = normalizeSaveDialogOptions(options);
      return this.#requestDialog(
        window,
        "save-dialog",
        (request) => {
          {
            binding.showHostedSaveDialog(this.nativeId, window?.nativeId, request, nativeOptions);
          }
        },
        (event) =>
          event.value === undefined
            ? { canceled: true }
            : { canceled: false, filePath: event.value },
      );
    } catch (error) {
      return Promise.reject(asError(error));
    }
  }

  #requestDialog<T>(
    window: Window | undefined,
    kind: DialogEventKind,
    invoke: (request: number) => void | Promise<void>,
    result: (event: binding.NativeEvent) => T,
  ): Promise<T> {
    this.#assertAlive();
    if (window && (window.closed || this.windows.get(window.nativeId) !== window)) {
      return Promise.reject(new Error("the native dialog parent must be an open window"));
    }
    const request = this.#allocateDialogRequest();
    return new Promise<T>((resolve, reject) => {
      this.#pendingDialogs.set(request, {
        kind,
        window,
        complete: (event) => resolve(result(event)),
        reject,
      });
      try {
        void Promise.resolve(invoke(request)).catch((error) => {
          if (!this.#pendingDialogs.delete(request)) return;
          reject(asError(error));
        });
      } catch (error) {
        this.#pendingDialogs.delete(request);
        reject(asError(error));
      }
    });
  }

  #dispatchDialog(event: binding.NativeEvent): void {
    const pending = this.#pendingDialogs.get(event.target);
    if (!pending) return;
    this.#pendingDialogs.delete(event.target);
    if ((pending.window?.nativeId ?? 0) !== event.window) {
      pending.reject(new Error("the native dialog response had the wrong owner window"));
      return;
    }
    if (pending.kind !== event.kind) {
      pending.reject(new Error("the native dialog response had the wrong response type"));
      return;
    }
    if (event.error !== undefined) {
      pending.reject(new Error(event.error));
      return;
    }
    try {
      pending.complete(event);
    } catch (error) {
      pending.reject(asError(error));
    }
  }

  #allocateDialogRequest(): number {
    for (let attempt = 0; attempt <= this.#pendingDialogs.size; attempt += 1) {
      const request = this.#nextDialogRequest;
      this.#nextDialogRequest = request >= 0xffff_ffff ? 1 : request + 1;
      if (!this.#pendingDialogs.has(request)) return request;
    }
    throw new Error("the native dialog request id space is exhausted");
  }

  #rejectDialogs(window: Window | undefined, error: Error): void {
    for (const [request, pending] of this.#pendingDialogs) {
      if (window && pending.window !== window) continue;
      this.#pendingDialogs.delete(request);
      pending.reject(error);
    }
  }

  #assertAlive(): void {
    if (this.#destroyed) throw new Error("this QuickGUI app has been destroyed");
  }
}

export class Window {
  readonly root: NativeNode;
  readonly nativeId: number;
  readonly app: App;
  readonly nodes = new Map<number, NativeNode>();
  #batch = new MutationBatch();
  #flushScheduled = false;
  #nativeReady = false;
  readonly #nativeReadyCallbacks = new Set<() => void>();
  #closed = false;
  readonly #closeListeners = new Set<WindowCloseListener>();
  readonly #closeRequestListeners = new Set<WindowCloseRequestListener>();
  #closeIntercepting = false;
  readonly #mountDisposers = new Set<() => void>();
  readonly #lifecycleListeners = new Map<keyof WindowEventMap, Set<(payload: unknown) => void>>();

  /** Return the Window whose renderer or native event callback is currently executing. */
  static getCurrentWindow(): Window {
    if (!currentWindow) {
      throw new Error(
        "Window.getCurrentWindow() must be called while rendering or handling a window event",
      );
    }
    return currentWindow;
  }

  /** @internal Create one retained renderer whose native view is owned by a SwiftUI host. */
  static _createEmbedded(
    owner: Window,
    options: WindowOptions,
    matchContents: { horizontal: boolean; vertical: boolean },
  ): Window {
    if (owner.closed) {
      throw new Error("an embedded QuickGUI view requires an open owner Window");
    }
    owner.flush();
    return new Window(options, { owner, matchContents });
  }

  constructor(
    options: WindowOptions,
    embedded?: {
      owner: Window;
      matchContents: { horizontal: boolean; vertical: boolean };
    },
  ) {
    const app = activeApp;
    if (!app) throw new Error("the QuickGUI app is unavailable");
    if (!app.isReady()) {
      throw new Error("await app.whenReady() before creating a QuickGUI Window");
    }
    const nativeOptions: binding.NativeWindowOptions = {};
    const serializedMenu = options.menu ? serializeNativeMenu(options.menu) : undefined;
    if (options.title !== undefined) nativeOptions.title = options.title;
    if (options.width !== undefined) nativeOptions.width = options.width;
    if (options.height !== undefined) nativeOptions.height = options.height;
    if (
      options.minimumSize !== undefined &&
      (options.minimumWidth !== undefined || options.minimumHeight !== undefined)
    ) {
      throw new TypeError("minimumSize cannot be combined with minimumWidth or minimumHeight");
    }
    if (options.minimumSize === null) {
      nativeOptions.minimumSizeEnabled = false;
    } else if (options.minimumSize !== undefined) {
      nativeOptions.minimumWidth = options.minimumSize.width;
      nativeOptions.minimumHeight = options.minimumSize.height;
    } else {
      if (options.minimumWidth !== undefined) nativeOptions.minimumWidth = options.minimumWidth;
      if (options.minimumHeight !== undefined) nativeOptions.minimumHeight = options.minimumHeight;
    }
    if (
      options.maximumSize !== undefined &&
      (options.maximumWidth !== undefined || options.maximumHeight !== undefined)
    ) {
      throw new TypeError("maximumSize cannot be combined with maximumWidth or maximumHeight");
    }
    if (options.maximumSize !== undefined) {
      nativeOptions.maximumWidth = options.maximumSize.width;
      nativeOptions.maximumHeight = options.maximumSize.height;
    } else {
      if (options.maximumWidth !== undefined) nativeOptions.maximumWidth = options.maximumWidth;
      if (options.maximumHeight !== undefined) nativeOptions.maximumHeight = options.maximumHeight;
    }
    if (options.position !== undefined) {
      nativeOptions.x = options.position.x;
      nativeOptions.y = options.position.y;
    }
    if (options.initialState !== undefined) nativeOptions.initialState = options.initialState;
    if (options.displayId !== undefined) nativeOptions.displayId = options.displayId;
    if (options.representedFile !== undefined)
      nativeOptions.representedFile = options.representedFile;
    if (options.documentEdited !== undefined) nativeOptions.documentEdited = options.documentEdited;
    if (options.tabbingIdentifier !== undefined) {
      nativeOptions.tabbingIdentifier = options.tabbingIdentifier;
    }
    if (options.background !== undefined) nativeOptions.background = parseColor(options.background);
    if (options.performanceProfile !== undefined) {
      nativeOptions.performanceProfile = options.performanceProfile;
    }
    if (options.appearance !== undefined) nativeOptions.appearance = options.appearance;
    if (options.vibrancy !== undefined) nativeOptions.vibrancy = options.vibrancy;
    if (options.visualEffectState !== undefined)
      nativeOptions.visualEffectState = options.visualEffectState;
    if (options.titleBarStyle !== undefined) nativeOptions.titleBarStyle = options.titleBarStyle;
    if (options.kind !== undefined) nativeOptions.kind = options.kind;
    if (options.focus !== undefined) nativeOptions.focus = options.focus;
    if (options.focusable !== undefined) nativeOptions.focusable = options.focusable;
    if (options.visible !== undefined) nativeOptions.show = options.visible;
    if (options.movable !== undefined) nativeOptions.movable = options.movable;
    if (options.resizable !== undefined) nativeOptions.resizable = options.resizable;
    if (options.minimizable !== undefined) nativeOptions.minimizable = options.minimizable;
    if (options.maximizable !== undefined) nativeOptions.maximizable = options.maximizable;
    if (options.closable !== undefined) nativeOptions.closable = options.closable;
    if (options.decorated !== undefined) nativeOptions.decorated = options.decorated;
    if (options.shadow !== undefined) nativeOptions.shadow = options.shadow;
    if (options.contentProtected !== undefined) {
      nativeOptions.contentProtected = options.contentProtected;
    }
    if (options.windowLevel !== undefined) nativeOptions.windowLevel = options.windowLevel;
    if (options.skipTaskbar !== undefined) nativeOptions.skipTaskbar = options.skipTaskbar;
    if (options.visibleOnAllWorkspaces !== undefined) {
      nativeOptions.visibleOnAllWorkspaces = options.visibleOnAllWorkspaces;
    }
    if (options.opacity !== undefined) nativeOptions.opacity = options.opacity;
    if (options.icon !== undefined) nativeOptions.icon = nativeImageSource(options.icon);
    if (options.taskbarProgress !== undefined) {
      nativeOptions.taskbarProgressState = options.taskbarProgress.state;
      nativeOptions.taskbarProgress = options.taskbarProgress.progress;
    }
    if (options.taskbarOverlay !== undefined) {
      nativeOptions.taskbarOverlayIcon = nativeImageSource(options.taskbarOverlay.icon);
      nativeOptions.taskbarOverlayDescription = options.taskbarOverlay.description;
    }
    if (options.cursorVisible !== undefined) nativeOptions.cursorVisible = options.cursorVisible;
    if (options.cursorGrab !== undefined) nativeOptions.cursorGrab = options.cursorGrab;
    if (options.cursorHitTest !== undefined) nativeOptions.cursorHitTest = options.cursorHitTest;
    if (options.cursorPosition !== undefined) {
      nativeOptions.cursorX = options.cursorPosition.x;
      nativeOptions.cursorY = options.cursorPosition.y;
    }
    if (serializedMenu !== undefined) nativeOptions.menu = serializedMenu.json;
    if (options.restoreState !== undefined) {
      nativeOptions.restoreState = nativeWindowRestoreState(options.restoreState);
    }
    if (options.lineScrollPixels !== undefined) {
      nativeOptions.lineScrollPixels = options.lineScrollPixels;
    }
    if (options.keySequenceTimeoutMs !== undefined) {
      nativeOptions.keySequenceTimeoutMs = options.keySequenceTimeoutMs;
    }
    if (options.reduceMotion !== undefined) nativeOptions.reduceMotion = options.reduceMotion;
    if (options.trafficLightPosition !== undefined) {
      nativeOptions.trafficLightX = options.trafficLightPosition.x;
      nativeOptions.trafficLightY = options.trafficLightPosition.y;
    }
    if (options.backgroundAppearance !== undefined) {
      nativeOptions.transparent = options.backgroundAppearance === "transparent";
      nativeOptions.blur = options.backgroundAppearance === "blurred";
    } else {
      if (options.transparent !== undefined) nativeOptions.transparent = options.transparent;
      if (options.blur !== undefined) nativeOptions.blur = options.blur;
    }
    if (options.placement !== undefined) nativeOptions.popoverPlacement = options.placement;
    if (options.gap !== undefined) nativeOptions.popoverGap = options.gap;
    if (options.offset !== undefined) {
      nativeOptions.popoverOffsetX = options.offset.x;
      nativeOptions.popoverOffsetY = options.offset.y;
    }
    if (options.viewportMargin !== undefined) {
      nativeOptions.popoverViewportMargin = options.viewportMargin;
    }
    if (options.dismissOnEscape !== undefined) {
      nativeOptions.popoverDismissOnEscape = options.dismissOnEscape;
    }
    if (options.dismissOnPointerOutside !== undefined) {
      nativeOptions.popoverDismissOnPointerOutside = options.dismissOnPointerOutside;
    }
    if (options.grab !== undefined) nativeOptions.popoverGrab = options.grab;
    if (options.acceptsKeyFocus !== undefined) {
      nativeOptions.popoverAcceptsKeyFocus = options.acceptsKeyFocus;
    }
    const parent = options.anchor?.host;
    if (options.anchor && (!parent || parent.closed || !options.anchor.materialized)) {
      throw new Error("a system popover requires a mounted node in an open parent Window");
    }
    parent?.flush();
    this.app = app;
    this.root = new NativeNode(NativeNodeTag.View, "", ROOT_NODE_ID);
    this.root.host = this;
    this.root.materialized = true;
    this.nodes.set(ROOT_NODE_ID, this.root);
    try {
      const dispose = withCurrentWindow(this, () => options.renderer(this));
      if (typeof dispose !== "function") {
        throw new TypeError("a QuickGUI Window renderer must return a dispose function");
      }
      this._trackMount(dispose);
      const initialBatch = this.#takePendingBatch();
      if (embedded) {
        if (process.platform !== "darwin") {
          throw new Error("embedded SwiftUI QuickGUI views require macOS");
        }
        this.nativeId = binding.createHostedEmbeddedView(
          app.nativeId,
          embedded.owner.nativeId,
          embedded.matchContents.horizontal,
          embedded.matchContents.vertical,
          nativeOptions,
          initialBatch,
        );
      } else if (options.anchor) {
        this.nativeId = binding.createHostedSystemPopover(
          app.nativeId,
          parent!.nativeId,
          options.anchor.id,
          nativeOptions,
          initialBatch,
        );
      } else {
        this.nativeId = binding.createHostedWindow(app.nativeId, nativeOptions, initialBatch);
      }
      this.#nativeReady = true;
      app._registerWindow(this);
      for (const callback of [...this.#nativeReadyCallbacks]) {
        this.#nativeReadyCallbacks.delete(callback);
        callback();
      }
      if (serializedMenu !== undefined) this._trackMount(serializedMenu.install());
    } catch (error) {
      this._didClose();
      throw error;
    }
  }

  whenReady(): Promise<void> {
    return binding.windowReady(this.nativeId);
  }

  get closed(): boolean {
    return this.#closed;
  }

  onClose(listener: WindowCloseListener): () => void {
    if (this.#closed) {
      listener(this);
      return () => {};
    }
    this.#closeListeners.add(listener);
    return () => this.#closeListeners.delete(listener);
  }

  /** @internal Run after this window has a native id, including during its initial construction. */
  _afterNativeReady(callback: () => void): () => void {
    if (this.#closed) return () => {};
    if (this.#nativeReady) {
      callback();
      return () => {};
    }
    this.#nativeReadyCallbacks.add(callback);
    return () => this.#nativeReadyCallbacks.delete(callback);
  }

  /**
   * Complete a held close, or begin an ordinary one.
   *
   * Native close requests are intercepted only while `onCloseRequested` listeners exist; this
   * call always completes the close.
   */
  close(): void {
    this.app._closeWindow(this);
  }

  /** Close the window immediately, bypassing every registered `closeRequested` listener. */
  destroy(): void {
    this.#closeRequestListeners.clear();
    this.#syncCloseInterception();
    this.app._closeWindow(this);
  }

  /**
   * Hold native close requests for this window and decide in JavaScript.
   *
   * While at least one listener is registered the core prevents the native close and delivers a
   * `closeRequested` event instead. Call `window.close()` (or `window.destroy()`) to complete it.
   * With no listener left, closes proceed natively again.
   */
  onCloseRequested(listener: WindowCloseRequestListener): () => void {
    if (this.#closed) return () => {};
    this.#closeRequestListeners.add(listener);
    this.#syncCloseInterception();
    return () => {
      this.#closeRequestListeners.delete(listener);
      this.#syncCloseInterception();
    };
  }

  /** Subscribe to one window lifecycle event. */
  on<K extends keyof WindowEventMap>(
    type: K,
    listener: (payload: WindowEventMap[K]) => void,
  ): () => void {
    if (type === "closeRequested") {
      return this.onCloseRequested(listener as WindowCloseRequestListener);
    }
    if (type === "closed") {
      return this.onClose((window) =>
        (listener as (payload: WindowEventMap["closed"]) => void)({ window }),
      );
    }
    if (this.#closed) return () => {};
    const listeners = this.#lifecycleListeners.get(type) ?? new Set();
    const wrapped = (payload: unknown) => listener(payload as WindowEventMap[K]);
    listeners.add(wrapped);
    this.#lifecycleListeners.set(type, listeners);
    return () => {
      listeners.delete(wrapped);
      if (listeners.size === 0) this.#lifecycleListeners.delete(type);
    };
  }

  #emitLifecycle<K extends keyof WindowEventMap>(type: K, payload: WindowEventMap[K]): void {
    const listeners = this.#lifecycleListeners.get(type);
    if (!listeners?.size) return;
    withCurrentWindow(this, () => {
      for (const listener of [...listeners]) listener(payload);
    });
  }

  /** @internal One core window lifecycle notification reached JavaScript. */
  _didObserveLifecycle(kind: string, value: string | undefined): void {
    if (this.#closed) return;
    switch (kind) {
      case "window-minimize":
        this.#emitLifecycle(value === "true" ? "minimize" : "restore", {
          window: this,
        });
        return;
      case "window-maximize":
        this.#emitLifecycle(value === "true" ? "maximize" : "unmaximize", {
          window: this,
        });
        return;
      case "window-fullscreen":
        this.#emitLifecycle(value === "true" ? "enterFullScreen" : "leaveFullScreen", {
          window: this,
        });
        return;
      case "window-ready-to-show":
        this.#emitLifecycle("readyToShow", { window: this });
        return;
      case "window-occlusion":
        this.#emitLifecycle("occlusionChange", {
          window: this,
          occluded: value === "true",
        });
        return;
      case "window-level":
        this.#emitLifecycle("levelChange", {
          window: this,
          level: (value ?? "normal") as WindowLevel,
        });
        return;
      case "window-focus":
        this.#emitLifecycle(value === "true" ? "focus" : "blur", {
          window: this,
        });
        return;
      case "window-appearance":
        this.#emitLifecycle("appearanceChange", {
          window: this,
          appearance: value === "dark" ? "dark" : "light",
        });
        return;
      case "window-will-resize":
      case "window-resize": {
        const size = parseWindowSize(value);
        if (!size) return;
        this.#emitLifecycle(kind === "window-resize" ? "resize" : "willResize", {
          window: this,
          size,
        });
        return;
      }
      case "window-will-move":
      case "window-move": {
        const position = parseWindowPoint(value);
        if (!position) return;
        this.#emitLifecycle(kind === "window-move" ? "move" : "willMove", {
          window: this,
          position,
        });
        return;
      }
      default:
    }
  }

  #syncCloseInterception(): void {
    const intercepting = this.#closeRequestListeners.size > 0;
    if (this.#closed || intercepting === this.#closeIntercepting) return;
    this.#closeIntercepting = intercepting;
    this._afterNativeReady(() => {
      try {
        performNativeWindowAction(this, "set-close-interception", String(intercepting));
      } catch {
        // The window is already gone; the core drops its interception with it.
      }
    });
  }

  /** @internal A held native close request reached JavaScript. */
  _didRequestClose(): void {
    if (this.#closed) return;
    withCurrentWindow(this, () => {
      for (const listener of [...this.#closeRequestListeners]) {
        listener({ window: this });
      }
    });
  }

  getState(): Promise<WindowState> {
    return getNativeWindowState(this);
  }

  onStateChange(listener: (state: WindowState) => void): () => void {
    if (this.#closed) return () => {};
    return onNativeWindowStateChange(this, listener);
  }

  setTitle(title: string): void {
    performNativeWindowAction(this, "set-title", title);
  }

  setBounds(bounds: WindowBounds): void {
    performNativeWindowAction(this, "set-bounds", JSON.stringify(bounds));
  }

  setPosition(position: Point): void {
    performNativeWindowAction(this, "move", JSON.stringify(position));
  }

  setSize(size: Size): void {
    performNativeWindowAction(this, "resize", JSON.stringify(size));
  }

  minimize(): void {
    performNativeWindowAction(this, "minimize");
  }

  maximize(): void {
    performNativeWindowAction(this, "maximize");
  }

  restore(): void {
    performNativeWindowAction(this, "restore");
  }

  setFullscreen(fullscreen: boolean): void {
    performNativeWindowAction(this, "set-fullscreen", String(fullscreen));
  }

  setResizable(resizable: boolean): void {
    performNativeWindowAction(this, "set-resizable", String(resizable));
  }

  setMovable(movable: boolean): void {
    performNativeWindowAction(this, "set-movable", String(movable));
  }

  setMinimumSize(size?: Size): void {
    performNativeWindowAction(
      this,
      "set-minimum-size",
      size === undefined ? undefined : JSON.stringify(size),
    );
  }

  setMaximumSize(size?: Size): void {
    performNativeWindowAction(
      this,
      "set-maximum-size",
      size === undefined ? undefined : JSON.stringify(size),
    );
  }

  setMinimizable(minimizable: boolean): void {
    performNativeWindowAction(this, "set-minimizable", String(minimizable));
  }

  setMaximizable(maximizable: boolean): void {
    performNativeWindowAction(this, "set-maximizable", String(maximizable));
  }

  setClosable(closable: boolean): void {
    performNativeWindowAction(this, "set-closable", String(closable));
  }

  setDecorated(decorated: boolean): void {
    performNativeWindowAction(this, "set-decorated", String(decorated));
  }

  setShadow(shadow: boolean): void {
    performNativeWindowAction(this, "set-shadow", String(shadow));
  }

  setContentProtected(protected_: boolean): void {
    performNativeWindowAction(this, "set-content-protected", String(protected_));
  }

  setWindowLevel(level: WindowLevel | "automatic"): void {
    performNativeWindowAction(this, "set-window-level", level);
  }

  setFocusable(focusable: boolean): void {
    performNativeWindowAction(this, "set-focusable", String(focusable));
  }

  setSkipTaskbar(skip: boolean): void {
    performNativeWindowAction(this, "set-skip-taskbar", String(skip));
  }

  setVisibleOnAllWorkspaces(visible: boolean): void {
    performNativeWindowAction(this, "set-visible-on-all-workspaces", String(visible));
  }

  setOpacity(opacity: number): void {
    performNativeWindowAction(this, "set-opacity", String(opacity));
  }

  setIcon(icon: ImageSource): void {
    performNativeWindowImageAction(this, "set-icon", icon);
  }

  clearIcon(): void {
    performNativeWindowImageAction(this, "clear-icon");
  }

  setTaskbarProgress(state: TaskbarProgressState, progress: number): void {
    performNativeWindowAction(this, "set-taskbar-progress", JSON.stringify({ state, progress }));
  }

  setTaskbarOverlayIcon(icon: ImageSource, description: string): void {
    performNativeWindowImageAction(this, "set-taskbar-overlay-icon", icon, description);
  }

  clearTaskbarOverlayIcon(): void {
    performNativeWindowAction(this, "clear-taskbar-overlay-icon");
  }

  setCursorVisible(visible: boolean): void {
    performNativeWindowAction(this, "set-cursor-visible", String(visible));
  }

  setCursorGrab(mode: CursorGrabMode): void {
    performNativeWindowAction(this, "set-cursor-grab", mode);
  }

  setCursorHitTest(hitTest: boolean): void {
    performNativeWindowAction(this, "set-cursor-hit-test", String(hitTest));
  }

  setCursorPosition(position: Point): void {
    performNativeWindowAction(this, "set-cursor-position", JSON.stringify(position));
  }

  show(): void {
    performNativeWindowAction(this, "set-visible", "true");
  }

  hide(): void {
    performNativeWindowAction(this, "set-visible", "false");
  }

  focus(): void {
    performNativeWindowAction(this, "focus");
  }

  requestAttention(): void {
    performNativeWindowAction(this, "request-attention");
  }

  setRepresentedFile(path?: string): void {
    performNativeWindowAction(this, "set-represented-file", path ?? "");
  }

  setDocumentEdited(edited: boolean): void {
    performNativeWindowAction(this, "set-document-edited", String(edited));
  }

  setAppearance(appearance: AppearancePreference): void {
    performNativeWindowAction(this, "set-appearance", appearance);
  }

  setBackgroundAppearance(appearance: WindowBackgroundAppearance): void {
    performNativeWindowAction(this, "set-background-appearance", appearance);
  }

  setVibrancy(vibrancy?: MacOSVibrancy): void {
    performNativeWindowAction(this, "set-vibrancy", vibrancy);
  }

  setVisualEffectState(state: MacOSVisualEffectState): void {
    performNativeWindowAction(this, "set-visual-effect-state", state);
  }

  /** Present AppKit's character palette above this window. */
  showCharacterPalette(): void {
    performNativeWindowAction(this, "show-character-palette");
  }

  /**
   * Raise or restore this window's stacking level.
   *
   * `level` names the level applied while `flag` is true and accepts the Electron names
   * (`floating`, `modalPanel`, `mainMenu`, `status`, `popUpMenu`, `screenSaver`) as well as their
   * kebab-case forms. Turning it off returns the window to `normal`.
   */
  setAlwaysOnTop(flag: boolean, level?: WindowLevel | ElectronWindowLevel): void {
    performNativeWindowAction(
      this,
      "set-always-on-top",
      JSON.stringify(level === undefined ? { flag } : { flag, level }),
    );
  }

  /** Raise this window to the front of its stacking level without activating the app. */
  moveTop(): void {
    performNativeWindowAction(this, "move-top");
  }

  /** Order this window immediately above another open window. */
  moveAbove(other: Window): void {
    if (other === this) {
      throw new RangeError("a window cannot be ordered above itself");
    }
    performNativeWindowAction(this, "move-above", String(other.nativeId));
  }

  /**
   * Let clicks pass through this window to whatever is behind it.
   *
   * `forward` keeps pointer motion and hover events flowing to this window; it is ignored when
   * `ignore` is false.
   */
  setIgnoreMouseEvents(ignore: boolean, options: { forward?: boolean } = {}): void {
    performNativeWindowAction(
      this,
      "set-ignore-mouse-events",
      JSON.stringify({ ignore, forward: options.forward ?? false }),
    );
  }

  /**
   * Block or restore every native input event for this window.
   *
   * A disabled window stays visible and keeps rendering; it simply stops receiving pointer and
   * keyboard input, which is the native way to express an application-modal owner.
   */
  setEnabled(enabled: boolean): void {
    performNativeWindowAction(this, "set-enabled", String(enabled));
  }

  /** Constrain live native resizing to one `width:height` content ratio, or pass null to clear. */
  setAspectRatio(ratio: Size | null): void {
    performNativeWindowAction(
      this,
      "set-aspect-ratio",
      ratio === null ? undefined : JSON.stringify(ratio),
    );
  }

  /** Show or hide the macOS close/minimize/zoom buttons. */
  setWindowButtonVisibility(visible: boolean): void {
    performNativeWindowAction(this, "set-window-button-visibility", String(visible));
  }

  /** Electron-compatible alias for `setShadow`. */
  setHasShadow(shadow: boolean): void {
    this.setShadow(shadow);
  }

  /**
   * Declare how the core narrows a window-manager resize.
   *
   * The core answers the platform synchronously, so the constraint is declared ahead instead of
   * being asked of JavaScript inside the `willResize` event. Pass `null` to withdraw it.
   */
  setResizePolicy(policy: WindowResizePolicy | null): void {
    performNativeWindowAction(
      this,
      "set-resize-policy",
      policy === null ? undefined : JSON.stringify(policy),
    );
  }

  /** Declare how the core narrows a window-manager move. Pass `null` to withdraw it. */
  setMovePolicy(policy: WindowMovePolicy | null): void {
    performNativeWindowAction(
      this,
      "set-move-policy",
      policy === null ? undefined : JSON.stringify(policy),
    );
  }

  /**
   * Replace this window's native menu declaration, or pass `null` to inherit the app menu.
   *
   * On macOS the declaration becomes the process menu bar while this window is active.
   */
  setMenu(definitions: readonly MenuDefinition[] | null): void {
    setNativeWindowMenu(this, definitions);
  }

  /**
   * Capture this window's persistable geometry and display identity.
   *
   * Store the result and hand it back as `WindowOptions.restoreState` on the next launch. The
   * rectangle is the windowed restore geometry, so a maximized or fullscreen window still
   * persists the size it returns to.
   */
  getRestoreState(): Promise<WindowRestoreState> {
    return getNativeWindowRestoreState(this);
  }

  /** Join a named native system-tab group, or leave it by passing no identifier. */
  setTabbingIdentifier(identifier?: string): void {
    performNativeWindowAction(this, "set-tabbing-identifier", identifier ?? "");
  }

  selectNextTab(): void {
    performNativeWindowAction(this, "select-next-tab");
  }

  selectPreviousTab(): void {
    performNativeWindowAction(this, "select-previous-tab");
  }

  selectTab(index: number): void {
    if (!Number.isInteger(index) || index < 0) {
      throw new RangeError("a native tab index must be a non-negative integer");
    }
    performNativeWindowAction(this, "select-tab", String(index));
  }

  mergeAllWindows(): void {
    performNativeWindowAction(this, "merge-all-windows");
  }

  moveTabToNewWindow(): void {
    performNativeWindowAction(this, "move-tab-to-new-window");
  }

  toggleTabBar(): void {
    performNativeWindowAction(this, "toggle-tab-bar");
  }

  toggleTabOverview(): void {
    performNativeWindowAction(this, "toggle-tab-overview");
  }

  _focusNode(node: NativeNode): boolean {
    if (this.#closed || node.host !== this) return false;
    this.flush();
    {
      binding.focusHostedNode(this.app.nativeId, this.nativeId, node.id);
      return true;
    }
    return false;
  }

  flush(): number | undefined {
    if (this.#closed || !this.#nativeReady) return undefined;
    this.#flushScheduled = false;
    if (this.#batch.empty) return undefined;
    const batch = this.#batch;
    this.#batch = new MutationBatch();
    const bytes = batch.finish();
    return binding.applyHostedBatch(this.app.nativeId, this.nativeId, bytes);
  }

  #takePendingBatch() {
    this.#flushScheduled = false;
    const batch = this.#batch;
    this.#batch = new MutationBatch();
    return batch.finish();
  }

  _dispatchEvent(type: NativeEventType, targetId: number, value?: string): void {
    withCurrentWindow(this, () => {
      const target = this.nodes.get(targetId);
      if (!target) return;
      const quickGuiEvent = new QuickGuiEvent(type, target, value);
      if (type === "mouseenter" || type === "mouseleave") {
        target.listeners.get(type)?.(quickGuiEvent);
        return;
      }
      let current: NativeNode | undefined = target;
      while (current) {
        quickGuiEvent.currentTarget = current;
        current.listeners.get(type)?.(quickGuiEvent);
        if (quickGuiEvent.propagationStopped) break;
        current = current.parent;
      }
    });
  }

  _didClose(): void {
    if (this.#closed) return;
    this.#closed = true;
    this.#flushScheduled = false;
    this.#nativeReadyCallbacks.clear();
    const disposers = [...this.#mountDisposers];
    this.#mountDisposers.clear();
    for (const dispose of disposers) dispose();
    this.#batch = new MutationBatch();
    removeNativeWindowStateListeners(this);
    releaseNativeWindowMenu(this.nativeId);
    this.#lifecycleListeners.clear();
    this.#closeRequestListeners.clear();
    this.#closeIntercepting = false;
    for (const listener of this.#closeListeners) listener(this);
    this.#closeListeners.clear();
    this.nodes.clear();
  }

  _didDestroy(): void {
    this._didClose();
  }

  _trackMount(dispose: () => void): () => void {
    if (this.#closed) {
      dispose();
      return () => {};
    }
    this.#mountDisposers.add(dispose);
    return () => this.#mountDisposers.delete(dispose);
  }

  _enqueueCreate(node: NativeNode): void {
    switch (node.tag) {
      case NativeNodeTag.Text:
        this.#batch.createText(node.id, node.text);
        break;
      case NativeNodeTag.Sentinel:
        this.#batch.createSentinel(node.id);
        break;
      default:
        this.#batch.createElement(node.id, node.tag);
    }
  }

  _enqueueProperty(
    node: NativeNode,
    property: PropertyCode,
    value: NativePropertyValue,
    color: boolean,
  ): void {
    this.#batch.setProperty(node.id, property, value, color);
    this._scheduleFlush();
  }

  _enqueueText(node: NativeNode): void {
    this.#batch.replaceText(node.id, node.text);
    this._scheduleFlush();
  }

  _enqueueInsert(parent: NativeNode, child: NativeNode, before?: NativeNode): void {
    this.#batch.insert(parent.id, child.id, before?.id);
    this._scheduleFlush();
  }

  _enqueueRemove(parent: NativeNode, child: NativeNode): void {
    this.#batch.remove(parent.id, child.id);
    this._scheduleFlush();
  }

  _enqueueCleanup(parent: NativeNode, children: readonly NativeNode[]): void {
    this.#batch.cleanup(
      parent.id,
      children.map((child) => child.id),
    );
    this._scheduleFlush();
  }

  _scheduleFlush(): void {
    if (this.#flushScheduled || this.#closed) return;
    this.#flushScheduled = true;
    queueMicrotask(() => {
      if (!this.#closed) this.flush();
    });
  }
}

function showAlertDialog(options: AlertDialogOptions): Promise<number>;
function showAlertDialog(window: Window, options: AlertDialogOptions): Promise<number>;
function showAlertDialog(
  windowOrOptions: Window | AlertDialogOptions,
  maybeOptions?: AlertDialogOptions,
): Promise<number> {
  const hasWindow = windowOrOptions instanceof Window;
  const window = hasWindow ? windowOrOptions : undefined;
  const options = hasWindow ? maybeOptions : (windowOrOptions as AlertDialogOptions);
  if (!options) return Promise.reject(new TypeError("showAlertDialog requires options"));
  const app = window?.app ?? activeApp;
  if (!app) return Promise.reject(new Error("create a QuickGUI App before showing a dialog"));
  return app._showAlertDialog(window, options);
}

function showOpenDialog(options?: OpenDialogOptions): Promise<OpenDialogResult>;
function showOpenDialog(window: Window, options?: OpenDialogOptions): Promise<OpenDialogResult>;
function showOpenDialog(
  windowOrOptions: Window | OpenDialogOptions = {},
  maybeOptions: OpenDialogOptions = {},
): Promise<OpenDialogResult> {
  const hasWindow = windowOrOptions instanceof Window;
  const window = hasWindow ? windowOrOptions : undefined;
  const options = hasWindow ? maybeOptions : (windowOrOptions as OpenDialogOptions);
  const app = window?.app ?? activeApp;
  if (!app) return Promise.reject(new Error("create a QuickGUI App before showing a dialog"));
  return app._showOpenDialog(window, options);
}

function showSaveDialog(options?: SaveDialogOptions): Promise<SaveDialogResult>;
function showSaveDialog(window: Window, options?: SaveDialogOptions): Promise<SaveDialogResult>;
function showSaveDialog(
  windowOrOptions: Window | SaveDialogOptions = {},
  maybeOptions: SaveDialogOptions = {},
): Promise<SaveDialogResult> {
  const hasWindow = windowOrOptions instanceof Window;
  const window = hasWindow ? windowOrOptions : undefined;
  const options = hasWindow ? maybeOptions : (windowOrOptions as SaveDialogOptions);
  const app = window?.app ?? activeApp;
  if (!app) return Promise.reject(new Error("create a QuickGUI App before showing a dialog"));
  return app._showSaveDialog(window, options);
}

/** Platform-native dialogs. Pass a Window first to attach the dialog; omit it for app-modal UI. */
export const Dialog = Object.freeze({
  showAlertDialog,
  showOpenDialog,
  showSaveDialog,
});

function quitReason(value: string | undefined): QuitReason {
  return value === "relaunch" || value === "last-window-closed" || value === "operating-system"
    ? value
    : "explicit";
}

function asError(error: unknown): Error {
  return error instanceof Error ? error : new Error(String(error));
}

/** Parse one bounded native window-size notification, ignoring anything malformed. */
function parseWindowSize(value: string | undefined): Size | undefined {
  const parsed = parseWindowGeometry(value);
  if (!parsed) return undefined;
  const { width, height } = parsed;
  return typeof width === "number" && typeof height === "number" ? { width, height } : undefined;
}

/** Parse one bounded native window-position notification, ignoring anything malformed. */
function parseWindowPoint(value: string | undefined): Point | undefined {
  const parsed = parseWindowGeometry(value);
  if (!parsed) return undefined;
  const { x, y } = parsed;
  return typeof x === "number" && typeof y === "number" ? { x, y } : undefined;
}

function parseWindowGeometry(value: string | undefined): Record<string, unknown> | undefined {
  if (value === undefined) return undefined;
  try {
    const parsed: unknown = JSON.parse(value);
    return typeof parsed === "object" && parsed !== null
      ? (parsed as Record<string, unknown>)
      : undefined;
  } catch {
    return undefined;
  }
}

export type Application = App;
export const app = new App();

export { ExtensionSession, invokeExtension } from "./extension.ts";
export { parseColor as color } from "./native-tree.ts";

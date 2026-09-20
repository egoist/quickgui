/** The complete native facade over the shared Rust C ABI. All UI work is asynchronous. */
import { CString, JSCallback, dlopen } from "bun:ffi";
import { dirname, join } from "node:path";
import { abort, check, json, listen, loadLibrary, type Library } from "./ffi.ts";
import { PROTOCOL_VERSION } from "./protocol.generated.ts";
import type * as T from "./binding-types.ts";
export type * from "./binding-types.ts";

let library: Library | undefined;
let initialized = false;
let appId = 0;
let exitCode: number | undefined;
let nextRequest = 1;
let startupOptions: T.NativeAppOptions = {};
let failHandler: (error: unknown) => void = (error) => {
  if (library) abort(library, error);
  else throw error;
};
const started = Promise.withResolvers<void>();
const requests = new Map<number, ReturnType<typeof Promise.withResolvers<any>>>();
const events: T.NativeEvent[] = [];
const extensionEvents = new Map<number, (value: string) => void>();
let waiting: ReturnType<typeof Promise.withResolvers<T.HostedAppUpdate>> | undefined;
const windows = new Map<
  number,
  {
    ready: ReturnType<typeof Promise.withResolvers<void>>;
    mounted: boolean;
    submitted: boolean;
    parent?: number;
    closing: boolean;
    work: (() => void)[];
  }
>();
const extensionLibraries: ReturnType<typeof dlopen>[] = [];

function backend(): Library {
  return (library ??= loadLibrary());
}
export function protocolVersion(): number {
  return PROTOCOL_VERSION;
}
export function initialize(
  options: T.NativeAppOptions,
  fail: (error: unknown) => void,
  extensions: { name: string; library: string; version: string }[] = [],
) {
  if (initialized) throw new Error("QuickGUI transport already initialized");
  initialized = true;
  startupOptions = options;
  failHandler = fail;
  const native = backend();
  for (const extension of extensions) {
    const folder =
      process.platform === "darwin"
        ? join(dirname(process.execPath), "..", "Frameworks")
        : dirname(process.execPath);
    const provider = dlopen(join(folder, extension.library), {
      quickgui_extension_v1: { args: [], returns: "ptr" },
    });
    const name = Buffer.from(extension.name),
      version = Buffer.from(extension.version);
    check(
      native.symbols.quickgui_register_extension_versioned(
        provider.symbols.quickgui_extension_v1(),
        name,
        name.length,
        version,
        version.length,
      ),
    );
    extensionLibraries.push(provider);
  }
  listen(
    native,
    (incoming) => {
      const event: T.NativeEvent = {
        kind: incoming.kind,
        window: incoming.window,
        target: incoming.target,
        ...(incoming.value === undefined ? {} : { value: incoming.value }),
        ...(incoming.extra ?? {}),
        ...(incoming.data ? { data: Buffer.from(incoming.data) } : {}),
      };
      if (["command", "invoke", "app-ready"].includes(event.kind)) {
        const request = requests.get(event.target);
        requests.delete(event.target);
        if (event.error) request?.reject(new Error(event.error));
        else request?.resolve(event.value === undefined ? undefined : JSON.parse(event.value));
        return;
      }
      if (event.kind === "host-error") {
        finish(1);
        failHandler(new Error(event.error ?? "Native host failed"));
        return;
      }
      if (event.kind === "extension-event") {
        extensionEvents.get(event.target)?.(event.value ?? "null");
        return;
      }
      const window = windows.get(event.window);
      if (event.kind === "window-created" && window) {
        window.mounted = true;
        for (const work of window.work.splice(0)) if (!window.closing) work();
        window.ready.resolve();
      } else if (event.kind === "close" && window) {
        window.closing = true;
        window.work.length = 0;
        window.ready.reject(new Error("Window closed before mounting"));
        windows.delete(event.window);
        for (const [id, child] of windows)
          if (child.parent === event.window && !child.submitted) closeHostedWindow(appId, id);
      }
      events.push(event);
      if (event.kind === "exit") exitCode = event.target;
      if (events.length > 8192) {
        failHandler(new Error("QuickGUI application event queue is full"));
        return;
      }
      wake();
    },
    fail,
  );
  started.resolve();
}
function wake() {
  if (!waiting || (!events.length && exitCode === undefined)) return;
  const next = waiting;
  waiting = undefined;
  next.resolve({ events: events.splice(0), ...(exitCode === undefined ? {} : { exitCode }) });
}
export function finish(code: number) {
  exitCode ??= code;
  library?.symbols.quickgui_clear_event_notifier();
  for (const state of windows.values()) {
    state.work.length = 0;
    state.ready.reject(new Error("Application exited"));
  }
  windows.clear();
  // Native exit requests normally resolve before exit. Reject every genuinely unfinished call.
  for (const request of requests.values()) request.reject(new Error("Application exited"));
  requests.clear();
  extensionEvents.clear();
  wake();
}
export function takeEvents(_app: number): T.NativeEvent[] {
  return events.splice(0);
}
export function waitForHostedEvents(_app: number): Promise<T.HostedAppUpdate> {
  if (waiting) throw new Error("Only one application event consumer is allowed");
  waiting = Promise.withResolvers<T.HostedAppUpdate>();
  const promise = waiting.promise;
  wake();
  return promise;
}
function request<T>(submit: (id: number) => void): Promise<T> {
  if (exitCode !== undefined) return Promise.reject(new Error("Application exited"));
  if (requests.size >= 8192 || nextRequest >= 0xffffffff)
    throw new Error("Too many native requests");
  const id = nextRequest++,
    result = Promise.withResolvers<T>();
  requests.set(id, result);
  try {
    submit(id);
  } catch (error) {
    requests.delete(id);
    result.reject(error);
  }
  return result.promise;
}
export function command<T = void>(app: number, value: object): Promise<T> {
  return request((id) => {
    const bytes = json(value);
    check(backend().symbols.quickgui_command(app, id, bytes, bytes.length));
  });
}
function mutation(app: number, value: object) {
  if (exitCode !== undefined) return;
  const bytes = json(value);
  check(backend().symbols.quickgui_command(app, 0, bytes, bytes.length));
}
export function invoke<T = void>(method: string, value: unknown = null): Promise<T> {
  return request((id) => {
    const name = Buffer.from(method),
      bytes = json(value);
    check(backend().symbols.quickgui_invoke(id, name, name.length, bytes, bytes.length));
  });
}
export function startExtension(name: string, options: unknown, changed: (value: string) => void) {
  let session = 0;
  const ready = request<void>((id) => {
    session = id;
    extensionEvents.set(id, changed);
    const method = Buffer.from(`extension/${name}/start`),
      bytes = json(options);
    check(backend().symbols.quickgui_invoke(id, method, method.length, bytes, bytes.length));
  });
  void ready.catch(() => extensionEvents.delete(session));
  return { session, ready };
}
export function stopExtension(name: string, session: number): Promise<void> {
  extensionEvents.delete(session);
  return exitCode === undefined ? invoke(`extension/${name}/stop`, { session }) : Promise.resolve();
}
export function call<T>(method: string, value: unknown = null): T {
  let result: { ok: boolean; value?: T; error?: string } | undefined;
  const reply = new JSCallback(
    (pointer, length) => {
      result = JSON.parse(String(new CString(pointer, 0, Number(length))));
    },
    { args: ["ptr", "usize", "ptr"], returns: "void" },
  );
  try {
    const name = Buffer.from(method),
      bytes = json(value);
    check(backend().symbols.quickgui_call(name, name.length, bytes, bytes.length, reply.ptr, null));
  } finally {
    reply.close();
  }
  if (!result?.ok) throw new Error(result?.error ?? `No native reply for ${method}`);
  return result.value as T;
}

export function createHostedApp(options?: T.NativeAppOptions): number {
  if (!initialized) return 1; // Imports remain safe in headless component/model tests.
  const bytes = json({ ...startupOptions, ...options });
  appId = backend().symbols.quickgui_create_app(bytes, bytes.length);
  if (!appId) throw new Error("Native application creation failed");
  return appId;
}
export async function prepareHostedApp(app: number): Promise<void> {
  if (!initialized)
    throw new Error("Launch native TypeScript applications with quickgui dev or quickgui build");
  await started.promise;
  if (!appId) createHostedApp();
  await request((id) => check(backend().symbols.quickgui_prepare_app(app, id)));
}
export const configureHostedApp = (app: number, options: T.NativeAppOptions) =>
  command(app, { method: "configure-app", options });
function createWindow(
  app: number,
  options: T.NativeWindowOptions | null | undefined,
  batch: Buffer | null | undefined,
  owner?: { parent: number; anchor?: number; horizontal?: boolean; vertical?: boolean },
): number {
  const native = backend().symbols,
    id = native.quickgui_allocate_window();
  if (!id) throw new Error("Window allocation failed");
  const ready = Promise.withResolvers<void>();
  void ready.promise.catch(() => {});
  const state = {
    ready,
    mounted: false,
    submitted: false,
    closing: false,
    work: [] as (() => void)[],
    ...(owner ? { parent: owner.parent } : {}),
  };
  windows.set(id, state);
  const bytes = json(options ?? {}),
    initial = batch ?? Buffer.alloc(0);
  const submit = () => {
    if (state.closing) return;
    state.submitted = true;
    if (owner?.anchor !== undefined)
      check(
        native.quickgui_create_system_popover(
          app,
          id,
          owner.parent,
          owner.anchor,
          bytes,
          bytes.length,
          initial,
          initial.length,
        ),
      );
    else if (owner)
      check(
        native.quickgui_create_embedded_view(
          app,
          id,
          owner.parent,
          Number(owner.horizontal),
          Number(owner.vertical),
          bytes,
          bytes.length,
          initial,
          initial.length,
        ),
      );
    else
      check(native.quickgui_create_window(app, id, bytes, bytes.length, initial, initial.length));
  };
  if (owner) windowWork(owner.parent, submit);
  else submit();
  return id;
}
export const createHostedWindow = (
  app: number,
  options?: T.NativeWindowOptions | null,
  batch?: Buffer | null,
) => createWindow(app, options, batch);
export const createHostedSystemPopover = (
  app: number,
  parent: number,
  anchor: number,
  options?: T.NativeWindowOptions | null,
  batch?: Buffer | null,
) => createWindow(app, options, batch, { parent, anchor });
export const createHostedEmbeddedView = (
  app: number,
  parent: number,
  horizontal: boolean,
  vertical: boolean,
  options?: T.NativeWindowOptions | null,
  batch?: Buffer | null,
) => createWindow(app, options, batch, { parent, horizontal, vertical });
export function windowReady(id: number): Promise<void> {
  return windows.get(id)?.ready.promise ?? Promise.reject(new Error("Window is closed"));
}
function windowWork(id: number, work: () => void) {
  const state = windows.get(id);
  if (!state || state.closing) return;
  if (state.mounted) work();
  else {
    if (state.work.length >= 8192) throw new Error("Too many pending window mutations");
    state.work.push(work);
  }
}
export function applyHostedBatch(app: number, window: number, batch: Buffer): number {
  if (batch.length)
    windowWork(window, () =>
      check(backend().symbols.quickgui_apply_batch(app, window, batch, batch.length)),
    );
  return 0;
}
export function closeHostedWindow(app: number, window: number): void {
  const state = windows.get(window);
  if (!state || state.closing) return;
  state.closing = true;
  state.work.length = 0;
  for (const [id, child] of windows) if (child.parent === window) closeHostedWindow(app, id);
  if (!state.submitted) {
    state.ready.reject(new Error("Window closed before mounting"));
    windows.delete(window);
    events.push({ kind: "close", window, target: 0 });
    wake();
    return;
  }
  check(backend().symbols.quickgui_close_window(app, window));
}
export function focusHostedNode(app: number, window: number, node: number): void {
  windowWork(window, () => check(backend().symbols.quickgui_focus_node(app, window, node)));
}
export function destroyHostedApp(app: number): void {
  if (exitCode === undefined) check(backend().symbols.quickgui_destroy_app(app));
}
export const exitHostedApp = (app: number) => command<boolean>(app, { method: "exit" });
export const exitHostedAppWithCode = (app: number, code: number) =>
  command<boolean>(app, { method: "exit-with-code", code });
export const requestHostedAppQuit = (app: number) =>
  command<boolean>(app, { method: "request-quit" });
export const setHostedQuitInterception = (app: number, intercepting: boolean) =>
  mutation(app, { method: "set-quit-interception", intercepting });
export const requestHostedSingleInstanceLock = (app: number, identifier: string) =>
  command<boolean>(app, { method: "request-single-instance-lock", identifier });
export const releaseHostedSingleInstanceLock = (app: number) =>
  exitCode === undefined
    ? command<boolean>(app, { method: "release-single-instance-lock" })
    : Promise.resolve(false);
export const relaunchHostedApp = (app: number, options: T.NativeRelaunchOptions) =>
  command<boolean>(app, { method: "relaunch", options });

export const getHostedAppInfo = (app: number) =>
  command<T.NativeAppInfo | null>(app, { method: "get-app-info" });
export const getHostedAppPaths = (app: number) =>
  command<T.NativeAppPaths | null>(app, { method: "get-app-paths" });
export const getHostedSystemInfo = (app: number) =>
  command<T.NativeSystemInfo>(app, { method: "get-system-info" });
export const getHostedApplicationsFolderSupport = (app: number) =>
  command<T.NativeApplicationsFolderSupport>(app, { method: "get-applications-folder-support" });
export const getHostedWindowRegistry = (app: number) =>
  command<T.NativeWindowRegistry>(app, { method: "get-window-registry" });
export const getHostedDisplays = (app: number) =>
  command<T.NativeDisplay[]>(app, { method: "get-displays" });
export const getHostedCursorScreenPosition = (app: number) =>
  command<T.NativePoint>(app, { method: "get-cursor-screen-position" });
export const getHostedDesktopIntegrationSupport = (app: number) =>
  command<T.NativeDesktopIntegrationSupport>(app, { method: "get-desktop-integration-support" });
export const getHostedSystemPreferences = (app: number) =>
  command<T.NativeSystemPreferences>(app, { method: "get-system-preferences" });
export const getHostedKeyboardLayout = (app: number) =>
  command<T.NativeKeyboardLayout>(app, { method: "get-keyboard-layout" });
export async function getHostedWindowState(
  app: number,
  window: number,
): Promise<T.NativeWindowState> {
  await windowReady(window);
  return command(app, { method: "get-window-state", window });
}
export async function getHostedFrameMetrics(
  app: number,
  window: number,
): Promise<T.NativeFrameMetrics> {
  await windowReady(window);
  return command(app, { method: "get-window-frame-metrics", window });
}
export async function getHostedWindowRestoreState(
  app: number,
  window: number,
): Promise<T.NativeWindowRestoreState> {
  await windowReady(window);
  return command(app, { method: "get-window-restore-state", window });
}
export const performHostedWindowAction = (
  app: number,
  window: number,
  action: string,
  value?: string | null,
) => windowWork(window, () => mutation(app, { method: "window-action", window, action, value }));
export const performHostedWindowImageAction = (
  app: number,
  window: number,
  action: string,
  image?: T.NativeImageSource | null,
  description?: string | null,
) =>
  windowWork(window, () =>
    mutation(app, { method: "window-image-action", window, action, image, description }),
  );
export const performHostedAppMutation = (app: number, action: string, value?: string | null) =>
  mutation(app, { method: "app-mutation", action, value });
export const performHostedAppService = (
  app: number,
  request: number,
  action: string,
  value?: string | null,
) => command(app, { method: "app-service", request, action, value });
export const performHostedShellAction = (
  app: number,
  request: number,
  action: string,
  value: string,
) => command(app, { method: "shell-action", request, action, value });
export const performHostedGlobalShortcutAction = (
  app: number,
  request: number,
  action: string,
  registration?: number | null,
  accelerator?: string | null,
) => command(app, { method: "global-shortcut", request, action, registration, accelerator });
export const performHostedNotificationPermissionRequest = (
  app: number,
  request: number,
  prompt: boolean,
) => command(app, { method: "notification-permission", request, prompt });
export const requestHostedFileIcon = (app: number, request: number, path: string, size: string) =>
  command(app, { method: "file-icon", request, path, size });
export const readHostedClipboard = (app: number) =>
  command<T.NativeClipboardItem | null>(app, { method: "read-clipboard" });
export const readHostedFindClipboard = (app: number) =>
  command<T.NativeClipboardItem | null>(app, { method: "read-find-clipboard" });
export const writeHostedClipboard = (app: number, item: T.NativeClipboardItem) =>
  command(app, { method: "write-clipboard", item });
export const writeHostedFindClipboard = (app: number, item: T.NativeClipboardItem) =>
  command(app, { method: "write-find-clipboard", item });
export const setHostedApplicationMenu = (app: number, menu: string) =>
  mutation(app, { method: "set-application-menu", menu });
export const setHostedDockMenu = (app: number, menu?: string | null) =>
  mutation(app, { method: "set-dock-menu", menu });
export const setHostedDockIcon = (app: number, icon?: T.NativeImageSource | null) =>
  mutation(app, { method: "set-dock-icon", icon });
export const setHostedDockBadge = (app: number, value?: string | null) =>
  mutation(app, { method: "set-dock-badge", value });
export const addHostedRecentDocument = (app: number, path: string) =>
  mutation(app, { method: "add-recent-document", path });
export const clearHostedRecentDocuments = (app: number) =>
  mutation(app, { method: "clear-recent-documents" });
export const showHostedAboutPanel = (app: number, options: T.NativeAboutPanelOptions) =>
  mutation(app, { method: "show-about-panel", options });
export const showHostedNotification = (app: number, options: T.NativeNotificationOptions) =>
  command(app, { method: "show-notification", options });
export const dismissHostedNotification = (app: number, tag: string) =>
  command(app, { method: "dismiss-notification", tag });
export const setHostedUserTasks = (app: number, request: number, tasks: T.NativeUserTask[]) =>
  command(app, { method: "set-user-tasks", request, tasks });
export const setHostedTrayIcon = (app: number, request: number, options: T.NativeTrayIconOptions) =>
  command(app, { method: "set-tray-icon", request, options });
export const removeHostedTrayIcon = (app: number, request: number, id: number) =>
  command(app, { method: "remove-tray-icon", request, id });
export const showHostedTrayMenu = (app: number, request: number, id: number) =>
  command(app, { method: "show-tray-menu", request, id });
export const showHostedWindowPopupMenu = (
  app: number,
  request: number,
  window: number,
  menu: string,
  x?: number | null,
  y?: number | null,
) => command(app, { method: "window-popup-menu", request, window, menu, x, y });
async function dialog(
  app: number,
  window: number | null | undefined,
  request: number,
  kind: number,
  options: object,
) {
  if (window) await windowReady(window);
  const bytes = json(options);
  check(
    backend().symbols.quickgui_show_dialog(app, window ?? 0, request, kind, bytes, bytes.length),
  );
}
export const showHostedAlertDialog = (
  app: number,
  window: number | null | undefined,
  request: number,
  options: T.NativeDialogOptions,
) => dialog(app, window, request, 0, options);
export const showHostedOpenDialog = (
  app: number,
  window: number | null | undefined,
  request: number,
  options: T.NativeOpenDialogOptions,
) => dialog(app, window, request, 1, options);
export const showHostedSaveDialog = (
  app: number,
  window: number | null | undefined,
  request: number,
  options: T.NativeSaveDialogOptions,
) => dialog(app, window, request, 2, options);

export const isApplicationPackaged = () => call<boolean>("is-application-packaged");
export const isAutoStartSupported = () => call<boolean>("is-auto-start-supported");
export const supportsDynamicProtocolRegistration = () =>
  call<boolean>("supports-dynamic-protocol-registration");
export const isSecureStorageSupported = () => call<boolean>("is-secure-storage-supported");
export const isCrashReporterStarted = () => call<boolean>("is-crash-reporter-started");
export const getPowerState = () => call<T.NativePowerState>("get-power-state");
export const getSystemIdleTime = () => call<number>("get-system-idle-time");
export const getSystemIdleState = (thresholdSeconds: number) =>
  call<string>("get-system-idle-state", { thresholdSeconds });
export const getSessionState = () => call<string>("get-session-state");
export const getPermissionStatus = (kind: string) =>
  call<string>("get-permission-status", { kind });
export const requestPermission = (kind: string) => invoke<string>("request-permission", { kind });
export const enableAutoStart = (options: T.NativeAutoStartOptions) =>
  invoke("enable-auto-start", options);
export const disableAutoStart = (options: T.NativeAutoStartOptions) =>
  invoke("disable-auto-start", options);
export const isAutoStartEnabled = (options: T.NativeAutoStartOptions) =>
  invoke<boolean>("is-auto-start-enabled", options);
export const registerProtocol = (options: T.NativeProtocolRegistrationOptions) =>
  invoke("register-protocol", options);
export const unregisterProtocol = (options: T.NativeProtocolRegistrationOptions) =>
  invoke<boolean>("unregister-protocol", options);
export const isProtocolRegistered = (options: T.NativeProtocolRegistrationOptions) =>
  invoke<boolean>("is-protocol-registered", options);
export const setSecureStorage = (service: string, account: string, secret: Buffer) =>
  invoke("set-secure-storage", { service, account, value: [...secret] });
export const getSecureStorage = async (service: string, account: string) => {
  const value = await invoke<string | null>("get-secure-storage", { service, account });
  return value == null ? undefined : Buffer.from(value, "base64");
};
export const deleteSecureStorage = (service: string, account: string) =>
  invoke<boolean>("delete-secure-storage", { service, account });
export const startCrashReporter = (options: T.NativeCrashReporterOptions) =>
  invoke<string>("start-crash-reporter", options);
export const getLastCrashReport = () => invoke<T.NativeCrashReport[]>("get-last-crash-report");
export const getPendingCrashReports = () =>
  invoke<T.NativeCrashReport[]>("get-pending-crash-reports");
export const addCrashExtraParameter = (key: string, value: string) =>
  invoke<boolean>("add-crash-extra-parameter", { key, value });
export const removeCrashExtraParameter = (key: string) =>
  invoke<boolean>("remove-crash-extra-parameter", { key });
export const deleteCrashReport = (id: string) => invoke<boolean>("delete-crash-report", { id });
export const uploadPendingCrashReports = (endpoint?: string | null) =>
  invoke<T.NativeCrashUploadSummary>("upload-pending-crash-reports", { endpoint });
export const getProcessMetrics = () => invoke<T.NativeProcessMetrics>("get-process-metrics");
export const getSystemMemory = () => invoke<T.NativeSystemMemory>("get-system-memory");

const resources = new FinalizationRegistry<{ method: string; id: number }>((resource) => {
  if (exitCode === undefined)
    try {
      call(resource.method, { id: resource.id });
    } catch {
      /* Runtime teardown. */
    }
});
export class NativeRouter {
  private id: number;
  constructor(routes: T.NativeRouteDefinition[], initialDestination?: string | null) {
    this.id = call("router-create", { routes, initialDestination });
    resources.register(this, { method: "router-release", id: this.id }, this);
  }
  private use<T>(method: string, args: object = {}): T {
    if (!this.id) throw new Error("Router disposed");
    return call(method, { id: this.id, ...args });
  }
  state() {
    return this.use<T.NativeRouterState>("router-state");
  }
  resolve(destination: string) {
    return this.use<T.NativeRouteLocation>("router-resolve", { destination });
  }
  isActive(destination: string, end = false) {
    return this.use<boolean>("router-is-active", { destination, end });
  }
  push(destination: string) {
    return this.use<T.NativeRouterState>("router-push", { destination });
  }
  replace(destination: string) {
    return this.use<T.NativeRouterState>("router-replace", { destination });
  }
  go(delta: number) {
    return this.use<T.NativeRouterState>("router-go", { delta });
  }
  back() {
    return this.use<T.NativeRouterState>("router-back");
  }
  forward() {
    return this.use<T.NativeRouterState>("router-forward");
  }
  dispose() {
    if (this.id) {
      this.use("router-release");
      resources.unregister(this);
      this.id = 0;
    }
  }
}
export class NativePowerAssertion {
  private id: number;
  constructor(
    readonly kind: string,
    readonly reason: string,
  ) {
    this.id = call("power-assertion-acquire", { kind, reason });
    resources.register(this, { method: "power-assertion-release", id: this.id }, this);
  }
  get active() {
    return (
      this.id !== 0 && call<{ active: boolean }>("power-assertion-info", { id: this.id }).active
    );
  }
  release() {
    if (!this.id) return false;
    const result = call<boolean>("power-assertion-release", { id: this.id });
    resources.unregister(this);
    this.id = 0;
    return result;
  }
}
export class NativeCpuUsageSampler {
  private id = call<number>("cpu-sampler-create");
  constructor() {
    resources.register(this, { method: "cpu-sampler-release", id: this.id }, this);
  }
  async sample() {
    if (!this.id) throw new Error("Sampler disposed");
    return call<T.NativeCpuUsage>("cpu-sampler-sample", { id: this.id });
  }
  dispose() {
    if (this.id) {
      call("cpu-sampler-release", { id: this.id });
      resources.unregister(this);
      this.id = 0;
    }
  }
}

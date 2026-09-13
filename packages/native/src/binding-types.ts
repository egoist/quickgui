/** Wire DTOs retained from the native ABI; runtime calls are implemented in binding.ts. */
/* eslint-disable */
/** Stateful CPU sampler. Each `sample()` reports usage since the previous call on this instance. */

/**
 * Synchronous binding for core-owned, CPU-only route matching and memory history.
 *
 * This object never reaches the application or window runtime and therefore never waits for the
 * native main thread. Solid turns the snapshots returned by its mutation methods into signals.
 */

export interface HostedAppUpdate {
  events: Array<NativeEvent>;
  exitCode?: number;
}

/** Whether this process is running from an installed application bundle. */

export interface NativeAboutPanelOptions {
  applicationName?: string;
  applicationVersion?: string;
  version?: string;
  copyright?: string;
  credits?: string;
  icon?: NativeImageSource;
}

export interface NativeAppInfo {
  name: string;
  version: string;
  identifier: string;
}

/** Whether this process can relocate its bundle into an `/Applications` directory. */
export interface NativeApplicationsFolderSupport {
  supported: boolean;
  alreadyInstalled: boolean;
}

export interface NativeAppOptions {
  name?: string;
  version?: string;
  identifier?: string;
  resourceDir?: string;
  configDir?: string;
  dataDir?: string;
  localDataDir?: string;
  cacheDir?: string;
  logDir?: string;
  runtimeDir?: string;
  tempDir?: string;
  /** `default`, `last-window-closed`, or `explicit`. */
  quitMode?: string;
  /** OpenType font files embedded by the JavaScript host and registered by the Rust core. */
  fonts?: Array<string>;
}

export interface NativeAppPaths {
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

export interface NativeAutoStartOptions {
  appName: string;
  executable?: string;
  arguments?: Array<string>;
  mode?: string;
  bundleIdentifier?: string;
}

export interface NativeAvailableUpdate {
  version: string;
  currentVersion: string;
  target: string;
  url: string;
  signature: string;
  notes?: string;
  publishedAt?: string;
}

export interface NativeBatteryState {
  chargePercent?: number;
  status: string;
}

export interface NativeClipboardEntry {
  kind: string;
  text?: string;
  metadata?: string;
  format?: string;
  data?: Buffer;
  paths?: Array<string>;
  url?: string;
}

export interface NativeClipboardItem {
  entries: Array<NativeClipboardEntry>;
}

export interface NativeCpuUsage {
  percent?: number;
  intervalSeconds: number;
  cpuSeconds: number;
  totalCpuSeconds: number;
}

export interface NativeCrashLocation {
  file: string;
  line: number;
  column: number;
}

export interface NativeCrashParameter {
  key: string;
  value: string;
}

export interface NativeCrashReport {
  schemaVersion: number;
  id: string;
  kind: string;
  timestamp: string;
  appName: string;
  appVersion: string;
  appIdentifier: string;
  operatingSystem: string;
  operatingSystemVersion?: string;
  architecture: string;
  processId: number;
  thread?: string;
  message: string;
  location?: NativeCrashLocation;
  backtrace?: string;
  signal?: number;
  signalName?: string;
  faultAddress?: string;
  parameters: Array<NativeCrashParameter>;
}

export interface NativeCrashReporterOptions {
  appName: string;
  appVersion: string;
  appIdentifier: string;
  directory?: string;
  maxReports?: number;
  maxReportBytes?: number;
  parameters?: Array<NativeCrashParameter>;
  uploadEndpoint?: string;
  backtrace?: string;
  captureSignals?: boolean;
}

export interface NativeCrashUploadSummary {
  attempted: number;
  uploaded: number;
  failed: number;
}

export interface NativeDesktopIntegrationSupport {
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

export interface NativeDialogButton {
  label: string;
  role?: string;
}

export interface NativeDialogOptions {
  level?: string;
  message: string;
  detail?: string;
  buttons: Array<NativeDialogButton>;
}

export interface NativeDisplay {
  id: string;
  uuid?: string;
  name: string;
  bounds: NativeRect;
  workArea: NativeRect;
  scaleFactor: number;
  refreshRate?: number;
  primary: boolean;
}

export interface NativeEvent {
  kind: string;
  window: number;
  target: number;
  value?: string;
  paths?: Array<string>;
  data?: Buffer;
  width?: number;
  height?: number;
  error?: string;
}

export interface NativeFileDialogFilter {
  name: string;
  extensions: Array<string>;
}

export interface NativeImageSource {
  /** Encoded image bytes, or raw RGBA8 when width and height are both supplied. */
  data?: Buffer;
  /** Image path used when data is omitted. */
  path?: string;
  width?: number;
  height?: number;
  /** macOS template-image flag. When omitted, `*Template.png` paths are inferred. */
  template?: boolean;
}

export interface NativeInstalledUpdate {
  version: string;
  disposition: string;
  installedPath: string;
  backupPath?: string;
  installerProcessId?: number;
  requiresApplicationExit: boolean;
  relaunchRecommended: boolean;
}

export interface NativeKeyboardLayout {
  id: string;
  name: string;
}

export interface NativeNotificationAction {
  id: string;
  label: string;
  kind?: string;
  placeholder?: string;
}

export interface NativeNotificationAttachment {
  id: string;
  path: string;
}

export interface NativeNotificationOptions {
  tag: string;
  title: string;
  body: string;
  subtitle?: string;
  actions: Array<NativeNotificationAction>;
  /** `default`, `silent`, or a platform-recognized named sound. */
  sound?: string;
  iconPath?: string;
  attachments?: Array<NativeNotificationAttachment>;
  /** Absolute Unix epoch milliseconds. */
  deliveryAtMs?: number;
}

export interface NativeOpenDialogOptions {
  files: boolean;
  directories: boolean;
  multiple: boolean;
  title?: string;
  prompt?: string;
  directory?: string;
  suggestedName?: string;
  filters: Array<NativeFileDialogFilter>;
  showsHiddenFiles: boolean;
}

export interface NativePoint {
  x: number;
  y: number;
}

export interface NativePowerState {
  source: string;
  battery?: NativeBatteryState;
  thermalState: string;
  lowPowerMode?: boolean;
  cpuSpeedLimitPercent?: number;
}

export interface NativeProcessMetrics {
  cpuUserSeconds: number;
  cpuSystemSeconds: number;
  residentBytes: number;
  footprintBytes?: number;
  virtualBytes: number;
  threadCount?: number;
  uptimeSeconds: number;
}

export interface NativeProtocolRegistrationOptions {
  scheme: string;
  appName: string;
  appId: string;
  executable?: string;
  arguments?: Array<string>;
}

export interface NativeRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface NativeRelaunchOptions {
  executable?: string;
  arguments?: Array<string>;
  clearArguments?: boolean;
  workingDirectory?: string;
}

export interface NativeRouteDefinition {
  id: string;
  path?: string;
  parentId?: string;
}

export interface NativeRouteLocation {
  href: string;
  pathname: string;
  search: string;
  hash: string;
  query: Array<NativeRouteValue>;
}

export interface NativeRouteMatch {
  routeIds: Array<string>;
  params: Array<NativeRouteValue>;
}

export interface NativeRouterState {
  location: NativeRouteLocation;
  matched?: NativeRouteMatch;
  historyIndex: number;
  historyLength: number;
  canGoBack: boolean;
  canGoForward: boolean;
}

export interface NativeRouteValue {
  name: string;
  value: string;
}

export interface NativeSaveDialogOptions {
  directory: string;
  title?: string;
  suggestedName?: string;
  prompt?: string;
  filters: Array<NativeFileDialogFilter>;
  showsHiddenFiles: boolean;
}

export interface NativeSystemColor {
  red: number;
  green: number;
  blue: number;
  alpha: number;
}

export interface NativeSystemInfo {
  operatingSystem: string;
  family: string;
  name: string;
  version?: string;
  edition?: string;
  codename?: string;
  architecture: string;
  bitness: string;
  hostname?: string;
  locale?: string;
  preferredLanguages: Array<string>;
  languagesTruncated: boolean;
}

export interface NativeSystemMemory {
  totalBytes: number;
  availableBytes: number;
  freeBytes: number;
  usedBytes: number;
}

export interface NativeSystemPreferences {
  colorScheme: string;
  reduceMotion?: boolean;
  reduceTransparency?: boolean;
  increaseContrast?: boolean;
  differentiateWithoutColor?: boolean;
  invertColors?: boolean;
  forcedColors?: boolean;
  screenReader?: boolean;
  switchControl?: boolean;
  accentColor?: NativeSystemColor;
  highlightColor?: NativeSystemColor;
  highlightTextColor?: NativeSystemColor;
  windowBackgroundColor?: NativeSystemColor;
  windowTextColor?: NativeSystemColor;
  controlBackgroundColor?: NativeSystemColor;
  controlTextColor?: NativeSystemColor;
  linkColor?: NativeSystemColor;
}

export interface NativeTrayIconOptions {
  id: number;
  /** Encoded image bytes, or raw RGBA8 when width and height are both supplied. */
  iconData?: Buffer;
  /** Image path used when iconData is omitted. */
  iconPath?: string;
  width?: number;
  height?: number;
  tooltip?: string;
  title?: string;
  iconIsTemplate?: boolean;
  menuOnLeftClick?: boolean;
  visible?: boolean;
  /** Bounded JSON encoding of the declarative tray menu. */
  menu: string;
}

export interface NativeUpdateClientOptions {
  currentVersion: string;
  publicKey: string;
  target?: string;
  maximumDownloadBytes?: number;
}

export interface NativeUpdateInstallOptions {
  targetExecutable?: string;
  retainBackup?: boolean;
  windowsMode?: string;
  installerArguments?: Array<string>;
}

export interface NativeUpdateProgress {
  phase: string;
  chunkBytes?: number;
  downloadedBytes?: number;
  totalBytes?: number;
  path?: string;
}

export interface NativeUserTask {
  title: string;
  arguments: string;
  program?: string;
  description?: string;
  workingDirectory?: string;
  iconPath?: string;
  iconIndex?: number;
}

export interface NativeWindowOptions {
  title?: string;
  width?: number;
  height?: number;
  x?: number;
  y?: number;
  /** `normal`, `maximized`, or `fullscreen`. */
  initialState?: string;
  displayId?: string;
  /** Set to false to remove QuickGUI's default minimum size. */
  minimumSizeEnabled?: boolean;
  minimumWidth?: number;
  minimumHeight?: number;
  maximumWidth?: number;
  maximumHeight?: number;
  representedFile?: string;
  documentEdited?: boolean;
  tabbingIdentifier?: string;
  background?: number;
  performanceProfile?: string;
  appearance?: string;
  vibrancy?: string;
  visualEffectState?: string;
  titleBarStyle?: string;
  kind?: string;
  focus?: boolean;
  focusable?: boolean;
  show?: boolean;
  movable?: boolean;
  resizable?: boolean;
  minimizable?: boolean;
  maximizable?: boolean;
  closable?: boolean;
  decorated?: boolean;
  shadow?: boolean;
  contentProtected?: boolean;
  windowLevel?: string;
  skipTaskbar?: boolean;
  visibleOnAllWorkspaces?: boolean;
  opacity?: number;
  icon?: NativeImageSource;
  taskbarProgressState?: string;
  taskbarProgress?: number;
  taskbarOverlayIcon?: NativeImageSource;
  taskbarOverlayDescription?: string;
  cursorVisible?: boolean;
  cursorGrab?: string;
  cursorHitTest?: boolean;
  cursorX?: number;
  cursorY?: number;
  /** Bounded JSON encoding of a per-window native menu. Omitted windows inherit the app menu. */
  menu?: string;
  /**
   * Persisted geometry and display identity captured with `window.getRestoreState()`.
   *
   * The core re-validates every field, so a stale value can never place a window off every
   * connected display.
   */
  restoreState?: NativeWindowRestoreState;
  lineScrollPixels?: number;
  keySequenceTimeoutMs?: number;
  reduceMotion?: boolean;
  trafficLightX?: number;
  trafficLightY?: number;
  transparent?: boolean;
  blur?: boolean;
  popoverPlacement?: string;
  popoverGap?: number;
  popoverOffsetX?: number;
  popoverOffsetY?: number;
  popoverViewportMargin?: number;
  popoverDismissOnEscape?: boolean;
  popoverDismissOnPointerOutside?: boolean;
  popoverGrab?: boolean;
  popoverAcceptsKeyFocus?: boolean;
}

export interface NativeWindowRegistry {
  windows: Array<number>;
  activeWindow?: number;
  truncated: boolean;
}

/**
 * Persistable window geometry and display identity.
 *
 * `displayUuid` is the textual form of the stable physical display identity, so a stored state
 * survives a reboot that renumbers process-level display ids.
 */
export interface NativeWindowRestoreState {
  x: number;
  y: number;
  width: number;
  height: number;
  maximized: boolean;
  fullscreen: boolean;
  displayId?: string;
  displayUuid?: string;
  scaleFactor: number;
}

export interface NativeWindowState {
  displayId?: string;
  kind: string;
  x: number;
  y: number;
  width: number;
  height: number;
  viewportWidth: number;
  viewportHeight: number;
  minimumWidth?: number;
  minimumHeight?: number;
  maximumWidth?: number;
  maximumHeight?: number;
  scaleFactor: number;
  appearance: string;
  backgroundAppearance: string;
  vibrancy?: string;
  visualEffectState: string;
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
  windowLevel: string;
  skipTaskbar: boolean;
  visibleOnAllWorkspaces: boolean;
  opacity: number;
  hasIcon: boolean;
  taskbarProgressState: string;
  taskbarProgress: number;
  hasTaskbarOverlayIcon: boolean;
  cursorVisible: boolean;
  cursorGrab: string;
  cursorHitTest: boolean;
  cursorX?: number;
  cursorY?: number;
  representedFile: boolean;
  documentEdited: boolean;
  nativeTabbing: boolean;
  nativeTabCount: number;
  nativeSelectedTab?: number;
  nativeTabBarVisible: boolean;
  nativeTabOverviewVisible: boolean;
  nativeTabsTruncated: boolean;
}

/** Apply one fire-and-forget application-shell mutation. */

/** Start one application-shell service whose outcome arrives as an `app-service` event. */

/** Return `-1` while running or the non-negative native exit code after termination. */

/** Present a native popup menu owned by one window; completion arrives as a `popup-menu` event. */

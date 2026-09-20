use std::{
    collections::HashMap,
    future::Future,
    path::PathBuf,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::{Duration, UNIX_EPOCH},
};

use quickgui::{
    AboutPanelOptions, AppInfo, AppPaths, ClipboardEntry, ClipboardImage, ClipboardImageFormat,
    ClipboardItem, ClipboardString, DesktopIntegrationSupport, Display, Displays, ExternalPaths,
    FileIconResponse, FileIconSize, Image, KeyboardLayout, MacOsVibrancy, MacOsVisualEffectState,
    Menu, NotificationPermissionResponse, NotificationPermissionStatus, PermissionKind,
    PermissionStatus, Point, PowerAssertion, PowerAssertionKind, PowerState, Rect, RelaunchOptions,
    ShellResponse, Size, SystemColor, SystemColorRole, SystemInfo, SystemNotification,
    SystemNotificationAction, SystemNotificationAttachment, SystemNotificationSound,
    SystemPreferences, TaskbarProgressState, UserTask, WindowAppearance,
    WindowBackgroundAppearance, WindowBounds, WindowLevel, WindowState,
};
use serde::{Deserialize, Serialize};

use super::{
    NativeAppOptions, NativeImageSource, NativeRuntime, QueuedEvent, ROOT_NODE, native_font_data,
    native_image, update_native_app_configuration,
};
use crate::runtime::{parse_macos_vibrancy, parse_macos_visual_effect_state};

pub(crate) mod menu;
mod tray;
pub use tray::NativeTrayIconOptions;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeDisplay {
    pub id: String,
    pub uuid: Option<String>,
    pub name: String,
    pub bounds: NativeRect,
    pub work_area: NativeRect,
    pub scale_factor: f64,
    pub refresh_rate: Option<f64>,
    pub primary: bool,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeKeyboardLayout {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativePoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeAppInfo {
    pub name: String,
    pub version: String,
    pub identifier: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeAppPaths {
    pub executable: String,
    pub executable_dir: String,
    pub resource_dir: String,
    pub home_dir: Option<String>,
    pub config_dir: Option<String>,
    pub data_dir: Option<String>,
    pub local_data_dir: Option<String>,
    pub cache_dir: Option<String>,
    pub log_dir: Option<String>,
    pub runtime_dir: Option<String>,
    pub temp_dir: String,
    pub audio_dir: Option<String>,
    pub desktop_dir: Option<String>,
    pub document_dir: Option<String>,
    pub download_dir: Option<String>,
    pub picture_dir: Option<String>,
    pub video_dir: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeSystemInfo {
    pub operating_system: String,
    pub family: String,
    pub name: String,
    pub version: Option<String>,
    pub edition: Option<String>,
    pub codename: Option<String>,
    pub architecture: String,
    pub bitness: String,
    pub hostname: Option<String>,
    pub locale: Option<String>,
    pub preferred_languages: Vec<String>,
    pub languages_truncated: bool,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeWindowRegistry {
    pub windows: Vec<u32>,
    pub active_window: Option<u32>,
    pub truncated: bool,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeDesktopIntegrationSupport {
    pub system_notifications: bool,
    pub scheduled_notifications: bool,
    pub notification_replies: bool,
    pub native_application_menus: bool,
    pub native_popup_menus: bool,
    pub tray_icons: bool,
    pub programmable_tray_popup: bool,
    pub global_shortcuts: bool,
    pub single_instance: bool,
    pub dynamic_protocol_registration: bool,
    pub autostart: bool,
    pub window_icons: bool,
    pub window_focusability: bool,
    pub window_opacity: bool,
    pub skip_taskbar: bool,
    pub visible_on_all_workspaces: bool,
    pub cursor_control: bool,
    pub cursor_screen_position: bool,
    pub taskbar_progress: bool,
    pub taskbar_overlay_icons: bool,
    pub dock_badges: bool,
    pub dock_icons: bool,
    pub dock_menus: bool,
    pub recent_documents: bool,
    pub file_icons: bool,
    pub native_about_panel: bool,
    pub user_tasks: bool,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeBatteryState {
    pub charge_percent: Option<u32>,
    pub status: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativePowerState {
    pub source: String,
    pub battery: Option<NativeBatteryState>,
    pub thermal_state: String,
    pub low_power_mode: Option<bool>,
    pub cpu_speed_limit_percent: Option<u32>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeSystemColor {
    pub red: u32,
    pub green: u32,
    pub blue: u32,
    pub alpha: u32,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeSystemPreferences {
    pub color_scheme: String,
    pub reduce_motion: Option<bool>,
    pub reduce_transparency: Option<bool>,
    pub increase_contrast: Option<bool>,
    pub differentiate_without_color: Option<bool>,
    pub invert_colors: Option<bool>,
    pub forced_colors: Option<bool>,
    pub screen_reader: Option<bool>,
    pub switch_control: Option<bool>,
    pub accent_color: Option<NativeSystemColor>,
    pub highlight_color: Option<NativeSystemColor>,
    pub highlight_text_color: Option<NativeSystemColor>,
    pub window_background_color: Option<NativeSystemColor>,
    pub window_text_color: Option<NativeSystemColor>,
    pub control_background_color: Option<NativeSystemColor>,
    pub control_text_color: Option<NativeSystemColor>,
    pub link_color: Option<NativeSystemColor>,
}

/// One acquired power assertion owned by the application thread.
pub struct NativePowerAssertion {
    inner: PowerAssertion,
}

impl NativePowerAssertion {
    pub fn new(kind: &str, reason: String) -> std::result::Result<Self, String> {
        let kind = match kind {
            "prevent-application-suspension" | "preventAppSuspension" => {
                PowerAssertionKind::PreventApplicationSuspension
            }
            "prevent-display-sleep" | "preventDisplaySleep" => {
                PowerAssertionKind::PreventDisplaySleep
            }
            value => {
                return Err(format!("unknown power assertion kind `{value}`"));
            }
        };
        Ok(Self {
            inner: PowerAssertion::acquire(kind, reason).map_err(|error| error.to_string())?,
        })
    }

    pub fn kind(&self) -> &'static str {
        match self.inner.kind() {
            PowerAssertionKind::PreventApplicationSuspension => "prevent-application-suspension",
            PowerAssertionKind::PreventDisplaySleep => "prevent-display-sleep",
        }
    }

    pub fn reason(&self) -> &str {
        self.inner.reason()
    }

    pub fn active(&self) -> bool {
        self.inner.is_active()
    }

    pub fn release(&mut self) -> std::result::Result<bool, String> {
        self.inner.release().map_err(|error| error.to_string())
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeWindowState {
    pub display_id: Option<String>,
    pub kind: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub viewport_width: f64,
    pub viewport_height: f64,
    pub minimum_width: Option<f64>,
    pub minimum_height: Option<f64>,
    pub maximum_width: Option<f64>,
    pub maximum_height: Option<f64>,
    pub scale_factor: f64,
    pub appearance: String,
    pub background_appearance: String,
    pub vibrancy: Option<String>,
    pub visual_effect_state: String,
    pub focused: bool,
    pub focusable: bool,
    pub visible: bool,
    pub minimized: bool,
    pub maximized: bool,
    pub fullscreen: bool,
    pub occluded: bool,
    pub movable: bool,
    pub resizable: bool,
    pub minimizable: bool,
    pub maximizable: bool,
    pub closable: bool,
    pub decorated: bool,
    pub shadow: bool,
    pub content_protected: bool,
    pub window_level: String,
    pub skip_taskbar: bool,
    pub visible_on_all_workspaces: bool,
    pub opacity: f64,
    pub has_icon: bool,
    pub taskbar_progress_state: String,
    pub taskbar_progress: f64,
    pub has_taskbar_overlay_icon: bool,
    pub cursor_visible: bool,
    pub cursor_grab: String,
    pub cursor_hit_test: bool,
    pub cursor_x: Option<f64>,
    pub cursor_y: Option<f64>,
    pub represented_file: bool,
    pub document_edited: bool,
    pub native_tabbing: bool,
    pub native_tab_count: u32,
    pub native_selected_tab: Option<u32>,
    pub native_tab_bar_visible: bool,
    pub native_tab_overview_visible: bool,
    pub native_tabs_truncated: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeFrameMetrics {
    pub frame_number: u64,
    pub cpu_milliseconds: f64,
    pub smoothed_cpu_milliseconds: f64,
    pub frame_milliseconds: f64,
    pub smoothed_frame_milliseconds: f64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeClipboardEntry {
    pub kind: String,
    pub text: Option<String>,
    pub metadata: Option<String>,
    pub format: Option<String>,
    #[serde(default, with = "crate::base64_bytes")]
    pub data: Option<Vec<u8>>,
    pub paths: Option<Vec<String>>,
    pub url: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeClipboardItem {
    pub entries: Vec<NativeClipboardEntry>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeNotificationAction {
    pub id: String,
    pub label: String,
    pub kind: Option<String>,
    pub placeholder: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeNotificationAttachment {
    pub id: String,
    pub path: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeNotificationOptions {
    pub tag: String,
    pub title: String,
    pub body: String,
    pub subtitle: Option<String>,
    pub actions: Vec<NativeNotificationAction>,
    /// `default`, `silent`, or a platform-recognized named sound.
    pub sound: Option<String>,
    pub icon_path: Option<String>,
    pub attachments: Option<Vec<NativeNotificationAttachment>>,
    /// Absolute Unix epoch milliseconds.
    pub delivery_at_ms: Option<f64>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeRelaunchOptions {
    pub executable: Option<String>,
    pub arguments: Option<Vec<String>>,
    pub clear_arguments: Option<bool>,
    pub working_directory: Option<String>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeAboutPanelOptions {
    pub application_name: Option<String>,
    pub application_version: Option<String>,
    pub version: Option<String>,
    pub copyright: Option<String>,
    pub credits: Option<String>,
    pub icon: Option<NativeImageSource>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeUserTask {
    pub title: String,
    pub arguments: String,
    pub program: Option<String>,
    pub description: Option<String>,
    pub working_directory: Option<String>,
    pub icon_path: Option<String>,
    pub icon_index: Option<i32>,
}

/// Whether this process can relocate its bundle into an `/Applications` directory.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeApplicationsFolderSupport {
    pub supported: bool,
    pub already_installed: bool,
}

/// Persistable window geometry and display identity.
///
/// `displayUuid` is the textual form of the stable physical display identity, so a stored state
/// survives a reboot that renumbers process-level display ids.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeWindowRestoreState {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub maximized: bool,
    pub fullscreen: bool,
    pub display_id: Option<String>,
    pub display_uuid: Option<String>,
    pub scale_factor: f64,
}

/// One application-shell service requested by JavaScript.
///
/// Requests that the operating system answers asynchronously complete through an `app-service`
/// event carrying the same request id; the rest resolve to `SystemCommandResult::Unit` at once.
pub(super) enum AppServiceAction {
    SetActivationPolicy(quickgui::ActivationPolicy),
    RequestDockAttention(quickgui::DockAttention),
    SetDockVisible(bool),
    MoveToApplicationsFolder,
}

/// One fire-and-forget application-shell mutation requested by JavaScript.
pub(super) enum AppMutationAction {
    Activate(bool),
    Hide,
    Unhide,
    CancelDockAttention(i64),
    SetSecureKeyboardEntry(bool),
    Beep,
    /// Add one word to the user dictionary through the installed spell-check provider.
    LearnWord(String),
    /// Ignore one word for the remainder of the shared checking session.
    IgnoreWord(String),
}

/// Maximum UTF-8 bytes accepted for a learned or ignored word.
const MAX_SPELL_WORD_BYTES: usize = 256;

pub(super) fn validate_spell_word(word: String) -> std::result::Result<String, String> {
    if word.is_empty() || word.len() > MAX_SPELL_WORD_BYTES || word.contains('\0') {
        return Err(format!(
            "a spell-check word must be nonempty, NUL-free, and at most {MAX_SPELL_WORD_BYTES} UTF-8 bytes"
        ));
    }
    Ok(word)
}

thread_local! {
    /// Live critical Dock bounces, keyed by the identifier JavaScript observed.
    ///
    /// The core's request identity is opaque, so the resolved value is retained here and looked up
    /// again when JavaScript cancels the bounce.
    static DOCK_ATTENTION_REQUESTS: std::cell::RefCell<HashMap<i64, quickgui::DockAttentionRequest>> =
        std::cell::RefCell::new(HashMap::new());
}

/// Maximum simultaneously retained Dock attention requests.
const MAX_DOCK_ATTENTION_REQUESTS: usize = 64;

pub(super) fn retain_dock_attention_request(request: quickgui::DockAttentionRequest) {
    DOCK_ATTENTION_REQUESTS.with_borrow_mut(|requests| {
        if requests.len() < MAX_DOCK_ATTENTION_REQUESTS {
            requests.insert(request.get(), request);
        }
    });
}

pub(super) fn take_dock_attention_request(id: i64) -> Option<quickgui::DockAttentionRequest> {
    DOCK_ATTENTION_REQUESTS.with_borrow_mut(|requests| requests.remove(&id))
}

pub(super) fn reset_dock_attention_requests() {
    DOCK_ATTENTION_REQUESTS.with_borrow_mut(HashMap::clear);
}

pub(super) enum SystemCommand {
    ConfigureApp(NativeAppOptions),
    Exit,
    ExitWithCode(i32),
    GetApplicationsFolderSupport,
    GetWindowRestoreState(u32),
    AppService {
        request: u32,
        action: AppServiceAction,
    },
    AppMutation(AppMutationAction),
    WindowPopupMenu {
        request: u32,
        window: u32,
        menu: String,
        position: Option<Point>,
    },
    Relaunch(NativeRelaunchOptions),
    GetAppInfo,
    GetAppPaths,
    GetSystemInfo,
    GetWindowRegistry,
    GetCursorScreenPosition,
    GetDesktopIntegrationSupport,
    GetSystemPreferences,
    GetDisplays,
    GetKeyboardLayout,
    GetWindowState(u32),
    GetWindowFrameMetrics(u32),
    ReadClipboard,
    WriteClipboard(ClipboardItem),
    ShowNotification(SystemNotification),
    DismissNotification(String),
    NotificationPermission {
        request: u32,
        prompt: bool,
    },
    SetDockBadge(Option<String>),
    SetDockIcon(Option<Image>),
    SetDockMenu(Option<String>),
    AddRecentDocument(PathBuf),
    ClearRecentDocuments,
    ShowAboutPanel(AboutPanelOptions),
    FileIcon {
        request: u32,
        path: PathBuf,
        size: FileIconSize,
    },
    SetUserTasks {
        request: u32,
        tasks: Vec<UserTask>,
    },
    SetApplicationMenu(String),
    SetQuitInterception(bool),
    RequestQuit,
    ReadFindClipboard,
    WriteFindClipboard(ClipboardItem),
    RequestSingleInstanceLock(String),
    ReleaseSingleInstanceLock,
    GlobalShortcut {
        request: u32,
        action: GlobalShortcutAction,
    },
    Tray {
        request: u32,
        action: tray::TrayAction,
    },
    WindowAction {
        window: u32,
        action: WindowAction,
    },
    ShellAction {
        request: u32,
        action: ShellAction,
    },
}

pub(super) enum GlobalShortcutAction {
    Register {
        registration: u32,
        accelerator: String,
    },
    Unregister {
        registration: u32,
    },
    UnregisterAll,
}

pub(super) enum WindowAction {
    SetTitle(String),
    SetBounds(WindowBounds),
    Move(Point),
    Resize(Size),
    Minimize,
    Maximize,
    Restore,
    SetFullscreen(bool),
    SetVisible(bool),
    SetResizable(bool),
    SetMovable(bool),
    SetMinimumSize(Option<Size>),
    SetMaximumSize(Option<Size>),
    SetMinimizable(bool),
    SetMaximizable(bool),
    SetClosable(bool),
    SetDecorated(bool),
    SetShadow(bool),
    SetContentProtected(bool),
    SetWindowLevel(Option<WindowLevel>),
    SetFocusable(bool),
    SetSkipTaskbar(bool),
    SetVisibleOnAllWorkspaces(bool),
    SetOpacity(f32),
    SetIcon(Option<Image>),
    SetCursorVisible(bool),
    SetCursorGrab(quickgui::CursorGrabMode),
    SetCursorHitTest(bool),
    SetCursorPosition(Point),
    SetTaskbarProgress(TaskbarProgressState, f32),
    SetTaskbarOverlayIcon(Image, String),
    ClearTaskbarOverlayIcon,
    Focus,
    RequestAttention,
    SetRepresentedFile(Option<PathBuf>),
    SetDocumentEdited(bool),
    SetAppearance(Option<WindowAppearance>),
    SetBackgroundAppearance(WindowBackgroundAppearance),
    SetMacOsVibrancy(Option<MacOsVibrancy>),
    SetMacOsVisualEffectState(MacOsVisualEffectState),
    /// Ask the core to hand `Event::CloseRequested` to JavaScript instead of closing.
    SetCloseInterception(bool),
    MoveTop,
    /// Order this window above another hosted window, named by its hosted window id.
    MoveAbove(u32),
    SetIgnoreMouseEvents(bool, bool),
    SetWindowEnabled(bool),
    SetAspectRatio(Option<Size>),
    SetWindowButtonVisibility(bool),
    SetAlwaysOnTop(bool, Option<WindowLevel>),
    /// Replace this window's native menu declaration, or inherit the application menus again.
    SetMenu(Option<String>),
    /// Declare the constraint applied when the core asks for a `WillResize` answer.
    SetResizePolicy(Option<String>),
    /// Declare the constraint applied when the core asks for a `WillMove` answer.
    SetMovePolicy(Option<String>),
    ShowCharacterPalette,
    SetTabbingIdentifier(Option<String>),
    SelectNextTab,
    SelectPreviousTab,
    SelectTab(u32),
    MergeAllWindows,
    MoveTabToNewWindow,
    ToggleTabBar,
    ToggleTabOverview,
}

pub(super) enum ShellAction {
    OpenExternal(String),
    OpenPath(PathBuf),
    RevealPath(PathBuf),
    TrashPath(PathBuf),
}

pub(super) enum SystemCommandResult {
    Unit,
    Boolean(bool),
    AppInfo(Option<NativeAppInfo>),
    AppPaths(Option<NativeAppPaths>),
    SystemInfo(NativeSystemInfo),
    WindowRegistry(NativeWindowRegistry),
    Point(NativePoint),
    DesktopIntegrationSupport(NativeDesktopIntegrationSupport),
    SystemPreferences(NativeSystemPreferences),
    Displays(Vec<NativeDisplay>),
    KeyboardLayout(NativeKeyboardLayout),
    WindowState(NativeWindowState),
    FrameMetrics(NativeFrameMetrics),
    WindowRestoreState(NativeWindowRestoreState),
    ApplicationsFolderSupport(NativeApplicationsFolderSupport),
    Clipboard(Option<ClipboardItem>),
}

/// A native application-shell or popup-menu operation whose outcome arrives asynchronously.
pub(super) enum AppServiceResponse {
    Unit(quickgui::PlatformResponse<()>),
    Boolean(quickgui::PlatformResponse<bool>),
    DockAttention(quickgui::PlatformResponse<quickgui::DockAttentionRequest>),
    /// An outcome known up front, so tests can exercise event ordering without a runner.
    #[cfg(test)]
    Completed(Option<std::result::Result<(), quickgui::PlatformError>>),
}

pub(super) struct PendingAppService {
    request: u32,
    kind: &'static str,
    response: AppServiceResponse,
}

impl PendingAppService {
    pub(super) fn new(request: u32, kind: &'static str, response: AppServiceResponse) -> Self {
        Self {
            request,
            kind,
            response,
        }
    }

    /// A request whose native operation has already succeeded.
    #[cfg(test)]
    pub(super) fn completed(request: u32, kind: &'static str) -> Self {
        Self::new(request, kind, AppServiceResponse::Completed(Some(Ok(()))))
    }

    pub(super) fn request(&self) -> u32 {
        self.request
    }

    pub(super) fn poll(&mut self, context: &mut Context<'_>) -> Poll<super::NativeEvent> {
        let (value, error) = match &mut self.response {
            AppServiceResponse::Unit(response) => match Pin::new(response).poll(context) {
                Poll::Ready(Ok(())) => (None, None),
                Poll::Ready(Err(error)) => (None, Some(error.to_string())),
                Poll::Pending => return Poll::Pending,
            },
            AppServiceResponse::Boolean(response) => match Pin::new(response).poll(context) {
                Poll::Ready(Ok(value)) => (Some(value.to_string()), None),
                Poll::Ready(Err(error)) => (None, Some(error.to_string())),
                Poll::Pending => return Poll::Pending,
            },
            AppServiceResponse::DockAttention(response) => match Pin::new(response).poll(context) {
                Poll::Ready(Ok(value)) => {
                    retain_dock_attention_request(value);
                    (Some(value.get().to_string()), None)
                }
                Poll::Ready(Err(error)) => (None, Some(error.to_string())),
                Poll::Pending => return Poll::Pending,
            },
            #[cfg(test)]
            AppServiceResponse::Completed(result) => match result.take() {
                Some(Ok(())) => (None, None),
                Some(Err(error)) => (None, Some(error.to_string())),
                None => return Poll::Pending,
            },
        };
        Poll::Ready(super::NativeEvent {
            kind: self.kind.to_owned(),
            window: 0,
            target: self.request,
            value,
            paths: None,
            data: None,
            width: None,
            height: None,
            error,
        })
    }
}

#[derive(Default)]
pub(super) struct SystemObservation {
    displays: Option<Displays>,
    windows: HashMap<u32, WindowState>,
    preferences: Option<SystemPreferences>,
}

pub(super) struct PendingShell {
    request: u32,
    response: ShellResponse,
}

pub(super) struct PendingNotificationPermission {
    request: u32,
    response: NotificationPermissionResponse,
}

pub(super) struct PendingFileIcon {
    request: u32,
    response: FileIconResponse,
}

pub(super) struct PendingUserTasks {
    request: u32,
    response: ShellResponse,
}

pub(super) struct PendingGlobalShortcut {
    request: u32,
    response: quickgui::PlatformResponse<()>,
}

pub(super) struct PendingTray {
    request: u32,
    response: quickgui::PlatformResponse<()>,
}

impl PendingTray {
    fn new(request: u32, response: quickgui::PlatformResponse<()>) -> Self {
        Self { request, response }
    }

    pub(super) fn request(&self) -> u32 {
        self.request
    }

    pub(super) fn poll(&mut self, context: &mut Context<'_>) -> Poll<super::NativeEvent> {
        let error = match Pin::new(&mut self.response).poll(context) {
            Poll::Ready(Ok(())) => None,
            Poll::Ready(Err(error)) => Some(error.to_string()),
            Poll::Pending => return Poll::Pending,
        };
        Poll::Ready(super::NativeEvent {
            kind: "tray-operation".to_owned(),
            window: 0,
            target: self.request,
            value: None,
            paths: None,
            data: None,
            width: None,
            height: None,
            error,
        })
    }
}

impl PendingGlobalShortcut {
    fn new(request: u32, response: quickgui::PlatformResponse<()>) -> Self {
        Self { request, response }
    }

    pub(super) fn request(&self) -> u32 {
        self.request
    }

    pub(super) fn poll(&mut self, context: &mut Context<'_>) -> Poll<super::NativeEvent> {
        let error = match Pin::new(&mut self.response).poll(context) {
            Poll::Ready(Ok(())) => None,
            Poll::Ready(Err(error)) => Some(error.to_string()),
            Poll::Pending => return Poll::Pending,
        };
        Poll::Ready(super::NativeEvent {
            kind: "global-shortcut-operation".to_owned(),
            window: 0,
            target: self.request,
            value: None,
            paths: None,
            data: None,
            width: None,
            height: None,
            error,
        })
    }
}

impl PendingShell {
    fn new(request: u32, response: ShellResponse) -> Self {
        Self { request, response }
    }

    pub(super) fn request(&self) -> u32 {
        self.request
    }

    pub(super) fn poll(&mut self, context: &mut Context<'_>) -> Poll<super::NativeEvent> {
        let error = match Pin::new(&mut self.response).poll(context) {
            Poll::Ready(Ok(())) => None,
            Poll::Ready(Err(error)) => Some(error.to_string()),
            Poll::Pending => return Poll::Pending,
        };
        Poll::Ready(super::NativeEvent {
            kind: "shell".to_owned(),
            window: 0,
            target: self.request,
            value: None,
            paths: None,
            data: None,
            width: None,
            height: None,
            error,
        })
    }
}

impl PendingNotificationPermission {
    fn new(request: u32, response: NotificationPermissionResponse) -> Self {
        Self { request, response }
    }

    pub(super) fn request(&self) -> u32 {
        self.request
    }

    pub(super) fn poll(&mut self, context: &mut Context<'_>) -> Poll<super::NativeEvent> {
        let (value, error) = match Pin::new(&mut self.response).poll(context) {
            Poll::Ready(Ok(status)) => {
                let status = match status {
                    NotificationPermissionStatus::NotDetermined => "not-determined",
                    NotificationPermissionStatus::Granted => "granted",
                    NotificationPermissionStatus::Denied => "denied",
                    NotificationPermissionStatus::Unsupported => "unsupported",
                };
                (Some(status.to_owned()), None)
            }
            Poll::Ready(Err(error)) => (None, Some(error.to_string())),
            Poll::Pending => return Poll::Pending,
        };
        Poll::Ready(super::NativeEvent {
            kind: "notification-permission".to_owned(),
            window: 0,
            target: self.request,
            value,
            paths: None,
            data: None,
            width: None,
            height: None,
            error,
        })
    }
}

impl PendingFileIcon {
    fn new(request: u32, response: FileIconResponse) -> Self {
        Self { request, response }
    }

    pub(super) fn request(&self) -> u32 {
        self.request
    }

    pub(super) fn poll(&mut self, context: &mut Context<'_>) -> Poll<super::NativeEvent> {
        let (data, width, height, error) = match Pin::new(&mut self.response).poll(context) {
            Poll::Ready(Ok(image)) => (
                Some(image.rgba().to_vec()),
                Some(image.width()),
                Some(image.height()),
                None,
            ),
            Poll::Ready(Err(error)) => (None, None, None, Some(error.to_string())),
            Poll::Pending => return Poll::Pending,
        };
        Poll::Ready(super::NativeEvent {
            kind: "file-icon".to_owned(),
            window: 0,
            target: self.request,
            value: None,
            paths: None,
            data,
            width,
            height,
            error,
        })
    }
}

impl PendingUserTasks {
    fn new(request: u32, response: ShellResponse) -> Self {
        Self { request, response }
    }

    pub(super) fn request(&self) -> u32 {
        self.request
    }

    pub(super) fn poll(&mut self, context: &mut Context<'_>) -> Poll<super::NativeEvent> {
        let error = match Pin::new(&mut self.response).poll(context) {
            Poll::Ready(Ok(())) => None,
            Poll::Ready(Err(error)) => Some(error.to_string()),
            Poll::Pending => return Poll::Pending,
        };
        Poll::Ready(super::NativeEvent {
            kind: "user-tasks".to_owned(),
            window: 0,
            target: self.request,
            value: None,
            paths: None,
            data: None,
            width: None,
            height: None,
            error,
        })
    }
}

mod parsing;
mod runtime;

use parsing::*;
pub(crate) use parsing::{
    about_panel_options, clipboard_item, file_icon_size, native_clipboard_item,
    parse_app_mutation_action, parse_app_service_action, parse_global_shortcut_action,
    parse_move_policy, parse_permission_kind, parse_resize_policy, parse_shell_action,
    parse_window_action, parse_window_image_action, parse_window_level, parse_window_restore_state,
    permission_status_name, system_notification, user_tasks, window_level_name,
};
pub(crate) use tray::{TrayAction, tray_options};

#[cfg(test)]
mod tests;

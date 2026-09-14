#[cfg(target_os = "macos")]
use std::sync::atomic::AtomicBool;
use std::{
    any::{Any, TypeId},
    cell::{Ref, RefCell},
    collections::{HashMap, HashSet, VecDeque},
    fmt,
    future::Future,
    marker::PhantomData,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
use web_time::{Duration, Instant};

use accesskit::{Action as AccessibilityAction, ActionData, ActionRequest};
use accesskit_winit::{
    Adapter as AccessibilityAdapter, Event as AccessibilityEvent,
    WindowEvent as AccessibilityWindowEvent,
};
use thiserror::Error;
#[cfg(target_os = "macos")]
use winit::platform::macos::{ActiveEventLoopExtMacOS, WindowAttributesExtMacOS, WindowExtMacOS};
#[cfg(not(target_arch = "wasm32"))]
use winit::platform::pump_events::{EventLoopExtPumpEvents, PumpStatus};
#[cfg(target_os = "windows")]
use winit::platform::windows::{WindowAttributesExtWindows, WindowExtWindows};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalPosition, LogicalSize, PhysicalSize},
    event::{ElementState, Force, Ime, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    keyboard::ModifiersState,
    window::{
        CursorGrabMode as WinitCursorGrabMode, CursorIcon, Fullscreen, Icon, Theme,
        UserAttentionType, Window, WindowButtons, WindowId, WindowLevel as WinitWindowLevel,
    },
};
#[cfg(not(target_os = "macos"))]
use winit::{dpi::PhysicalPosition, raw_window_handle::HasWindowHandle};

#[cfg(any(
    target_os = "macos",
    target_os = "windows",
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "openbsd",
    target_os = "netbsd"
))]
use crate::platform::PlatformDialogId;
#[cfg(any(target_os = "macos", target_os = "windows"))]
use crate::platform::PlatformResponder;
use crate::{
    AboutPanelOptions, Action, ActionListener, AnyAction, AppInfo, AppPaths, AssetError, Assets,
    Color, ColorScheme, CursorStyle, Display, DisplayEvent, DisplayId, Displays, Element,
    ElementId, Entity, EntityId, EventEmitter, FileIconResponse, FileIconSize, FocusHandle, Font,
    FontSource, Global, Image, IntoElement, KeyBinding, KeyboardLayout, Keymap, Keystroke,
    MAX_ENTITY_EVENT_DELIVERIES_PER_TURN, MAX_ENTITY_SUBSCRIPTIONS_PER_WINDOW,
    MAX_GLOBAL_OBSERVER_DELIVERIES_PER_TURN, MAX_GLOBAL_SUBSCRIPTIONS_PER_WINDOW,
    MAX_OBSERVED_ENTITIES_PER_WINDOW, MAX_OBSERVED_GLOBALS_PER_WINDOW, MAX_PENDING_ENTITY_EVENTS,
    MAX_PENDING_GLOBAL_NOTIFICATIONS, MAX_TASKBAR_OVERLAY_DESCRIPTION_BYTES, Menu,
    NotificationPermissionResponse, NotificationPermissionStatus, OpenUrls, OsAction, Point,
    PowerSource, Rect, RelaunchOptions, RelaunchRequest, RelaunchedProcess, Scene, Size,
    SystemInfo, SystemNotification, SystemNotificationResponse, SystemPreferences, ThermalState,
    UserTask, Vector,
    action::{ActionListenerBinding, ActionListenerKey},
    background::{
        BackgroundCompletion, BackgroundTaskError, BackgroundTaskPoolHandle, TaskSpawnError,
    },
    clipboard::{ClipboardItem, ClipboardService, ClipboardTarget},
    element::{
        KeyListenerBinding, KeyListenerKey, KeyListenerKind, MouseListenerKey, MouseListenerKind,
    },
    entity::{EntityEvent, Subscription, SubscriptionState},
    event::{
        ContextMenuEvent, DragOrigin, DragStartEvent, DropEvent, DroppedFiles, Event, EventContext,
        EventRuntimeContext, ExternalDragPayload, ExternalDragText, ExternalDragUrl, FileDragPaths,
        FormSubmitEvent, GesturePhase, Key, KeyDownEvent, KeyUpEvent,
        MAX_ACTIVE_TOUCHES_PER_WINDOW, MAX_DROPPED_FILES, MAX_PINCH_DELTA_PER_EVENT,
        MAX_ROTATION_DEGREES_PER_EVENT, Modifiers, MouseButton, MouseDownEvent, MouseExitEvent,
        MouseMoveEvent, MousePressureEvent, MouseUpEvent, PinchEvent, PointerEvent, PointerPhase,
        PressureStage, RotationEvent, ScrollDelta, ScrollWheelEvent, SmartMagnifyEvent, TouchEvent,
        TouchId, TouchPhase, ValidationReport,
    },
    foreground::{
        AsyncViewContext, ForegroundTaskSpawnError, ForegroundTaskSpawner, ScheduledForegroundTask,
        Task,
    },
    global::GlobalStore,
    image_resource::{ImageAssetCache, ImageLoadCompletion, ImageWorkerPoolHandle},
    keyboard::KeyboardState,
    menu::{MenuAction, collect_menu_actions, validate_menus},
    metrics::{FrameMetrics, FrameTimer, MetricsTracker},
    platform::{
        PathPromptOptions, PathPromptResponse, PlatformError, PlatformRequest, PlatformResponse,
        PromptButton, PromptLevel, SavePathOptions, SavePathResponse, ShellResponse,
    },
    renderer::{
        GpuContext, GpuRenderer, RenderOutcome, SharedFontSystem, create_shared_font_system,
    },
    scheduler::FrameScheduler,
    ui_tree::{
        DismissRequest, FormAttempt, InputDispatchScope, InputModality, InputResult,
        MouseHoverChange, PendingFocus, TabNavigationTarget, UiTree,
    },
};

#[cfg(feature = "inspector")]
use crate::inspector::{
    InspectorFrameDamage, InspectorMode, InspectorPointerAction, InspectorState,
};

const MAX_NESTED_FORM_SUBMISSIONS: u8 = 8;
const MAX_WINDOW_LIFECYCLE_TURNS: usize = 1_024;
const ACCESSIBILITY_GEOMETRY_UPDATE_INTERVAL: Duration = Duration::from_millis(100);

/// Maximum targeted desktop mouse callbacks declared by one window render.
pub const MAX_MOUSE_LISTENERS_PER_WINDOW: usize = 8_192;

/// Maximum focused key callbacks declared by one window render.
pub const MAX_KEY_LISTENERS_PER_WINDOW: usize = 4_096;

/// Maximum typed action callbacks declared by one window render.
pub const MAX_ACTION_LISTENERS_PER_WINDOW: usize = 4_096;

#[cfg(target_os = "macos")]
use crate::event::{ExternalDragEndEvent, ExternalDragOperation};
#[cfg(target_os = "macos")]
use crate::macos::{
    MacExternalDragMonitor, MacExternalDragSession, MacFirstFrameGuard, MacMouseDownEvent,
    MacNativeDropHost, MacNativeDropOffer, MacNativeDropPayload, MacNativeDropPending,
    MacNativeHost, MacPlatformDialog, MacPlatformDialogContext, MacPlatformDialogFocus,
    MacPopoverMonitor, MacTrafficLightHost, MacTypedDragPayload, MacTypedDragRegistry,
    MacVibrancyHost, MacWindowTabAction, capture_left_mouse_down, configure_document_window,
    configure_gpu_window_resize, configure_window_kind,
    current_cursor_screen_position as macos_cursor_screen_position, current_pointer_position,
    dismiss_window_relation, is_window_fullscreen, is_window_maximized, is_window_miniaturized,
    order_window_above, order_window_front, perform_window_close, perform_window_drag,
    perform_window_tab_action, position_system_popover, position_traffic_lights,
    present_native_open_panel, present_native_prompt, present_native_save_panel,
    present_window_relation, set_window_aspect_ratio, set_window_button_visibility,
    set_window_document_edited, set_window_focusable, set_window_ignores_mouse_events,
    set_window_input_enabled, set_window_level, set_window_movable, set_window_opacity,
    set_window_represented_file, set_window_tabbing_identifier, set_window_visibility,
    set_window_visible_on_all_workspaces, shell_open_path, shell_open_url, shell_reveal_path,
    shell_trash_path, show_character_palette, start_external_drag, window_tab_state,
};
#[cfg(target_os = "macos")]
use crate::macos_application::MacApplicationHost;
#[cfg(target_os = "macos")]
use crate::macos_menu::{MacMenuHost, MacMenuItemState};

pub(crate) enum RuntimeEvent {
    ExternalCommandsReady,
    InvalidateWindow(WindowHandle),
    InvalidateElement(WindowHandle, ElementId),
    Accessibility(AccessibilityEvent),
    ImageLoaded(WindowHandle, ImageLoadCompletion),
    BackgroundCompleted(BackgroundCompletion),
    ForegroundTasksReady,
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    MenuWillOpen,
    #[cfg_attr(not(any(target_os = "macos", target_os = "windows")), allow(dead_code))]
    MenuAction(usize),
    #[cfg(target_os = "macos")]
    DockMenuAction(usize),
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    NativePopupMenuAction(u64, usize),
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    NativePopupMenuClosed(u64),
    #[cfg(target_os = "macos")]
    ExternalDragBoundary(WindowHandle, Point),
    #[cfg(target_os = "macos")]
    ExternalDragEnded(WindowHandle, ExternalDragOperation),
    #[cfg(target_os = "macos")]
    NativeDropChanged(WindowHandle),
    #[cfg(target_os = "macos")]
    ApplicationActivated,
    #[cfg(target_os = "macos")]
    ApplicationDeactivated,
    #[cfg(target_os = "macos")]
    PopoverPointerDismissRequested(WindowHandle),
    #[cfg(any(
        target_os = "macos",
        target_os = "windows",
        target_os = "linux",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "openbsd",
        target_os = "netbsd"
    ))]
    PlatformDialogClosed(Option<WindowHandle>, PlatformDialogId),
    #[cfg(target_os = "macos")]
    PlatformDialogCancelled(Option<WindowHandle>, PlatformDialogId),
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    OpenUrls(OpenUrls),
    #[cfg(target_os = "macos")]
    Reopen {
        has_visible_windows: bool,
    },
    #[cfg(target_os = "macos")]
    QuitRequested,
    #[cfg(target_os = "macos")]
    SystemWake,
    #[cfg(target_os = "macos")]
    DisplaysChanged,
    #[cfg(target_os = "macos")]
    KeyboardLayoutChanged,
    #[cfg(target_os = "macos")]
    NativePanel(native_panels::NativePanelEvent),
    #[cfg(target_os = "macos")]
    SystemNotificationAuthorization {
        granted: bool,
        error: Option<Arc<str>>,
    },
    #[cfg(target_os = "macos")]
    SystemNotificationPermissionStatus(NotificationPermissionStatus),
    SystemNotificationResponse(SystemNotificationResponse),
    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    GlobalShortcut(u32),
    #[cfg(any(
        target_os = "macos",
        target_os = "windows",
        target_os = "linux",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "openbsd",
        target_os = "netbsd"
    ))]
    SecondInstance(SecondInstanceEvent),
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    SystemPreferencesChanged(SystemPreferences),
    Power(PowerEvent),
    Tray(TrayEvent),
}

impl From<AccessibilityEvent> for RuntimeEvent {
    fn from(event: AccessibilityEvent) -> Self {
        Self::Accessibility(event)
    }
}

mod component_scope;
mod effects;
mod event_loop_lifecycle;
mod event_loop_user;
mod event_loop_wait;
mod event_loop_window;
mod keyboard_input;
mod lifecycle;
mod listener_scope;
mod pointer_input;
mod rendering;
mod runtime_state;
mod view_context;
mod window;
mod window_creation;
mod window_processing;

pub use view_context::*;
pub use window::*;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("could not create the application event loop: {0}")]
    EventLoop(#[from] winit::error::EventLoopError),
    #[error("could not create the application window: {0}")]
    Window(String),
    #[error("could not initialize GPU rendering: {0}")]
    GraphicsInitialization(String),
    #[error("GPU rendering failed: {0}")]
    Render(String),
    #[error("view layout or painting failed: {0}")]
    View(String),
    #[error("platform integration failed: {0}")]
    Platform(String),
    #[error(transparent)]
    Asset(#[from] AssetError),
}

/// Result of one externally driven application-loop turn.
///
/// [`AppRunner`] returns control after a native redraw or after its caller-provided timeout. This
/// lets another runtime, such as Bun, service its own tasks without moving QuickGUI or AppKit off
/// the platform application thread.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AppRunStatus {
    Continue,
    Exited(i32),
}

/// Thread-safe wake handle for an externally pumped [`AppRunner`].
///
/// Embedding runtimes can block the platform event loop indefinitely, then use this handle from a
/// worker or command producer when native work becomes ready. A blocked [`AppRunner::pump`] call
/// returns without forcing a periodic polling timeout.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone)]
pub struct AppRunnerWaker {
    proxy: EventLoopProxy<RuntimeEvent>,
}

#[cfg(not(target_arch = "wasm32"))]
impl AppRunnerWaker {
    /// Wake the application event loop, returning `false` after it has closed.
    ///
    /// A blocked [`AppRunner::pump`] returns promptly on every platform. On macOS the proxy
    /// wake-up alone is dropped whenever the main run loop is not asleep, so the loop is also
    /// stopped through the main dispatch queue; see `macos_application::interrupt_pump`.
    pub fn wake(&self) -> bool {
        if self
            .proxy
            .send_event(RuntimeEvent::ExternalCommandsReady)
            .is_err()
        {
            return false;
        }
        #[cfg(target_os = "macos")]
        crate::macos_application::interrupt_pump();
        true
    }
}

/// Thread-safe invalidation handle for one retained window.
///
/// Clones can be moved into background threads. Calling [`Self::invalidate`] is coalesced by the
/// window scheduler, so a burst of updates results in at most one pending redraw.
#[derive(Clone)]
pub struct WindowInvalidator {
    runtime: Option<(EventLoopProxy<RuntimeEvent>, WindowHandle)>,
}

impl WindowInvalidator {
    /// Wake and redeclare one retained embedding scope. Missing scopes use the ordinary fallback.
    pub fn invalidate_element(&self, element: ElementId) -> bool {
        self.runtime.as_ref().is_some_and(|(proxy, window)| {
            proxy
                .send_event(RuntimeEvent::InvalidateElement(*window, element))
                .is_ok()
        })
    }

    /// Wake the application and rebuild this window, returning `false` after the event loop closes.
    pub fn invalidate(&self) -> bool {
        self.runtime.as_ref().is_some_and(|(proxy, window)| {
            proxy
                .send_event(RuntimeEvent::InvalidateWindow(*window))
                .is_ok()
        })
    }
}

type OpenUrlsCallback = Box<dyn FnMut(OpenUrls, &mut EventContext)>;
type ReopenCallback = Box<dyn FnMut(bool, &mut EventContext)>;
type SystemWakeCallback = Box<dyn FnMut(&mut EventContext)>;
type KeyboardLayoutCallback = Box<dyn FnMut(&KeyboardLayout, &mut EventContext)>;
type SystemNotificationResponseCallback =
    Box<dyn FnMut(SystemNotificationResponse, &mut EventContext)>;
type GlobalShortcutCallback = Box<dyn FnMut(GlobalShortcutEvent, &mut EventContext)>;
#[cfg(not(target_arch = "wasm32"))]
type SecondInstanceCallback = Box<dyn FnMut(SecondInstanceEvent, &mut EventContext)>;
type PowerEventCallback = Box<dyn FnMut(PowerEvent, &mut EventContext)>;
type TrayEventCallback = Box<dyn FnMut(TrayEvent, &mut EventContext)>;
type DisplayEventCallback = Box<dyn FnMut(DisplayEvent, &mut EventContext)>;
type ColorPanelChangeCallback = Box<dyn FnMut(Color, &mut EventContext)>;
type FontPanelChangeCallback = Box<dyn FnMut(Font, &mut EventContext)>;
type WindowClosedCallback = Box<dyn FnMut(WindowHandle, &mut EventContext)>;
type QuitCallback = Box<dyn FnMut(QuitRequest, &mut EventContext)>;
type FinishLaunchingCallback = Box<dyn FnOnce(&mut EventContext)>;
type ApplicationActionCallback = Box<dyn FnMut(&AnyAction, &mut EventContext)>;

#[derive(Default)]
struct ApplicationCallbacks {
    actions: HashMap<TypeId, ApplicationActionCallback>,
    finish_launching: Option<FinishLaunchingCallback>,
    open_urls: Option<OpenUrlsCallback>,
    reopen: Option<ReopenCallback>,
    system_wake: Option<SystemWakeCallback>,
    keyboard_layout: Option<KeyboardLayoutCallback>,
    system_notification_response: Option<SystemNotificationResponseCallback>,
    global_shortcut: Option<GlobalShortcutCallback>,
    #[cfg(not(target_arch = "wasm32"))]
    second_instance: Option<SecondInstanceCallback>,
    power_event: Option<PowerEventCallback>,
    tray_event: Option<TrayEventCallback>,
    did_become_active: Option<SystemWakeCallback>,
    did_resign_active: Option<SystemWakeCallback>,
    display_event: Option<DisplayEventCallback>,
    color_panel_change: Option<ColorPanelChangeCallback>,
    font_panel_change: Option<FontPanelChangeCallback>,
    window_closed: Option<WindowClosedCallback>,
    before_quit: Option<QuitCallback>,
    will_quit: Option<QuitCallback>,
}

impl ApplicationCallbacks {
    fn dispatch_action(&mut self, action: &AnyAction, cx: &mut EventContext) -> Option<bool> {
        let callback = self.actions.get_mut(&action.type_id())?;
        callback(action, cx);
        Some(!cx.propagate_action)
    }
}

mod application;
#[cfg(not(target_arch = "wasm32"))]
mod deep_link;
mod display_events;
mod native_panels;
#[cfg(target_os = "macos")]
pub(crate) use native_panels::NativePanelEvent;
mod external;
#[cfg(not(target_arch = "wasm32"))]
pub use application::AppRunner;
pub use application::{App, Application};
mod global_shortcut;
mod integration;
mod platform_dialog;
mod power_monitor;
#[cfg(any(
    target_os = "macos",
    target_os = "windows",
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "openbsd",
    target_os = "netbsd"
))]
mod single_instance;
mod tray;
#[cfg(target_os = "windows")]
mod windows_menu;
#[cfg(target_os = "windows")]
mod windows_shell;
#[cfg(target_os = "windows")]
mod windows_window;

#[cfg(target_os = "windows")]
pub(crate) fn validate_windows_notification_app_info(
    app_info: Option<&AppInfo>,
) -> Result<(), PlatformError> {
    let app_id = app_info.map(AppInfo::identifier).ok_or_else(|| {
        PlatformError::Platform(
            "Windows system notifications require AppInfo with an application identifier".into(),
        )
    })?;
    windows_shell::validate_app_id(app_id)
}

#[cfg(target_os = "windows")]
use windows_window::{
    current_cursor_screen_position as windows_cursor_screen_position, set_window_focusable,
    set_window_opacity,
};

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn set_window_focusable(_window: &Arc<Window>, _focusable: bool) -> Result<(), String> {
    Ok(())
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn set_window_opacity(_window: &Arc<Window>, _opacity: f32) -> Result<(), String> {
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn set_window_visible_on_all_workspaces(
    _window: &Arc<Window>,
    _visible: bool,
) -> Result<(), String> {
    Ok(())
}

pub(crate) fn cursor_screen_position(displays: &Displays) -> Result<Point, PlatformError> {
    #[cfg(target_os = "macos")]
    {
        let _ = displays;
        macos_cursor_screen_position().ok_or(PlatformError::Unavailable)
    }
    #[cfg(target_os = "windows")]
    {
        windows_cursor_screen_position(displays)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = displays;
        Err(PlatformError::Unsupported)
    }
}
#[cfg(not(target_arch = "wasm32"))]
pub use deep_link::MAX_DEEP_LINK_ARGUMENTS;
pub use global_shortcut::{
    GlobalShortcutEvent, MAX_GLOBAL_SHORTCUT_ACCELERATOR_BYTES, MAX_GLOBAL_SHORTCUTS,
};
pub use integration::DesktopIntegrationSupport;
pub use power_monitor::PowerEvent;
#[cfg(any(
    target_os = "macos",
    target_os = "windows",
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "openbsd",
    target_os = "netbsd"
))]
pub use single_instance::{
    MAX_SECOND_INSTANCE_ARGUMENTS, MAX_SECOND_INSTANCE_MESSAGE_BYTES,
    MAX_SINGLE_INSTANCE_IDENTIFIER_BYTES, SecondInstanceEvent, SingleInstanceError,
};
pub(crate) use tray::validate_tray_options;
pub use tray::{
    MAX_TRAY_ENCODED_ICON_BYTES, MAX_TRAY_ICON_DIMENSION, MAX_TRAY_ICONS, MAX_TRAY_MENU_DEPTH,
    MAX_TRAY_MENU_ITEMS, MAX_TRAY_TEXT_BYTES, TrayEvent, TrayEventKind, TrayIconImage,
    TrayIconOptions, TrayMenuItem, TrayMouseButton,
};
#[cfg(any(test, feature = "test-support"))]
mod test_context;
#[cfg(any(test, feature = "test-support"))]
pub use test_context::{
    MAX_TEST_EFFECT_TURNS, TestAppContext, TestAppError, TestApplicationShell, TestWindowHandle,
    VisualTestContext,
};

#[derive(Clone, Copy)]
struct PopoverWindowContext {
    owner: WindowHandle,
    root: WindowHandle,
}

#[derive(Clone, Copy)]
struct ClosedWindow {
    handle: WindowHandle,
    parent: Option<WindowHandle>,
    restore_focus: Option<ElementId>,
}

struct RuntimeWindow {
    parent: Option<WindowHandle>,
    restore_focus_on_close: Option<ElementId>,
    #[cfg(all(target_os = "macos", feature = "swift-ui"))]
    embedded: Option<crate::MacEmbeddedView>,
    view: Box<dyn AnyView>,
    renderer: GpuRenderer,
    image_assets: ImageAssetCache,
    #[cfg(target_os = "macos")]
    native_host: Option<MacNativeHost>,
    #[cfg(target_os = "macos")]
    vibrancy_host: Option<MacVibrancyHost>,
    #[cfg(target_os = "macos")]
    _traffic_light_host: Option<MacTrafficLightHost>,
    #[cfg(target_os = "macos")]
    native_drop_host: MacNativeDropHost,
    #[cfg(target_os = "macos")]
    first_frame_guard: Option<MacFirstFrameGuard>,
    #[cfg(target_os = "windows")]
    window_menu_host: Option<windows_menu::WindowsMenuHost>,
    #[cfg(target_os = "windows")]
    taskbar_state_applied: bool,
    #[cfg(target_os = "windows")]
    taskbar_apply_attempts: u8,
    ui: UiTree,
    #[cfg(feature = "inspector")]
    inspector: Option<InspectorState>,
    scheduler: FrameScheduler,
    scene: Scene,
    metrics: MetricsTracker,
    scale_factor: f32,
    logical_size: Size,
    logical_position: Point,
    display_id: Option<DisplayId>,
    appearance: WindowAppearance,
    native_tabs: WindowTabState,
    restore_bounds: Rect,
    maximized: bool,
    pointer: Option<Point>,
    pointer_capture: Option<PointerCapture>,
    pressed_mouse_buttons: PressedMouseButtons,
    mouse_clicks: MouseClickTracker,
    mouse_event_path_scratch: Vec<ElementId>,
    mouse_dispatch_scratch: Vec<MouseListenerKey>,
    mouse_hover_changes_scratch: Vec<MouseHoverChange>,
    key_dispatch_scratch: Vec<KeyListenerBinding>,
    action_dispatch_scratch: Vec<ActionListenerBinding>,
    touch_captures: HashMap<TouchId, TouchCapture>,
    drag_candidate: Option<DragCandidate>,
    drag_session: Option<DragSession>,
    native_file_drag: Option<NativeFileDrag>,
    #[cfg(target_os = "macos")]
    native_external_drag: Option<MacNativeDropOffer>,
    #[cfg(target_os = "macos")]
    external_drag_mouse_down: Option<MacMouseDownEvent>,
    #[cfg(target_os = "macos")]
    external_drag_monitor: Option<MacExternalDragMonitor>,
    #[cfg(target_os = "macos")]
    outbound_external_drag: Option<OutboundExternalDrag>,
    #[cfg(target_os = "macos")]
    suppress_external_drag_release: bool,
    cursor: CursorIcon,
    ime_target: Option<ElementId>,
    /// A focus request waiting for the rebuild that introduces its target, with the input that
    /// made it, so the focus it lands paints the way that input dictates.
    pending_focus: Option<PendingFocus>,
    occluded: bool,
    minimized: bool,
    fullscreen: bool,
    /// Whether `Event::FirstPresented` has already been delivered for this window.
    first_presented: bool,
    /// One in-flight corrective inner size requested by `constrain_resize`/`aspect_ratio`.
    resize_correction: Option<Size>,
    /// The last raw Force Touch pressure stage, so a force click fires once per transition.
    pressure_stage: i64,
    /// One in-flight corrective outer position requested by `constrain_move`.
    move_correction: Option<Point>,
    focused: bool,
    visible: bool,
    relation_presented: bool,
    reduce_motion: bool,
    view_dirty: bool,
    layout_dirty: bool,
    view_deadline: Option<Instant>,
    accessibility_updates: AccessibilityUpdateSchedule,
    listeners: ListenerRegistry,
    accessibility: AccessibilityAdapter,
    // The window is last so GPU surface state is dropped before its native handle.
    window: Arc<Window>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AccessibilityUpdateKind {
    Full,
    ScrollGeometry,
    LayoutGeometry,
}

#[derive(Default)]
struct AccessibilityUpdateSchedule {
    active: bool,
    geometry_deadline: Option<Instant>,
    pending_geometry: Option<AccessibilityUpdateKind>,
    update_due: bool,
}

impl AccessibilityUpdateSchedule {
    fn activate(&mut self) {
        self.active = true;
    }

    fn deactivate(&mut self) {
        *self = Self::default();
    }

    fn semantic_change(&mut self) {
        self.geometry_deadline = None;
        self.pending_geometry = None;
        self.update_due = false;
    }

    /// Coalesce geometry-only scroll and resize updates while keeping accessibility responsive at
    /// 10 Hz. Semantic redraws are never delayed, and an idle correction is scheduled after the
    /// last geometry frame without keeping the event loop awake in between.
    fn should_update(
        &mut self,
        retained_geometry: Option<AccessibilityUpdateKind>,
        now: Instant,
    ) -> Option<AccessibilityUpdateKind> {
        if !self.active {
            return None;
        }

        if let Some(kind) = retained_geometry {
            debug_assert!(kind != AccessibilityUpdateKind::Full);
            self.pending_geometry = Some(match (self.pending_geometry, kind) {
                (Some(AccessibilityUpdateKind::LayoutGeometry), _)
                | (_, AccessibilityUpdateKind::LayoutGeometry) => {
                    AccessibilityUpdateKind::LayoutGeometry
                }
                _ => AccessibilityUpdateKind::ScrollGeometry,
            });
            if self.pending_geometry == Some(AccessibilityUpdateKind::LayoutGeometry) {
                // A resize can produce a new full layout every display refresh. Intermediate
                // accessibility geometry is immediately obsolete, so debounce it and publish one
                // complete correction after the live resize settles. Scroll geometry remains
                // throttled below because assistive navigation benefits from progress updates.
                self.update_due = false;
                self.geometry_deadline = Some(now + ACCESSIBILITY_GEOMETRY_UPDATE_INTERVAL);
                return None;
            }
            let update = self
                .update_due
                .then(|| self.pending_geometry.take())
                .flatten();
            self.update_due = false;
            self.geometry_deadline
                .get_or_insert(now + ACCESSIBILITY_GEOMETRY_UPDATE_INTERVAL);
            return update;
        }

        if std::mem::take(&mut self.update_due) {
            // This is the correction frame requested by `advance`, not a semantic redraw. Retain
            // the pending geometry kind so a finished scroll can still use the one-node update.
            return self.pending_geometry.take();
        }

        self.geometry_deadline = None;
        self.pending_geometry = None;
        Some(AccessibilityUpdateKind::Full)
    }

    fn advance(&mut self, now: Instant) -> bool {
        if !self.active {
            return false;
        }
        if self.geometry_deadline.is_none_or(|deadline| deadline > now) {
            return false;
        }
        self.geometry_deadline = None;
        self.update_due = true;
        true
    }

    fn deadline(&self) -> Option<Instant> {
        self.geometry_deadline
    }
}

impl RuntimeWindow {
    fn any_drag_active(&self) -> bool {
        let active = self.drag_session.is_some() || self.native_file_drag.is_some();
        #[cfg(target_os = "macos")]
        {
            active || self.native_external_drag.is_some() || self.outbound_external_drag.is_some()
        }
        #[cfg(not(target_os = "macos"))]
        {
            active
        }
    }
}

struct WindowEntry {
    handle: WindowHandle,
    config: WindowOptions,
    pending_input: Option<PendingInput>,
    modifiers: Modifiers,
    state: RuntimeWindow,
}

#[derive(Clone, Copy, Debug)]
struct PointerCapture {
    target: ElementId,
    button: MouseButton,
    origin: Point,
    position: Point,
    /// Keep the pressed control's cursor while its bounds follow an asynchronous layout update.
    cursor: CursorIcon,
}

const MAX_SIMULTANEOUS_MOUSE_BUTTONS: usize = 8;
const MOUSE_MULTI_CLICK_INTERVAL: Duration = Duration::from_millis(500);
const MOUSE_MULTI_CLICK_DISTANCE: f32 = 4.0;

#[derive(Clone, Copy, Debug)]
struct PressedMouseButtons {
    buttons: [MouseButton; MAX_SIMULTANEOUS_MOUSE_BUTTONS],
    len: u8,
}

impl Default for PressedMouseButtons {
    fn default() -> Self {
        Self {
            buttons: [MouseButton::Left; MAX_SIMULTANEOUS_MOUSE_BUTTONS],
            len: 0,
        }
    }
}

impl PressedMouseButtons {
    fn press(&mut self, button: MouseButton) {
        self.release(button);
        let len = usize::from(self.len);
        if len < self.buttons.len() {
            self.buttons[len] = button;
            self.len += 1;
        } else {
            self.buttons.rotate_left(1);
            self.buttons[MAX_SIMULTANEOUS_MOUSE_BUTTONS - 1] = button;
        }
    }

    fn release(&mut self, button: MouseButton) {
        let len = usize::from(self.len);
        let Some(index) = self.buttons[..len]
            .iter()
            .position(|pressed| *pressed == button)
        else {
            return;
        };
        self.buttons.copy_within(index + 1..len, index);
        self.len -= 1;
    }

    fn current(self) -> Option<MouseButton> {
        self.len
            .checked_sub(1)
            .map(|index| self.buttons[usize::from(index)])
    }
}

#[derive(Clone, Copy, Debug)]
struct MouseClick {
    button: MouseButton,
    position: Point,
    at: Instant,
    count: usize,
}

#[derive(Clone, Copy, Debug, Default)]
struct ActiveMouseClick {
    button: MouseButton,
    count: usize,
}

#[derive(Debug)]
struct MouseClickTracker {
    last_press: Option<MouseClick>,
    active: [ActiveMouseClick; MAX_SIMULTANEOUS_MOUSE_BUTTONS],
    active_len: u8,
}

impl Default for MouseClickTracker {
    fn default() -> Self {
        Self {
            last_press: None,
            active: [ActiveMouseClick::default(); MAX_SIMULTANEOUS_MOUSE_BUTTONS],
            active_len: 0,
        }
    }
}

impl MouseClickTracker {
    fn press(
        &mut self,
        button: MouseButton,
        position: Point,
        now: Instant,
        native_count: Option<usize>,
    ) -> usize {
        let count = native_count.unwrap_or_else(|| {
            self.last_press
                .filter(|last| {
                    let delta = position - last.position;
                    last.button == button
                        && now.saturating_duration_since(last.at) <= MOUSE_MULTI_CLICK_INTERVAL
                        && delta.x * delta.x + delta.y * delta.y
                            <= MOUSE_MULTI_CLICK_DISTANCE * MOUSE_MULTI_CLICK_DISTANCE
                })
                .map_or(1, |last| last.count.saturating_add(1))
        });
        let count = count.max(1);
        self.last_press = Some(MouseClick {
            button,
            position,
            at: now,
            count,
        });
        self.remove_active(button);
        let len = usize::from(self.active_len);
        if len < self.active.len() {
            self.active[len] = ActiveMouseClick { button, count };
            self.active_len += 1;
        }
        count
    }

    fn release(&mut self, button: MouseButton, native_count: Option<usize>) -> usize {
        let retained = self.active[..usize::from(self.active_len)]
            .iter()
            .find(|active| active.button == button)
            .map(|active| active.count);
        self.remove_active(button);
        native_count.or(retained).unwrap_or(1).max(1)
    }

    fn remove_active(&mut self, button: MouseButton) {
        let len = usize::from(self.active_len);
        let Some(index) = self.active[..len]
            .iter()
            .position(|active| active.button == button)
        else {
            return;
        };
        self.active.copy_within(index + 1..len, index);
        self.active_len -= 1;
    }

    fn cancel(&mut self) {
        self.last_press = None;
        self.active_len = 0;
    }
}

#[derive(Clone, Copy, Debug)]
struct TouchCapture {
    target: ElementId,
    last_event: TouchEvent,
}

const DRAG_THRESHOLD: f32 = 2.0;

#[derive(Clone, Copy, Debug)]
struct DragCandidate {
    source: ElementId,
    origin: Point,
}

struct DragSession {
    source: ElementId,
    position: Point,
    value: Arc<dyn Any>,
    value_type: TypeId,
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    external_payload: Option<ExternalDragPayload>,
}

#[cfg(target_os = "macos")]
struct OutboundExternalDrag {
    source: ElementId,
    _session: MacExternalDragSession,
}

#[cfg(target_os = "macos")]
fn native_drop_origin(
    destination: Option<WindowHandle>,
    payload: &MacNativeDropPayload,
) -> DragOrigin {
    match payload {
        MacNativeDropPayload::Typed(payload) if destination == Some(payload.source_window()) => {
            DragOrigin::Internal(payload.source())
        }
        MacNativeDropPayload::Typed(payload) => DragOrigin::CrossWindow {
            window: payload.source_window(),
            source: payload.source(),
        },
        MacNativeDropPayload::Text(_) | MacNativeDropPayload::Url(_) => DragOrigin::External,
    }
}

#[derive(Default)]
struct NativeFileDrag {
    hovered_count: usize,
    dropped_count: usize,
    hovered_paths: Vec<PathBuf>,
    dropped_paths: Vec<PathBuf>,
    truncated: bool,
    hover_pending: bool,
    hover_value: Option<Arc<DroppedFiles>>,
}

impl NativeFileDrag {
    fn hover(&mut self, path: PathBuf) {
        self.hovered_count = self.hovered_count.saturating_add(1);
        if self.hovered_paths.len() < MAX_DROPPED_FILES {
            self.hovered_paths.push(path);
        } else {
            self.truncated = true;
        }
        self.hover_pending = true;
    }

    fn drop_path(&mut self, path: PathBuf) -> bool {
        self.dropped_count = self.dropped_count.saturating_add(1);
        if self.dropped_paths.len() < MAX_DROPPED_FILES {
            self.dropped_paths.push(path);
        } else {
            self.truncated = true;
        }
        self.hovered_count == 0 || self.dropped_count >= self.hovered_count
    }

    fn take_hovered_files(&mut self) -> Option<DroppedFiles> {
        std::mem::take(&mut self.hover_pending)
            .then(|| DroppedFiles::from_retained(self.hovered_paths.clone(), self.truncated))
    }

    fn into_dropped_files(self) -> DroppedFiles {
        DroppedFiles::from_retained(self.dropped_paths, self.truncated)
    }
}

fn enqueue_entity_events(
    pending: &mut VecDeque<EntityEvent>,
    incoming: &mut Vec<EntityEvent>,
) -> bool {
    if pending.len().saturating_add(incoming.len()) > MAX_PENDING_ENTITY_EVENTS {
        return false;
    }
    pending.extend(incoming.drain(..));
    true
}

fn reserve_entity_event_delivery(deliveries: &mut usize) -> bool {
    if *deliveries >= MAX_ENTITY_EVENT_DELIVERIES_PER_TURN {
        return false;
    }
    *deliveries += 1;
    true
}

fn reserve_global_observer_delivery(deliveries: &mut usize) -> bool {
    if *deliveries >= MAX_GLOBAL_OBSERVER_DELIVERIES_PER_TURN {
        return false;
    }
    *deliveries += 1;
    true
}

fn enqueue_global_notifications(
    pending: &mut VecDeque<TypeId>,
    pending_types: &mut HashSet<TypeId>,
    pending_all: &mut bool,
    incoming: &[TypeId],
    all: bool,
) -> bool {
    if all {
        pending.clear();
        pending_types.clear();
        *pending_all = true;
        return true;
    }
    if *pending_all {
        return true;
    }
    for &global_type in incoming {
        if !pending_types.insert(global_type) {
            continue;
        }
        if pending.len() == MAX_PENDING_GLOBAL_NOTIFICATIONS {
            pending_types.remove(&global_type);
            return false;
        }
        pending.push_back(global_type);
    }
    true
}

fn enqueue_platform_requests(
    pending: &mut VecDeque<PlatformRequest>,
    incoming: &mut Vec<PlatformRequest>,
) {
    for request in incoming.drain(..) {
        if pending.len() == crate::MAX_PENDING_PLATFORM_REQUESTS {
            request.complete_error(PlatformError::PendingQueueFull);
        } else {
            pending.push_back(request);
        }
    }
}

struct Runtime {
    pending_windows: VecDeque<WindowRequest>,
    pending_entity_events: VecDeque<EntityEvent>,
    pending_global_notifications: VecDeque<TypeId>,
    pending_global_notification_types: HashSet<TypeId>,
    pending_all_globals: bool,
    targeted_actions: VecDeque<(WindowHandle, AnyAction)>,
    windows: HashMap<WindowId, WindowEntry>,
    window_handles: HashMap<WindowHandle, WindowId>,
    window_registry_cache: RefCell<WindowRegistryCache>,
    current_window: Option<(WindowId, WindowHandle)>,
    active_window: Option<WindowId>,
    focus_history: Vec<WindowId>,
    close_requests: Vec<WindowHandle>,
    focus_requests: Vec<WindowHandle>,
    invalidate_requests: Vec<WindowHandle>,
    window_commands: Vec<WindowCommand>,
    /// Bounded window lifecycle events produced while every window is deactivated.
    pending_window_events: Vec<(WindowHandle, Event)>,
    external_menus: Option<Vec<Menu>>,
    /// Per-window native menu replacements declared through [`AppRunner`].
    external_window_menus: VecDeque<ExternalWindowMenus>,
    /// Native popup-menu requests declared through [`AppRunner`].
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    external_popup_menus: VecDeque<ExternalPopupMenuRequest>,
    pending_initial_open_urls: Option<OpenUrls>,
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    pending_native_popup_menus: HashMap<u64, PendingNativePopupMenu>,
    platform_requests: VecDeque<PlatformRequest>,
    pending_global_shortcut_commands: VecDeque<global_shortcut::GlobalShortcutCommand>,
    pending_tray_commands: VecDeque<tray::TrayCommand>,
    pending_display_events: VecDeque<DisplayEvent>,
    global_shortcut_state: global_shortcut::GlobalShortcutState,
    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    global_shortcuts: HashMap<u32, global_shortcut::RegisteredGlobalShortcut>,
    tray_icons: HashMap<u32, tray::NativeTrayIcon>,
    #[cfg(any(
        target_os = "macos",
        target_os = "windows",
        target_os = "linux",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "openbsd",
        target_os = "netbsd"
    ))]
    single_instance: Option<single_instance::SingleInstanceGuard>,
    image_workers: ImageWorkerPoolHandle,
    background_tasks: BackgroundTaskPoolHandle,
    foreground_tasks: ForegroundTaskSpawner,
    app_info: Option<AppInfo>,
    app_paths: Option<AppPaths>,
    system_info: SystemInfo,
    system_preferences: SystemPreferences,
    globals: GlobalStore,
    assets: Assets,
    font_system: SharedFontSystem,
    gpu_contexts: HashMap<PerformanceProfile, GpuContext>,
    #[cfg(target_arch = "wasm32")]
    web_canvas: Option<web_sys::HtmlCanvasElement>,
    displays: Displays,
    keyboard: KeyboardState,
    #[cfg(target_os = "macos")]
    native_drag_registry: MacTypedDragRegistry,
    #[cfg(target_os = "macos")]
    popover_monitor: MacPopoverMonitor,
    #[cfg(any(
        target_os = "macos",
        target_os = "windows",
        target_os = "linux",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "openbsd",
        target_os = "netbsd"
    ))]
    active_platform_dialogs: HashMap<Option<WindowHandle>, platform_dialog::ActivePlatformDialog>,
    #[cfg(target_os = "macos")]
    automatic_tabbing_baseline: Option<bool>,
    #[cfg(target_os = "macos")]
    tabbing_window_count: usize,
    #[cfg(target_os = "macos")]
    mac_application_host: Option<MacApplicationHost>,
    #[cfg(target_os = "macos")]
    native_termination_pending: bool,
    #[cfg(target_os = "windows")]
    _windows_power_monitor: Option<power_monitor::WindowsPowerMonitor>,
    #[cfg(target_os = "linux")]
    _linux_power_monitor: Option<power_monitor::LinuxPowerMonitor>,
    application_callbacks: ApplicationCallbacks,
    quit_mode: QuitMode,
    ready: bool,
    opened_window: bool,
    exit_requested: bool,
    /// Process exit code requested through `EventContext::exit_with_code`.
    exit_code: Option<i32>,
    pending_quit: Option<QuitReason>,
    quit_phase_active: bool,
    last_window_quit_prevented: bool,
    relaunch_request: Option<RelaunchRequest>,
    process_services_finalized: bool,
    // The following four fields are the currently activated window. Event delivery is serialized
    // by Winit, so moving one entry into this slot keeps the mature single-window hot path narrow
    // while every inactive window remains independently retained in `windows`.
    config: WindowOptions,
    keymap: Keymap,
    menus: Vec<Menu>,
    menu_actions: Vec<MenuAction>,
    #[cfg(target_os = "macos")]
    dock_menu: Option<Menu>,
    #[cfg(target_os = "macos")]
    dock_menu_actions: Vec<MenuAction>,
    #[cfg(target_os = "macos")]
    dock_badge: Option<Arc<str>>,
    #[cfg(target_os = "macos")]
    dock_icon: Option<Image>,
    #[cfg(target_os = "macos")]
    menu_host: Option<MacMenuHost>,
    #[cfg(target_os = "windows")]
    windows_menu_host: Option<windows_menu::WindowsMenuHost>,
    pending_input: Option<PendingInput>,
    window: Option<RuntimeWindow>,
    modifiers: Modifiers,
    fatal_error: Option<AppError>,
    event_proxy: EventLoopProxy<RuntimeEvent>,
    clipboard: ClipboardService,
    form_submission_depth: u8,
    animation_epoch: Instant,
}

struct RuntimeStartup {
    initial_window: Option<WindowRequest>,
    app_info: Option<AppInfo>,
    app_paths: Option<AppPaths>,
    globals: GlobalStore,
    keymap: Keymap,
    menus: Vec<Menu>,
    assets: Assets,
    fonts: Vec<FontSource>,
    application_callbacks: ApplicationCallbacks,
    quit_mode: QuitMode,
}

#[derive(Default)]
struct WindowRegistryCache {
    handles: Arc<[WindowHandle]>,
    truncated: bool,
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
struct PendingNativePopupMenu {
    window: WindowHandle,
    actions: Vec<MenuAction>,
    /// Completed once the native popup closes, so an externally pumped host can await it.
    responder: Option<PlatformResponder<()>>,
}

/// One `AppRunner`-scoped native popup-menu request awaiting a window-scoped effect cycle.
///
/// `EventContext::show_native_popup_menu` needs the runtime's current window, which only exists
/// inside an effect cycle. An embedding host therefore declares the request here and the runtime
/// resolves it during the next `process_window_commands` turn.
#[cfg(any(target_os = "macos", target_os = "windows"))]
pub(crate) struct ExternalPopupMenuRequest {
    pub(crate) window: WindowHandle,
    pub(crate) menu: Menu,
    pub(crate) position: Option<Point>,
    pub(crate) responder: PlatformResponder<()>,
}

/// One `AppRunner`-scoped per-window native menu replacement awaiting an effect cycle.
pub(crate) struct ExternalWindowMenus {
    pub(crate) window: WindowHandle,
    /// `Some(menus)` overrides the window; `None` restores inheritance of the application menus.
    pub(crate) menus: Option<Vec<Menu>>,
}

#[derive(Clone, Debug)]
struct PendingKey {
    stroke: Keystroke,
    repeat: bool,
    text: Option<String>,
}

impl PendingKey {
    fn keystroke(&self) -> Keystroke {
        self.stroke.clone()
    }
}

#[derive(Clone, Debug)]
struct PendingInput {
    keys: Vec<PendingKey>,
    focus: Option<ElementId>,
    deadline: Instant,
}

impl Drop for Runtime {
    fn drop(&mut self) {
        self.finalize_process_services();
    }
}

impl ApplicationHandler<RuntimeEvent> for Runtime {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.handle_resumed(event_loop);
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        self.handle_suspended(event_loop);
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        self.handle_exiting(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        self.handle_window_event(event_loop, window_id, event);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: RuntimeEvent) {
        self.handle_user_event(event_loop, event);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.handle_about_to_wait(event_loop);
    }
}

fn apply_windowed_geometry(state: &mut RuntimeWindow, bounds: Rect) {
    state.restore_bounds = bounds;
    state.logical_position = Point::new(bounds.x, bounds.y);
    state
        .window
        .set_outer_position(LogicalPosition::new(bounds.x as f64, bounds.y as f64));
    if let Some(physical) = state
        .window
        .request_inner_size(LogicalSize::new(bounds.width as f64, bounds.height as f64))
    {
        state.renderer.resize(physical.width, physical.height);
        state.logical_size = logical_window_size(physical, state.scale_factor);
    }
    state.layout_dirty = true;
    state.view_dirty |= state.listeners.observes_viewport;
}

/// Apply a resize command without sending a redundant native move for the unchanged origin.
///
/// On macOS, repeatedly calling `setFrameOrigin:` as part of a size-only animation forces extra
/// window-server work and can make the content visibly lag behind the resize wave. Exiting a
/// special state still follows the same contract as `set_window_bounds`; AppKit restores the
/// saved origin and this function changes only the requested content size.
fn apply_window_size(state: &mut RuntimeWindow, size: Size) {
    state.window.set_minimized(false);
    state.window.set_fullscreen(None);
    if state.maximized {
        state.window.set_maximized(false);
    }
    state.maximized = false;
    state.restore_bounds.width = size.width;
    state.restore_bounds.height = size.height;
    if let Some(physical) = state
        .window
        .request_inner_size(LogicalSize::new(size.width as f64, size.height as f64))
    {
        state.renderer.resize(physical.width, physical.height);
        state.logical_size = logical_window_size(physical, state.scale_factor);
    }
    state.layout_dirty = true;
    state.view_dirty |= state.listeners.observes_viewport;
}

fn apply_window_bounds(state: &mut RuntimeWindow, bounds: WindowBounds) {
    let restore = bounds.bounds();
    state.window.set_minimized(false);
    state.window.set_fullscreen(None);
    if state.maximized {
        state.window.set_maximized(false);
    }
    state.maximized = false;
    apply_windowed_geometry(state, restore);
    match bounds {
        WindowBounds::Windowed(_) => {}
        WindowBounds::Maximized(_) => {
            state.window.set_maximized(true);
            state.maximized = true;
        }
        WindowBounds::Fullscreen(_) => state
            .window
            .set_fullscreen(Some(Fullscreen::Borderless(state.window.current_monitor()))),
    }
}

fn set_runtime_window_fullscreen(state: &mut RuntimeWindow, fullscreen: bool) -> bool {
    if state.window.fullscreen().is_some() == fullscreen {
        return false;
    }
    if fullscreen {
        if !state.maximized {
            state.restore_bounds = Rect::new(
                state.logical_position.x,
                state.logical_position.y,
                state.logical_size.width,
                state.logical_size.height,
            );
        }
        if state.maximized {
            state.window.set_maximized(false);
            state.maximized = false;
        }
        state
            .window
            .set_fullscreen(Some(Fullscreen::Borderless(state.window.current_monitor())));
    } else {
        apply_window_bounds(state, WindowBounds::Windowed(state.restore_bounds));
    }
    true
}

fn constrained_window_size(size: Size, minimum: Option<Size>, maximum: Option<Size>) -> Size {
    let size = minimum.map_or(size, |minimum| {
        Size::new(
            size.width.max(minimum.width),
            size.height.max(minimum.height),
        )
    });
    maximum.map_or(size, |maximum| {
        Size::new(
            size.width.min(maximum.width),
            size.height.min(maximum.height),
        )
    })
}

fn sane_scale_factor(value: f64) -> f32 {
    if value.is_finite() && value > 0.0 {
        value as f32
    } else {
        1.0
    }
}

fn map_window_appearance(theme: Theme) -> WindowAppearance {
    match theme {
        Theme::Light => WindowAppearance::Light,
        Theme::Dark => WindowAppearance::Dark,
    }
}

fn to_winit_theme(appearance: WindowAppearance) -> Theme {
    match appearance {
        WindowAppearance::Light => Theme::Light,
        WindowAppearance::Dark => Theme::Dark,
    }
}

fn logical_window_size(physical: PhysicalSize<u32>, scale_factor: f32) -> Size {
    Size::new(
        physical.width as f32 / scale_factor,
        physical.height as f32 / scale_factor,
    )
}

fn logical_window_position(window: &Window, scale_factor: f32) -> Option<Point> {
    let position = window.outer_position().ok()?;
    Some(Point::new(
        position.x as f32 / scale_factor,
        position.y as f32 / scale_factor,
    ))
}

fn runtime_window_is_fullscreen(state: &RuntimeWindow) -> bool {
    #[cfg(target_os = "macos")]
    {
        if !runtime_window_content_attached(state) {
            state.window.fullscreen().is_some()
        } else {
            is_window_fullscreen(&state.window)
                .unwrap_or_else(|_| state.window.fullscreen().is_some())
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        state.window.fullscreen().is_some()
    }
}

fn runtime_window_is_maximized(state: &RuntimeWindow, config: &WindowOptions) -> bool {
    #[cfg(target_os = "macos")]
    {
        if !runtime_window_content_attached(state)
            || config.title_bar_style == TitleBarStyle::Hidden
        {
            state.maximized
        } else {
            is_window_maximized(&state.window).unwrap_or(state.maximized)
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = config;
        state.window.is_maximized()
    }
}

fn runtime_window_display_id(state: &RuntimeWindow, displays: &Displays) -> Option<DisplayId> {
    crate::display::display_for_rect(
        displays,
        Rect::new(
            state.logical_position.x,
            state.logical_position.y,
            state.logical_size.width,
            state.logical_size.height,
        ),
    )
    .or_else(|| {
        state
            .window
            .current_monitor()
            .map(|monitor| crate::display::native_display_id(&monitor))
            .filter(|id| displays.find(*id).is_some())
    })
}

fn runtime_window_content_attached(state: &RuntimeWindow) -> bool {
    #[cfg(target_os = "macos")]
    {
        state
            .first_frame_guard
            .as_ref()
            .is_none_or(MacFirstFrameGuard::content_attached)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = state;
        true
    }
}

fn runtime_window_state(
    handle: WindowHandle,
    config: &WindowOptions,
    state: &RuntimeWindow,
) -> WindowState {
    let platform_content_attached = runtime_window_content_attached(state);
    let fullscreen = runtime_window_is_fullscreen(state);
    let maximized = !fullscreen && runtime_window_is_maximized(state, config);
    let minimized = platform_content_attached && state.window.is_minimized().unwrap_or(false);
    let current_bounds = Rect::new(
        state.logical_position.x,
        state.logical_position.y,
        state.logical_size.width,
        state.logical_size.height,
    );
    let bounds = if fullscreen {
        WindowBounds::Fullscreen(state.restore_bounds)
    } else if maximized {
        WindowBounds::Maximized(state.restore_bounds)
    } else {
        WindowBounds::Windowed(current_bounds)
    };
    WindowState {
        handle,
        display_id: state.display_id,
        kind: config.kind,
        bounds,
        viewport_size: state.logical_size,
        minimum_size: config.minimum_size,
        maximum_size: config.maximum_size,
        scale_factor: state.scale_factor,
        appearance: state.appearance,
        background_appearance: config.window_background,
        macos_vibrancy: config.macos_vibrancy,
        macos_visual_effect_state: config.macos_visual_effect_state,
        focused: state.focused,
        focusable: config.focusable,
        visible: state.visible,
        minimized,
        maximized,
        fullscreen,
        occluded: state.occluded,
        movable: config.is_movable,
        resizable: config.is_resizable,
        minimizable: config.is_minimizable,
        maximizable: config.is_maximizable,
        closable: config.is_closable,
        decorated: config.decorated,
        shadow: config.shadow,
        content_protected: config.content_protected,
        window_level: effective_window_level(config),
        ignore_mouse_events: config.ignore_mouse_events,
        forward_mouse_events: config.forward_mouse_events,
        window_enabled: config.window_enabled,
        aspect_ratio: config.aspect_ratio,
        window_buttons_visible: config.window_buttons_visible,
        skip_taskbar: config.skip_taskbar,
        visible_on_all_workspaces: effective_visible_on_all_workspaces(config),
        opacity: config.opacity,
        has_icon: config.icon.is_some(),
        taskbar_progress_state: config.taskbar_progress_state,
        taskbar_progress: config.taskbar_progress,
        has_taskbar_overlay_icon: config.taskbar_overlay_icon.is_some(),
        cursor_visible: config.cursor_visible,
        cursor_grab: config.cursor_grab,
        cursor_hit_test: config.cursor_hit_test,
        cursor_position: state.pointer,
        represented_file: config.represented_file.is_some(),
        document_edited: config.document_edited,
        native_tabbing: config.tabbing_identifier.is_some(),
        native_tabs: state.native_tabs,
        #[cfg(feature = "inspector")]
        inspector_active: state.inspector.is_some(),
    }
}

fn window_buttons(config: &WindowOptions) -> WindowButtons {
    let mut buttons = WindowButtons::empty();
    if config.is_closable {
        buttons |= WindowButtons::CLOSE;
    }
    if config.is_minimizable {
        buttons |= WindowButtons::MINIMIZE;
    }
    if config.is_resizable && config.is_maximizable {
        buttons |= WindowButtons::MAXIMIZE;
    }
    buttons
}

fn effective_window_level(config: &WindowOptions) -> WindowLevel {
    config.window_level.unwrap_or(match config.kind {
        WindowKind::Floating | WindowKind::Popover | WindowKind::SystemPopover => {
            WindowLevel::AlwaysOnTop
        }
        WindowKind::Normal | WindowKind::Dialog => WindowLevel::Normal,
    })
}

fn effective_visible_on_all_workspaces(config: &WindowOptions) -> bool {
    config.visible_on_all_workspaces
        || matches!(config.kind, WindowKind::Popover | WindowKind::SystemPopover)
}

#[cfg(any(target_os = "macos", test))]
fn window_presentation_activates_application(config: &WindowOptions) -> bool {
    config.focus
        && config.focusable
        && !matches!(config.kind, WindowKind::Popover | WindowKind::SystemPopover)
}

fn winit_window_icon(image: &Image) -> Icon {
    Icon::from_rgba(image.rgba().to_vec(), image.width(), image.height())
        .expect("QuickGUI Image has already validated its RGBA dimensions")
}

fn window_dismisses_system_popover_on_escape(config: &WindowOptions) -> bool {
    config.kind == WindowKind::SystemPopover
        && config
            .popover
            .as_ref()
            .is_some_and(|popover| popover.dismiss_on_escape)
}

fn window_dismisses_system_popover_on_pointer_outside(config: &WindowOptions) -> bool {
    config.kind == WindowKind::SystemPopover
        && config
            .popover
            .as_ref()
            .is_some_and(|popover| popover.dismiss_on_pointer_outside)
}

fn window_is_never_key_popover(config: &WindowOptions) -> bool {
    config.kind == WindowKind::SystemPopover
        && config
            .popover
            .as_ref()
            .is_some_and(|popover| !popover.accepts_key_focus)
}

#[cfg(target_os = "macos")]
fn implicit_native_movable(config: &WindowOptions) -> bool {
    config.is_movable && config.title_bar_style == TitleBarStyle::Default
}

#[cfg(target_os = "macos")]
fn point_outside_viewport(point: Point, viewport: Size) -> bool {
    point.x < 0.0 || point.y < 0.0 || point.x > viewport.width || point.y > viewport.height
}

fn map_mouse_button(button: winit::event::MouseButton) -> MouseButton {
    match button {
        winit::event::MouseButton::Left => MouseButton::Left,
        winit::event::MouseButton::Right => MouseButton::Right,
        winit::event::MouseButton::Middle => MouseButton::Middle,
        winit::event::MouseButton::Back => MouseButton::Back,
        winit::event::MouseButton::Forward => MouseButton::Forward,
        winit::event::MouseButton::Other(value) => MouseButton::Other(value),
    }
}

fn platform_cursor(style: CursorStyle) -> CursorIcon {
    match style {
        CursorStyle::Arrow => CursorIcon::Default,
        CursorStyle::IBeam => CursorIcon::Text,
        CursorStyle::Crosshair => CursorIcon::Crosshair,
        CursorStyle::ClosedHand => CursorIcon::Grabbing,
        CursorStyle::OpenHand => CursorIcon::Grab,
        CursorStyle::PointingHand => CursorIcon::Pointer,
        CursorStyle::ResizeLeft => CursorIcon::WResize,
        CursorStyle::ResizeRight => CursorIcon::EResize,
        CursorStyle::ResizeLeftRight => CursorIcon::EwResize,
        CursorStyle::ResizeUp => CursorIcon::NResize,
        CursorStyle::ResizeDown => CursorIcon::SResize,
        CursorStyle::ResizeUpDown => CursorIcon::NsResize,
        CursorStyle::ResizeUpLeftDownRight => CursorIcon::NwseResize,
        CursorStyle::ResizeUpRightDownLeft => CursorIcon::NeswResize,
        CursorStyle::ResizeColumn => CursorIcon::ColResize,
        CursorStyle::ResizeRow => CursorIcon::RowResize,
        CursorStyle::IBeamCursorForVerticalLayout => CursorIcon::VerticalText,
        CursorStyle::OperationNotAllowed => CursorIcon::NotAllowed,
        CursorStyle::DragLink => CursorIcon::Alias,
        CursorStyle::DragCopy => CursorIcon::Copy,
        CursorStyle::ContextualMenu => CursorIcon::ContextMenu,
    }
}

#[cfg(feature = "inspector")]
fn reconcile_inspector_pointer_state(state: &mut RuntimeWindow) -> bool {
    state.pointer_capture = None;
    state.drag_candidate = None;
    let now = Instant::now();
    let repaint = if state.inspector.is_some() {
        state.ui.cancel_pointer_interaction()
            | state.ui.pointer_left()
            | state.ui.update_scrollbar_hover(None, now)
    } else {
        let scrollbar_changed = state.ui.update_scrollbar_hover(state.pointer, now);
        let pointer_changed = state.pointer.is_some_and(|point| {
            let RuntimeWindow { ui, renderer, .. } = state;
            ui.pointer_moved(point, renderer)
        });
        scrollbar_changed | pointer_changed
    };
    let cursor = state
        .pointer
        .map_or(CursorIcon::Default, |point| desired_cursor(state, point));
    set_cursor_if_changed(state, cursor);
    repaint
}

fn desired_cursor(state: &RuntimeWindow, point: Point) -> CursorIcon {
    #[cfg(feature = "inspector")]
    if let Some(inspector) = &state.inspector
        && inspector.captures_pointer(point)
    {
        return if inspector.mode() == InspectorMode::Picking && !inspector.panel_contains(point) {
            CursorIcon::Crosshair
        } else {
            CursorIcon::Default
        };
    }
    if state.drag_session.is_some() {
        return CursorIcon::Grabbing;
    }
    if let Some(capture) = state.pointer_capture {
        return capture.cursor;
    }
    if state.ui.scrollbar_drag_active()
        || state.ui.is_over_scrollbar(point)
        || state.ui.is_app_region_drag(point)
    {
        return CursorIcon::Default;
    }
    state
        .ui
        .cursor_style_at(point)
        .map_or(CursorIcon::Default, platform_cursor)
}

fn set_cursor_if_changed(state: &mut RuntimeWindow, cursor: CursorIcon) {
    if cursor != state.cursor {
        state.cursor = cursor;
        state.window.set_cursor(cursor);
    }
}

fn map_gesture_phase(phase: winit::event::TouchPhase) -> GesturePhase {
    match phase {
        winit::event::TouchPhase::Started => GesturePhase::Started,
        winit::event::TouchPhase::Moved => GesturePhase::Moved,
        winit::event::TouchPhase::Ended => GesturePhase::Ended,
        winit::event::TouchPhase::Cancelled => GesturePhase::Cancelled,
    }
}

fn map_touch_phase(phase: winit::event::TouchPhase) -> TouchPhase {
    match phase {
        winit::event::TouchPhase::Started => TouchPhase::Started,
        winit::event::TouchPhase::Moved => TouchPhase::Moved,
        winit::event::TouchPhase::Ended => TouchPhase::Ended,
        winit::event::TouchPhase::Cancelled => TouchPhase::Cancelled,
    }
}

fn bounded_touch_force(force: Force) -> f32 {
    bounded_pressure(force.normalized() as f32)
}

fn bounded_pressure(pressure: f32) -> f32 {
    if pressure.is_finite() {
        pressure.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn bounded_gesture_delta(delta: f64, limit: f32) -> f32 {
    if delta.is_finite() {
        (delta as f32).clamp(-limit, limit)
    } else {
        0.0
    }
}

fn map_pressure_stage(stage: i64) -> PressureStage {
    match stage {
        0 => PressureStage::Zero,
        1 => PressureStage::Normal,
        2 => PressureStage::Force,
        stage => PressureStage::Other(stage),
    }
}

fn map_modifiers(state: ModifiersState) -> Modifiers {
    let mut result = Modifiers::empty();
    result.set(Modifiers::SHIFT, state.shift_key());
    result.set(Modifiers::CONTROL, state.control_key());
    result.set(Modifiers::ALT, state.alt_key());
    result.set(Modifiers::SUPER, state.super_key());
    result
}

fn primary_modifier(modifiers: Modifiers) -> bool {
    if cfg!(target_os = "macos") {
        modifiers.contains(Modifiers::SUPER)
    } else {
        modifiers.contains(Modifiers::CONTROL)
    }
}

fn word_modifier(modifiers: Modifiers) -> bool {
    if cfg!(target_os = "macos") {
        modifiers.contains(Modifiers::ALT)
    } else {
        modifiers.contains(Modifiers::CONTROL)
    }
}

#[cfg(target_os = "macos")]
fn is_default_close_shortcut(key: &Key, modifiers: Modifiers, repeat: bool) -> bool {
    !repeat
        && modifiers == Modifiers::SUPER
        && matches!(key, Key::Character(value) if value.eq_ignore_ascii_case("w"))
}

#[cfg(test)]
mod tests;

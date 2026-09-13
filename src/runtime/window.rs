use super::*;

/// Defines when closing the final native window should terminate the application.
///
/// The default follows native desktop convention: macOS applications stay resident for Dock
/// reopen and menu commands, while other desktop targets quit with their last window.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum QuitMode {
    /// Use [`Self::Explicit`] on macOS and [`Self::LastWindowClosed`] elsewhere.
    #[default]
    Default,
    /// Quit automatically after the final window and all of its owned children close.
    LastWindowClosed,
    /// Stay in the event loop until [`EventContext::exit`] or the operating system quits the app.
    Explicit,
}

/// Source of one application-level quit request.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum QuitReason {
    Explicit,
    Relaunch,
    LastWindowClosed,
    OperatingSystem,
}

/// Immutable input delivered to the preventable before-quit and will-quit callbacks.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct QuitRequest {
    pub reason: QuitReason,
}

impl QuitMode {
    pub(super) const fn quits_when_empty(self) -> bool {
        match self {
            Self::Default => cfg!(not(target_os = "macos")),
            Self::LastWindowClosed => true,
            Self::Explicit => false,
        }
    }
}

/// GPU selection policy. `Balanced` lets WGPU choose the most appropriate adapter.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum PerformanceProfile {
    #[default]
    Balanced,
    LowPower,
    HighPerformance,
}

/// Effective light or dark appearance of a native window.
///
/// QuickGUI intentionally exposes the semantic palette instead of platform appearance names.
/// Application content can observe this value through [`ViewContext::appearance`], while native
/// chrome follows the system unless an explicit per-window preference is configured.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum WindowAppearance {
    #[default]
    Light,
    Dark,
}

impl WindowAppearance {
    pub const fn is_dark(self) -> bool {
        matches!(self, Self::Dark)
    }
}

/// Compositor treatment for pixels not covered by fully opaque application content.
///
/// `Transparent` exposes the desktop directly. `Blurred` uses the platform's native background
/// blur behind the same alpha-capable WGPU surface. Both remain damage-driven; they do not add an
/// application animation or polling loop.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum WindowBackgroundAppearance {
    #[default]
    Opaque,
    Transparent,
    Blurred,
}

impl WindowBackgroundAppearance {
    pub const fn is_transparent(self) -> bool {
        !matches!(self, Self::Opaque)
    }

    pub const fn is_blurred(self) -> bool {
        matches!(self, Self::Blurred)
    }

    #[cfg(test)]
    pub(super) const fn changes_from(self, previous: Self) -> WindowBackgroundChanges {
        WindowBackgroundChanges {
            transparency: self.is_transparent() != previous.is_transparent(),
            blur: self.is_blurred() != previous.is_blurred(),
        }
    }
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct WindowBackgroundChanges {
    pub(super) transparency: bool,
    pub(super) blur: bool,
}

/// Semantic macOS material rendered behind transparent application pixels.
///
/// The variants intentionally match Electron's current `vibrancy` vocabulary while retaining a
/// typed Rust-core source of truth. On macOS QuickGUI projects these values through one
/// `NSVisualEffectView`; other platforms retain the requested state without installing a native
/// effect.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MacOsVibrancy {
    AppearanceBased,
    Titlebar,
    Selection,
    Menu,
    Popover,
    Sidebar,
    Header,
    Sheet,
    Window,
    Hud,
    FullscreenUi,
    Tooltip,
    Content,
    UnderWindow,
    UnderPage,
}

/// Whether a macOS vibrancy material follows the window's active state.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum MacOsVisualEffectState {
    #[default]
    FollowWindow,
    Active,
    Inactive,
}

/// Native window titlebar presentation.
///
/// `HiddenInset` is currently meaningful on macOS. It keeps the native window controls while
/// extending application content through a transparent, titleless titlebar.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum TitleBarStyle {
    #[default]
    Default,
    HiddenInset,
    /// Hide native titlebar chrome and let application content own the complete window.
    Hidden,
}

/// Native role and parent relationship of a top-level window.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum WindowKind {
    #[default]
    Normal,
    /// A high-level utility or notification window.
    Popover,
    /// A transient native popover positioned relative to its parent window's content.
    SystemPopover,
    /// A utility window that stays above ordinary application windows.
    Floating,
    /// A parent-owned modal sheet on macOS.
    Dialog,
}

/// Requested native stacking level independent from a window's ownership role.
///
/// The first three variants are the portable levels every backend understands. The remaining
/// variants name AppKit's `NSWindowLevel` constants; other platforms collapse them to the
/// topmost hint their window manager exposes. [`WindowLevel::macos_level`] documents the exact
/// mapping and is used by the macOS backend.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum WindowLevel {
    AlwaysOnBottom,
    #[default]
    Normal,
    AlwaysOnTop,
    /// AppKit `NSFloatingWindowLevel`.
    Floating,
    /// AppKit `NSModalPanelWindowLevel`.
    ModalPanel,
    /// AppKit `NSMainMenuWindowLevel`.
    MainMenu,
    /// AppKit `NSStatusWindowLevel`.
    Status,
    /// AppKit `NSPopUpMenuWindowLevel`.
    PopUpMenu,
    /// AppKit `NSScreenSaverWindowLevel`.
    ScreenSaver,
}

/// Native taskbar progress presentation for one window.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum TaskbarProgressState {
    /// Remove progress from the taskbar button.
    #[default]
    None,
    Normal,
    Indeterminate,
    Paused,
    Error,
}

/// Native pointer confinement policy for one window.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum CursorGrabMode {
    /// Let the pointer move without confinement.
    #[default]
    None,
    /// Keep the pointer inside the window when the backend supports confinement.
    Confined,
    /// Lock the pointer to the window while continuing to report relative movement.
    Locked,
}

impl CursorGrabMode {
    pub(super) const fn to_winit(self) -> WinitCursorGrabMode {
        match self {
            Self::None => WinitCursorGrabMode::None,
            Self::Confined => WinitCursorGrabMode::Confined,
            Self::Locked => WinitCursorGrabMode::Locked,
        }
    }
}

impl WindowLevel {
    pub(super) const fn to_winit(self) -> WinitWindowLevel {
        match self {
            Self::AlwaysOnBottom => WinitWindowLevel::AlwaysOnBottom,
            Self::Normal => WinitWindowLevel::Normal,
            Self::AlwaysOnTop
            | Self::Floating
            | Self::ModalPanel
            | Self::MainMenu
            | Self::Status
            | Self::PopUpMenu
            | Self::ScreenSaver => WinitWindowLevel::AlwaysOnTop,
        }
    }

    /// Raw AppKit `NSWindowLevel` for this stacking policy.
    ///
    /// `Normal` is `NSNormalWindowLevel` (0), `AlwaysOnBottom` is one level below it, and
    /// `AlwaysOnTop` shares `NSFloatingWindowLevel` with [`Self::Floating`] so the historical
    /// portable value keeps its established behavior.
    pub const fn macos_level(self) -> i32 {
        match self {
            Self::AlwaysOnBottom => -1,
            Self::Normal => 0,
            Self::AlwaysOnTop | Self::Floating => 3,
            Self::ModalPanel => 8,
            Self::MainMenu => 24,
            Self::Status => 25,
            Self::PopUpMenu => 101,
            Self::ScreenSaver => 1_000,
        }
    }

    /// Whether this level keeps the window above ordinary application windows.
    pub const fn is_above_normal(self) -> bool {
        self.macos_level() > 0
    }
}

/// Persistable window state together with its windowed restore geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WindowBounds {
    Windowed(Rect),
    Maximized(Rect),
    Fullscreen(Rect),
}

impl WindowBounds {
    pub const fn bounds(self) -> Rect {
        match self {
            Self::Windowed(bounds) | Self::Maximized(bounds) | Self::Fullscreen(bounds) => bounds,
        }
    }

    pub const fn windowed(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self::Windowed(Rect::new(x, y, width, height))
    }
}

impl Default for WindowBounds {
    fn default() -> Self {
        Self::Windowed(Rect::new(0.0, 0.0, 960.0, 640.0))
    }
}

/// Retained native state available during declarative rendering.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowState {
    pub handle: WindowHandle,
    /// Display currently containing the native window, when known.
    pub display_id: Option<DisplayId>,
    pub kind: WindowKind,
    pub bounds: WindowBounds,
    pub viewport_size: Size,
    /// Effective native minimum inner size, or `None` when unconstrained.
    pub minimum_size: Option<Size>,
    /// Effective native maximum inner size, or `None` when unconstrained.
    pub maximum_size: Option<Size>,
    pub scale_factor: f32,
    pub appearance: WindowAppearance,
    pub background_appearance: WindowBackgroundAppearance,
    /// Active macOS semantic vibrancy material, when requested.
    pub macos_vibrancy: Option<MacOsVibrancy>,
    pub macos_visual_effect_state: MacOsVisualEffectState,
    pub focused: bool,
    /// Whether the native window is permitted to receive keyboard focus.
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
    pub window_level: WindowLevel,
    /// Whether native pointer events pass through this window to whatever is behind it.
    pub ignore_mouse_events: bool,
    /// Whether pointer motion is still delivered to this window while clicks pass through.
    pub forward_mouse_events: bool,
    /// Whether the native window currently accepts any input at all.
    pub window_enabled: bool,
    /// Retained content aspect ratio as `width:height`, or `None` when unconstrained.
    pub aspect_ratio: Option<Size>,
    /// Whether the macOS close/minimize/zoom buttons are visible.
    pub window_buttons_visible: bool,
    /// Whether this window is omitted from the taskbar on supported platforms.
    pub skip_taskbar: bool,
    /// Whether this window follows the user across virtual desktops/spaces.
    pub visible_on_all_workspaces: bool,
    /// Whole-window native alpha in the inclusive `0.0..=1.0` range.
    pub opacity: f32,
    /// Whether an explicit native window icon is installed.
    pub has_icon: bool,
    /// Retained taskbar progress mode and bounded completion value.
    pub taskbar_progress_state: TaskbarProgressState,
    pub taskbar_progress: f32,
    /// Whether a Windows taskbar overlay icon is installed.
    pub has_taskbar_overlay_icon: bool,
    pub cursor_visible: bool,
    pub cursor_grab: CursorGrabMode,
    /// Whether the native window participates in pointer hit testing.
    pub cursor_hit_test: bool,
    /// Last known pointer position in logical window coordinates.
    pub cursor_position: Option<Point>,
    /// Whether the native titlebar currently represents a document file.
    pub represented_file: bool,
    /// Whether native chrome indicates that the represented document has unsaved changes.
    pub document_edited: bool,
    /// Whether this window opted into native system tabbing.
    pub native_tabbing: bool,
    /// Bounded cached state for the native AppKit tab group.
    pub native_tabs: WindowTabState,
    /// Whether the feature-gated retained-tree inspector is open for this window.
    #[cfg(feature = "inspector")]
    pub inspector_active: bool,
}

impl WindowState {
    /// Capture persistable geometry and display identity for this window.
    ///
    /// The rectangle is the windowed restore geometry, so a maximized or fullscreen window still
    /// persists the size it returns to. Pass the result to
    /// [`WindowOptions::restore`](WindowOptions::restore) on the next launch.
    pub fn restore_state(&self, displays: &Displays) -> WindowRestoreState {
        let bounds = self.bounds.bounds();
        let display = self.display_id.and_then(|id| displays.find(id));
        WindowRestoreState {
            x: bounds.x,
            y: bounds.y,
            width: bounds.width,
            height: bounds.height,
            maximized: self.maximized,
            fullscreen: self.fullscreen,
            display_id: self.display_id.map(DisplayId::get),
            display_uuid: display
                .and_then(Display::uuid)
                .map(crate::DisplayUuid::into_bytes),
            scale_factor: self.scale_factor,
        }
    }
}

/// Maximum UTF-8 bytes accepted for a native window title.
pub const MAX_WINDOW_TITLE_BYTES: usize = 16 * 1024;
/// Maximum encoded bytes accepted for a represented document path.
pub const MAX_WINDOW_DOCUMENT_PATH_BYTES: usize = 16 * 1024;
/// Maximum UTF-8 bytes accepted for a native window tabbing identifier.
pub const MAX_WINDOW_TABBING_IDENTIFIER_BYTES: usize = 4 * 1024;
/// Maximum native tabs inspected or exposed through one retained [`WindowState`].
pub const MAX_SYSTEM_WINDOW_TABS: usize = 256;
/// Maximum logical width or height accepted by a programmatic window-bounds request.
pub const MAX_WINDOW_LOGICAL_DIMENSION: f32 = 32_768.0;
/// Maximum absolute desktop coordinate accepted by a programmatic window-bounds request.
pub const MAX_WINDOW_LOGICAL_COORDINATE: f32 = 16_777_216.0;
/// Maximum ratio between the two components of a window aspect ratio.
pub const MAX_WINDOW_ASPECT_RATIO: f32 = 1_000.0;
/// Maximum window mutations one event callback may queue.
pub const MAX_WINDOW_COMMANDS_PER_EVENT: usize = 256;
/// Maximum deferred native window mutations retained by one application effect cycle.
pub const MAX_PENDING_WINDOW_COMMANDS: usize = 1_024;
/// Maximum declarative child-window close callbacks retained by one parent window.
pub const MAX_CHILD_WINDOW_CLOSE_LISTENERS_PER_WINDOW: usize = 256;
/// Maximum handles returned by one immutable application-window registry snapshot.
pub const MAX_APPLICATION_WINDOWS: usize = 4_096;
/// Maximum selected native popup menus retained until their callback is routed.
pub const MAX_PENDING_NATIVE_POPUP_MENUS: usize = 16;

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum WindowCommandError {
    #[error("this context is not attached to a native window")]
    Unavailable,
    #[error("one event cannot queue more than {MAX_WINDOW_COMMANDS_PER_EVENT} window commands")]
    QueueFull,
    #[error("window bounds must be finite, positive, and within the supported desktop range")]
    InvalidBounds,
    #[error("a window title cannot exceed {MAX_WINDOW_TITLE_BYTES} UTF-8 bytes")]
    TitleTooLong,
    #[error(
        "a represented document path must be non-empty, NUL-free, and at most {MAX_WINDOW_DOCUMENT_PATH_BYTES} encoded bytes"
    )]
    InvalidDocumentPath,
    #[error(
        "a native tabbing identifier must be non-empty, NUL-free, and at most {MAX_WINDOW_TABBING_IDENTIFIER_BYTES} UTF-8 bytes"
    )]
    InvalidTabbingIdentifier,
    #[error("a native tab index must be smaller than {MAX_SYSTEM_WINDOW_TABS}")]
    InvalidTabIndex,
    #[error("a hidden titlebar cannot expose or reposition native traffic-light buttons")]
    HiddenTitleBarTrafficLights,
    #[error("system popovers require one finite parent-relative popover configuration")]
    InvalidPopoverConfiguration,
    #[error("a system popover must be opened from an existing parent window")]
    PopoverParentRequired,
    #[error("minimum window size cannot exceed maximum window size")]
    InvalidSizeConstraints,
    #[error("window opacity must be finite and between 0.0 and 1.0")]
    InvalidOpacity,
    #[error("taskbar progress must be finite and between 0.0 and 1.0")]
    InvalidTaskbarProgress,
    #[error(
        "a taskbar overlay description must be NUL-free and at most {MAX_TASKBAR_OVERLAY_DESCRIPTION_BYTES} UTF-8 bytes"
    )]
    InvalidTaskbarOverlayDescription,
    #[error("the per-window native menu declaration is invalid")]
    InvalidMenus,
    #[error(
        "a window aspect ratio must be finite, positive, and within {MAX_WINDOW_ASPECT_RATIO}:1"
    )]
    InvalidAspectRatio,
    #[error("a window cannot be ordered above itself")]
    InvalidWindowOrder,
}

/// Constant-size snapshot of one native system window-tab group.
///
/// AppKit can technically retain an arbitrary number of windows. QuickGUI inspects at most
/// [`MAX_SYSTEM_WINDOW_TABS`] at an event boundary and reports `truncated` rather than allocating
/// a per-frame list.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct WindowTabState {
    pub count: usize,
    pub selected_index: Option<usize>,
    pub tab_bar_visible: bool,
    pub overview_visible: bool,
    pub truncated: bool,
}

impl Default for WindowTabState {
    fn default() -> Self {
        Self {
            count: 1,
            selected_index: Some(0),
            tab_bar_visible: false,
            overview_visible: false,
            truncated: false,
        }
    }
}

impl WindowTabState {
    pub const fn is_valid(self) -> bool {
        self.count > 0
            && self.count <= MAX_SYSTEM_WINDOW_TABS
            && match self.selected_index {
                Some(index) => index < self.count,
                None => true,
            }
    }
}

static NEXT_WINDOW_HANDLE: AtomicU64 = AtomicU64::new(1);

/// Stable application-level identity for a QuickGUI window.
///
/// Unlike Winit's native identifier, this handle exists before the platform window is created and
/// can therefore be returned immediately from [`EventContext::open_window`](crate::EventContext::open_window).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WindowHandle(u64);

impl WindowHandle {
    pub(crate) fn next() -> Self {
        Self(NEXT_WINDOW_HANDLE.fetch_add(1, Ordering::Relaxed).max(1))
    }

    pub const fn as_u64(self) -> u64 {
        self.0
    }

    pub const fn from_u64(value: u64) -> Option<Self> {
        if value == 0 { None } else { Some(Self(value)) }
    }
}

/// Immutable bounded application-wide window lookup captured at one core event boundary.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WindowRegistry {
    handles: Arc<[WindowHandle]>,
    active_window: Option<WindowHandle>,
    truncated: bool,
}

impl WindowRegistry {
    pub(crate) fn new(
        handles: impl Into<Arc<[WindowHandle]>>,
        active_window: Option<WindowHandle>,
        truncated: bool,
    ) -> Self {
        Self {
            handles: handles.into(),
            active_window,
            truncated,
        }
    }

    pub fn windows(&self) -> &[WindowHandle] {
        &self.handles
    }

    pub const fn active_window(&self) -> Option<WindowHandle> {
        self.active_window
    }

    pub fn contains(&self, handle: WindowHandle) -> bool {
        self.handles.binary_search(&handle).is_ok()
    }

    pub const fn is_truncated(&self) -> bool {
        self.truncated
    }
}

/// Initial native and renderer configuration for a window opened by [`App`].
#[derive(Clone, Debug)]
pub struct WindowOptions {
    pub title: String,
    pub size: Size,
    pub window_bounds: Option<WindowBounds>,
    /// Preferred display for default placement and borderless fullscreen.
    ///
    /// A disconnected or unknown display falls back to the current primary display.
    pub display_id: Option<DisplayId>,
    pub minimum_size: Option<Size>,
    pub maximum_size: Option<Size>,
    /// File represented by native document chrome, if any.
    pub represented_file: Option<PathBuf>,
    /// Initial unsaved-document indication in native chrome.
    pub document_edited: bool,
    /// Non-empty identifier opting this window into native system tabbing.
    pub tabbing_identifier: Option<String>,
    pub background: Color,
    /// Native compositor treatment behind transparent application pixels.
    pub window_background: WindowBackgroundAppearance,
    /// macOS semantic material behind the alpha-capable application surface.
    pub macos_vibrancy: Option<MacOsVibrancy>,
    /// Activity-state policy for [`Self::macos_vibrancy`].
    pub macos_visual_effect_state: MacOsVisualEffectState,
    pub performance_profile: PerformanceProfile,
    /// Explicit native light/dark preference. `None` follows the current system appearance.
    pub preferred_appearance: Option<WindowAppearance>,
    pub title_bar_style: TitleBarStyle,
    pub kind: WindowKind,
    /// Parent-relative native placement when `kind` is [`WindowKind::SystemPopover`].
    pub popover: Option<crate::PopoverOptions>,
    pub focus: bool,
    /// Whether the window may receive native keyboard focus after creation.
    pub focusable: bool,
    pub show: bool,
    pub is_movable: bool,
    pub is_resizable: bool,
    pub is_minimizable: bool,
    pub is_maximizable: bool,
    pub is_closable: bool,
    /// Whether native border and titlebar decorations are present.
    pub decorated: bool,
    /// Requested native shadow. Some platforms always draw decorated-window shadows.
    pub shadow: bool,
    /// Prevent supported desktop capture APIs from reading this window's contents.
    pub content_protected: bool,
    /// Explicit stacking override. `None` derives a role-appropriate level from [`Self::kind`].
    pub window_level: Option<WindowLevel>,
    /// Let native pointer events pass through this window to whatever is behind it.
    pub ignore_mouse_events: bool,
    /// Keep delivering pointer motion while [`Self::ignore_mouse_events`] passes clicks through.
    pub forward_mouse_events: bool,
    /// Whether the native window accepts input at all. A disabled window stays visible.
    pub window_enabled: bool,
    /// Content aspect ratio as `width:height`. `None` leaves resizing unconstrained.
    pub aspect_ratio: Option<Size>,
    /// Whether the macOS close/minimize/zoom buttons are visible.
    pub window_buttons_visible: bool,
    /// Hide the per-window taskbar entry where the platform exposes one.
    pub skip_taskbar: bool,
    /// Keep the window visible on every virtual desktop/space where supported.
    pub visible_on_all_workspaces: bool,
    /// Whole-window native alpha.
    pub opacity: f32,
    /// Native window icon. macOS uses an application icon rather than per-window icons.
    pub icon: Option<Image>,
    /// Initial Windows taskbar progress and overlay state.
    pub taskbar_progress_state: TaskbarProgressState,
    pub taskbar_progress: f32,
    pub taskbar_overlay_icon: Option<Image>,
    pub taskbar_overlay_description: Option<String>,
    /// Initial pointer visibility and confinement policy.
    pub cursor_visible: bool,
    pub cursor_grab: CursorGrabMode,
    /// Whether pointer events hit this native window.
    pub cursor_hit_test: bool,
    /// Optional initial logical pointer position relative to the window.
    pub cursor_position: Option<Point>,
    /// Per-window native menus. `None` inherits the application's current menu declaration.
    pub window_menus: Option<Vec<Menu>>,
    /// Top-left position of the macOS close button, in logical points from the window's top-left.
    pub traffic_light_position: Option<Point>,
    /// Logical pixels represented by one platform line-wheel unit.
    pub line_scroll_pixels: f32,
    /// How long an incomplete multi-stroke key binding waits before its prefix is replayed.
    pub key_sequence_timeout: Duration,
    /// Disable non-essential image animation. macOS Reduce Motion is always respected as well.
    pub reduce_motion: bool,
    /// Open the retained-tree inspector with this window.
    #[cfg(feature = "inspector")]
    pub inspector: bool,
}

impl Default for WindowOptions {
    fn default() -> Self {
        Self {
            title: "QuickGUI".to_owned(),
            size: Size::new(960.0, 640.0),
            window_bounds: None,
            display_id: None,
            minimum_size: Some(Size::new(320.0, 240.0)),
            maximum_size: None,
            represented_file: None,
            document_edited: false,
            tabbing_identifier: None,
            background: Color::rgb8(18, 18, 20),
            window_background: WindowBackgroundAppearance::Opaque,
            macos_vibrancy: None,
            macos_visual_effect_state: MacOsVisualEffectState::FollowWindow,
            performance_profile: PerformanceProfile::Balanced,
            preferred_appearance: None,
            title_bar_style: TitleBarStyle::Default,
            kind: WindowKind::Normal,
            popover: None,
            focus: true,
            focusable: true,
            show: true,
            is_movable: true,
            is_resizable: true,
            is_minimizable: true,
            is_maximizable: true,
            is_closable: true,
            decorated: true,
            shadow: true,
            content_protected: false,
            window_level: None,
            ignore_mouse_events: false,
            forward_mouse_events: false,
            window_enabled: true,
            aspect_ratio: None,
            window_buttons_visible: true,
            skip_taskbar: false,
            visible_on_all_workspaces: false,
            opacity: 1.0,
            icon: None,
            taskbar_progress_state: TaskbarProgressState::None,
            taskbar_progress: 0.0,
            taskbar_overlay_icon: None,
            taskbar_overlay_description: None,
            cursor_visible: true,
            cursor_grab: CursorGrabMode::None,
            cursor_hit_test: true,
            cursor_position: None,
            window_menus: None,
            traffic_light_position: None,
            line_scroll_pixels: 40.0,
            key_sequence_timeout: Duration::from_secs(1),
            reduce_motion: false,
            #[cfg(feature = "inspector")]
            inspector: false,
        }
    }
}

impl WindowOptions {
    /// Create window options with a title and otherwise production-safe defaults.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Self::default()
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.size = Size::new(width, height);
        if let Some(bounds) = &mut self.window_bounds {
            let state = *bounds;
            let rect = state.bounds();
            let rect = Rect::new(rect.x, rect.y, width, height);
            *bounds = match state {
                WindowBounds::Windowed(_) => WindowBounds::Windowed(rect),
                WindowBounds::Maximized(_) => WindowBounds::Maximized(rect),
                WindowBounds::Fullscreen(_) => WindowBounds::Fullscreen(rect),
            };
        }
        self
    }

    pub fn window_bounds(mut self, bounds: WindowBounds) -> Self {
        let rect = bounds.bounds();
        self.size = Size::new(rect.width, rect.height);
        self.window_bounds = Some(bounds);
        self
    }

    pub fn position(mut self, x: f32, y: f32) -> Self {
        let bounds = self
            .window_bounds
            .unwrap_or_else(|| WindowBounds::Windowed(Rect::from_size(self.size)));
        let rect = bounds.bounds();
        let rect = Rect::new(x, y, rect.width, rect.height);
        self.window_bounds = Some(match bounds {
            WindowBounds::Windowed(_) => WindowBounds::Windowed(rect),
            WindowBounds::Maximized(_) => WindowBounds::Maximized(rect),
            WindowBounds::Fullscreen(_) => WindowBounds::Fullscreen(rect),
        });
        self
    }

    /// Select the display used for automatic placement and fullscreen creation.
    pub fn display(mut self, display: DisplayId) -> Self {
        self.display_id = Some(display);
        self
    }

    pub fn without_display(mut self) -> Self {
        self.display_id = None;
        self
    }

    pub fn maximized(mut self, maximized: bool) -> Self {
        let rect = self
            .window_bounds
            .map(WindowBounds::bounds)
            .unwrap_or_else(|| Rect::from_size(self.size));
        self.window_bounds = Some(if maximized {
            WindowBounds::Maximized(rect)
        } else {
            WindowBounds::Windowed(rect)
        });
        self
    }

    pub fn fullscreen(mut self, fullscreen: bool) -> Self {
        let rect = self
            .window_bounds
            .map(WindowBounds::bounds)
            .unwrap_or_else(|| Rect::from_size(self.size));
        self.window_bounds = Some(if fullscreen {
            WindowBounds::Fullscreen(rect)
        } else {
            WindowBounds::Windowed(rect)
        });
        self
    }

    pub fn minimum_size(mut self, width: f32, height: f32) -> Self {
        self.minimum_size = Some(Size::new(width, height));
        self
    }

    pub fn without_minimum_size(mut self) -> Self {
        self.minimum_size = None;
        self
    }

    pub fn maximum_size(mut self, width: f32, height: f32) -> Self {
        self.maximum_size = Some(Size::new(width, height));
        self
    }

    pub fn without_maximum_size(mut self) -> Self {
        self.maximum_size = None;
        self
    }

    /// Represent a file in native document chrome.
    pub fn represented_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.represented_file = Some(path.into());
        self
    }

    /// GPUI-compatible alias for [`Self::represented_file`].
    pub fn document_path(self, path: impl Into<PathBuf>) -> Self {
        self.represented_file(path)
    }

    pub fn without_represented_file(mut self) -> Self {
        self.represented_file = None;
        self
    }

    /// Override the application's native menus for this window.
    pub fn window_menus(mut self, menus: impl IntoIterator<Item = Menu>) -> Self {
        self.window_menus = Some(menus.into_iter().collect());
        self
    }

    /// Inherit the application's native menus, including later app-wide replacements.
    pub fn use_application_menus(mut self) -> Self {
        self.window_menus = None;
        self
    }

    pub fn document_edited(mut self, edited: bool) -> Self {
        self.document_edited = edited;
        self
    }

    /// Opt this window into AppKit system tabbing with an application-defined group identifier.
    pub fn tabbing_identifier(mut self, identifier: impl Into<String>) -> Self {
        self.tabbing_identifier = Some(identifier.into());
        self
    }

    pub fn without_tabbing_identifier(mut self) -> Self {
        self.tabbing_identifier = None;
        self
    }

    pub fn background(mut self, background: Color) -> Self {
        self.background = background;
        self
    }

    pub fn window_background(mut self, appearance: WindowBackgroundAppearance) -> Self {
        self.window_background = appearance;
        self
    }

    pub(super) const fn uses_transparent_surface(&self) -> bool {
        self.window_background.is_transparent()
            || (cfg!(target_os = "macos") && self.macos_vibrancy.is_some())
    }

    pub(super) const fn uses_legacy_background_blur(&self) -> bool {
        self.window_background.is_blurred()
            && !(cfg!(target_os = "macos") && self.macos_vibrancy.is_some())
    }

    /// Apply one Electron-compatible semantic vibrancy material on macOS.
    pub fn macos_vibrancy(mut self, vibrancy: MacOsVibrancy) -> Self {
        self.macos_vibrancy = Some(vibrancy);
        self
    }

    pub fn without_macos_vibrancy(mut self) -> Self {
        self.macos_vibrancy = None;
        self
    }

    pub fn macos_visual_effect_state(mut self, state: MacOsVisualEffectState) -> Self {
        self.macos_visual_effect_state = state;
        self
    }

    pub fn performance_profile(mut self, profile: PerformanceProfile) -> Self {
        self.performance_profile = profile;
        self
    }

    /// Force this window's native chrome to use one appearance.
    pub fn window_appearance(mut self, appearance: WindowAppearance) -> Self {
        self.preferred_appearance = Some(appearance);
        self
    }

    /// Follow the operating system's effective light/dark appearance.
    pub fn follow_system_appearance(mut self) -> Self {
        self.preferred_appearance = None;
        self
    }

    pub fn title_bar_style(mut self, style: TitleBarStyle) -> Self {
        self.title_bar_style = style;
        self.decorated = style != TitleBarStyle::Hidden;
        self
    }

    pub fn window_kind(mut self, kind: WindowKind) -> Self {
        self.kind = kind;
        if kind != WindowKind::SystemPopover {
            self.popover = None;
        }
        self
    }

    /// Configure a borderless native `SystemPopover`.
    ///
    /// Menu-style grabs focus the panel and dismiss on Escape or an outside mouse press.
    /// Non-grabbing popovers install no event monitor; `PopoverOptions::accepts_key_focus` independently
    /// decides whether pointer interaction may make the panel key.
    pub fn system_popover(mut self, popover: crate::PopoverOptions) -> Self {
        self.kind = WindowKind::SystemPopover;
        self.focus = popover.grab;
        self.focusable = popover.accepts_key_focus;
        self.popover = Some(popover);
        self.minimum_size = None;
        self.maximum_size = None;
        self.title_bar_style = TitleBarStyle::Hidden;
        self.decorated = false;
        self.traffic_light_position = None;
        self.is_movable = false;
        self.is_resizable = false;
        self.is_minimizable = false;
        self.is_maximizable = false;
        self.is_closable = false;
        self
    }

    pub fn focus(mut self, focus: bool) -> Self {
        self.focus = focus;
        self
    }

    /// Allow or prevent this window from receiving native keyboard focus.
    pub fn focusable(mut self, focusable: bool) -> Self {
        self.focusable = focusable;
        if !focusable {
            self.focus = false;
        }
        self
    }

    pub fn show(mut self, show: bool) -> Self {
        self.show = show;
        self
    }

    pub fn movable(mut self, movable: bool) -> Self {
        self.is_movable = movable;
        self
    }

    pub fn resizable(mut self, resizable: bool) -> Self {
        self.is_resizable = resizable;
        self
    }

    pub fn minimizable(mut self, minimizable: bool) -> Self {
        self.is_minimizable = minimizable;
        self
    }

    pub fn maximizable(mut self, maximizable: bool) -> Self {
        self.is_maximizable = maximizable;
        self
    }

    pub fn closable(mut self, closable: bool) -> Self {
        self.is_closable = closable;
        self
    }

    pub fn decorations(mut self, decorated: bool) -> Self {
        self.decorated = decorated;
        if !decorated {
            self.title_bar_style = TitleBarStyle::Hidden;
            self.traffic_light_position = None;
        } else if self.title_bar_style == TitleBarStyle::Hidden {
            self.title_bar_style = TitleBarStyle::Default;
        }
        self
    }

    pub fn shadow(mut self, shadow: bool) -> Self {
        self.shadow = shadow;
        self
    }

    pub fn content_protected(mut self, protected: bool) -> Self {
        self.content_protected = protected;
        self
    }

    pub fn window_level(mut self, level: WindowLevel) -> Self {
        self.window_level = Some(level);
        self
    }

    pub fn automatic_window_level(mut self) -> Self {
        self.window_level = None;
        self
    }

    pub fn skip_taskbar(mut self, skip: bool) -> Self {
        self.skip_taskbar = skip;
        self
    }

    pub fn visible_on_all_workspaces(mut self, visible: bool) -> Self {
        self.visible_on_all_workspaces = visible;
        self
    }

    /// Set whole-window native opacity. Invalid values are rejected when the window is opened.
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }

    pub fn icon(mut self, icon: Image) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn without_icon(mut self) -> Self {
        self.icon = None;
        self
    }

    pub fn taskbar_progress(mut self, state: TaskbarProgressState, progress: f32) -> Self {
        self.taskbar_progress_state = state;
        self.taskbar_progress = progress;
        self
    }

    pub fn taskbar_overlay_icon(mut self, icon: Image, description: impl Into<String>) -> Self {
        self.taskbar_overlay_icon = Some(icon);
        self.taskbar_overlay_description = Some(description.into());
        self
    }

    pub fn without_taskbar_overlay_icon(mut self) -> Self {
        self.taskbar_overlay_icon = None;
        self.taskbar_overlay_description = None;
        self
    }

    pub fn cursor_visible(mut self, visible: bool) -> Self {
        self.cursor_visible = visible;
        self
    }

    pub fn cursor_grab(mut self, mode: CursorGrabMode) -> Self {
        self.cursor_grab = mode;
        self
    }

    pub fn cursor_hit_test(mut self, hit_test: bool) -> Self {
        self.cursor_hit_test = hit_test;
        self
    }

    pub fn cursor_position(mut self, position: Point) -> Self {
        self.cursor_position = Some(position);
        self
    }

    pub fn without_cursor_position(mut self) -> Self {
        self.cursor_position = None;
        self
    }

    pub fn always_on_top(self, always_on_top: bool) -> Self {
        self.window_level(if always_on_top {
            WindowLevel::AlwaysOnTop
        } else {
            WindowLevel::Normal
        })
    }

    pub fn always_on_bottom(self, always_on_bottom: bool) -> Self {
        self.window_level(if always_on_bottom {
            WindowLevel::AlwaysOnBottom
        } else {
            WindowLevel::Normal
        })
    }

    /// Position the macOS traffic lights in logical points from the window's top-left.
    ///
    /// The close button uses this exact position; minimize and zoom retain native spacing.
    pub fn traffic_light_position(mut self, x: f32, y: f32) -> Self {
        self.traffic_light_position = Some(Point::new(x, y));
        self
    }

    pub fn without_traffic_light_position(mut self) -> Self {
        self.traffic_light_position = None;
        self
    }

    pub fn reduce_motion(mut self, reduce_motion: bool) -> Self {
        self.reduce_motion = reduce_motion;
        self
    }

    /// Open or suppress the retained-tree inspector when this window is created.
    #[cfg(feature = "inspector")]
    pub fn inspector(mut self, inspector: bool) -> Self {
        self.inspector = inspector;
        self
    }

    /// Let clicks fall through this window to whatever is behind it.
    ///
    /// `forward` keeps pointer motion and hover events flowing to this window while every button
    /// press reaches the window below. `false` makes the window completely inert to the pointer.
    pub fn ignore_mouse_events(mut self, ignore: bool, forward: bool) -> Self {
        self.ignore_mouse_events = ignore;
        self.forward_mouse_events = ignore && forward;
        self
    }

    /// Block every native input event while keeping the window visible.
    pub fn window_enabled(mut self, enabled: bool) -> Self {
        self.window_enabled = enabled;
        self
    }

    /// Constrain live resizing to one `width:height` content aspect ratio.
    pub fn aspect_ratio(mut self, width: f32, height: f32) -> Self {
        self.aspect_ratio = Some(Size::new(width, height));
        self
    }

    pub fn without_aspect_ratio(mut self) -> Self {
        self.aspect_ratio = None;
        self
    }

    /// Show or hide the macOS close/minimize/zoom buttons without removing the titlebar.
    pub fn window_button_visibility(mut self, visible: bool) -> Self {
        self.window_buttons_visible = visible;
        self
    }

    /// Restore persisted geometry, validating it against the currently connected displays.
    ///
    /// Bounds that no longer intersect a connected display work area are clamped into the
    /// remembered display when it is still present, and otherwise centered on the primary
    /// display. See [`WindowRestoreState`].
    pub fn restore(mut self, state: &WindowRestoreState, displays: &Displays) -> Self {
        let resolved = state.resolve(displays);
        self.window_bounds = Some(resolved.bounds);
        self.size = Size::new(
            resolved.bounds.bounds().width,
            resolved.bounds.bounds().height,
        );
        self.display_id = resolved.display_id;
        self
    }
}

/// Persistable window geometry and display identity.
///
/// Capture it with [`WindowState::restore_state`] and apply it with [`WindowOptions::restore`].
/// The struct is `serde`-serializable so an application can store it next to its own settings.
/// Every field is validated on the way back in; a stale or hostile value can never place a
/// window off every connected display.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct WindowRestoreState {
    /// Windowed restore rectangle in global logical desktop coordinates.
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub maximized: bool,
    pub fullscreen: bool,
    /// Process-level display identifier captured with the bounds, when one was known.
    pub display_id: Option<u64>,
    /// Stable physical display identity captured with the bounds, when the platform exposes one.
    pub display_uuid: Option<[u8; 16]>,
    /// Scale factor of the capturing display, used only as a sanity signal.
    pub scale_factor: f32,
}

/// Outcome of validating a [`WindowRestoreState`] against the current display snapshot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResolvedWindowRestoreState {
    pub bounds: WindowBounds,
    pub display_id: Option<DisplayId>,
    /// Whether the persisted rectangle had to be clamped or re-centered.
    pub adjusted: bool,
}

impl WindowRestoreState {
    /// Build a restore state from an explicit windowed rectangle.
    pub const fn new(bounds: Rect) -> Self {
        Self {
            x: bounds.x,
            y: bounds.y,
            width: bounds.width,
            height: bounds.height,
            maximized: false,
            fullscreen: false,
            display_id: None,
            display_uuid: None,
            scale_factor: 1.0,
        }
    }

    /// Persisted windowed rectangle in global logical desktop coordinates.
    pub const fn bounds(&self) -> Rect {
        Rect::new(self.x, self.y, self.width, self.height)
    }

    /// Whether the persisted values are finite and inside the supported desktop range.
    pub fn is_valid(&self) -> bool {
        validate_window_bounds(WindowBounds::Windowed(self.bounds())).is_ok()
            && self.scale_factor.is_finite()
            && self.scale_factor > 0.0
    }

    /// Resolve this state against a live display snapshot.
    ///
    /// The remembered display is matched by stable UUID first and by process identifier second.
    /// A rectangle that still intersects that display's work area is kept exactly; otherwise it
    /// is clamped into the work area. When no remembered display is connected and the rectangle
    /// intersects no work area at all, the window is centered on the primary display.
    pub fn resolve(&self, displays: &Displays) -> ResolvedWindowRestoreState {
        let requested = self.bounds();
        let valid = self.is_valid();
        let remembered = if valid {
            self.display_uuid
                .and_then(|uuid| {
                    let uuid = crate::DisplayUuid::from_bytes(uuid);
                    displays
                        .all()
                        .iter()
                        .find(|display| display.uuid() == Some(uuid))
                })
                .or_else(|| {
                    self.display_id
                        .and_then(|id| displays.find(DisplayId::new(id)))
                })
        } else {
            None
        };
        let intersecting = valid
            .then(|| {
                displays
                    .all()
                    .iter()
                    .find(|display| display.visible_bounds().intersection(requested).is_some())
            })
            .flatten();
        let (rect, display, adjusted) = match (valid, remembered, intersecting) {
            (false, _, _) => match displays.primary() {
                Some(primary) => (
                    primary.centered_bounds(Size::new(960.0, 640.0)),
                    Some(primary.id()),
                    true,
                ),
                None => (Rect::new(0.0, 0.0, 960.0, 640.0), None, true),
            },
            (true, Some(display), _)
                if display.visible_bounds().intersection(requested).is_some() =>
            {
                (requested, Some(display.id()), false)
            }
            (true, Some(display), _) => (
                display.constrain_bounds(requested),
                Some(display.id()),
                true,
            ),
            (true, None, Some(display)) => (requested, Some(display.id()), false),
            (true, None, None) => match displays.primary() {
                Some(primary) => (
                    primary.centered_bounds(Size::new(requested.width, requested.height)),
                    Some(primary.id()),
                    true,
                ),
                None => (requested, None, false),
            },
        };
        let bounds = if self.fullscreen {
            WindowBounds::Fullscreen(rect)
        } else if self.maximized {
            WindowBounds::Maximized(rect)
        } else {
            WindowBounds::Windowed(rect)
        };
        ResolvedWindowRestoreState {
            bounds,
            display_id: display,
            adjusted,
        }
    }
}

pub(crate) fn validate_window_bounds(bounds: WindowBounds) -> Result<(), WindowCommandError> {
    let bounds = bounds.bounds();
    let valid = bounds.x.is_finite()
        && bounds.y.is_finite()
        && bounds.width.is_finite()
        && bounds.height.is_finite()
        && bounds.x.abs() <= MAX_WINDOW_LOGICAL_COORDINATE
        && bounds.y.abs() <= MAX_WINDOW_LOGICAL_COORDINATE
        && bounds.width > 0.0
        && bounds.height > 0.0
        && bounds.width <= MAX_WINDOW_LOGICAL_DIMENSION
        && bounds.height <= MAX_WINDOW_LOGICAL_DIMENSION;
    if valid {
        Ok(())
    } else {
        Err(WindowCommandError::InvalidBounds)
    }
}

pub(crate) fn validate_window_aspect_ratio(ratio: Option<Size>) -> Result<(), WindowCommandError> {
    let Some(ratio) = ratio else {
        return Ok(());
    };
    let valid = ratio.width.is_finite()
        && ratio.height.is_finite()
        && ratio.width > 0.0
        && ratio.height > 0.0
        && ratio.width / ratio.height <= MAX_WINDOW_ASPECT_RATIO
        && ratio.height / ratio.width <= MAX_WINDOW_ASPECT_RATIO;
    if valid {
        Ok(())
    } else {
        Err(WindowCommandError::InvalidAspectRatio)
    }
}

/// Clamp one inner size to a `width:height` aspect ratio, preserving its area as closely as the
/// portable fallback allows. The width is authoritative because horizontal edge drags are the
/// common interactive resize.
pub(crate) fn clamp_size_to_aspect_ratio(size: Size, ratio: Size) -> Size {
    if !(ratio.width > 0.0
        && ratio.height > 0.0
        && ratio.width.is_finite()
        && ratio.height.is_finite())
    {
        return size;
    }
    let width = size.width.max(1.0);
    Size::new(width, (width * ratio.height / ratio.width).max(1.0))
}

pub(crate) fn validate_window_size(size: Size) -> Result<(), WindowCommandError> {
    validate_window_bounds(WindowBounds::Windowed(Rect::from_size(size)))
}

pub(crate) fn validate_window_position(position: Point) -> Result<(), WindowCommandError> {
    validate_window_bounds(WindowBounds::Windowed(Rect::new(
        position.x, position.y, 1.0, 1.0,
    )))
}

pub(crate) fn validate_window_title(title: &str) -> Result<(), WindowCommandError> {
    if title.len() <= MAX_WINDOW_TITLE_BYTES {
        Ok(())
    } else {
        Err(WindowCommandError::TitleTooLong)
    }
}

pub(crate) fn validate_window_document_path(path: &Path) -> Result<(), WindowCommandError> {
    let bytes = path.as_os_str().as_encoded_bytes();
    if !bytes.is_empty() && bytes.len() <= MAX_WINDOW_DOCUMENT_PATH_BYTES && !bytes.contains(&0) {
        Ok(())
    } else {
        Err(WindowCommandError::InvalidDocumentPath)
    }
}

pub(crate) fn validate_window_tabbing_identifier(
    identifier: &str,
) -> Result<(), WindowCommandError> {
    if !identifier.is_empty()
        && identifier.len() <= MAX_WINDOW_TABBING_IDENTIFIER_BYTES
        && !identifier.as_bytes().contains(&0)
    {
        Ok(())
    } else {
        Err(WindowCommandError::InvalidTabbingIdentifier)
    }
}

pub(super) fn validate_window_options(options: &WindowOptions) -> Result<(), WindowCommandError> {
    validate_window_title(&options.title)?;
    validate_window_size(options.size)?;
    if let Some(bounds) = options.window_bounds {
        validate_window_bounds(bounds)?;
    }
    if let Some(minimum) = options.minimum_size {
        validate_window_size(minimum)?;
    }
    if let Some(maximum) = options.maximum_size {
        validate_window_size(maximum)?;
    }
    validate_window_opacity(options.opacity)?;
    validate_window_aspect_ratio(options.aspect_ratio)?;
    validate_taskbar_progress(options.taskbar_progress)?;
    validate_taskbar_overlay_description(options.taskbar_overlay_description.as_deref())?;
    if let Some(position) = options.cursor_position {
        validate_window_position(position)?;
    }
    if let (Some(minimum), Some(maximum)) = (options.minimum_size, options.maximum_size)
        && (minimum.width > maximum.width || minimum.height > maximum.height)
    {
        return Err(WindowCommandError::InvalidSizeConstraints);
    }
    if let Some(path) = options.represented_file.as_deref() {
        validate_window_document_path(path)?;
    }
    if let Some(identifier) = options.tabbing_identifier.as_deref() {
        validate_window_tabbing_identifier(identifier)?;
    }
    if let Some(menus) = options.window_menus.as_deref() {
        validate_menus(menus).map_err(|_| WindowCommandError::InvalidMenus)?;
    }
    if options.title_bar_style == TitleBarStyle::Hidden && options.traffic_light_position.is_some()
    {
        return Err(WindowCommandError::HiddenTitleBarTrafficLights);
    }
    match (options.kind, options.popover.as_ref()) {
        (WindowKind::SystemPopover, Some(popover))
            if popover.is_valid(MAX_WINDOW_LOGICAL_COORDINATE, MAX_WINDOW_LOGICAL_DIMENSION)
                && !options.decorated
                && options.title_bar_style == TitleBarStyle::Hidden
                && options.minimum_size.is_none()
                && options.maximum_size.is_none()
                && !options.is_movable
                && !options.is_resizable
                && !options.is_minimizable
                && !options.is_maximizable
                && !options.is_closable
                && !matches!(
                    options.window_bounds,
                    Some(WindowBounds::Maximized(_) | WindowBounds::Fullscreen(_))
                ) => {}
        (WindowKind::SystemPopover, _) | (_, Some(_)) => {
            return Err(WindowCommandError::InvalidPopoverConfiguration);
        }
        (_, None) => {}
    }
    Ok(())
}

pub(crate) fn validate_window_opacity(opacity: f32) -> Result<(), WindowCommandError> {
    if opacity.is_finite() && (0.0..=1.0).contains(&opacity) {
        Ok(())
    } else {
        Err(WindowCommandError::InvalidOpacity)
    }
}

pub(crate) fn validate_taskbar_progress(progress: f32) -> Result<(), WindowCommandError> {
    if progress.is_finite() && (0.0..=1.0).contains(&progress) {
        Ok(())
    } else {
        Err(WindowCommandError::InvalidTaskbarProgress)
    }
}

pub(crate) fn validate_taskbar_overlay_description(
    description: Option<&str>,
) -> Result<(), WindowCommandError> {
    if description.is_some_and(|description| {
        description.len() > crate::MAX_TASKBAR_OVERLAY_DESCRIPTION_BYTES
            || description.as_bytes().contains(&0)
    }) {
        Err(WindowCommandError::InvalidTaskbarOverlayDescription)
    } else {
        Ok(())
    }
}

/// A retained application view. It is only rendered after explicit invalidation or OS damage.
pub trait View: Sized + 'static {
    fn event(&mut self, _event: &Event, _cx: &mut EventContext) {}
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement;
    /// Embedding renderers can rebuild a subtree declared with `ViewContext::with_scope`.
    /// Return `None` when a mutation needs an ordinary full declaration instead.
    fn render_scope(&mut self, _id: ElementId, _cx: &mut ViewContext<'_, Self>) -> Option<Element> {
        None
    }
}

pub(super) trait AnyView {
    fn event(&mut self, event: &Event, cx: &mut EventContext);
    fn render_scopes(&mut self, cx: &mut ViewContext<'_, ()>) -> Option<Vec<crate::ElementUpdate>>;

    #[allow(clippy::too_many_arguments)]
    fn render(
        &mut self,
        size: Size,
        scale_factor: f32,
        metrics: FrameMetrics,
        focused: Option<ElementId>,
        focused_path: Vec<ElementId>,
        listeners: &mut ListenerRegistry,
        window: WindowHandle,
        window_state: WindowState,
        displays: &Displays,
        keyboard_layout: &KeyboardLayout,
        font_system: &SharedFontSystem,
        assets: &Assets,
        app_info: Option<&AppInfo>,
        app_paths: Option<&AppPaths>,
        system_info: &SystemInfo,
        system_preferences: &SystemPreferences,
        background_tasks: Option<&BackgroundTaskPoolHandle>,
        foreground_tasks: &ForegroundTaskSpawner,
        globals: &GlobalStore,
        event_proxy: Option<&EventLoopProxy<RuntimeEvent>>,
    ) -> (Element, bool, Option<Instant>);

    fn as_any_mut(&mut self) -> &mut dyn Any;

    #[cfg(any(test, feature = "test-support"))]
    fn as_any(&self) -> &dyn Any;
}

pub(super) struct ViewAdapter<V>(V);

impl<V: View> AnyView for ViewAdapter<V> {
    fn event(&mut self, event: &Event, cx: &mut EventContext) {
        self.0.event(event, cx);
    }

    fn render_scopes(&mut self, cx: &mut ViewContext<'_, ()>) -> Option<Vec<crate::ElementUpdate>> {
        cx.with_type::<V, _>(|cx| {
            let roots = cx.listeners.prepare_component_updates();
            let mut updates = Vec::with_capacity(roots.len());
            for scope in roots {
                let id = scope.id;
                updates.push(cx.refresh_component(scope, |cx| self.0.render_scope(id, cx))?);
            }
            Some(updates)
        })
    }

    fn render(
        &mut self,
        size: Size,
        scale_factor: f32,
        metrics: FrameMetrics,
        focused: Option<ElementId>,
        focused_path: Vec<ElementId>,
        listeners: &mut ListenerRegistry,
        window: WindowHandle,
        window_state: WindowState,
        displays: &Displays,
        keyboard_layout: &KeyboardLayout,
        font_system: &SharedFontSystem,
        assets: &Assets,
        app_info: Option<&AppInfo>,
        app_paths: Option<&AppPaths>,
        system_info: &SystemInfo,
        system_preferences: &SystemPreferences,
        background_tasks: Option<&BackgroundTaskPoolHandle>,
        foreground_tasks: &ForegroundTaskSpawner,
        globals: &GlobalStore,
        event_proxy: Option<&EventLoopProxy<RuntimeEvent>>,
    ) -> (Element, bool, Option<Instant>) {
        let mut cx = ViewContext::<V> {
            size,
            scale_factor,
            metrics,
            focused,
            focused_path,
            request_animation_frame: false,
            repaint_deadline: None,
            listeners,
            window,
            window_state,
            displays,
            keyboard_layout,
            font_system,
            assets,
            app_info,
            app_paths,
            system_info,
            system_preferences,
            background_tasks,
            foreground_tasks,
            globals,
            event_proxy,
            marker: PhantomData,
        };
        cx.listeners.clear();
        let root = self.0.render(&mut cx).into_element();
        cx.listeners.retain_components(&root, None);
        (root, cx.request_animation_frame, cx.repaint_deadline)
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        &mut self.0
    }

    #[cfg(any(test, feature = "test-support"))]
    fn as_any(&self) -> &dyn Any {
        &self.0
    }
}

pub(crate) struct WindowRequest {
    pub(crate) handle: WindowHandle,
    pub(super) view: Box<dyn AnyView>,
    pub(crate) options: WindowOptions,
    pub(crate) parent: Option<WindowHandle>,
    /// Resolve this anchor from the parent window's retained layout at the event boundary and
    /// retain it as the focus-restoration target for the child lifetime.
    pub(crate) popover_anchor_element: Option<ElementId>,
    #[cfg(all(target_os = "macos", feature = "swift-ui"))]
    pub(crate) embedded: Option<crate::MacEmbeddedView>,
}

impl WindowRequest {
    pub(crate) fn new<V: View>(view: V, options: WindowOptions) -> Self {
        Self::with_parent(view, options, None)
    }

    pub(crate) fn with_parent<V: View>(
        view: V,
        options: WindowOptions,
        parent: Option<WindowHandle>,
    ) -> Self {
        Self::with_handle(view, options, parent, WindowHandle::next())
    }

    pub(crate) fn with_handle<V: View>(
        view: V,
        options: WindowOptions,
        parent: Option<WindowHandle>,
        handle: WindowHandle,
    ) -> Self {
        Self {
            handle,
            view: Box::new(ViewAdapter(view)),
            options,
            parent,
            popover_anchor_element: None,
            #[cfg(all(target_os = "macos", feature = "swift-ui"))]
            embedded: None,
        }
    }
}

impl fmt::Debug for WindowRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = formatter.debug_struct("WindowRequest");
        debug
            .field("handle", &self.handle)
            .field("parent", &self.parent)
            .field("popover_anchor_element", &self.popover_anchor_element);
        #[cfg(all(target_os = "macos", feature = "swift-ui"))]
        debug.field("embedded", &self.embedded);
        debug
            .field("options", &self.options)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub(crate) enum WindowCommand {
    SetTitle(WindowHandle, String),
    SetRepresentedFile(WindowHandle, Option<PathBuf>),
    SetDocumentEdited(WindowHandle, bool),
    ShowCharacterPalette(WindowHandle),
    /// Present the platform dictionary definition for the focused text input's selection.
    LookUpSelection(WindowHandle),
    SetTabbingIdentifier(WindowHandle, Option<String>),
    SelectNextTab(WindowHandle),
    SelectPreviousTab(WindowHandle),
    SelectTab(WindowHandle, usize),
    MergeAllWindows(WindowHandle),
    MoveTabToNewWindow(WindowHandle),
    ToggleTabBar(WindowHandle),
    ToggleTabOverview(WindowHandle),
    SetBounds(WindowHandle, WindowBounds),
    Move(WindowHandle, Point),
    Resize(WindowHandle, Size),
    Minimize(WindowHandle),
    Restore(WindowHandle),
    Zoom(WindowHandle),
    ToggleFullscreen(WindowHandle),
    SetFullscreen(WindowHandle, bool),
    SetVisible(WindowHandle, bool),
    SetMovable(WindowHandle, bool),
    SetResizable(WindowHandle, bool),
    SetMinimumSize(WindowHandle, Option<Size>),
    SetMaximumSize(WindowHandle, Option<Size>),
    SetMinimizable(WindowHandle, bool),
    SetMaximizable(WindowHandle, bool),
    SetClosable(WindowHandle, bool),
    SetDecorated(WindowHandle, bool),
    SetShadow(WindowHandle, bool),
    SetContentProtected(WindowHandle, bool),
    SetWindowLevel(WindowHandle, Option<WindowLevel>),
    MoveToTop(WindowHandle),
    MoveAbove(WindowHandle, WindowHandle),
    SetIgnoreMouseEvents(WindowHandle, bool, bool),
    SetWindowEnabled(WindowHandle, bool),
    SetAspectRatio(WindowHandle, Option<Size>),
    SetWindowButtonVisibility(WindowHandle, bool),
    SetFocusable(WindowHandle, bool),
    SetSkipTaskbar(WindowHandle, bool),
    SetVisibleOnAllWorkspaces(WindowHandle, bool),
    SetOpacity(WindowHandle, f32),
    SetIcon(WindowHandle, Option<Image>),
    SetTaskbarProgress(WindowHandle, TaskbarProgressState, f32),
    SetTaskbarOverlayIcon(WindowHandle, Option<Image>, Option<String>),
    SetCursorVisible(WindowHandle, bool),
    SetCursorGrab(WindowHandle, CursorGrabMode),
    SetCursorHitTest(WindowHandle, bool),
    SetCursorPosition(WindowHandle, Point),
    SetAppearance(WindowHandle, Option<WindowAppearance>),
    SetBackgroundAppearance(WindowHandle, WindowBackgroundAppearance),
    SetMacOsVibrancy(WindowHandle, Option<MacOsVibrancy>),
    SetMacOsVisualEffectState(WindowHandle, MacOsVisualEffectState),
    #[cfg(feature = "inspector")]
    SetInspector(WindowHandle, bool),
    #[cfg(feature = "inspector")]
    ToggleInspector(WindowHandle),
    RequestAttention(WindowHandle),
}

impl WindowCommand {
    pub(crate) fn handle(&self) -> WindowHandle {
        match self {
            Self::SetTitle(handle, _)
            | Self::SetRepresentedFile(handle, _)
            | Self::SetDocumentEdited(handle, _)
            | Self::ShowCharacterPalette(handle)
            | Self::LookUpSelection(handle)
            | Self::SetTabbingIdentifier(handle, _)
            | Self::SelectNextTab(handle)
            | Self::SelectPreviousTab(handle)
            | Self::SelectTab(handle, _)
            | Self::MergeAllWindows(handle)
            | Self::MoveTabToNewWindow(handle)
            | Self::ToggleTabBar(handle)
            | Self::ToggleTabOverview(handle)
            | Self::SetBounds(handle, _)
            | Self::Move(handle, _)
            | Self::Resize(handle, _)
            | Self::Minimize(handle)
            | Self::Restore(handle)
            | Self::Zoom(handle)
            | Self::ToggleFullscreen(handle)
            | Self::SetFullscreen(handle, _)
            | Self::SetVisible(handle, _)
            | Self::SetMovable(handle, _)
            | Self::SetResizable(handle, _)
            | Self::SetMinimumSize(handle, _)
            | Self::SetMaximumSize(handle, _)
            | Self::SetMinimizable(handle, _)
            | Self::SetMaximizable(handle, _)
            | Self::SetClosable(handle, _)
            | Self::SetDecorated(handle, _)
            | Self::SetShadow(handle, _)
            | Self::SetContentProtected(handle, _)
            | Self::SetWindowLevel(handle, _)
            | Self::MoveToTop(handle)
            | Self::MoveAbove(handle, _)
            | Self::SetIgnoreMouseEvents(handle, _, _)
            | Self::SetWindowEnabled(handle, _)
            | Self::SetAspectRatio(handle, _)
            | Self::SetWindowButtonVisibility(handle, _)
            | Self::SetFocusable(handle, _)
            | Self::SetSkipTaskbar(handle, _)
            | Self::SetVisibleOnAllWorkspaces(handle, _)
            | Self::SetOpacity(handle, _)
            | Self::SetIcon(handle, _)
            | Self::SetTaskbarProgress(handle, _, _)
            | Self::SetTaskbarOverlayIcon(handle, _, _)
            | Self::SetCursorVisible(handle, _)
            | Self::SetCursorGrab(handle, _)
            | Self::SetCursorHitTest(handle, _)
            | Self::SetCursorPosition(handle, _)
            | Self::SetAppearance(handle, _)
            | Self::SetBackgroundAppearance(handle, _)
            | Self::SetMacOsVibrancy(handle, _)
            | Self::SetMacOsVisualEffectState(handle, _)
            | Self::RequestAttention(handle) => *handle,
            #[cfg(feature = "inspector")]
            Self::SetInspector(handle, _) | Self::ToggleInspector(handle) => *handle,
        }
    }
}

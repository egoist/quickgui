use std::{
    any::type_name,
    cell::Cell,
    collections::{HashMap, HashSet, VecDeque},
    marker::PhantomData,
    panic::{AssertUnwindSafe, catch_unwind},
    rc::Rc,
};
use web_time::{Duration, Instant};

use thiserror::Error;

use super::*;
use crate::{
    ActivationPolicy, ClipboardError, DockAttention, DockAttentionRequest, MAX_MOUSE_EVENT_PATH,
    TextHighlight, TextId, TextStyle, TextWrap,
    renderer::{
        OffscreenRenderer, SharedFontSystem, StyledTextGeometry, TextLayoutEngine,
        create_shared_font_system,
    },
};

/// Maximum scheduler/effect passes one public test operation may execute before failing.
///
/// This turns accidental recursive invalidation, action, entity-event, or foreground-task loops
/// into a deterministic test failure instead of hanging a test process.
pub const MAX_TEST_EFFECT_TURNS: usize = 1_024;
const MAX_TEST_PENDING_DISPATCHES: usize = 4_096;

/// CPU-only text estimates used to materialize size-dependent declarations for semantic tests.
/// Exact geometry and screenshots still replace these estimates through `OffscreenRenderer`.
struct SemanticTextLayout;

impl SemanticTextLayout {
    fn measure(content: &str, style: &TextStyle, max_width: Option<f32>) -> Size {
        let line_count = content.lines().count().max(1);
        let longest = content
            .lines()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0) as f32;
        let natural_width = longest * style.font_size * 0.55;
        let mut visual_lines = line_count;
        let width = if style.wrap == TextWrap::Word {
            if let Some(max_width) = max_width.filter(|width| *width > 0.0) {
                visual_lines = visual_lines.max((natural_width / max_width).ceil() as usize);
                natural_width.min(max_width)
            } else {
                natural_width
            }
        } else {
            natural_width
        };
        Size::new(width, style.line_height * visual_lines as f32)
    }
}

impl TextLayoutEngine for SemanticTextLayout {
    fn measure_text(
        &mut self,
        _id: TextId,
        content: &std::sync::Arc<str>,
        style: &TextStyle,
        max_width: Option<f32>,
        _scale_factor: f32,
    ) -> Size {
        Self::measure(content, style, max_width)
    }

    fn measure_styled_text(
        &mut self,
        id: TextId,
        content: &std::sync::Arc<str>,
        style: &TextStyle,
        _highlights: &std::sync::Arc<[TextHighlight]>,
        max_width: Option<f32>,
        scale_factor: f32,
    ) -> Size {
        self.measure_text(id, content, style, max_width, scale_factor)
    }

    fn text_geometry(
        &mut self,
        _id: TextId,
        _content: &std::sync::Arc<str>,
        _style: &TextStyle,
        _highlights: Option<&std::sync::Arc<[TextHighlight]>>,
        _width: f32,
        _scale_factor: f32,
        _visible_y: std::ops::Range<f32>,
    ) -> StyledTextGeometry {
        StyledTextGeometry {
            backgrounds: Vec::new(),
            decorations: Vec::new(),
        }
    }

    fn text_caret_position_with_highlights(
        &mut self,
        _id: TextId,
        _content: &std::sync::Arc<str>,
        _style: &TextStyle,
        _highlights: Option<&std::sync::Arc<[TextHighlight]>>,
        _width: f32,
        _scale_factor: f32,
        _index: usize,
    ) -> Point {
        Point::ZERO
    }

    fn text_index_for_point_with_highlights(
        &mut self,
        _id: TextId,
        _content: &std::sync::Arc<str>,
        _style: &TextStyle,
        _highlights: Option<&std::sync::Arc<[TextHighlight]>>,
        _width: f32,
        _scale_factor: f32,
        _point: Point,
    ) -> usize {
        0
    }

    fn text_selection_rects_with_highlights(
        &mut self,
        _id: TextId,
        _content: &std::sync::Arc<str>,
        _style: &TextStyle,
        _highlights: Option<&std::sync::Arc<[TextHighlight]>>,
        _width: f32,
        _scale_factor: f32,
        _visible_y: std::ops::Range<f32>,
        _start: usize,
        _end: usize,
    ) -> Vec<Rect> {
        Vec::new()
    }
}

/// A deterministic test-context failure.
#[derive(Debug, Error)]
pub enum TestAppError {
    #[error("invalid test-window configuration: {0}")]
    InvalidWindow(#[from] WindowCommandError),
    #[error("a simulated native window-tab snapshot is internally inconsistent")]
    InvalidWindowTabState,
    #[error(transparent)]
    Asset(#[from] AssetError),
    #[error("test window {0:?} does not exist")]
    UnknownWindow(WindowHandle),
    #[error("test window {window:?} does not contain a view of type {expected}")]
    WrongViewType {
        window: WindowHandle,
        expected: &'static str,
    },
    #[error("test window {window:?} does not contain element {element:?}")]
    UnknownElement {
        window: WindowHandle,
        element: ElementId,
    },
    #[error("element {element:?} in test window {window:?} is not focusable")]
    NotFocusable {
        window: WindowHandle,
        element: ElementId,
    },
    #[error("element {element:?} in test window {window:?} is not clickable")]
    NotClickable {
        window: WindowHandle,
        element: ElementId,
    },
    #[error("element {element:?} in test window {window:?} does not listen for {kind}")]
    NotListening {
        window: WindowHandle,
        element: ElementId,
        kind: &'static str,
    },
    #[error("element {element:?} in test window {window:?} is not a retained scroll container")]
    NotScrollable {
        window: WindowHandle,
        element: ElementId,
    },
    #[error("test window {0:?} has no focused text input")]
    NoFocusedTextInput(WindowHandle),
    #[error("test view declaration failed: {0}")]
    View(String),
    #[error("a foreground task panicked while the deterministic executor was running")]
    ForegroundTaskPanicked,
    #[error("the deterministic test executor exceeded {MAX_TEST_EFFECT_TURNS} effect turns")]
    EffectTurnLimit,
    #[error("the deterministic test dispatch queue exceeded {MAX_TEST_PENDING_DISPATCHES} entries")]
    DispatchQueueFull,
    #[error("the retained mouse target path exceeded {MAX_MOUSE_EVENT_PATH} ancestors")]
    MouseDispatchPathLimit,
    #[error("the keystroke sequence ended while a longer binding was still pending")]
    IncompleteKeystrokeSequence,
    #[error("native platform requests require an explicit platform test adapter")]
    UnsupportedPlatformRequest,
    #[error(transparent)]
    Visual(#[from] crate::VisualTestError),
    #[error(
        "element {element:?} in test window {window:?} has bounds {actual:?}, expected {expected:?} within {tolerance} logical pixels"
    )]
    GeometryMismatch {
        window: WindowHandle,
        element: ElementId,
        actual: Rect,
        expected: Rect,
        tolerance: f32,
    },
    #[error("visual geometry tolerance must be finite and non-negative")]
    InvalidGeometryTolerance,
}

/// A typed handle for one window owned by [`TestAppContext`].
pub struct TestWindowHandle<V> {
    handle: WindowHandle,
    marker: PhantomData<fn() -> V>,
}

impl<V> TestWindowHandle<V> {
    fn new(handle: WindowHandle) -> Self {
        Self {
            handle,
            marker: PhantomData,
        }
    }

    pub const fn window_handle(self) -> WindowHandle {
        self.handle
    }
}

impl<V> Clone for TestWindowHandle<V> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<V> Copy for TestWindowHandle<V> {}

impl<V> fmt::Debug for TestWindowHandle<V> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("TestWindowHandle")
            .field(&self.handle)
            .finish()
    }
}

impl<V> PartialEq for TestWindowHandle<V> {
    fn eq(&self, other: &Self) -> bool {
        self.handle == other.handle
    }
}

impl<V> Eq for TestWindowHandle<V> {}

/// Window-scoped facade for deterministic geometry and screenshot tests.
pub struct VisualTestContext<'a> {
    context: &'a mut TestAppContext,
    window: WindowHandle,
}

impl VisualTestContext<'_> {
    pub const fn window_handle(&self) -> WindowHandle {
        self.window
    }

    pub fn element_bounds(&mut self, element: impl Into<ElementId>) -> Result<Rect, TestAppError> {
        self.context.element_bounds(self.window, element)
    }

    pub fn assert_element_bounds(
        &mut self,
        element: impl Into<ElementId>,
        expected: Rect,
        tolerance: f32,
    ) -> Result<(), TestAppError> {
        self.context
            .assert_element_bounds(self.window, element, expected, tolerance)
    }

    pub fn capture_screenshot(&mut self) -> Result<crate::VisualSnapshot, TestAppError> {
        self.context.capture_screenshot(self.window)
    }

    /// Move the deterministic pointer through the production hit-test and paint-only hover path.
    pub fn move_pointer(&mut self, point: Point) -> Result<bool, TestAppError> {
        self.context.move_visual_pointer(self.window, point)
    }

    /// Resolve the platform cursor from the production painted hit stack at one logical point.
    pub fn cursor_style_at(&mut self, point: Point) -> Result<Option<CursorStyle>, TestAppError> {
        self.context.render_visual(self.window, false)?;
        Ok(self.context.window(self.window)?.ui.cursor_style_at(point))
    }

    /// Advance the injected clock and paint the resulting visual frame without rebuilding a
    /// clean view declaration.
    pub fn advance_time(&mut self, duration: Duration) -> Result<(), TestAppError> {
        self.context.advance_time(duration)?;
        let now = self.context.now();
        if let Some(state) = self.context.windows.get_mut(&self.window) {
            state.ui.advance_tooltips(now);
            state.ui.advance_spell_check(now);
        }
        self.context.render_visual(self.window, false)?;
        Ok(())
    }
}

struct TestWindow {
    parent: Option<WindowHandle>,
    restore_focus_on_close: Option<ElementId>,
    view: Box<dyn AnyView>,
    ui: UiTree,
    #[cfg(feature = "inspector")]
    inspector: Option<InspectorState>,
    listeners: ListenerRegistry,
    key_dispatch_scratch: Vec<KeyListenerBinding>,
    action_dispatch_scratch: Vec<ActionListenerBinding>,
    touch_captures: HashMap<TouchId, TouchCapture>,
    config: WindowOptions,
    state: WindowState,
    system_appearance: WindowAppearance,
    dirty: bool,
    /// A focus request waiting for the rebuild that introduces its target, with the simulated
    /// input that made it, exactly as the production window keeps one.
    pending_focus: Option<PendingFocus>,
    render_count: usize,
    retained_geometry_ready: bool,
    requested_animation_frame: bool,
    repaint_deadline: Option<Instant>,
    pointer: Option<Point>,
    /// Whether the one-time ready-to-show event was already delivered for this window.
    first_presented: bool,
}

/// Deterministic record of the application-shell state a test application requested.
///
/// Every field mirrors one native service that has no observable effect in a headless test.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TestApplicationShell {
    pub activation_policy: ActivationPolicy,
    /// Whether every window is hidden by `hide_application`.
    pub hidden: bool,
    /// Number of `activate_application` requests, and whether the last one forced activation.
    pub activations: usize,
    pub last_activation_forced: bool,
    pub dock_visible: bool,
    /// The in-flight attention request, if one has not been cancelled.
    pub dock_attention: Option<DockAttention>,
    pub secure_keyboard_entry: bool,
    pub beeps: usize,
    /// Number of accepted `move_to_applications_folder` requests.
    pub applications_folder_moves: usize,
}

impl Default for TestApplicationShell {
    fn default() -> Self {
        Self {
            activation_policy: ActivationPolicy::Regular,
            hidden: false,
            activations: 0,
            last_activation_forced: false,
            dock_visible: true,
            dock_attention: None,
            secure_keyboard_entry: false,
            beeps: 0,
            applications_folder_moves: 0,
        }
    }
}

enum TestDispatch {
    Event(WindowHandle, Event),
    Action(WindowHandle, AnyAction),
    Form(WindowHandle, ElementId, Option<ElementId>),
}

/// Headless application context for deterministic view and interaction tests.
///
/// It reuses QuickGUI's production view adapter, retained identity/focus/form tree, listener
/// registry, entity/global delivery, keymap, and foreground executor. No native window, WGPU
/// adapter, background thread, wall-clock polling loop, or accessibility service is created.
/// Geometry-dependent pointer and visual assertions belong in the future visual test adapter.
pub struct TestAppContext {
    windows: HashMap<WindowHandle, TestWindow>,
    active_window: Option<WindowHandle>,
    pending_windows: VecDeque<WindowRequest>,
    pending_closes: Vec<WindowHandle>,
    pending_dispatches: VecDeque<TestDispatch>,
    pending_entity_events: VecDeque<EntityEvent>,
    pending_global_notifications: VecDeque<TypeId>,
    pending_global_notification_types: HashSet<TypeId>,
    pending_all_globals: bool,
    foreground_tasks: ForegroundTaskSpawner,
    app_info: Option<AppInfo>,
    app_paths: Option<AppPaths>,
    system_info: SystemInfo,
    system_preferences: SystemPreferences,
    clipboard: ClipboardService,
    displays: Displays,
    keyboard_layout: KeyboardLayout,
    globals: GlobalStore,
    assets: Assets,
    font_system: SharedFontSystem,
    keymap: Keymap,
    menus: Vec<Menu>,
    system_notifications: HashMap<Arc<str>, SystemNotification>,
    tray_icons: HashMap<u32, TrayIconOptions>,
    notification_permission_status: NotificationPermissionStatus,
    application_callbacks: ApplicationCallbacks,
    quit_mode: QuitMode,
    focus_history: Vec<WindowHandle>,
    now: Rc<Cell<Instant>>,
    animation_epoch: Instant,
    application_shell: TestApplicationShell,
    next_dock_attention_id: i64,
    exited: bool,
    exit_code: Option<i32>,
    quit_phase_active: bool,
    last_window_quit_prevented: bool,
    relaunch_request: Option<RelaunchRequest>,
    form_submission_depth: u8,
    visual_renderer: Option<OffscreenRenderer>,
}

impl TestAppContext {
    /// Create one headless application window with default configuration.
    pub fn new<V: View>(view: V) -> Result<(Self, TestWindowHandle<V>), TestAppError> {
        Self::from_application(Application::new(), WindowOptions::default(), view)
    }

    /// Consume a configured application without starting Winit or WGPU.
    pub fn from_application<V: View>(
        application: Application,
        options: WindowOptions,
        view: V,
    ) -> Result<(Self, TestWindowHandle<V>), TestAppError> {
        validate_window_options(&options)?;
        let font_system = create_shared_font_system(&application.assets, &application.fonts)?;
        let app_info = application.app_info;
        let app_paths = match (application.app_paths, app_info.as_ref()) {
            (Some(paths), _) => Some(paths),
            (None, Some(info)) => Some(
                info.paths()
                    .map_err(|error| TestAppError::View(error.to_string()))?,
            ),
            (None, None) => None,
        };
        let system_info = SystemInfo::current();
        let system_preferences = SystemPreferences::default();
        let request = WindowRequest::new(view, options);
        let initial = TestWindowHandle::new(request.handle);
        let now = Rc::new(Cell::new(Instant::now()));
        let animation_epoch = now.get();
        let mut keymap = application.keymap;
        keymap.set_key_equivalents(crate::keyboard::key_equivalents_for_layout(
            &KeyboardLayout::default(),
        ));
        let mut context = Self {
            windows: HashMap::new(),
            active_window: None,
            pending_windows: VecDeque::from([request]),
            pending_closes: Vec::new(),
            pending_dispatches: VecDeque::with_capacity(8),
            pending_entity_events: VecDeque::with_capacity(8),
            pending_global_notifications: VecDeque::with_capacity(8),
            pending_global_notification_types: HashSet::with_capacity(8),
            pending_all_globals: false,
            foreground_tasks: ForegroundTaskSpawner::new_for_test(now.clone()),
            app_info,
            app_paths,
            system_info,
            system_preferences,
            clipboard: ClipboardService::memory(),
            displays: Displays::test_default(),
            keyboard_layout: KeyboardLayout::default(),
            globals: application.globals,
            assets: application.assets,
            font_system,
            keymap,
            menus: application.menus,
            system_notifications: HashMap::new(),
            tray_icons: HashMap::new(),
            notification_permission_status: NotificationPermissionStatus::Granted,
            application_callbacks: application.application_callbacks,
            quit_mode: application.quit_mode,
            focus_history: Vec::with_capacity(4),
            now,
            animation_epoch,
            application_shell: TestApplicationShell::default(),
            next_dock_attention_id: 0,
            exited: false,
            exit_code: None,
            quit_phase_active: false,
            last_window_quit_prevented: false,
            relaunch_request: None,
            form_submission_depth: 0,
            visual_renderer: None,
        };
        context.run_until_idle()?;
        Ok((context, initial))
    }

    pub fn windows(&self) -> Vec<WindowHandle> {
        let mut windows = self.windows.keys().copied().collect::<Vec<_>>();
        windows.sort_unstable();
        windows
    }

    pub fn active_window(&self) -> Option<WindowHandle> {
        self.active_window
    }

    /// Process exit code requested through
    /// [`EventContext::exit_with_code`](crate::EventContext::exit_with_code).
    pub const fn exit_code(&self) -> Option<i32> {
        self.exit_code
    }

    /// Deterministic snapshot of every application-shell service a test requested.
    pub const fn application_shell(&self) -> TestApplicationShell {
        self.application_shell
    }

    pub fn is_exited(&self) -> bool {
        self.exited
    }

    /// Inspect the prepared process request without spawning it in the headless runtime.
    pub fn relaunch_request(&self) -> Option<&RelaunchRequest> {
        self.relaunch_request.as_ref()
    }

    pub fn is_window_open(&self, window: WindowHandle) -> bool {
        self.windows.contains_key(&window)
    }

    pub fn menus(&self) -> &[Menu] {
        &self.menus
    }

    /// The explicit native menu override for one headless window, if any.
    pub fn window_menu_override(&self, window: WindowHandle) -> Option<&[Menu]> {
        self.windows.get(&window)?.config.window_menus.as_deref()
    }

    /// Inspect a deterministic posted notification by stable replacement tag.
    pub fn system_notification(&self, tag: &str) -> Option<&SystemNotification> {
        self.system_notifications.get(tag)
    }

    /// Set the deterministic notification authorization used by platform-request futures.
    pub fn set_notification_permission_status(&mut self, status: NotificationPermissionStatus) {
        self.notification_permission_status = status;
    }

    pub fn app_info(&self) -> Option<&AppInfo> {
        self.app_info.as_ref()
    }

    pub fn app_paths(&self) -> Option<&AppPaths> {
        self.app_paths.as_ref()
    }

    pub fn system_info(&self) -> &SystemInfo {
        &self.system_info
    }

    /// Inspect the deterministic system-preference snapshot.
    pub const fn system_preferences(&self) -> SystemPreferences {
        self.system_preferences
    }

    pub fn window_registry(&self) -> WindowRegistry {
        let total = self.windows.len() + self.pending_windows.len();
        let mut handles = self.windows();
        handles.extend(
            self.pending_windows
                .iter()
                .take(MAX_APPLICATION_WINDOWS.saturating_sub(handles.len()))
                .map(|request| request.handle),
        );
        handles.sort_unstable();
        handles.dedup();
        handles.truncate(MAX_APPLICATION_WINDOWS);
        WindowRegistry::new(
            Arc::<[WindowHandle]>::from(handles),
            self.active_window,
            total > MAX_APPLICATION_WINDOWS,
        )
    }

    /// Inspect the current deterministic display snapshot.
    pub fn displays(&self) -> &[Display] {
        self.displays.all()
    }

    pub fn primary_display(&self) -> Option<&Display> {
        self.displays.primary()
    }

    /// Inspect the deterministic active keyboard layout.
    pub fn keyboard_layout(&self) -> &KeyboardLayout {
        &self.keyboard_layout
    }

    /// Replace the deterministic active-display snapshot and rebuild only observing views.
    pub fn simulate_displays_change(&mut self, displays: Displays) -> Result<(), TestAppError> {
        if self.displays == displays {
            return Ok(());
        }
        self.displays = displays;
        for state in self.windows.values_mut() {
            let previous = state.state.display_id;
            state.state.display_id =
                crate::display::display_for_rect(&self.displays, state.state.bounds.bounds());
            if state.listeners.observes_displays
                || previous != state.state.display_id && state.listeners.observes_window_state
            {
                state.dirty = true;
            }
        }
        self.run_until_idle()
    }

    /// Replace the deterministic keyboard-layout snapshot and rebuild only observing views.
    pub fn simulate_keyboard_layout_change(
        &mut self,
        keyboard_layout: KeyboardLayout,
    ) -> Result<(), TestAppError> {
        if self.keyboard_layout == keyboard_layout {
            return Ok(());
        }
        self.keyboard_layout = keyboard_layout;
        self.keymap
            .set_key_equivalents(crate::keyboard::key_equivalents_for_layout(
                &self.keyboard_layout,
            ));
        for state in self.windows.values_mut() {
            if state.listeners.observes_keyboard_layout {
                state.dirty = true;
            }
        }
        if let Some(mut callback) = self.application_callbacks.keyboard_layout.take() {
            let layout = self.keyboard_layout.clone();
            let mut cx = self.event_context(None);
            callback(&layout, &mut cx);
            self.application_callbacks.keyboard_layout = Some(callback);
            self.apply_context(None, cx)?;
        }
        self.run_until_idle()
    }

    /// Replace the deterministic system-preference snapshot and rebuild only observing views.
    pub fn simulate_system_preferences_change(
        &mut self,
        system_preferences: SystemPreferences,
    ) -> Result<(), TestAppError> {
        if self.system_preferences == system_preferences {
            return Ok(());
        }
        self.system_preferences = system_preferences;
        let now = self.now();
        for state in self.windows.values_mut() {
            let reduce_motion = state.config.reduce_motion
                || system_preferences
                    .reduce_motion()
                    .is_some_and(|enabled| enabled);
            state.ui.set_reduce_motion(reduce_motion);
            state.ui.set_animations_enabled(!reduce_motion, now);
            if state.listeners.observes_system_preferences {
                state.dirty = true;
            }
        }
        self.run_until_idle()
    }

    /// Inspect the deterministic in-memory general clipboard.
    pub fn read_from_clipboard(&self) -> Result<Option<ClipboardItem>, ClipboardError> {
        self.clipboard.read(ClipboardTarget::General)
    }

    /// Seed or replace the deterministic in-memory general clipboard.
    pub fn write_to_clipboard(&self, item: ClipboardItem) -> Result<(), ClipboardError> {
        self.clipboard.write(ClipboardTarget::General, item)
    }

    /// Inspect the deterministic in-memory Linux primary selection.
    #[cfg(target_os = "linux")]
    pub fn read_from_selection_clipboard(&self) -> Result<Option<ClipboardItem>, ClipboardError> {
        self.clipboard.read(ClipboardTarget::Selection)
    }

    /// Seed or replace the deterministic in-memory Linux primary selection.
    #[cfg(target_os = "linux")]
    pub fn write_to_selection_clipboard(&self, item: ClipboardItem) -> Result<(), ClipboardError> {
        self.clipboard.write(ClipboardTarget::Selection, item)
    }

    /// Inspect the deterministic in-memory macOS Find pasteboard.
    #[cfg(target_os = "macos")]
    pub fn read_from_find_pasteboard(&self) -> Result<Option<ClipboardItem>, ClipboardError> {
        self.clipboard.read(ClipboardTarget::Find)
    }

    /// Seed or replace the deterministic in-memory macOS Find pasteboard.
    #[cfg(target_os = "macos")]
    pub fn write_to_find_pasteboard(&self, item: ClipboardItem) -> Result<(), ClipboardError> {
        self.clipboard.write(ClipboardTarget::Find, item)
    }

    /// Validate and type one raw window handle returned by [`EventContext::open_window`].
    pub fn typed_window<V: View>(
        &self,
        window: WindowHandle,
    ) -> Result<TestWindowHandle<V>, TestAppError> {
        let state = self.window(window)?;
        if state.view.as_any().is::<V>() {
            Ok(TestWindowHandle::new(window))
        } else {
            Err(TestAppError::WrongViewType {
                window,
                expected: type_name::<V>(),
            })
        }
    }

    pub fn read<V: View, R>(
        &self,
        window: TestWindowHandle<V>,
        read: impl FnOnce(&V) -> R,
    ) -> Result<R, TestAppError> {
        let state = self.window(window.handle)?;
        let view = state
            .view
            .as_any()
            .downcast_ref::<V>()
            .ok_or(TestAppError::WrongViewType {
                window: window.handle,
                expected: type_name::<V>(),
            })?;
        Ok(read(view))
    }

    pub fn update<V: View, R>(
        &mut self,
        window: TestWindowHandle<V>,
        update: impl FnOnce(&mut V, &mut EventContext) -> R,
    ) -> Result<R, TestAppError> {
        let mut cx = self.event_context(Some(window.handle));
        let result =
            {
                let state = self.window_mut(window.handle)?;
                let view = state.view.as_any_mut().downcast_mut::<V>().ok_or(
                    TestAppError::WrongViewType {
                        window: window.handle,
                        expected: type_name::<V>(),
                    },
                )?;
                update(view, &mut cx)
            };
        self.apply_context(Some(window.handle), cx)?;
        self.run_until_idle()?;
        Ok(result)
    }

    pub fn read_global<G: Global, R>(&self, read: impl FnOnce(&G) -> R) -> R {
        let global = self.globals.get::<G>();
        read(&global)
    }

    pub fn update_global<G: Global, R>(
        &mut self,
        update: impl FnOnce(&mut G, &mut EventContext) -> R,
    ) -> Result<R, TestAppError> {
        let mut cx = self.event_context(None);
        let result = {
            let globals = self.globals.clone();
            let mut global = globals.get_mut::<G>();
            update(&mut global, &mut cx)
        };
        cx.update_global::<G, _>(|_| {});
        self.apply_context(None, cx)?;
        self.run_until_idle()?;
        Ok(result)
    }

    pub fn render_count(&self, window: WindowHandle) -> Result<usize, TestAppError> {
        Ok(self.window(window)?.render_count)
    }

    /// Headless counterpart to [`AppRunner::update_elements`], using the same retained tree
    /// update and layout paths without declaring the view again.
    pub fn update_elements(
        &mut self,
        window: WindowHandle,
        updates: &[crate::ElementUpdate],
    ) -> Result<bool, TestAppError> {
        let state = self.window_mut(window)?;
        if state.dirty {
            return Ok(true);
        }
        if state.listeners.needs_scoped_replacement(updates) {
            return Ok(false);
        }
        let Some(kind) = state
            .ui
            .update_elements(updates)
            .map_err(|error| TestAppError::View(error.to_string()))?
        else {
            return Ok(false);
        };
        if kind != crate::ui_tree::ElementUpdateKind::None {
            state.retained_geometry_ready = false;
        }
        self.prepare_retained_geometry(window)?;
        Ok(true)
    }

    /// Headless counterpart to `AppRunner::invalidate_elements`.
    pub fn invalidate_elements(
        &mut self,
        window: WindowHandle,
        ids: &[ElementId],
    ) -> Result<(), TestAppError> {
        let state = self.window_mut(window)?;
        state.dirty |= !state.listeners.scopes.invalidate(ids);
        self.run_until_idle()
    }

    pub fn window_state(&self, window: WindowHandle) -> Result<WindowState, TestAppError> {
        Ok(self.window(window)?.state)
    }

    /// Build the accessibility tree QuickGUI would hand a platform adapter for one window.
    ///
    /// The update is the same one a live screen reader receives, so a test can assert on the
    /// roles, names, and structure of what an assistive client sees.
    #[cfg(any(test, feature = "test-support"))]
    pub fn accessibility_update(
        &mut self,
        window: WindowHandle,
    ) -> Result<accesskit::TreeUpdate, TestAppError> {
        self.run_until_idle()?;
        self.prepare_retained_geometry(window)?;
        let title = self.window(window)?.config.title.clone();
        Ok(self.window(window)?.ui.accessibility_update(&title))
    }

    #[cfg(test)]
    pub(crate) fn popover_grabs_focus(
        &self,
        window: WindowHandle,
    ) -> Result<Option<bool>, TestAppError> {
        Ok(self
            .window(window)?
            .config
            .popover
            .as_ref()
            .map(|popover| popover.grab))
    }

    /// Inject a bounded native tab-group snapshot and rebuild only an observing view.
    pub fn simulate_window_tab_state(
        &mut self,
        window: WindowHandle,
        tabs: WindowTabState,
    ) -> Result<(), TestAppError> {
        if !tabs.is_valid() {
            return Err(TestAppError::InvalidWindowTabState);
        }
        let state = self.window_mut(window)?;
        if state.state.native_tabs != tabs {
            state.state.native_tabs = tabs;
            if state.listeners.observes_window_state {
                state.dirty = true;
            }
        }
        self.run_until_idle()
    }

    /// Capture the current bounded retained-tree snapshot without creating native or GPU state.
    #[cfg(feature = "inspector")]
    pub fn inspector_snapshot(
        &mut self,
        window: WindowHandle,
    ) -> Result<Option<crate::InspectorSnapshot>, TestAppError> {
        if self.window(window)?.inspector.is_none() {
            return Ok(None);
        }
        self.prepare_retained_geometry(window)?;
        let state = self.window_mut(window)?;
        let viewport = state.state.viewport_size;
        let scale_factor = state.state.scale_factor;
        let TestWindow { ui, inspector, .. } = state;
        let inspector = inspector
            .as_mut()
            .expect("inspector presence checked before geometry preparation");
        inspector.refresh(
            ui,
            FrameMetrics::default(),
            InspectorFrameDamage::default(),
            viewport,
            scale_factor,
        );
        Ok(Some(inspector.snapshot().clone()))
    }

    /// Deliver a deterministic operating-system appearance change.
    ///
    /// A window with an explicit preference retains that effective appearance. The simulated
    /// system value is still remembered so returning the window to system-following mode uses the
    /// latest value without a polling source.
    pub fn simulate_appearance_change(
        &mut self,
        window: WindowHandle,
        appearance: WindowAppearance,
    ) -> Result<(), TestAppError> {
        let changed = {
            let state = self.window_mut(window)?;
            state.system_appearance = appearance;
            if state.config.preferred_appearance.is_some() || state.state.appearance == appearance {
                false
            } else {
                state.state.appearance = appearance;
                if state.listeners.observes_window_state {
                    state.dirty = true;
                }
                true
            }
        };
        if changed {
            self.queue_dispatch(TestDispatch::Event(
                window,
                Event::AppearanceChanged(appearance),
            ))?;
        }
        self.run_until_idle()
    }

    pub fn window_title(&self, window: WindowHandle) -> Result<&str, TestAppError> {
        Ok(&self.window(window)?.config.title)
    }

    /// Scope visual operations to one existing deterministic window.
    pub fn visual(&mut self, window: WindowHandle) -> Result<VisualTestContext<'_>, TestAppError> {
        self.window(window)?;
        Ok(VisualTestContext {
            context: self,
            window,
        })
    }

    pub fn contains_element(
        &self,
        window: WindowHandle,
        element: impl Into<ElementId>,
    ) -> Result<bool, TestAppError> {
        Ok(self.window(window)?.ui.contains_element(element.into()))
    }

    /// Lay out and paint the current declaration, then return one element's logical bounds.
    ///
    /// The first geometry or screenshot request lazily initializes one offscreen WGPU renderer.
    /// Ordinary semantic tests continue to create no GPU resources.
    pub fn element_bounds(
        &mut self,
        window: WindowHandle,
        element: impl Into<ElementId>,
    ) -> Result<Rect, TestAppError> {
        let element = element.into();
        self.render_visual(window, false)?;
        self.window(window)?
            .ui
            .element_bounds(element)
            .ok_or(TestAppError::UnknownElement { window, element })
    }

    /// Assert deterministic logical geometry with an explicit floating-point tolerance.
    pub fn assert_element_bounds(
        &mut self,
        window: WindowHandle,
        element: impl Into<ElementId>,
        expected: Rect,
        tolerance: f32,
    ) -> Result<(), TestAppError> {
        if !tolerance.is_finite() || tolerance < 0.0 {
            return Err(TestAppError::InvalidGeometryTolerance);
        }
        let element = element.into();
        let actual = self.element_bounds(window, element)?;
        if [
            (actual.x, expected.x),
            (actual.y, expected.y),
            (actual.width, expected.width),
            (actual.height, expected.height),
        ]
        .into_iter()
        .all(|(actual, expected)| (actual - expected).abs() <= tolerance)
        {
            Ok(())
        } else {
            Err(TestAppError::GeometryMismatch {
                window,
                element,
                actual,
                expected,
                tolerance,
            })
        }
    }

    /// Capture one bounded physical-pixel frame through the production WGPU pipelines.
    ///
    /// The headless target has no presentation loop and performs work only for this call. Native
    /// `NSView` children are rejected instead of silently producing an incomplete baseline.
    pub fn capture_screenshot(
        &mut self,
        window: WindowHandle,
    ) -> Result<crate::VisualSnapshot, TestAppError> {
        self.render_visual(window, true)?.ok_or_else(|| {
            TestAppError::Visual(crate::VisualTestError::Render(
                "the visual frame did not produce a snapshot".to_owned(),
            ))
        })
    }
}

mod dispatch;
mod effects;
mod input;
mod lifecycle;
mod window_commands;
mod window_lifecycle;

impl Application {
    /// Consume this configured application into a deterministic headless test context.
    pub fn into_test_context<V: View>(
        self,
        options: WindowOptions,
        view: V,
    ) -> Result<(TestAppContext, TestWindowHandle<V>), TestAppError> {
        TestAppContext::from_application(self, options, view)
    }
}

impl Drop for TestAppContext {
    fn drop(&mut self) {
        self.foreground_tasks.shutdown();
    }
}

fn test_window_state(
    handle: WindowHandle,
    options: &WindowOptions,
    focused: bool,
    displays: &Displays,
    parent: Option<WindowState>,
) -> WindowState {
    let preferred_display = options.display_id.and_then(|id| displays.find(id));
    let placement_display = preferred_display.or_else(|| displays.primary());
    let popover_bounds = options
        .popover
        .as_ref()
        .zip(parent)
        .map(|(popover, parent)| {
            let parent = parent.bounds.bounds();
            let anchor = Rect::new(
                parent.x + popover.anchor_rect.x,
                parent.y + popover.anchor_rect.y,
                popover.anchor_rect.width,
                popover.anchor_rect.height,
            );
            #[cfg(any(target_os = "macos", test))]
            {
                let anchor_center = Point::new(
                    anchor.x + anchor.width * 0.5,
                    anchor.y + anchor.height * 0.5,
                );
                let visible = displays
                    .all()
                    .iter()
                    .find(|display| display.visible_bounds().contains(anchor_center))
                    .or_else(|| displays.primary())
                    .map_or_else(
                        || Rect::new(-1_000_000.0, -1_000_000.0, 2_000_000.0, 2_000_000.0),
                        Display::visible_bounds,
                    );
                crate::popover::place_popover(anchor, options.size, visible, popover)
            }
            #[cfg(not(any(target_os = "macos", test)))]
            {
                crate::popover::unconstrained_popover_rect(anchor, options.size, popover)
            }
        });
    let bounds = popover_bounds.map_or_else(
        || {
            options.window_bounds.unwrap_or_else(|| {
                WindowBounds::Windowed(if options.display_id.is_some() {
                    placement_display.map_or_else(
                        || Rect::from_size(options.size),
                        |display| display.centered_bounds(options.size),
                    )
                } else {
                    Rect::from_size(options.size)
                })
            })
        },
        WindowBounds::Windowed,
    );
    let rect = bounds.bounds();
    let display_id = preferred_display
        .map(Display::id)
        .or_else(|| crate::display::display_for_rect(displays, rect));
    WindowState {
        handle,
        display_id,
        kind: options.kind,
        bounds,
        viewport_size: Size::new(rect.width, rect.height),
        minimum_size: options.minimum_size,
        maximum_size: options.maximum_size,
        scale_factor: display_id
            .and_then(|id| displays.find(id))
            .map_or(1.0, Display::scale_factor),
        appearance: options.preferred_appearance.unwrap_or_default(),
        background_appearance: options.window_background,
        macos_vibrancy: options.macos_vibrancy,
        macos_visual_effect_state: options.macos_visual_effect_state,
        focused,
        focusable: options.focusable,
        visible: options.show,
        minimized: false,
        maximized: matches!(bounds, WindowBounds::Maximized(_)),
        fullscreen: matches!(bounds, WindowBounds::Fullscreen(_)),
        occluded: false,
        movable: options.is_movable,
        resizable: options.is_resizable,
        minimizable: options.is_minimizable,
        maximizable: options.is_maximizable,
        closable: options.is_closable,
        decorated: options.decorated,
        shadow: options.shadow,
        content_protected: options.content_protected,
        window_level: effective_window_level(options),
        ignore_mouse_events: options.ignore_mouse_events,
        forward_mouse_events: options.forward_mouse_events,
        window_enabled: options.window_enabled,
        aspect_ratio: options.aspect_ratio,
        window_buttons_visible: options.window_buttons_visible,
        skip_taskbar: options.skip_taskbar,
        visible_on_all_workspaces: effective_visible_on_all_workspaces(options),
        opacity: options.opacity,
        has_icon: options.icon.is_some(),
        taskbar_progress_state: options.taskbar_progress_state,
        taskbar_progress: options.taskbar_progress,
        has_taskbar_overlay_icon: options.taskbar_overlay_icon.is_some(),
        cursor_visible: options.cursor_visible,
        cursor_grab: options.cursor_grab,
        cursor_hit_test: options.cursor_hit_test,
        cursor_position: options.cursor_position,
        represented_file: options.represented_file.is_some(),
        document_edited: options.document_edited,
        native_tabbing: options.tabbing_identifier.is_some(),
        native_tabs: WindowTabState::default(),
        #[cfg(feature = "inspector")]
        inspector_active: options.inspector,
    }
}

fn set_test_window_bounds(window: &mut TestWindow, bounds: WindowBounds) {
    let rect = bounds.bounds();
    window.config.size = Size::new(rect.width, rect.height);
    window.config.window_bounds = Some(bounds);
    window.state.bounds = bounds;
    window.state.viewport_size = Size::new(rect.width, rect.height);
    window.state.maximized = matches!(bounds, WindowBounds::Maximized(_));
    window.state.fullscreen = matches!(bounds, WindowBounds::Fullscreen(_));
    window.dirty = true;
}

#[cfg(test)]
mod tests;

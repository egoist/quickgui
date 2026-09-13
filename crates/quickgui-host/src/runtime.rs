use super::*;

use std::cell::Cell;
use std::collections::HashSet;

thread_local! {
    /// Hosted windows whose JavaScript listeners intercept `Event::CloseRequested`.
    ///
    /// Interception is declared ahead of the native decision so the hosted view can answer
    /// synchronously on the application thread; JavaScript later completes the close with an
    /// explicit `closeWindow` command.
    static CLOSE_INTERCEPTING_WINDOWS: RefCell<HashSet<u32>> = RefCell::new(HashSet::new());
    /// Whether the hosted application declared a preventable before-quit listener.
    static QUIT_INTERCEPTION: Cell<bool> = const { Cell::new(false) };
}

/// Declare or withdraw close interception for one hosted window.
pub(crate) fn set_close_interception(window: u32, intercepting: bool) {
    CLOSE_INTERCEPTING_WINDOWS.with_borrow_mut(|windows| {
        if intercepting {
            if windows.len() < MAX_WINDOWS {
                windows.insert(window);
            }
        } else {
            windows.remove(&window);
        }
    });
}

pub(crate) fn intercepts_close(window: u32) -> bool {
    CLOSE_INTERCEPTING_WINDOWS.with_borrow(|windows| windows.contains(&window))
}

/// Declare or withdraw a preventable before-quit listener for the hosted application.
pub(crate) fn set_quit_interception(intercepting: bool) {
    QUIT_INTERCEPTION.with(|flag| flag.set(intercepting));
}

pub(crate) fn intercepts_quit() -> bool {
    QUIT_INTERCEPTION.with(Cell::get)
}

pub(crate) fn reset_interception_state() {
    CLOSE_INTERCEPTING_WINDOWS.with_borrow_mut(HashSet::clear);
    RESIZE_POLICIES.with_borrow_mut(HashMap::clear);
    MOVE_POLICIES.with_borrow_mut(HashMap::clear);
    crate::system::reset_dock_attention_requests();
    set_quit_interception(false);
}

thread_local! {
    /// Declared-ahead `Event::WillResize` answers, keyed by hosted window id.
    static RESIZE_POLICIES: RefCell<HashMap<u32, WindowResizePolicy>> =
        RefCell::new(HashMap::new());
    /// Declared-ahead `Event::WillMove` answers, keyed by hosted window id.
    static MOVE_POLICIES: RefCell<HashMap<u32, WindowMovePolicy>> = RefCell::new(HashMap::new());
}

/// The narrowing JavaScript declared for a window's live native resize.
///
/// The core answers `Event::WillResize` synchronously on the application thread, so the policy is
/// declared ahead instead of being asked of JavaScript during the event.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct WindowResizePolicy {
    /// Content `width / height` the resize is snapped to.
    pub(crate) aspect_ratio: Option<f32>,
    pub(crate) minimum: Option<(f32, f32)>,
    pub(crate) maximum: Option<(f32, f32)>,
    /// Grid step applied to the proposed inner size.
    pub(crate) snap: Option<(f32, f32)>,
}

/// The narrowing JavaScript declared for a window's live native move.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct WindowMovePolicy {
    /// Keep the window's origin inside the work area of the display that contains it.
    pub(crate) keep_on_screen: bool,
}

impl WindowResizePolicy {
    pub(crate) fn is_empty(self) -> bool {
        self == Self::default()
    }

    /// Narrow one proposed inner size, or return `None` when the proposal already complies.
    pub(crate) fn constrain(self, proposed: quickgui::Size) -> Option<quickgui::Size> {
        let mut width = proposed.width;
        let mut height = proposed.height;
        if let Some((step_width, step_height)) = self.snap {
            width = snap_to(width, step_width);
            height = snap_to(height, step_height);
        }
        if let Some(ratio) = self.aspect_ratio {
            height = width / ratio;
        }
        if let Some((minimum_width, minimum_height)) = self.minimum {
            width = width.max(minimum_width);
            height = height.max(minimum_height);
        }
        if let Some((maximum_width, maximum_height)) = self.maximum {
            width = width.min(maximum_width);
            height = height.min(maximum_height);
        }
        if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
            return None;
        }
        let constrained = quickgui::Size::new(width, height);
        (constrained != proposed).then_some(constrained)
    }
}

fn snap_to(value: f32, step: f32) -> f32 {
    if !(step.is_finite() && step > 0.0) {
        return value;
    }
    (value / step).round() * step
}

/// Declare or withdraw one window's resize policy.
pub(crate) fn set_resize_policy(window: u32, policy: Option<WindowResizePolicy>) {
    RESIZE_POLICIES.with_borrow_mut(|policies| match policy {
        Some(policy) if !policy.is_empty() => {
            if policies.len() < MAX_WINDOWS || policies.contains_key(&window) {
                policies.insert(window, policy);
            }
        }
        _ => {
            policies.remove(&window);
        }
    });
}

/// Declare or withdraw one window's move policy.
pub(crate) fn set_move_policy(window: u32, policy: Option<WindowMovePolicy>) {
    MOVE_POLICIES.with_borrow_mut(|policies| match policy {
        Some(policy) if policy.keep_on_screen => {
            if policies.len() < MAX_WINDOWS || policies.contains_key(&window) {
                policies.insert(window, policy);
            }
        }
        _ => {
            policies.remove(&window);
        }
    });
}

/// Retained resize and move policy counts, used to assert the declaration bound.
#[cfg(test)]
pub(crate) fn retained_window_policy_counts() -> (usize, usize) {
    (
        RESIZE_POLICIES.with_borrow(HashMap::len),
        MOVE_POLICIES.with_borrow(HashMap::len),
    )
}

pub(crate) fn forget_window_policies(window: u32) {
    RESIZE_POLICIES.with_borrow_mut(|policies| policies.remove(&window));
    MOVE_POLICIES.with_borrow_mut(|policies| policies.remove(&window));
}

fn resize_policy(window: u32) -> Option<WindowResizePolicy> {
    RESIZE_POLICIES.with_borrow(|policies| policies.get(&window).copied())
}

fn move_policy(window: u32) -> Option<WindowMovePolicy> {
    MOVE_POLICIES.with_borrow(|policies| policies.get(&window).copied())
}

/// Clamp one proposed window origin into the work area of the display that contains it.
pub(crate) fn keep_position_on_screen(
    displays: &[quickgui::Display],
    proposed: Point,
) -> Option<Point> {
    let display = displays
        .iter()
        .find(|display| display.bounds().contains(proposed))
        .or_else(|| displays.iter().find(|display| display.is_primary()))
        .or_else(|| displays.first())?;
    let area = display.visible_bounds();
    let x = proposed.x.clamp(area.x, (area.x + area.width).max(area.x));
    let y = proposed.y.clamp(area.y, (area.y + area.height).max(area.y));
    let clamped = Point::new(x, y);
    (clamped != proposed).then_some(clamped)
}

/// Whether one core event is a window-scoped notification the hosted view answers.
///
/// Everything else — pointer, keyboard, and drag traffic — leaves the hosted `View::event`
/// callback immediately, so the retained input path costs one discriminant test per event.
pub(crate) const fn is_hosted_window_event(event: &Event) -> bool {
    matches!(
        event,
        Event::CloseRequested
            | Event::Minimized(_)
            | Event::Maximized(_)
            | Event::FullscreenChanged(_)
            | Event::FirstPresented
            | Event::OcclusionChanged(_)
            | Event::WindowLevelChanged(_)
            | Event::Focused(_)
            | Event::AppearanceChanged(_)
            | Event::Resized { .. }
            | Event::Moved { .. }
            | Event::WillResize { .. }
            | Event::WillMove { .. }
    )
}

/// Forward one core window lifecycle event to JavaScript and answer any declared constraint.
///
/// Returns `true` when the event was a window lifecycle event, so the hosted view can keep its
/// remaining event handling untouched.
pub(crate) fn handle_window_lifecycle_event(
    window: u32,
    event: &Event,
    cx: &mut EventContext,
    events: &EventQueue,
) -> bool {
    let (kind, value): (&'static str, Option<String>) = match event {
        Event::Minimized(minimized) => ("window-minimize", Some(minimized.to_string())),
        Event::Maximized(maximized) => ("window-maximize", Some(maximized.to_string())),
        Event::FullscreenChanged(fullscreen) => ("window-fullscreen", Some(fullscreen.to_string())),
        Event::FirstPresented => ("window-ready-to-show", None),
        Event::OcclusionChanged(occluded) => ("window-occlusion", Some(occluded.to_string())),
        Event::WindowLevelChanged(level) => (
            "window-level",
            Some(crate::system::window_level_name(*level).to_owned()),
        ),
        Event::Focused(focused) => ("window-focus", Some(focused.to_string())),
        Event::AppearanceChanged(appearance) => (
            "window-appearance",
            Some(
                match appearance {
                    quickgui::WindowAppearance::Light => "light",
                    quickgui::WindowAppearance::Dark => "dark",
                }
                .to_owned(),
            ),
        ),
        Event::Resized { logical_size, .. } => (
            "window-resize",
            Some(format!(
                "{{\"width\":{},\"height\":{}}}",
                logical_size.width, logical_size.height
            )),
        ),
        Event::Moved {
            logical_position, ..
        } => (
            "window-move",
            Some(format!(
                "{{\"x\":{},\"y\":{}}}",
                logical_position.x, logical_position.y
            )),
        ),
        Event::WillResize { proposed_size } => {
            if let Some(policy) = resize_policy(window)
                && let Some(constrained) = policy.constrain(*proposed_size)
            {
                let _ = cx.constrain_resize(constrained);
            }
            (
                "window-will-resize",
                Some(format!(
                    "{{\"width\":{},\"height\":{}}}",
                    proposed_size.width, proposed_size.height
                )),
            )
        }
        Event::WillMove { proposed_position } => {
            if move_policy(window).is_some_and(|policy| policy.keep_on_screen)
                && let Some(clamped) = keep_position_on_screen(cx.displays(), *proposed_position)
            {
                let _ = cx.constrain_move(clamped);
            }
            (
                "window-will-move",
                Some(format!(
                    "{{\"x\":{},\"y\":{}}}",
                    proposed_position.x, proposed_position.y
                )),
            )
        }
        _ => return false,
    };
    enqueue_event(
        events,
        QueuedEvent {
            kind,
            window,
            target: ROOT_NODE,
            value: value.map(Arc::from),
        },
    );
    true
}

pub(crate) fn quit_reason_name(reason: quickgui::QuitReason) -> &'static str {
    match reason {
        quickgui::QuitReason::Explicit => "explicit",
        quickgui::QuitReason::Relaunch => "relaunch",
        quickgui::QuitReason::LastWindowClosed => "last-window-closed",
        quickgui::QuitReason::OperatingSystem => "operating-system",
    }
}

pub(super) struct NativeWindowRuntime {
    pub(super) config: WindowOptions,
    pub(super) tree: Rc<RefCell<NativeTree>>,
    pub(super) markdown: Rc<RefCell<HashMap<u32, Markdown>>>,
    pub(super) documents: Rc<RefCell<HashMap<u32, super::document::NativeDocument>>>,
    pub(super) svgs: Rc<RefCell<HashMap<u32, NativeSvgState>>>,
    pub(super) lists: Rc<RefCell<HashMap<u32, NativeListState>>>,
    pub(super) terminals: Rc<RefCell<HashMap<u32, NativeTerminalState>>>,
    pub(super) images: Rc<RefCell<HashMap<u32, NativeImageState>>>,
    /// Retained decoded raster backgrounds, decoded once per declared source.
    pub(super) background_images: Rc<RefCell<HashMap<u32, NativeBackgroundImageState>>>,
    pub(super) shaders: Rc<RefCell<HashMap<u32, NativeShaderState>>>,
    pub(super) menus: NativeMenuStates,
    #[cfg(target_os = "macos")]
    pub(super) swift_ui_hosts: Rc<RefCell<HashMap<u32, NativeSwiftUiHostState>>>,
    #[cfg(target_os = "macos")]
    pub(super) embedded_views: Rc<RefCell<HashMap<u32, MacEmbeddedView>>>,
    pub(super) handle: Option<WindowHandle>,
}

impl NativeWindowRuntime {
    pub(super) fn view(
        &self,
        window: u32,
        events: &EventQueue,
        handles: &Rc<RefCell<HashMap<WindowHandle, u32>>>,
    ) -> NativeView {
        NativeView {
            window,
            handles: Some(Rc::clone(handles)),
            tree: Rc::clone(&self.tree),
            events: Rc::clone(events),
            markdown: Rc::clone(&self.markdown),
            documents: Rc::clone(&self.documents),
            svgs: Rc::clone(&self.svgs),
            lists: Rc::clone(&self.lists),
            terminals: Rc::clone(&self.terminals),
            images: Rc::clone(&self.images),
            background_images: Rc::clone(&self.background_images),
            shaders: Rc::clone(&self.shaders),
            menus: Rc::clone(&self.menus),
            context_menu: ContextMenuState::new(),
            context_menu_owner: None,
            focused_node: None,
            components: NativeComponentStates::default(),
            motions: HashMap::new(),
            #[cfg(target_os = "macos")]
            swift_ui_hosts: Rc::clone(&self.swift_ui_hosts),
            #[cfg(target_os = "macos")]
            embedded_views: Rc::clone(&self.embedded_views),
        }
    }
}

pub(super) fn native_tree_from_initial_batch(
    batch: &[u8],
) -> std::result::Result<NativeTree, String> {
    if batch.is_empty() {
        return Ok(NativeTree::default());
    }
    if batch.len() > MAX_BATCH_BYTES {
        return Err(format!(
            "initial mutation batch exceeds {MAX_BATCH_BYTES} bytes"
        ));
    }
    let mutations = decode_batch(batch).map_err(|error| error.to_string())?;
    let mut tree = NativeTree::default();
    apply_mutations(&mut tree, mutations).map_err(|error| error.to_string())?;
    Ok(tree)
}

pub(super) struct NativeRuntime {
    pub(super) next_window_id: u32,
    pub(super) windows: HashMap<u32, NativeWindowRuntime>,
    pub(super) window_order: Vec<u32>,
    pub(super) events: EventQueue,
    pub(super) handles: Rc<RefCell<HashMap<WindowHandle, u32>>>,
    #[cfg(target_os = "macos")]
    pub(super) embedded_views: Rc<RefCell<HashMap<u32, MacEmbeddedView>>>,
    pub(super) closed_windows: Rc<RefCell<Vec<u32>>>,
    pub(super) pending_dialogs: Vec<PendingDialog>,
    pub(super) pending_shell: Vec<system::PendingShell>,
    pub(super) pending_notification_permissions: Vec<system::PendingNotificationPermission>,
    pub(super) pending_file_icons: Vec<system::PendingFileIcon>,
    pub(super) pending_user_tasks: Vec<system::PendingUserTasks>,
    pub(super) pending_global_shortcuts: Vec<system::PendingGlobalShortcut>,
    pub(super) pending_tray: Vec<system::PendingTray>,
    pub(super) pending_app_services: Vec<system::PendingAppService>,
    pub(super) system_observation: system::SystemObservation,
    pub(super) app_info: Option<AppInfo>,
    pub(super) app_paths: Option<AppPaths>,
    pub(super) quit_mode: QuitMode,
    pub(super) fonts: Vec<Arc<[u8]>>,
    pub(super) runner: Option<AppRunner>,
}

impl NativeRuntime {
    pub(super) fn new(options: NativeAppOptions) -> std::result::Result<Self, String> {
        let fonts = native_font_data(&options).unwrap_or_default();
        let (app_info, app_paths, quit_mode) = native_app_configuration(options)?;
        Ok(Self {
            next_window_id: 1,
            windows: HashMap::new(),
            window_order: Vec::with_capacity(2),
            events: Rc::new(RefCell::new(VecDeque::with_capacity(32))),
            handles: Rc::new(RefCell::new(HashMap::with_capacity(2))),
            #[cfg(target_os = "macos")]
            embedded_views: Rc::new(RefCell::new(HashMap::with_capacity(2))),
            closed_windows: Rc::new(RefCell::new(Vec::with_capacity(2))),
            pending_dialogs: Vec::with_capacity(2),
            pending_shell: Vec::with_capacity(2),
            pending_notification_permissions: Vec::with_capacity(1),
            pending_file_icons: Vec::with_capacity(1),
            pending_user_tasks: Vec::with_capacity(1),
            pending_global_shortcuts: Vec::with_capacity(2),
            pending_tray: Vec::with_capacity(2),
            pending_app_services: Vec::with_capacity(2),
            system_observation: system::SystemObservation::default(),
            app_info,
            app_paths,
            quit_mode,
            fonts,
            runner: None,
        })
    }

    pub(super) fn create_window(
        &mut self,
        options: NativeWindowOptions,
        initial_batch: &[u8],
    ) -> std::result::Result<u32, String> {
        let id = self.allocate_window_id()?;
        self.create_window_with_id(id, options, initial_batch)
    }

    pub(super) fn create_window_with_id(
        &mut self,
        id: u32,
        options: NativeWindowOptions,
        initial_batch: &[u8],
    ) -> std::result::Result<u32, String> {
        self.sync_closed_windows();
        self.claim_window_id(id)?;
        let mut config = window_config(&options)?;
        if let Some(state) = &options.restore_state {
            let state = crate::system::parse_window_restore_state(state)?;
            let displays = self
                .runner
                .as_ref()
                .map(quickgui::AppRunner::displays)
                .ok_or_else(|| {
                    "restoring window geometry requires a running QuickGUI application".to_owned()
                })?;
            config = config.restore(&state, &displays);
        }
        let mut window = NativeWindowRuntime {
            config,
            tree: Rc::new(RefCell::new(native_tree_from_initial_batch(initial_batch)?)),
            markdown: Rc::new(RefCell::new(HashMap::new())),
            documents: Rc::new(RefCell::new(HashMap::new())),
            svgs: Rc::new(RefCell::new(HashMap::new())),
            lists: Rc::new(RefCell::new(HashMap::new())),
            terminals: Rc::new(RefCell::new(HashMap::new())),
            images: Rc::new(RefCell::new(HashMap::new())),
            background_images: Rc::new(RefCell::new(HashMap::new())),
            shaders: Rc::new(RefCell::new(HashMap::new())),
            menus: Rc::new(RefCell::new(HashMap::new())),
            #[cfg(target_os = "macos")]
            swift_ui_hosts: Rc::new(RefCell::new(HashMap::new())),
            #[cfg(target_os = "macos")]
            embedded_views: Rc::clone(&self.embedded_views),
            handle: None,
        };
        if let Some(runner) = &mut self.runner {
            let handle = runner
                .open_window(
                    window.config.clone(),
                    window.view(id, &self.events, &self.handles),
                )
                .map_err(|error| error.to_string())?;
            self.handles.borrow_mut().insert(handle, id);
            window.handle = Some(handle);
        }
        self.windows.insert(id, window);
        self.window_order.push(id);
        Ok(id)
    }

    pub(super) fn create_system_popover(
        &mut self,
        parent: u32,
        anchor: u32,
        options: NativeWindowOptions,
        initial_batch: &[u8],
    ) -> std::result::Result<u32, String> {
        let id = self.allocate_window_id()?;
        self.create_system_popover_with_id(id, parent, anchor, options, initial_batch)
    }

    pub(super) fn create_system_popover_with_id(
        &mut self,
        id: u32,
        parent: u32,
        anchor: u32,
        options: NativeWindowOptions,
        initial_batch: &[u8],
    ) -> std::result::Result<u32, String> {
        self.sync_closed_windows();
        self.claim_window_id(id)?;
        let parent_handle = {
            let parent_window = self
                .windows
                .get(&parent)
                .ok_or_else(|| format!("unknown QuickGUI parent window {parent}"))?;
            if !parent_window.tree.borrow().nodes.contains_key(&anchor) {
                return Err(format!(
                    "system popover anchor node {anchor} is not mounted in parent window {parent}"
                ));
            }
            parent_window
                .handle
                .ok_or_else(|| "a system popover requires a running parent window".to_owned())?
        };
        let config = system_popover_config(&options)?;
        let mut window = NativeWindowRuntime {
            config,
            tree: Rc::new(RefCell::new(native_tree_from_initial_batch(initial_batch)?)),
            markdown: Rc::new(RefCell::new(HashMap::new())),
            documents: Rc::new(RefCell::new(HashMap::new())),
            svgs: Rc::new(RefCell::new(HashMap::new())),
            lists: Rc::new(RefCell::new(HashMap::new())),
            terminals: Rc::new(RefCell::new(HashMap::new())),
            images: Rc::new(RefCell::new(HashMap::new())),
            background_images: Rc::new(RefCell::new(HashMap::new())),
            shaders: Rc::new(RefCell::new(HashMap::new())),
            menus: Rc::new(RefCell::new(HashMap::new())),
            #[cfg(target_os = "macos")]
            swift_ui_hosts: Rc::new(RefCell::new(HashMap::new())),
            #[cfg(target_os = "macos")]
            embedded_views: Rc::clone(&self.embedded_views),
            handle: None,
        };
        let runner = self
            .runner
            .as_mut()
            .ok_or_else(|| "a system popover requires a running application".to_owned())?;
        let handle = runner
            .open_system_popover(
                parent_handle,
                ElementId::new(anchor as u64),
                window.config.clone(),
                window.view(id, &self.events, &self.handles),
            )
            .map_err(|error| error.to_string())?;
        self.handles.borrow_mut().insert(handle, id);
        window.handle = Some(handle);
        self.windows.insert(id, window);
        self.window_order.push(id);
        Ok(id)
    }

    #[cfg(target_os = "macos")]
    pub(super) fn create_embedded_view(
        &mut self,
        parent: u32,
        match_horizontal: bool,
        match_vertical: bool,
        options: NativeWindowOptions,
        initial_batch: &[u8],
    ) -> std::result::Result<u32, String> {
        let id = self.allocate_window_id()?;
        self.create_embedded_view_with_id(
            id,
            parent,
            match_horizontal,
            match_vertical,
            options,
            initial_batch,
        )
    }

    #[cfg(target_os = "macos")]
    pub(super) fn create_embedded_view_with_id(
        &mut self,
        id: u32,
        parent: u32,
        match_horizontal: bool,
        match_vertical: bool,
        options: NativeWindowOptions,
        initial_batch: &[u8],
    ) -> std::result::Result<u32, String> {
        self.sync_closed_windows();
        self.claim_window_id(id)?;
        let parent_handle = self
            .windows
            .get(&parent)
            .ok_or_else(|| format!("unknown QuickGUI parent window {parent}"))?
            .handle
            .ok_or_else(|| "an embedded view requires a running parent window".to_owned())?;
        let config = window_config(&options)?;
        let mut window = NativeWindowRuntime {
            config,
            tree: Rc::new(RefCell::new(native_tree_from_initial_batch(initial_batch)?)),
            markdown: Rc::new(RefCell::new(HashMap::new())),
            documents: Rc::new(RefCell::new(HashMap::new())),
            svgs: Rc::new(RefCell::new(HashMap::new())),
            lists: Rc::new(RefCell::new(HashMap::new())),
            terminals: Rc::new(RefCell::new(HashMap::new())),
            images: Rc::new(RefCell::new(HashMap::new())),
            background_images: Rc::new(RefCell::new(HashMap::new())),
            shaders: Rc::new(RefCell::new(HashMap::new())),
            menus: Rc::new(RefCell::new(HashMap::new())),
            swift_ui_hosts: Rc::new(RefCell::new(HashMap::new())),
            embedded_views: Rc::clone(&self.embedded_views),
            handle: None,
        };
        let runner = self
            .runner
            .as_mut()
            .ok_or_else(|| "an embedded view requires a running application".to_owned())?;
        let embedded = runner
            .open_embedded_view(
                parent_handle,
                window.config.clone(),
                match_horizontal,
                match_vertical,
                window.view(id, &self.events, &self.handles),
            )
            .map_err(|error| error.to_string())?;
        let handle = embedded.window_handle();
        self.handles.borrow_mut().insert(handle, id);
        self.embedded_views.borrow_mut().insert(id, embedded);
        window.handle = Some(handle);
        self.windows.insert(id, window);
        self.window_order.push(id);
        Ok(id)
    }

    fn allocate_window_id(&mut self) -> std::result::Result<u32, String> {
        let id = self.next_window_id.max(1);
        self.next_window_id = id
            .checked_add(1)
            .ok_or_else(|| "QuickGUI window id space exhausted".to_owned())?;
        Ok(id)
    }

    fn claim_window_id(&mut self, id: u32) -> std::result::Result<(), String> {
        if id == 0 {
            return Err("QuickGUI window ids must be nonzero".to_owned());
        }
        if self.windows.len() >= MAX_WINDOWS {
            return Err(format!(
                "an application cannot own more than {MAX_WINDOWS} windows"
            ));
        }
        if self.windows.contains_key(&id) {
            return Err(format!("QuickGUI window id {id} is already in use"));
        }
        self.next_window_id = self.next_window_id.max(
            id.checked_add(1)
                .ok_or_else(|| "QuickGUI window id space exhausted".to_owned())?,
        );
        Ok(())
    }

    pub(super) fn prepare(&mut self) -> std::result::Result<(), String> {
        if self.runner.is_some() {
            return self.finish_preparing();
        }
        let staged = self
            .window_order
            .iter()
            .filter_map(|id| {
                self.windows.get(id).map(|window| {
                    (
                        *id,
                        window.config.clone(),
                        window.view(*id, &self.events, &self.handles),
                    )
                })
            })
            .collect::<Vec<_>>();
        let handles = Rc::clone(&self.handles);
        let callback_handles = Rc::clone(&self.handles);
        let callback_events = Rc::clone(&self.events);
        let open_url_events = Rc::clone(&self.events);
        let reopen_events = Rc::clone(&self.events);
        let wake_events = Rc::clone(&self.events);
        let did_become_active_events = Rc::clone(&self.events);
        let did_resign_active_events = Rc::clone(&self.events);
        let keyboard_events = Rc::clone(&self.events);
        let notification_events = Rc::clone(&self.events);
        let global_shortcut_events = Rc::clone(&self.events);
        let second_instance_events = Rc::clone(&self.events);
        let power_events = Rc::clone(&self.events);
        let tray_events = Rc::clone(&self.events);
        let before_quit_events = Rc::clone(&self.events);
        let will_quit_events = Rc::clone(&self.events);
        let closed_windows = Rc::clone(&self.closed_windows);
        let menu_events = Rc::clone(&self.events);
        let mut application = QuickGuiApplication::new().quit_mode(self.quit_mode);
        application = application.on_action(move |action: &NativeMenuAction, _cx| {
            enqueue_event(
                &menu_events,
                QueuedEvent {
                    kind: "menu-action",
                    window: 0,
                    target: action.0,
                    value: None,
                },
            );
        });
        if let Some(info) = self.app_info.clone() {
            application = application.app_info(info);
        }
        if let Some(paths) = self.app_paths.clone() {
            application = application.app_paths(paths);
        }
        application = application.fonts(self.fonts.iter().cloned());
        // Declared menus adopt the core's own contextual navigation, typeahead, activation, and
        // dismissal bindings instead of a JavaScript keyboard implementation.
        application = application.bind_keys(quickgui::popover_menu_key_bindings());
        // A declared `Menu.Root` may be horizontal, which installs the core's own horizontal menu
        // key context instead; both sets are bound so either orientation navigates natively.
        application = application.bind_keys(quickgui::popover_menu_horizontal_key_bindings());
        // Declared range, ordering, and roving-focus components likewise adopt the core's typed
        // actions instead of a JavaScript keyboard implementation.
        application = application.bind_keys(quickgui::slider_key_bindings());
        application = application.bind_keys(quickgui::splitter_key_bindings());
        application = application.bind_keys(quickgui::toolbar_key_bindings());
        application = application.bind_keys(quickgui::toggle_group_key_bindings());
        // Declared pickers, collections, date and time fields, month grids, and menubars adopt
        // the core's own typed navigation, filtering, and commit bindings the same way.
        application = application.bind_keys(quickgui::picker_key_bindings());
        application = application.bind_keys(quickgui::select_key_bindings());
        application = application.bind_keys(quickgui::combobox_key_bindings());
        application = application.bind_keys(quickgui::table_key_bindings());
        application = application.bind_keys(quickgui::tree_key_bindings());
        application = application.bind_keys(quickgui::date_field_key_bindings());
        application = application.bind_keys(quickgui::time_field_key_bindings());
        application = application.bind_keys(quickgui::calendar_key_bindings());
        application = application.bind_keys(quickgui::menubar_key_bindings());
        // Base UI OTP fields and navigation menus adopt the core's own contextual editing and
        // roving-focus bindings rather than a JavaScript keyboard implementation.
        application = application.bind_keys(quickgui::otp_field_key_bindings());
        application = application.bind_keys(quickgui::navigation_menu_key_bindings());
        let mut runner = application
            .on_open_urls(move |urls, _cx| {
                let value = serde_json::to_string(&urls.iter().collect::<Vec<_>>())
                    .ok()
                    .map(Arc::<str>::from);
                enqueue_event(
                    &open_url_events,
                    QueuedEvent {
                        kind: "open-urls",
                        window: 0,
                        target: ROOT_NODE,
                        value,
                    },
                );
            })
            .on_reopen(move |has_visible_windows, _cx| {
                enqueue_event(
                    &reopen_events,
                    QueuedEvent {
                        kind: "reopen",
                        window: 0,
                        target: ROOT_NODE,
                        value: Some(Arc::from(if has_visible_windows {
                            "true"
                        } else {
                            "false"
                        })),
                    },
                );
            })
            .on_did_become_active(move |_cx| {
                enqueue_event(
                    &did_become_active_events,
                    QueuedEvent {
                        kind: "app-activate",
                        window: 0,
                        target: ROOT_NODE,
                        value: None,
                    },
                );
            })
            .on_did_resign_active(move |_cx| {
                enqueue_event(
                    &did_resign_active_events,
                    QueuedEvent {
                        kind: "app-deactivate",
                        window: 0,
                        target: ROOT_NODE,
                        value: None,
                    },
                );
            })
            .on_system_wake(move |_cx| {
                enqueue_event(
                    &wake_events,
                    QueuedEvent {
                        kind: "system-wake",
                        window: 0,
                        target: ROOT_NODE,
                        value: None,
                    },
                );
            })
            .on_keyboard_layout_change(move |_layout, _cx| {
                enqueue_event(
                    &keyboard_events,
                    QueuedEvent {
                        kind: "keyboard-layout-change",
                        window: 0,
                        target: ROOT_NODE,
                        value: None,
                    },
                );
            })
            .on_system_notification_response(move |response, _cx| {
                let value = serde_json::json!({
                    "tag": response.tag.as_ref(),
                    "actionId": response.action_id.as_deref(),
                    "reply": response.reply.as_deref(),
                })
                .to_string();
                enqueue_event(
                    &notification_events,
                    QueuedEvent {
                        kind: "notification-response",
                        window: 0,
                        target: ROOT_NODE,
                        value: Some(Arc::from(value)),
                    },
                );
            })
            .on_global_shortcut(move |shortcut, _cx| {
                enqueue_event(
                    &global_shortcut_events,
                    QueuedEvent {
                        kind: "global-shortcut",
                        window: 0,
                        target: shortcut.registration_id,
                        value: None,
                    },
                );
            })
            .on_second_instance(move |instance, _cx| {
                let value = serde_json::json!({
                    "argv": instance
                        .argv()
                        .iter()
                        .map(AsRef::as_ref)
                        .collect::<Vec<&str>>(),
                    "cwd": instance.cwd().to_string_lossy(),
                })
                .to_string();
                enqueue_event(
                    &second_instance_events,
                    QueuedEvent {
                        kind: "second-instance",
                        window: 0,
                        target: ROOT_NODE,
                        value: Some(Arc::from(value)),
                    },
                );
            })
            .on_power_event(move |event, _cx| {
                let value = match event {
                    quickgui::PowerEvent::Suspend => serde_json::json!({ "type": "suspend" }),
                    quickgui::PowerEvent::Resume => serde_json::json!({ "type": "resume" }),
                    quickgui::PowerEvent::LockScreen => {
                        serde_json::json!({ "type": "lock-screen" })
                    }
                    quickgui::PowerEvent::UnlockScreen => {
                        serde_json::json!({ "type": "unlock-screen" })
                    }
                    quickgui::PowerEvent::ShutdownRequested => {
                        serde_json::json!({ "type": "shutdown-requested" })
                    }
                    quickgui::PowerEvent::PowerSourceChanged(source) => serde_json::json!({
                        "type": "power-source-changed",
                        "source": match source {
                            quickgui::PowerSource::Ac => "ac",
                            quickgui::PowerSource::Battery => "battery",
                            quickgui::PowerSource::Unknown => "unknown",
                        },
                    }),
                    quickgui::PowerEvent::ThermalStateChanged(state) => serde_json::json!({
                        "type": "thermal-state-changed",
                        "state": match state {
                            quickgui::ThermalState::Unknown => "unknown",
                            quickgui::ThermalState::Nominal => "nominal",
                            quickgui::ThermalState::Fair => "fair",
                            quickgui::ThermalState::Serious => "serious",
                            quickgui::ThermalState::Critical => "critical",
                        },
                    }),
                    quickgui::PowerEvent::LowPowerModeChanged(enabled) => serde_json::json!({
                        "type": "low-power-mode-changed",
                        "enabled": enabled,
                    }),
                    quickgui::PowerEvent::CpuSpeedLimitChanged(percent) => serde_json::json!({
                        "type": "cpu-speed-limit-changed",
                        "percent": percent,
                    }),
                }
                .to_string();
                enqueue_event(
                    &power_events,
                    QueuedEvent {
                        kind: "power-event",
                        window: 0,
                        target: ROOT_NODE,
                        value: Some(Arc::from(value)),
                    },
                );
            })
            .on_tray_event(move |event, _cx| {
                let kind = match event.kind {
                    quickgui::TrayEventKind::Click => "click",
                    quickgui::TrayEventKind::DoubleClick => "double-click",
                    quickgui::TrayEventKind::Enter => "enter",
                    quickgui::TrayEventKind::Move => "move",
                    quickgui::TrayEventKind::Leave => "leave",
                    quickgui::TrayEventKind::MenuItem => "menu-item",
                    quickgui::TrayEventKind::Scroll => "scroll",
                };
                let button = event.button.map(|button| match button {
                    quickgui::TrayMouseButton::Left => "left",
                    quickgui::TrayMouseButton::Right => "right",
                    quickgui::TrayMouseButton::Middle => "middle",
                });
                let value = serde_json::json!({
                    "kind": kind,
                    "menuItemId": event.menu_item_id,
                    "button": button,
                    "position": event.position.map(|(x, y)| serde_json::json!({ "x": x, "y": y })),
                    "pressed": event.pressed,
                    "scrollDelta": event.scroll_delta,
                    "horizontal": event.horizontal,
                })
                .to_string();
                enqueue_event(
                    &tray_events,
                    QueuedEvent {
                        kind: "tray-event",
                        window: 0,
                        target: event.tray_id,
                        value: Some(Arc::from(value)),
                    },
                );
            })
            .on_before_quit(move |request, cx| {
                // A JavaScript listener can never veto synchronously, so the declared
                // interception flag prevents the quit and the decision returns as an explicit
                // `quit`/`exit` command once listeners have run.
                if intercepts_quit() {
                    cx.prevent_quit();
                }
                enqueue_event(
                    &before_quit_events,
                    QueuedEvent {
                        kind: "before-quit",
                        window: 0,
                        target: ROOT_NODE,
                        value: Some(Arc::from(quit_reason_name(request.reason))),
                    },
                );
            })
            .on_will_quit(move |request, _cx| {
                enqueue_event(
                    &will_quit_events,
                    QueuedEvent {
                        kind: "will-quit",
                        window: 0,
                        target: ROOT_NODE,
                        value: Some(Arc::from(quit_reason_name(request.reason))),
                    },
                );
            })
            .on_window_closed(move |handle, _cx| {
                let Some(window) = callback_handles.borrow_mut().remove(&handle) else {
                    return;
                };
                enqueue_event(
                    &callback_events,
                    QueuedEvent {
                        kind: "close",
                        window,
                        target: ROOT_NODE,
                        value: None,
                    },
                );
                let mut closed = closed_windows.borrow_mut();
                if closed.len() < MAX_WINDOWS {
                    closed.push(window);
                }
            })
            .into_runner()
            .map_err(|error| error.to_string())?;
        let mut mounted = Vec::with_capacity(self.windows.len());
        for (id, config, view) in staged {
            let handle = match runner.open_window(config, view) {
                Ok(handle) => handle,
                Err(error) => {
                    handles.borrow_mut().clear();
                    return Err(error.to_string());
                }
            };
            handles.borrow_mut().insert(handle, id);
            mounted.push((id, handle));
        }
        for (id, handle) in mounted {
            if let Some(window) = self.windows.get_mut(&id) {
                window.handle = Some(handle);
            }
        }
        self.runner = Some(runner);
        self.finish_preparing()
    }

    pub(super) fn finish_preparing(&mut self) -> std::result::Result<(), String> {
        let runner = self
            .runner
            .as_mut()
            .ok_or_else(|| "the QuickGUI application runner is unavailable".to_owned())?;
        for _ in 0..8 {
            if runner.is_ready() {
                return Ok(());
            }
            match runner
                .pump(Some(Duration::ZERO))
                .map_err(|error| error.to_string())?
            {
                AppRunStatus::Continue => {}
                AppRunStatus::Exited(code) => {
                    return Err(format!(
                        "the QuickGUI application exited with code {code} before becoming ready"
                    ));
                }
            }
        }
        Err("the native QuickGUI application did not become ready".to_owned())
    }

    pub(super) fn is_ready(&self) -> bool {
        self.runner.as_ref().is_some_and(AppRunner::is_ready)
    }

    pub(super) fn close_window(&mut self, window: u32) -> bool {
        self.sync_closed_windows();
        set_close_interception(window, false);
        let Some(handle) = self.windows.get(&window).and_then(|window| window.handle) else {
            if self.windows.remove(&window).is_none() {
                return false;
            }
            #[cfg(target_os = "macos")]
            self.embedded_views.borrow_mut().remove(&window);
            self.window_order.retain(|id| *id != window);
            enqueue_event(
                &self.events,
                QueuedEvent {
                    kind: "close",
                    window,
                    target: ROOT_NODE,
                    value: None,
                },
            );
            return true;
        };
        let Some(runner) = &mut self.runner else {
            return false;
        };
        if !runner.close_window(handle) {
            return false;
        }
        self.handles.borrow_mut().remove(&handle);
        self.windows.remove(&window);
        #[cfg(target_os = "macos")]
        self.embedded_views.borrow_mut().remove(&window);
        self.window_order.retain(|id| *id != window);
        enqueue_event(
            &self.events,
            QueuedEvent {
                kind: "close",
                window,
                target: ROOT_NODE,
                value: None,
            },
        );
        true
    }

    pub(super) fn apply_batch(
        &mut self,
        window: u32,
        batch: &[u8],
    ) -> std::result::Result<u32, String> {
        let mutations = decode_batch(batch).map_err(|error| error.to_string())?;
        self.sync_closed_windows();
        let native_window = self
            .windows
            .get(&window)
            .ok_or_else(|| format!("unknown QuickGUI window {window}"))?;
        let mut tree = native_window.tree.borrow_mut();
        let previous_revision = tree.revision;
        let retained_updates = retained_element_updates(&tree, &mutations);
        let retained_scopes = retained_updates
            .is_none()
            .then(|| retained_scope_updates(&tree, &mutations))
            .flatten();
        let revision = apply_mutations(&mut tree, mutations).map_err(|error| error.to_string())?;
        drop(tree);
        if revision != previous_revision
            && let (Some(runner), Some(handle)) = (&mut self.runner, native_window.handle)
        {
            let updated = match retained_updates {
                Some(updates) => runner
                    .update_elements(handle, &updates)
                    .map_err(|error| error.to_string())?,
                None => false,
            };
            if !updated {
                if let Some(scopes) = retained_scopes {
                    runner.invalidate_elements(handle, &scopes);
                } else {
                    runner.invalidate_window(handle);
                }
            }
        }
        Ok(revision)
    }

    pub(super) fn focus_node(
        &mut self,
        window: u32,
        node: u32,
    ) -> std::result::Result<bool, String> {
        self.sync_closed_windows();
        let native_window = self
            .windows
            .get(&window)
            .ok_or_else(|| format!("unknown QuickGUI window {window}"))?;
        if !native_window.tree.borrow().nodes.contains_key(&node) {
            return Ok(false);
        }
        let Some(handle) = native_window.handle else {
            return Ok(false);
        };
        let Some(runner) = &mut self.runner else {
            return Ok(false);
        };
        Ok(runner.focus_element(handle, ElementId::new(node as u64)))
    }

    pub(super) fn show_alert_dialog(
        &mut self,
        window: Option<u32>,
        request: u32,
        options: NativeDialogOptions,
    ) -> std::result::Result<(), String> {
        let handle = self.dialog_handle(window, request)?;
        let (level, buttons) = native_dialog_configuration(&options)?;
        let runner = self
            .runner
            .as_mut()
            .ok_or_else(|| "a native dialog requires a running application".to_owned())?;
        let response = match handle {
            Some(handle) => runner.prompt(
                handle,
                level,
                options.message,
                options.detail.as_deref(),
                &buttons,
            ),
            None => runner.prompt_application(
                level,
                options.message,
                options.detail.as_deref(),
                &buttons,
            ),
        }
        .map_err(|error| error.to_string())?;
        self.pending_dialogs
            .push(PendingDialog::alert(window, request, response));
        Ok(())
    }

    pub(super) fn show_open_dialog(
        &mut self,
        window: Option<u32>,
        request: u32,
        options: NativeOpenDialogOptions,
    ) -> std::result::Result<(), String> {
        let handle = self.dialog_handle(window, request)?;
        let native_options = native_open_dialog_options(options);
        let runner = self
            .runner
            .as_mut()
            .ok_or_else(|| "a native file dialog requires a running application".to_owned())?;
        let response = match handle {
            Some(handle) => runner.prompt_for_paths(handle, native_options),
            None => runner.prompt_for_paths_application(native_options),
        }
        .map_err(|error| error.to_string())?;
        self.pending_dialogs
            .push(PendingDialog::open(window, request, response));
        Ok(())
    }

    pub(super) fn show_save_dialog(
        &mut self,
        window: Option<u32>,
        request: u32,
        options: NativeSaveDialogOptions,
    ) -> std::result::Result<(), String> {
        let handle = self.dialog_handle(window, request)?;
        let native_options = native_save_dialog_options(options);
        let runner = self
            .runner
            .as_mut()
            .ok_or_else(|| "a native file dialog requires a running application".to_owned())?;
        let response = match handle {
            Some(handle) => runner.prompt_for_new_path(handle, native_options),
            None => runner.prompt_for_new_path_application(native_options),
        }
        .map_err(|error| error.to_string())?;
        self.pending_dialogs
            .push(PendingDialog::save(window, request, response));
        Ok(())
    }

    pub(super) fn dialog_handle(
        &mut self,
        window: Option<u32>,
        request: u32,
    ) -> std::result::Result<Option<WindowHandle>, String> {
        self.sync_closed_windows();
        if request == 0
            || self
                .pending_dialogs
                .iter()
                .any(|dialog| dialog.request() == request)
        {
            return Err("native dialog request ids must be nonzero and unique".to_owned());
        }
        window
            .map(|window| {
                self.windows
                    .get(&window)
                    .ok_or_else(|| format!("unknown QuickGUI window {window}"))?
                    .handle
                    .map(Some)
                    .ok_or_else(|| "a native dialog requires a running window".to_owned())
            })
            .unwrap_or(Ok(None))
    }

    pub(super) fn sync_closed_windows(&mut self) {
        let closed = std::mem::take(&mut *self.closed_windows.borrow_mut());
        if closed.is_empty() {
            return;
        }
        for id in &closed {
            self.windows.remove(id);
            crate::runtime::forget_window_policies(*id);
            #[cfg(target_os = "macos")]
            self.embedded_views.borrow_mut().remove(id);
        }
        self.window_order.retain(|id| !closed.contains(id));
    }

    pub(super) fn drain_events(&mut self) -> Vec<NativeEvent> {
        self.observe_system_state();
        let mut events = Vec::with_capacity(
            self.events.borrow().len()
                + self.pending_dialogs.len()
                + self.pending_shell.len()
                + self.pending_notification_permissions.len()
                + self.pending_file_icons.len()
                + self.pending_user_tasks.len()
                + self.pending_global_shortcuts.len()
                + self.pending_tray.len()
                + self.pending_app_services.len(),
        );
        // Queued events record what already happened; a polled completion only reports that a
        // request has finished. They must drain in that order: the core dispatches a chosen popup
        // item and completes the popup in the same turn, and JavaScript releases the item
        // callbacks on the `popup-menu` completion, so a completion emitted ahead of its
        // `menu-action` would lose the click.
        events.extend(self.events.borrow_mut().drain(..).map(|event| NativeEvent {
            kind: event.kind.to_owned(),
            window: event.window,
            target: event.target,
            value: event.value.map(|value| value.to_string()),
            paths: None,
            data: None,
            width: None,
            height: None,
            error: None,
        }));
        let waker = Waker::noop();
        let mut context = Context::from_waker(waker);
        let mut still_pending = Vec::with_capacity(self.pending_dialogs.len());
        for mut dialog in std::mem::take(&mut self.pending_dialogs) {
            match dialog.poll(&mut context) {
                Poll::Ready(event) => events.push(event),
                Poll::Pending => still_pending.push(dialog),
            }
        }
        self.pending_dialogs = still_pending;
        let mut still_pending = Vec::with_capacity(self.pending_shell.len());
        for mut request in std::mem::take(&mut self.pending_shell) {
            match request.poll(&mut context) {
                Poll::Ready(event) => events.push(event),
                Poll::Pending => still_pending.push(request),
            }
        }
        self.pending_shell = still_pending;
        let mut still_pending = Vec::with_capacity(self.pending_notification_permissions.len());
        for mut request in std::mem::take(&mut self.pending_notification_permissions) {
            match request.poll(&mut context) {
                Poll::Ready(event) => events.push(event),
                Poll::Pending => still_pending.push(request),
            }
        }
        self.pending_notification_permissions = still_pending;
        let mut still_pending = Vec::with_capacity(self.pending_file_icons.len());
        for mut request in std::mem::take(&mut self.pending_file_icons) {
            match request.poll(&mut context) {
                Poll::Ready(event) => events.push(event),
                Poll::Pending => still_pending.push(request),
            }
        }
        self.pending_file_icons = still_pending;
        let mut still_pending = Vec::with_capacity(self.pending_user_tasks.len());
        for mut request in std::mem::take(&mut self.pending_user_tasks) {
            match request.poll(&mut context) {
                Poll::Ready(event) => events.push(event),
                Poll::Pending => still_pending.push(request),
            }
        }
        self.pending_user_tasks = still_pending;
        let mut still_pending = Vec::with_capacity(self.pending_global_shortcuts.len());
        for mut request in std::mem::take(&mut self.pending_global_shortcuts) {
            match request.poll(&mut context) {
                Poll::Ready(event) => events.push(event),
                Poll::Pending => still_pending.push(request),
            }
        }
        self.pending_global_shortcuts = still_pending;
        let mut still_pending = Vec::with_capacity(self.pending_tray.len());
        for mut request in std::mem::take(&mut self.pending_tray) {
            match request.poll(&mut context) {
                Poll::Ready(event) => events.push(event),
                Poll::Pending => still_pending.push(request),
            }
        }
        self.pending_tray = still_pending;
        let mut still_pending = Vec::with_capacity(self.pending_app_services.len());
        for mut request in std::mem::take(&mut self.pending_app_services) {
            match request.poll(&mut context) {
                Poll::Ready(event) => events.push(event),
                Poll::Pending => still_pending.push(request),
            }
        }
        self.pending_app_services = still_pending;
        events
    }
}

pub(crate) fn native_font_data(options: &NativeAppOptions) -> Option<Vec<Arc<[u8]>>> {
    let fonts = options.fonts.as_ref()?;
    Some(
        fonts
            .iter()
            .filter_map(|path| {
                // Packaged applications declare fonts relative to their resources,
                // independent of the working directory used to launch the app.
                // Path::join preserves explicitly absolute font paths.
                let path =
                    std::path::Path::new(options.resource_dir.as_deref().unwrap_or(".")).join(path);
                match std::fs::read(&path) {
                    Ok(bytes) => Some(Arc::<[u8]>::from(bytes)),
                    Err(error) => {
                        eprintln!("quickgui: could not read font {}: {error}", path.display());
                        None
                    }
                }
            })
            .collect(),
    )
}

pub(crate) fn native_app_configuration(
    options: NativeAppOptions,
) -> std::result::Result<(Option<AppInfo>, Option<AppPaths>, QuitMode), String> {
    update_native_app_configuration(None, None, QuitMode::default(), options)
}

pub(crate) fn update_native_app_configuration(
    current_info: Option<AppInfo>,
    current_paths: Option<AppPaths>,
    current_quit_mode: QuitMode,
    options: NativeAppOptions,
) -> std::result::Result<(Option<AppInfo>, Option<AppPaths>, QuitMode), String> {
    let NativeAppOptions {
        name,
        version,
        identifier,
        resource_dir,
        config_dir,
        data_dir,
        local_data_dir,
        cache_dir,
        log_dir,
        runtime_dir,
        temp_dir,
        quit_mode,
        fonts: _,
    } = options;
    let identity_replaced = name.is_some() || version.is_some() || identifier.is_some();
    let info = match (name, version, identifier) {
        (None, None, None) => None,
        (Some(name), Some(version), Some(identifier)) => {
            Some(AppInfo::new(name, version, identifier).map_err(|error| error.to_string())?)
        }
        _ => {
            return Err(
                "application name, version, and identifier must be supplied together".to_owned(),
            );
        }
    }
    .or(current_info);
    let has_path_overrides = resource_dir.is_some()
        || config_dir.is_some()
        || data_dir.is_some()
        || local_data_dir.is_some()
        || cache_dir.is_some()
        || log_dir.is_some()
        || runtime_dir.is_some()
        || temp_dir.is_some();
    let mut paths = if identity_replaced {
        info.as_ref()
            .map(AppInfo::paths)
            .transpose()
            .map_err(|error| error.to_string())?
    } else if let Some(paths) = current_paths {
        Some(paths)
    } else {
        info.as_ref()
            .map(AppInfo::paths)
            .transpose()
            .map_err(|error| error.to_string())?
    };
    if has_path_overrides && paths.is_none() {
        return Err("application path overrides require application identity".to_owned());
    }
    if let Some(mut configured) = paths.take() {
        if let Some(path) = resource_dir {
            configured = configured.with_resource_dir(path);
        }
        if let Some(path) = config_dir {
            configured = configured.with_config_dir(Some(path));
        }
        if let Some(path) = data_dir {
            configured = configured.with_data_dir(Some(path));
        }
        if let Some(path) = local_data_dir {
            configured = configured.with_local_data_dir(Some(path));
        }
        if let Some(path) = cache_dir {
            configured = configured.with_cache_dir(Some(path));
        }
        if let Some(path) = log_dir {
            configured = configured.with_log_dir(Some(path));
        }
        if let Some(path) = runtime_dir {
            configured = configured.with_runtime_dir(Some(path));
        }
        if let Some(path) = temp_dir {
            configured = configured.with_temp_dir(path);
        }
        paths = Some(configured);
    }
    let quit_mode = match quit_mode.as_deref() {
        None => current_quit_mode,
        Some("default") => QuitMode::Default,
        Some("last-window-closed" | "lastWindowClosed") => QuitMode::LastWindowClosed,
        Some("explicit") => QuitMode::Explicit,
        Some(value) => return Err(format!("unknown application quit mode `{value}`")),
    };
    Ok((info, paths, quit_mode))
}

pub(super) fn window_config(
    options: &NativeWindowOptions,
) -> std::result::Result<WindowOptions, String> {
    let width = optional_finite(options.width, "window width")?.unwrap_or(960.0);
    let height = optional_finite(options.height, "window height")?.unwrap_or(640.0);
    let title_bar_style = match options.title_bar_style.as_deref() {
        Some("hiddenInset") | Some("hidden-inset") => TitleBarStyle::HiddenInset,
        Some("hidden") => TitleBarStyle::Hidden,
        Some("default") | None => TitleBarStyle::Default,
        Some(value) => return Err(format!("unknown titleBarStyle `{value}`")),
    };
    let background = options
        .background
        .map(unpack_color)
        .unwrap_or_else(|| Color::rgb8(18, 18, 20));
    let background_appearance = if options.blur.unwrap_or(false) {
        WindowBackgroundAppearance::Blurred
    } else if options.transparent.unwrap_or(false) {
        WindowBackgroundAppearance::Transparent
    } else {
        WindowBackgroundAppearance::Opaque
    };
    let mut config = WindowOptions::new(
        options
            .title
            .clone()
            .unwrap_or_else(|| "QuickGUI".to_owned()),
    )
    .size(width, height)
    .background(background)
    .window_background(background_appearance)
    .title_bar_style(title_bar_style);

    if let Some((x, y)) = optional_pair(options.x, options.y, "window position")? {
        config = config.position(x, y);
    }
    if let Some(display) = &options.display_id {
        let id = display
            .parse::<u64>()
            .map_err(|_| "displayId must be an unsigned 64-bit integer string".to_owned())?;
        config = config.display(DisplayId::new(id));
    }
    match options.initial_state.as_deref().unwrap_or("normal") {
        "normal" | "windowed" => {}
        "maximized" => config = config.maximized(true),
        "fullscreen" => config = config.fullscreen(true),
        state => return Err(format!("unknown initial window state `{state}`")),
    }

    if options.minimum_size_enabled == Some(false) {
        if options.minimum_width.is_some() || options.minimum_height.is_some() {
            return Err("minimumWidth/minimumHeight cannot accompany minimumSize: null".to_owned());
        }
        config = config.without_minimum_size();
    } else if options.minimum_width.is_some() || options.minimum_height.is_some() {
        let (width, height) = required_pair(
            options.minimum_width,
            options.minimum_height,
            "minimum window size",
        )?;
        config = config.minimum_size(width, height);
    }
    if options.maximum_width.is_some() || options.maximum_height.is_some() {
        let (width, height) = required_pair(
            options.maximum_width,
            options.maximum_height,
            "maximum window size",
        )?;
        config = config.maximum_size(width, height);
    }

    if let Some(path) = &options.represented_file {
        config = config.represented_file(PathBuf::from(path));
    }
    if let Some(edited) = options.document_edited {
        config = config.document_edited(edited);
    }
    if let Some(identifier) = &options.tabbing_identifier {
        config = config.tabbing_identifier(identifier.clone());
    }
    if let Some(profile) = options.performance_profile.as_deref() {
        config = config.performance_profile(match profile {
            "low-power" | "lowPower" => PerformanceProfile::LowPower,
            "balanced" => PerformanceProfile::Balanced,
            "performance" | "high-performance" | "highPerformance" => {
                PerformanceProfile::HighPerformance
            }
            value => return Err(format!("unknown performanceProfile `{value}`")),
        });
    }
    if let Some(appearance) = options.appearance.as_deref() {
        config = match appearance {
            "system" => config.follow_system_appearance(),
            "light" => config.window_appearance(WindowAppearance::Light),
            "dark" => config.window_appearance(WindowAppearance::Dark),
            value => return Err(format!("unknown window appearance `{value}`")),
        };
    }
    if let Some(vibrancy) = options.vibrancy.as_deref() {
        config = config.macos_vibrancy(parse_macos_vibrancy(vibrancy)?);
    }
    if let Some(state) = options.visual_effect_state.as_deref() {
        config = config.macos_visual_effect_state(parse_macos_visual_effect_state(state)?);
    }
    if let Some(kind) = options.kind.as_deref() {
        config = config.window_kind(match kind {
            "normal" => WindowKind::Normal,
            "popover" => WindowKind::Popover,
            "floating" => WindowKind::Floating,
            "dialog" => WindowKind::Dialog,
            "system-popover" | "systemPopover" => WindowKind::SystemPopover,
            value => return Err(format!("unknown window kind `{value}`")),
        });
    }
    if let Some(value) = options.focus {
        config = config.focus(value);
    }
    if let Some(value) = options.focusable {
        config = config.focusable(value);
    }
    if let Some(value) = options.show {
        config = config.show(value);
    }
    if let Some(value) = options.movable {
        config = config.movable(value);
    }
    if let Some(value) = options.resizable {
        config = config.resizable(value);
    }
    if let Some(value) = options.minimizable {
        config = config.minimizable(value);
    }
    if let Some(value) = options.maximizable {
        config = config.maximizable(value);
    }
    if let Some(value) = options.closable {
        config = config.closable(value);
    }
    if let Some(value) = options.decorated {
        config = config.decorations(value);
    }
    if let Some(value) = options.shadow {
        config = config.shadow(value);
    }
    if let Some(value) = options.content_protected {
        config = config.content_protected(value);
    }
    if let Some(level) = options.window_level.as_deref() {
        config = if level == "automatic" {
            config.automatic_window_level()
        } else {
            config.window_level(crate::system::parse_window_level(level)?)
        };
    }
    if let Some(value) = options.skip_taskbar {
        config = config.skip_taskbar(value);
    }
    if let Some(value) = options.visible_on_all_workspaces {
        config = config.visible_on_all_workspaces(value);
    }
    if let Some(opacity) = optional_finite(options.opacity, "window opacity")? {
        config = config.opacity(opacity);
    }
    if let Some(icon) = options.icon.clone() {
        config = config.icon(native_image(icon)?);
    }
    if options.taskbar_progress_state.is_some() || options.taskbar_progress.is_some() {
        let state = parse_taskbar_progress_state(
            options
                .taskbar_progress_state
                .as_deref()
                .unwrap_or("normal"),
        )?;
        let progress =
            optional_finite(options.taskbar_progress, "taskbar progress")?.unwrap_or(0.0);
        config = config.taskbar_progress(state, progress);
    }
    match (
        options.taskbar_overlay_icon.clone(),
        options.taskbar_overlay_description.as_deref(),
    ) {
        (Some(icon), Some(description)) => {
            config = config.taskbar_overlay_icon(native_image(icon)?, description);
        }
        (None, None) => {}
        _ => {
            return Err(
                "taskbarOverlayIcon and taskbarOverlayDescription must be supplied together"
                    .to_owned(),
            );
        }
    }
    if let Some(value) = options.cursor_visible {
        config = config.cursor_visible(value);
    }
    if let Some(mode) = options.cursor_grab.as_deref() {
        config = config.cursor_grab(parse_cursor_grab_mode(mode)?);
    }
    if let Some(value) = options.cursor_hit_test {
        config = config.cursor_hit_test(value);
    }
    if let Some((x, y)) = optional_pair(options.cursor_x, options.cursor_y, "cursor position")? {
        config = config.cursor_position(Point::new(x, y));
    }
    if let Some(menu) = &options.menu {
        config = config.window_menus(system::menu::application_menus(menu)?);
    }
    if let Some(value) = optional_finite(options.line_scroll_pixels, "line scroll pixels")? {
        config.line_scroll_pixels = value;
    }
    if let Some(milliseconds) =
        optional_finite(options.key_sequence_timeout_ms, "key sequence timeout")?
    {
        if milliseconds < 0.0 {
            return Err("keySequenceTimeoutMs cannot be negative".to_owned());
        }
        config.key_sequence_timeout = Duration::from_secs_f32(milliseconds / 1_000.0);
    }
    if let Some(value) = options.reduce_motion {
        config = config.reduce_motion(value);
    }

    if let Some((x, y)) = optional_pair(
        options.traffic_light_x,
        options.traffic_light_y,
        "traffic light position",
    )? {
        config = config.traffic_light_position(x, y);
    }
    Ok(config)
}

pub(crate) fn parse_macos_vibrancy(value: &str) -> std::result::Result<MacOsVibrancy, String> {
    match value {
        "appearance-based" | "appearanceBased" => Ok(MacOsVibrancy::AppearanceBased),
        "titlebar" => Ok(MacOsVibrancy::Titlebar),
        "selection" => Ok(MacOsVibrancy::Selection),
        "menu" => Ok(MacOsVibrancy::Menu),
        "popover" => Ok(MacOsVibrancy::Popover),
        "sidebar" => Ok(MacOsVibrancy::Sidebar),
        "header" => Ok(MacOsVibrancy::Header),
        "sheet" => Ok(MacOsVibrancy::Sheet),
        "window" => Ok(MacOsVibrancy::Window),
        "hud" => Ok(MacOsVibrancy::Hud),
        "fullscreen-ui" | "fullscreenUi" => Ok(MacOsVibrancy::FullscreenUi),
        "tooltip" => Ok(MacOsVibrancy::Tooltip),
        "content" => Ok(MacOsVibrancy::Content),
        "under-window" | "underWindow" => Ok(MacOsVibrancy::UnderWindow),
        "under-page" | "underPage" => Ok(MacOsVibrancy::UnderPage),
        value => Err(format!("unknown macOS vibrancy type `{value}`")),
    }
}

pub(crate) fn parse_macos_visual_effect_state(
    value: &str,
) -> std::result::Result<MacOsVisualEffectState, String> {
    match value {
        "followWindow" | "follow-window" => Ok(MacOsVisualEffectState::FollowWindow),
        "active" => Ok(MacOsVisualEffectState::Active),
        "inactive" => Ok(MacOsVisualEffectState::Inactive),
        value => Err(format!("unknown macOS visual effect state `{value}`")),
    }
}

pub(super) fn optional_finite(
    value: Option<f64>,
    name: &str,
) -> std::result::Result<Option<f32>, String> {
    value
        .map(|value| {
            let value = value as f32;
            if value.is_finite() {
                Ok(value)
            } else {
                Err(format!("{name} must be finite"))
            }
        })
        .transpose()
}

pub(super) fn required_pair(
    first: Option<f64>,
    second: Option<f64>,
    name: &str,
) -> std::result::Result<(f32, f32), String> {
    optional_pair(first, second, name)?.ok_or_else(|| format!("{name} requires both values"))
}

pub(super) fn optional_pair(
    first: Option<f64>,
    second: Option<f64>,
    name: &str,
) -> std::result::Result<Option<(f32, f32)>, String> {
    match (first, second) {
        (None, None) => Ok(None),
        (Some(first), Some(second)) => Ok(Some((
            optional_finite(Some(first), name)?.expect("present finite value"),
            optional_finite(Some(second), name)?.expect("present finite value"),
        ))),
        _ => Err(format!("{name} requires both values")),
    }
}

pub(super) fn parse_taskbar_progress_state(
    state: &str,
) -> std::result::Result<TaskbarProgressState, String> {
    match state {
        "none" => Ok(TaskbarProgressState::None),
        "normal" => Ok(TaskbarProgressState::Normal),
        "indeterminate" => Ok(TaskbarProgressState::Indeterminate),
        "paused" => Ok(TaskbarProgressState::Paused),
        "error" => Ok(TaskbarProgressState::Error),
        value => Err(format!("unknown taskbar progress state `{value}`")),
    }
}

pub(super) fn parse_cursor_grab_mode(mode: &str) -> std::result::Result<CursorGrabMode, String> {
    match mode {
        "none" => Ok(CursorGrabMode::None),
        "confined" => Ok(CursorGrabMode::Confined),
        "locked" => Ok(CursorGrabMode::Locked),
        value => Err(format!("unknown cursor grab mode `{value}`")),
    }
}

pub(crate) fn native_image(source: NativeImageSource) -> std::result::Result<Image, String> {
    match (source.data, source.path) {
        (Some(data), None) => match (source.width, source.height) {
            (Some(width), Some(height)) => Image::from_rgba(width, height, Arc::<[u8]>::from(data)),
            (None, None) => Image::decode(&data),
            _ => {
                return Err(
                    "raw image data requires both width and height, or neither for encoded data"
                        .to_owned(),
                );
            }
        },
        (None, Some(path)) if source.width.is_none() && source.height.is_none() => {
            Image::open(PathBuf::from(path))
        }
        (Some(_), Some(_)) => {
            return Err("an image accepts data or path, but not both".to_owned());
        }
        _ => return Err("an image requires data or path".to_owned()),
    }
    .map_err(|error| error.to_string())
}

pub(super) fn parse_anchor_placement(value: &str) -> Option<AnchorPlacement> {
    match value {
        "top-start" => Some(AnchorPlacement::TopStart),
        "top" => Some(AnchorPlacement::Top),
        "top-end" => Some(AnchorPlacement::TopEnd),
        "bottom-start" => Some(AnchorPlacement::BottomStart),
        "bottom" => Some(AnchorPlacement::Bottom),
        "bottom-end" => Some(AnchorPlacement::BottomEnd),
        "left-start" => Some(AnchorPlacement::LeftStart),
        "left" => Some(AnchorPlacement::Left),
        "left-end" => Some(AnchorPlacement::LeftEnd),
        "right-start" => Some(AnchorPlacement::RightStart),
        "right" => Some(AnchorPlacement::Right),
        "right-end" => Some(AnchorPlacement::RightEnd),
        _ => None,
    }
}

pub(super) fn system_popover_config(
    options: &NativeWindowOptions,
) -> std::result::Result<WindowOptions, String> {
    let placement = match options.popover_placement.as_deref() {
        Some(value) => parse_anchor_placement(value)
            .ok_or_else(|| format!("unknown popover placement `{value}`"))?,
        None => AnchorPlacement::BottomStart,
    };
    let mut popover = SystemPopover::new(
        finite_dimension(options.width, 420.0),
        finite_dimension(options.height, 300.0),
    )
    .placement(placement)
    .gap(finite_number(options.popover_gap).unwrap_or(6.0))
    .offset(
        finite_number(options.popover_offset_x).unwrap_or(0.0),
        finite_number(options.popover_offset_y).unwrap_or(0.0),
    )
    .viewport_margin(finite_number(options.popover_viewport_margin).unwrap_or(8.0));
    if let Some(grab) = options.popover_grab {
        popover = popover.grab(grab);
    }
    if let Some(accepts_key_focus) = options.popover_accepts_key_focus {
        popover = popover.accepts_key_focus(accepts_key_focus);
    }
    if let Some(dismiss_on_escape) = options.popover_dismiss_on_escape {
        popover = popover.dismiss_on_escape(dismiss_on_escape);
    }
    if let Some(dismiss_on_pointer_outside) = options.popover_dismiss_on_pointer_outside {
        popover = popover.dismiss_on_pointer_outside(dismiss_on_pointer_outside);
    }
    Ok(popover.window_options(
        options
            .title
            .clone()
            .unwrap_or_else(|| "QuickGUI popover".to_owned()),
    ))
}

use super::listener_scope::{ListenerSlots, OwnedListener};
use super::*;

/// Context provided while a view declares its element tree.
pub struct ViewContext<'a, V> {
    pub(super) size: Size,
    pub(super) scale_factor: f32,
    pub(super) metrics: FrameMetrics,
    pub(super) focused: Option<ElementId>,
    pub(super) focused_path: Vec<ElementId>,
    pub(super) request_animation_frame: bool,
    pub(super) repaint_deadline: Option<Instant>,
    pub(super) listeners: &'a mut ListenerRegistry,
    pub(super) window: WindowHandle,
    pub(super) window_state: WindowState,
    pub(super) displays: &'a Displays,
    pub(super) keyboard_layout: &'a KeyboardLayout,
    pub(super) font_system: &'a SharedFontSystem,
    pub(super) assets: &'a Assets,
    pub(super) app_info: Option<&'a AppInfo>,
    pub(super) app_paths: Option<&'a AppPaths>,
    pub(super) system_info: &'a SystemInfo,
    pub(super) system_preferences: &'a SystemPreferences,
    pub(super) background_tasks: Option<&'a BackgroundTaskPoolHandle>,
    pub(super) foreground_tasks: &'a ForegroundTaskSpawner,
    pub(super) globals: &'a GlobalStore,
    pub(super) event_proxy: Option<&'a EventLoopProxy<RuntimeEvent>>,
    pub(super) marker: PhantomData<fn(&mut V)>,
}

impl<V: 'static> ViewContext<'_, V> {
    /// Measure unwrapped styled text using the application's loaded fonts, without a GPU draw.
    /// Retained components should cache the result until their text, style, or scale changes.
    pub fn measure_styled_text(
        &mut self,
        text: &crate::StyledText,
        style: &crate::TextStyle,
    ) -> Size {
        self.listeners.observes_viewport = true;
        crate::renderer::measure_intrinsic_text(self.font_system, text, style, self.scale_factor)
    }

    /// Read the viewport size and observe future size or scale-factor changes for this view.
    ///
    /// Views that do not read viewport geometry stay mounted during native window resize; the
    /// runtime relays out their retained declaration directly.
    pub fn size(&mut self) -> Size {
        self.listeners.observes_viewport = true;
        self.size
    }

    pub fn scale_factor(&mut self) -> f32 {
        self.listeners.observes_viewport = true;
        self.scale_factor
    }

    pub fn window_handle(&self) -> WindowHandle {
        self.window
    }

    /// Create a thread-safe handle that invalidates this window from background work.
    ///
    /// The returned handle wakes the native event loop and marks only this view dirty. It is
    /// suitable for long-lived producers such as terminal sessions, file watchers, or streaming
    /// transports that should leave the application asleep while no updates are available.
    pub fn window_invalidator(&self) -> WindowInvalidator {
        WindowInvalidator {
            runtime: self.event_proxy.map(|proxy| (proxy.clone(), self.window)),
        }
    }

    /// Read retained native window state and observe future state changes for this render branch.
    ///
    /// The observation is rebuilt declaratively. A view that never calls this method does not
    /// rebuild merely because its window moved, minimized, or changed z-order state.
    pub fn window_state(&mut self) -> WindowState {
        self.listeners.observes_window_state = true;
        self.window_state
    }

    pub fn window_bounds(&mut self) -> WindowBounds {
        self.window_state().bounds
    }

    /// Read and observe the bounded active-display snapshot.
    ///
    /// AppKit screen-parameter notifications replace this immutable snapshot only when its value
    /// changes. Views which never call a display method do not rebuild for display changes.
    pub fn displays(&mut self) -> &[Display] {
        self.listeners.observes_displays = true;
        self.displays.all()
    }

    /// Read and observe the complete immutable display snapshot.
    pub fn display_snapshot(&mut self) -> &Displays {
        self.listeners.observes_displays = true;
        self.displays
    }

    /// Capture persistable geometry and display identity for this window.
    ///
    /// This observes both window state and displays, so the view rebuilds when either changes.
    /// See [`WindowRestoreState`](crate::WindowRestoreState).
    pub fn window_restore_state(&mut self) -> crate::WindowRestoreState {
        self.listeners.observes_displays = true;
        self.listeners.observes_window_state = true;
        self.window_state.restore_state(self.displays)
    }

    pub fn primary_display(&mut self) -> Option<&Display> {
        self.listeners.observes_displays = true;
        self.displays.primary()
    }

    pub fn find_display(&mut self, id: DisplayId) -> Option<&Display> {
        self.listeners.observes_displays = true;
        self.displays.find(id)
    }

    /// Display currently containing this window, when both native placement and the latest
    /// snapshot are known.
    pub fn current_display(&mut self) -> Option<&Display> {
        self.listeners.observes_displays = true;
        self.listeners.observes_window_state = true;
        self.window_state
            .display_id
            .and_then(|id| self.displays.find(id))
    }

    /// Read and observe the active native keyboard-layout snapshot.
    ///
    /// macOS input-source notifications replace this immutable value only when the layout or its
    /// command translation changes. Views which never call this method do not rebuild for keyboard
    /// layout changes.
    pub fn keyboard_layout(&mut self) -> &KeyboardLayout {
        self.listeners.observes_keyboard_layout = true;
        self.keyboard_layout
    }

    /// Access the application's immutable asset source without subscribing the view to changes.
    pub fn assets(&self) -> &Assets {
        self.assets
    }

    /// GPUI-shaped alias for [`Self::assets`].
    pub fn asset_source(&self) -> &Assets {
        self.assets()
    }

    /// Immutable package identity supplied before application startup.
    pub fn app_info(&self) -> Option<&AppInfo> {
        self.app_info
    }

    /// Standard application paths resolved once during startup.
    pub fn app_paths(&self) -> Option<&AppPaths> {
        self.app_paths
    }

    /// Immutable operating-system and preferred-language snapshot captured at startup.
    pub fn system_info(&self) -> &SystemInfo {
        self.system_info
    }

    /// Read and observe the current system appearance and accessibility preferences.
    pub fn system_preferences(&mut self) -> SystemPreferences {
        self.listeners.observes_system_preferences = true;
        *self.system_preferences
    }

    /// Read and observe the effective native light/dark appearance for this window.
    pub fn appearance(&mut self) -> WindowAppearance {
        self.window_state().appearance
    }

    /// Metrics from the previously completed frame.
    pub fn metrics(&self) -> FrameMetrics {
        self.metrics
    }

    /// Create a stable identity that can be attached with [`crate::Element::track_focus`].
    pub fn focus_handle(&self, id: impl Into<ElementId>) -> FocusHandle {
        FocusHandle::new(id)
    }

    pub fn focused(&self) -> Option<ElementId> {
        self.focused
    }

    pub fn is_focused(&self, handle: FocusHandle) -> bool {
        self.focused == Some(handle.id())
    }

    /// Whether this scope is the focused element or an ancestor of it.
    pub fn contains_focused(&self, handle: FocusHandle) -> bool {
        self.focused_path.contains(&handle.id())
    }

    /// Whether an application-global value of this type has been installed.
    pub fn has_global<G: Global>(&self) -> bool {
        self.globals.has::<G>()
    }

    /// Read an application-global value without retaining a render observation.
    pub fn global<G: Global>(&self) -> Ref<'_, G> {
        self.globals.get::<G>()
    }

    /// Read an application-global value if one has been installed.
    pub fn try_global<G: Global>(&self) -> Option<Ref<'_, G>> {
        self.globals.try_get::<G>()
    }

    /// Read a global and conditionally invalidate this window when that type changes later.
    ///
    /// The observation is refreshed on every declarative rebuild, so omitting this call from a
    /// later branch automatically unsubscribes the window from repaint notifications.
    pub fn watch_global<G: Global, R>(&mut self, read: impl FnOnce(&G) -> R) -> R {
        self.listeners.observe_global(TypeId::of::<G>());
        let global = self.globals.get::<G>();
        read(&global)
    }

    /// Observe application-global changes with an explicit RAII lifetime.
    ///
    /// This matches GPUI's `observe_global` shape. Delivery is deferred until the mutating callback
    /// releases its borrows. Store the returned [`Subscription`] on the view, or call
    /// [`Subscription::detach`] to keep it until the window closes. The callback can read the new
    /// value through [`EventContext::global`].
    pub fn observe_global<G: Global>(
        &mut self,
        mut callback: impl FnMut(&mut V, &mut EventContext) + 'static,
    ) -> Subscription {
        let callback: GlobalObserverCallback = Rc::new(RefCell::new(
            move |view: &mut dyn Any, cx: &mut EventContext| {
                callback(
                    view.downcast_mut::<V>()
                        .expect("global observer received the wrong view type"),
                    cx,
                );
            },
        ));
        self.listeners.subscribe_global(TypeId::of::<G>(), callback)
    }

    /// Explicitly named alias for [`Self::observe_global`].
    pub fn subscribe_global<G: Global>(
        &mut self,
        callback: impl FnMut(&mut V, &mut EventContext) + 'static,
    ) -> Subscription {
        self.observe_global::<G>(callback)
    }

    /// Read shared state and retain a window-level observation for later updates.
    ///
    /// Calling [`Entity::update`] from any window invalidates the current component scope, or the
    /// root view when called outside a scope. The
    /// observation is refreshed on each declarative rebuild, so conditional reads automatically
    /// unsubscribe when that branch is no longer rendered.
    pub fn observe<T, R>(&mut self, entity: &Entity<T>, read: impl FnOnce(&T) -> R) -> R {
        self.listeners.observe_entity(entity.id());
        entity.read(read)
    }

    /// Subscribe this retained view to one typed event emitted by another entity.
    ///
    /// Delivery passes a temporary strong handle to the source entity and runs only after the
    /// emitting callback releases its borrows. Store the returned [`Subscription`] on the view to
    /// control its lifetime, or call [`Subscription::detach`] to retain it until the declaring
    /// component scope is removed (or the window closes for a root subscription).
    pub fn subscribe<T, E>(
        &mut self,
        entity: &Entity<T>,
        mut callback: impl FnMut(&mut V, Entity<T>, &E, &mut EventContext) + 'static,
    ) -> Subscription
    where
        T: EventEmitter<E>,
        E: Any,
    {
        let source = entity.downgrade();
        let callback: EntityEventCallback = Rc::new(RefCell::new(
            move |view: &mut dyn Any, event: &dyn Any, cx: &mut EventContext| {
                let Some(source) = source.upgrade() else {
                    return;
                };
                callback(
                    view.downcast_mut::<V>()
                        .expect("entity-event subscriber received the wrong view type"),
                    source,
                    event
                        .downcast_ref::<E>()
                        .expect("entity-event subscriber received the wrong event type"),
                    cx,
                );
            },
        ));
        self.listeners
            .subscribe_entity_event(entity.id(), TypeId::of::<E>(), callback)
    }

    /// Keep rendering at the display's cadence until a future frame omits this call.
    pub fn request_animation_frame(&mut self) {
        self.request_animation_frame = true;
    }

    /// Request one view repaint at or after an exact deadline.
    ///
    /// Repeated calls keep only the earliest deadline. Unlike [`Self::request_animation_frame`],
    /// this leaves the application asleep between now and the requested repaint.
    pub fn request_repaint_at(&mut self, deadline: Instant) {
        self.repaint_deadline = Some(
            self.repaint_deadline
                .map_or(deadline, |current| current.min(deadline)),
        );
    }

    /// Run a non-blocking future on QuickGUI's application-thread executor.
    ///
    /// The future starts on the next event-loop turn. Dropping the returned handle cancels it;
    /// [`Task::detach`] lets it continue until completion or until this window closes. Use the
    /// supplied [`AsyncViewContext`] for fallible view updates and exact, idle event-loop timers.
    pub fn spawn<Build, Fut, R>(&self, build: Build) -> Result<Task<R>, ForegroundTaskSpawnError>
    where
        Build: FnOnce(AsyncViewContext<V>) -> Fut,
        Fut: Future<Output = R> + 'static,
        R: 'static,
    {
        self.foreground_tasks
            .spawn::<V, _, _, _>(self.window, build)
    }

    /// Run blocking or CPU-heavy application work on QuickGUI's bounded worker pool.
    ///
    /// The completion is delivered on this window's UI thread and wakes the event loop exactly
    /// once. The window remains asleep while work is pending; no polling frame is required.
    pub fn spawn_background<T, Work, Complete>(
        &self,
        work: Work,
        complete: Complete,
    ) -> Result<(), TaskSpawnError>
    where
        T: Send + 'static,
        Work: FnOnce() -> T + Send + 'static,
        Complete:
            FnOnce(&mut V, Result<T, BackgroundTaskError>, &mut EventContext) + Send + 'static,
    {
        self.background_tasks
            .ok_or(TaskSpawnError::Unavailable)?
            .spawn::<V, T, Work, Complete>(self.window, work, complete)
    }

    /// Explicit alias for [`Self::spawn_background`].
    pub fn spawn_blocking<T, Work, Complete>(
        &self,
        work: Work,
        complete: Complete,
    ) -> Result<(), TaskSpawnError>
    where
        T: Send + 'static,
        Work: FnOnce() -> T + Send + 'static,
        Complete:
            FnOnce(&mut V, Result<T, BackgroundTaskError>, &mut EventContext) + Send + 'static,
    {
        self.spawn_background(work, complete)
    }

    /// Observe the exact teardown of one child window owned by this view.
    ///
    /// Registration is declarative and refreshed on every rebuild. The callback runs after the
    /// child and all of its descendants have been removed, but only while this parent remains
    /// open. It owns no native observer, polling task, timer, or idle scheduler source.
    pub fn on_child_window_closed(
        &mut self,
        child: WindowHandle,
        callback: impl Fn(&mut V, WindowHandle, &mut EventContext) + 'static,
    ) {
        assert_ne!(
            child, self.window,
            "a window cannot observe itself as a child"
        );
        assert!(
            self.listeners.child_window_closed.len() + self.listeners.any_child_window_closed.len()
                < MAX_CHILD_WINDOW_CLOSE_LISTENERS_PER_WINDOW,
            "a window cannot declare more than {MAX_CHILD_WINDOW_CLOSE_LISTENERS_PER_WINDOW} child-window close listeners"
        );
        let callback: ChildWindowClosedCallback = Arc::new(move |view, child, context| {
            callback(
                view.downcast_mut::<V>()
                    .expect("child-window close listener received the wrong view type"),
                child,
                context,
            );
        });
        let previous = self.listeners.child_window_closed.insert(child, callback);
        self.listeners
            .own_listener(OwnedListener::ChildClosed(child));
        assert!(
            previous.is_none(),
            "child window {child:?} was observed more than once by the same view"
        );
    }

    /// Observe teardown of any direct child owned by this view.
    ///
    /// Unlike [`Self::on_child_window_closed`], this can be declared before a child handle exists,
    /// so a child opened and closed within the same event turn is still reported exactly once.
    /// Components should compare the delivered handle with their controlled child state.
    pub fn on_any_child_window_closed(
        &mut self,
        callback: impl Fn(&mut V, WindowHandle, &mut EventContext) + 'static,
    ) {
        assert!(
            self.listeners.child_window_closed.len() + self.listeners.any_child_window_closed.len()
                < MAX_CHILD_WINDOW_CLOSE_LISTENERS_PER_WINDOW,
            "a window cannot declare more than {MAX_CHILD_WINDOW_CLOSE_LISTENERS_PER_WINDOW} child-window close listeners"
        );
        let callback: ChildWindowClosedCallback = Arc::new(move |view, child, context| {
            callback(
                view.downcast_mut::<V>()
                    .expect("child-window close listener received the wrong view type"),
                child,
                context,
            );
        });
        self.listeners
            .any_child_window_closed
            .push(callback.clone());
        self.listeners
            .own_listener(OwnedListener::AnyChildClosed(callback));
    }

    /// Register a stable, view-local click callback for use with [`crate::Element::on_click`].
    pub fn listener(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &mut EventContext) + 'static,
    ) -> ClickListener<V> {
        let id = id.into();
        let callback = Arc::new(move |view: &mut dyn Any, context: &mut EventContext| {
            callback(
                view.downcast_mut::<V>()
                    .expect("click listener received the wrong view type"),
                context,
            );
        });
        let previous = self.listeners.clicks.insert(id, callback);
        self.listeners.own_listener(OwnedListener::Click(id));
        assert!(
            previous.is_none(),
            "listener id {id:?} was registered more than once"
        );
        ClickListener {
            id,
            marker: PhantomData,
        }
    }

    /// Register a web-style secondary-click callback.
    ///
    /// Attach the returned handle with [`crate::Element::on_context_menu`]. The callback runs on
    /// button press at the original logical pointer position, before a native right-button drag
    /// can begin.
    pub fn context_menu_listener(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &ContextMenuEvent, &mut EventContext) + 'static,
    ) -> ContextMenuListener<V> {
        let id = id.into();
        let callback: ContextMenuCallback = Arc::new(
            move |view: &mut dyn Any, event: &ContextMenuEvent, context: &mut EventContext| {
                callback(
                    view.downcast_mut::<V>()
                        .expect("context-menu listener received the wrong view type"),
                    event,
                    context,
                );
            },
        );
        let previous = self.listeners.context_menus.insert(id, callback);
        self.listeners.own_listener(OwnedListener::ContextMenu(id));
        assert!(
            previous.is_none(),
            "context-menu listener id {id:?} was registered more than once"
        );
        ContextMenuListener {
            id,
            marker: PhantomData,
        }
    }

    /// Register a stable callback for a captured pointer interaction.
    ///
    /// Attach the returned handle with [`crate::Element::on_pointer`]. A press inside the element
    /// starts capture; move events and the terminal up/cancel event continue outside its bounds.
    pub fn pointer_listener(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &PointerEvent, &mut EventContext) + 'static,
    ) -> PointerListener<V> {
        let id = id.into();
        let callback = Arc::new(
            move |view: &mut dyn Any, event: &PointerEvent, context: &mut EventContext| {
                callback(
                    view.downcast_mut::<V>()
                        .expect("pointer listener received the wrong view type"),
                    event,
                    context,
                );
            },
        );
        let previous = self.listeners.pointers.insert(id, callback);
        self.listeners.own_listener(OwnedListener::Pointer(id));
        assert!(
            previous.is_none(),
            "pointer listener id {id:?} was registered more than once"
        );
        PointerListener {
            id,
            marker: PhantomData,
        }
    }

    /// Register a desktop mouse-down callback for attachment to one retained element.
    pub fn mouse_down_listener(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &MouseDownEvent, &mut EventContext) + 'static,
    ) -> MouseDownListener<V> {
        let id = id.into();
        let callback: MouseListenerCallback = Arc::new(move |view, event, context| {
            let MouseListenerEvent::Down(event) = event else {
                unreachable!("mouse-down callback received the wrong event kind")
            };
            callback(
                view.downcast_mut::<V>()
                    .expect("mouse-down listener received the wrong view type"),
                event,
                context,
            );
        });
        MouseDownListener {
            id,
            key: self.listeners.push_mouse_listener(callback),
            marker: PhantomData,
        }
    }

    /// Register a desktop mouse-up callback for attachment to one retained element.
    pub fn mouse_up_listener(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &MouseUpEvent, &mut EventContext) + 'static,
    ) -> MouseUpListener<V> {
        let id = id.into();
        let callback: MouseListenerCallback = Arc::new(move |view, event, context| {
            let MouseListenerEvent::Up(event) = event else {
                unreachable!("mouse-up callback received the wrong event kind")
            };
            callback(
                view.downcast_mut::<V>()
                    .expect("mouse-up listener received the wrong view type"),
                event,
                context,
            );
        });
        MouseUpListener {
            id,
            key: self.listeners.push_mouse_listener(callback),
            marker: PhantomData,
        }
    }

    /// Register a desktop mouse-motion callback for attachment to one retained element.
    pub fn mouse_move_listener(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &MouseMoveEvent, &mut EventContext) + 'static,
    ) -> MouseMoveListener<V> {
        let id = id.into();
        let callback: MouseListenerCallback = Arc::new(move |view, event, context| {
            let MouseListenerEvent::Move(event) = event else {
                unreachable!("mouse-move callback received the wrong event kind")
            };
            callback(
                view.downcast_mut::<V>()
                    .expect("mouse-move listener received the wrong view type"),
                event,
                context,
            );
        });
        MouseMoveListener {
            id,
            key: self.listeners.push_mouse_listener(callback),
            marker: PhantomData,
        }
    }

    /// Register a native-window mouse-exit callback for attachment to one retained element.
    pub fn mouse_exit_listener(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &MouseExitEvent, &mut EventContext) + 'static,
    ) -> MouseExitListener<V> {
        let id = id.into();
        let callback: MouseListenerCallback = Arc::new(move |view, event, context| {
            let MouseListenerEvent::Exit(event) = event else {
                unreachable!("mouse-exit callback received the wrong event kind")
            };
            callback(
                view.downcast_mut::<V>()
                    .expect("mouse-exit listener received the wrong view type"),
                event,
                context,
            );
        });
        MouseExitListener {
            id,
            key: self.listeners.push_mouse_listener(callback),
            marker: PhantomData,
        }
    }

    /// Register a web-style hover transition callback.
    pub fn hover_listener(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &bool, &mut EventContext) + 'static,
    ) -> HoverListener<V> {
        let id = id.into();
        let callback: MouseListenerCallback = Arc::new(move |view, event, context| {
            let MouseListenerEvent::Hover(hovered) = event else {
                unreachable!("hover callback received the wrong event kind")
            };
            callback(
                view.downcast_mut::<V>()
                    .expect("hover listener received the wrong view type"),
                hovered,
                context,
            );
        });
        HoverListener {
            id,
            key: self.listeners.push_mouse_listener(callback),
            marker: PhantomData,
        }
    }

    /// Register focused key-down input for capture or bubble attachment on an element.
    ///
    /// Keymap actions run first. If they propagate, the raw key press traverses the retained
    /// focus path. Call [`EventContext::prevent_default`] to replace QuickGUI's editing, focus,
    /// activation, dismissal, or default close behavior without stopping another listener.
    pub fn key_down_listener(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &KeyDownEvent, &mut EventContext) + 'static,
    ) -> KeyDownListener<V> {
        let id = id.into();
        let callback: KeyListenerCallback = Arc::new(move |view, event, context| {
            let KeyListenerEvent::Down(event) = event else {
                unreachable!("key-down callback received the wrong event kind")
            };
            callback(
                view.downcast_mut::<V>()
                    .expect("key-down listener received the wrong view type"),
                event,
                context,
            );
        });
        KeyDownListener {
            id,
            key: self.listeners.push_key_listener(callback),
            marker: PhantomData,
        }
    }

    /// Register focused key-up input for capture or bubble attachment on an element.
    pub fn key_up_listener(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &KeyUpEvent, &mut EventContext) + 'static,
    ) -> KeyUpListener<V> {
        let id = id.into();
        let callback: KeyListenerCallback = Arc::new(move |view, event, context| {
            let KeyListenerEvent::Up(event) = event else {
                unreachable!("key-up callback received the wrong event kind")
            };
            callback(
                view.downcast_mut::<V>()
                    .expect("key-up listener received the wrong view type"),
                event,
                context,
            );
        });
        KeyUpListener {
            id,
            key: self.listeners.push_key_listener(callback),
            marker: PhantomData,
        }
    }

    /// Register scroll-wheel input for attachment with [`crate::Element::on_scroll_wheel`].
    ///
    /// Scroll events bubble through listening ancestors. Call
    /// [`EventContext::stop_propagation`] to stop that path or [`EventContext::prevent_default`]
    /// when the gesture should not move the retained scroll container underneath it.
    pub fn scroll_wheel_listener(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &ScrollWheelEvent, &mut EventContext) + 'static,
    ) -> ScrollWheelListener<V> {
        let id = id.into();
        let callback: ScrollWheelCallback = Arc::new(move |view, event, context| {
            callback(
                view.downcast_mut::<V>()
                    .expect("scroll-wheel listener received the wrong view type"),
                event,
                context,
            );
        });
        let previous = self.listeners.scroll_wheels.insert(id, callback);
        self.listeners.own_listener(OwnedListener::ScrollWheel(id));
        assert!(
            previous.is_none(),
            "scroll-wheel listener id {id:?} was registered more than once"
        );
        ScrollWheelListener {
            id,
            marker: PhantomData,
        }
    }

    /// Register raw multi-contact touch input for attachment with [`crate::Element::on_touch`].
    ///
    /// The contact is hit-tested once at start and remains captured until its terminal end or
    /// cancellation event. Touch callbacks bubble through listening ancestors by default.
    pub fn touch_listener(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &TouchEvent, &mut EventContext) + 'static,
    ) -> TouchListener<V> {
        let id = id.into();
        let callback: TouchCallback = Arc::new(move |view, event, context| {
            callback(
                view.downcast_mut::<V>()
                    .expect("touch listener received the wrong view type"),
                event,
                context,
            );
        });
        let previous = self.listeners.touches.insert(id, callback);
        self.listeners.own_listener(OwnedListener::Touch(id));
        assert!(
            previous.is_none(),
            "touch listener id {id:?} was registered more than once"
        );
        TouchListener {
            id,
            marker: PhantomData,
        }
    }

    /// Register Force Touch input for attachment with [`crate::Element::on_mouse_pressure`].
    pub fn mouse_pressure_listener(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &MousePressureEvent, &mut EventContext) + 'static,
    ) -> MousePressureListener<V> {
        let id = id.into();
        let callback: MousePressureCallback = Arc::new(move |view, event, context| {
            callback(
                view.downcast_mut::<V>()
                    .expect("mouse-pressure listener received the wrong view type"),
                event,
                context,
            );
        });
        let previous = self.listeners.mouse_pressures.insert(id, callback);
        self.listeners.own_listener(OwnedListener::Pressure(id));
        assert!(
            previous.is_none(),
            "mouse-pressure listener id {id:?} was registered more than once"
        );
        MousePressureListener {
            id,
            marker: PhantomData,
        }
    }

    /// Register pinch-to-zoom input for attachment with [`crate::Element::on_pinch`].
    pub fn pinch_listener(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &PinchEvent, &mut EventContext) + 'static,
    ) -> PinchListener<V> {
        let id = id.into();
        let callback: PinchCallback = Arc::new(move |view, event, context| {
            callback(
                view.downcast_mut::<V>()
                    .expect("pinch listener received the wrong view type"),
                event,
                context,
            );
        });
        let previous = self.listeners.pinches.insert(id, callback);
        self.listeners.own_listener(OwnedListener::Pinch(id));
        assert!(
            previous.is_none(),
            "pinch listener id {id:?} was registered more than once"
        );
        PinchListener {
            id,
            marker: PhantomData,
        }
    }

    /// Register two-finger rotation input for attachment with [`crate::Element::on_rotation`].
    pub fn rotation_listener(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &RotationEvent, &mut EventContext) + 'static,
    ) -> RotationListener<V> {
        let id = id.into();
        let callback: RotationCallback = Arc::new(move |view, event, context| {
            callback(
                view.downcast_mut::<V>()
                    .expect("rotation listener received the wrong view type"),
                event,
                context,
            );
        });
        let previous = self.listeners.rotations.insert(id, callback);
        self.listeners.own_listener(OwnedListener::Rotation(id));
        assert!(
            previous.is_none(),
            "rotation listener id {id:?} was registered more than once"
        );
        RotationListener {
            id,
            marker: PhantomData,
        }
    }

    /// Register smart-magnify input for attachment with [`crate::Element::on_smart_magnify`].
    pub fn smart_magnify_listener(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &SmartMagnifyEvent, &mut EventContext) + 'static,
    ) -> SmartMagnifyListener<V> {
        let id = id.into();
        let callback: SmartMagnifyCallback = Arc::new(move |view, event, context| {
            callback(
                view.downcast_mut::<V>()
                    .expect("smart-magnify listener received the wrong view type"),
                event,
                context,
            );
        });
        let previous = self.listeners.smart_magnifies.insert(id, callback);
        self.listeners.own_listener(OwnedListener::SmartMagnify(id));
        assert!(
            previous.is_none(),
            "smart-magnify listener id {id:?} was registered more than once"
        );
        SmartMagnifyListener {
            id,
            marker: PhantomData,
        }
    }

    /// Register a typed drag source.
    ///
    /// The callback runs once after primary-button motion crosses the drag threshold. Its payload
    /// is retained only for that gesture, and its optional preview is painted on the GPU overlay
    /// plane without rebuilding the application view on every pointer move. On macOS, the same
    /// arbitrary Rust value can cross directly into another QuickGUI window without serialization.
    pub fn drag_listener<T: 'static>(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &DragStartEvent, &mut EventContext) -> Drag<T> + 'static,
    ) -> DragListener<V, T> {
        let id = id.into();
        let callback: DragStartCallback = Arc::new(move |view, event, context| {
            let drag = callback(
                view.downcast_mut::<V>()
                    .expect("drag listener received the wrong view type"),
                event,
                context,
            );
            AnyDrag::new(drag)
        });
        let previous = self
            .listeners
            .drag_sources
            .insert(id, (TypeId::of::<T>(), callback));
        self.listeners.own_listener(OwnedListener::Drag(id));
        assert!(
            previous.is_none(),
            "drag listener id {id:?} was registered more than once"
        );
        DragListener {
            id,
            marker: PhantomData,
        }
    }

    /// Register a typed drop callback.
    ///
    /// The callback is considered compatible only when the active payload has exactly type `T`.
    /// [`DroppedFiles`] uses this same path for native file drops.
    pub fn drop_listener<T: 'static>(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &T, &DropEvent, &mut EventContext) + 'static,
    ) -> DropListener<V, T> {
        let id = id.into();
        let callback: DropCallback = Arc::new(move |view, value, event, context| {
            callback(
                view.downcast_mut::<V>()
                    .expect("drop listener received the wrong view type"),
                value
                    .downcast_ref::<T>()
                    .expect("drop listener received the wrong payload type"),
                event,
                context,
            );
        });
        let previous = self
            .listeners
            .drops
            .insert((id, TypeId::of::<T>()), callback);
        self.listeners
            .own_listener(OwnedListener::Drop(id, TypeId::of::<T>()));
        assert!(
            previous.is_none(),
            "drop listener id {id:?} for this payload type was registered more than once"
        );
        self.listeners.drop_order.push((id, TypeId::of::<T>()));
        DropListener {
            id,
            marker: PhantomData,
        }
    }

    /// Register a stable controlled-value callback for [`crate::Element::on_input`].
    pub fn input_listener(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &str, &mut EventContext) + 'static,
    ) -> InputListener<V> {
        let id = id.into();
        let callback = Arc::new(
            move |view: &mut dyn Any, value: &str, context: &mut EventContext| {
                callback(
                    view.downcast_mut::<V>()
                        .expect("input listener received the wrong view type"),
                    value,
                    context,
                );
            },
        );
        let previous = self.listeners.inputs.insert(id, callback);
        self.listeners.own_listener(OwnedListener::Input(id));
        assert!(
            previous.is_none(),
            "input listener id {id:?} was registered more than once"
        );
        InputListener {
            id,
            marker: PhantomData,
        }
    }

    /// Register a stable callback for Return in a valid, single-line text input.
    ///
    /// The callback receives the current committed value. IME preedit text is never submitted,
    /// key repeat is ignored, and an element marked with [`crate::Element::invalid`] blocks the
    /// callback while retaining focus.
    pub fn submit_listener(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &str, &mut EventContext) + 'static,
    ) -> SubmitListener<V> {
        let id = id.into();
        let callback = Arc::new(
            move |view: &mut dyn Any, value: &str, context: &mut EventContext| {
                callback(
                    view.downcast_mut::<V>()
                        .expect("submit listener received the wrong view type"),
                    value,
                    context,
                );
            },
        );
        let previous = self.listeners.submits.insert(id, callback);
        self.listeners.own_listener(OwnedListener::Submit(id));
        assert!(
            previous.is_none(),
            "submit listener id {id:?} was registered more than once"
        );
        SubmitListener {
            id,
            marker: PhantomData,
        }
    }

    /// Register a callback for a valid form submission.
    ///
    /// The event contains document-ordered, shared controlled values and identifies the input or
    /// button that initiated submission. Attach the returned binding with
    /// [`crate::Element::on_form_submit`].
    pub fn form_submit_listener(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &FormSubmitEvent, &mut EventContext) + 'static,
    ) -> FormSubmitListener<V> {
        let id = id.into();
        let callback: FormSubmitCallback = Arc::new(move |view, event, context| {
            callback(
                view.downcast_mut::<V>()
                    .expect("form submit listener received the wrong view type"),
                event,
                context,
            );
        });
        let previous = self.listeners.form_submits.insert(id, callback);
        self.listeners.own_listener(OwnedListener::FormSubmit(id));
        assert!(
            previous.is_none(),
            "form submit listener id {id:?} was registered more than once"
        );
        FormSubmitListener {
            id,
            marker: PhantomData,
        }
    }

    /// Register a callback for a form blocked by declaratively invalid controls.
    ///
    /// Reports are bounded, document ordered, and delivered after QuickGUI moves focus to the
    /// first focusable invalid control. Attach the binding with
    /// [`crate::Element::on_form_invalid`].
    pub fn form_invalid_listener(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &ValidationReport, &mut EventContext) + 'static,
    ) -> FormInvalidListener<V> {
        let id = id.into();
        let callback: FormInvalidCallback = Arc::new(move |view, report, context| {
            callback(
                view.downcast_mut::<V>()
                    .expect("form invalid listener received the wrong view type"),
                report,
                context,
            );
        });
        let previous = self.listeners.form_invalids.insert(id, callback);
        self.listeners.own_listener(OwnedListener::FormInvalid(id));
        assert!(
            previous.is_none(),
            "form invalid listener id {id:?} was registered more than once"
        );
        FormInvalidListener {
            id,
            marker: PhantomData,
        }
    }

    /// Register a callback for Escape and outside-pointer dismissal.
    pub fn dismiss_listener(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &mut EventContext) + 'static,
    ) -> DismissListener<V> {
        let id = id.into();
        let callback = Arc::new(move |view: &mut dyn Any, context: &mut EventContext| {
            callback(
                view.downcast_mut::<V>()
                    .expect("dismiss listener received the wrong view type"),
                context,
            );
        });
        let previous = self.listeners.dismisses.insert(id, callback);
        self.listeners.own_listener(OwnedListener::Dismiss(id));
        assert!(
            previous.is_none(),
            "dismiss listener id {id:?} was registered more than once"
        );
        DismissListener {
            id,
            marker: PhantomData,
        }
    }

    /// Register a typed action callback for attachment with [`crate::Element::on_action`].
    pub fn action_listener<A: Action>(
        &mut self,
        id: impl Into<ElementId>,
        callback: impl Fn(&mut V, &A, &mut EventContext) + 'static,
    ) -> ActionListener<V, A> {
        let id = id.into();
        let erased: ActionCallback = Arc::new(move |view, action, context| {
            let action = action
                .downcast_ref::<A>()
                .expect("action listener received the wrong concrete action type");
            callback(
                view.downcast_mut::<V>()
                    .expect("action listener received the wrong view type"),
                action,
                context,
            );
        });
        let action_type = TypeId::of::<A>();
        ActionListener {
            id,
            key: self.listeners.push_action_listener(erased),
            action_type,
            marker: PhantomData,
        }
    }
}

pub(super) type ClickCallback = Arc<dyn Fn(&mut dyn Any, &mut EventContext)>;
pub(super) type PointerCallback = Arc<dyn Fn(&mut dyn Any, &PointerEvent, &mut EventContext)>;
pub(super) type MouseListenerCallback =
    Arc<dyn Fn(&mut dyn Any, &MouseListenerEvent, &mut EventContext)>;
pub(super) type KeyListenerCallback =
    Arc<dyn Fn(&mut dyn Any, &KeyListenerEvent, &mut EventContext)>;
pub(super) type ScrollWheelCallback =
    Arc<dyn Fn(&mut dyn Any, &ScrollWheelEvent, &mut EventContext)>;
pub(super) type TouchCallback = Arc<dyn Fn(&mut dyn Any, &TouchEvent, &mut EventContext)>;
pub(super) type ContextMenuCallback =
    Arc<dyn Fn(&mut dyn Any, &ContextMenuEvent, &mut EventContext)>;
pub(super) type MousePressureCallback =
    Arc<dyn Fn(&mut dyn Any, &MousePressureEvent, &mut EventContext)>;
pub(super) type PinchCallback = Arc<dyn Fn(&mut dyn Any, &PinchEvent, &mut EventContext)>;
pub(super) type RotationCallback = Arc<dyn Fn(&mut dyn Any, &RotationEvent, &mut EventContext)>;
pub(super) type SmartMagnifyCallback =
    Arc<dyn Fn(&mut dyn Any, &SmartMagnifyEvent, &mut EventContext)>;
pub(super) type InputCallback = Arc<dyn Fn(&mut dyn Any, &str, &mut EventContext)>;
pub(super) type FormSubmitCallback = Arc<dyn Fn(&mut dyn Any, &FormSubmitEvent, &mut EventContext)>;
pub(super) type FormInvalidCallback =
    Arc<dyn Fn(&mut dyn Any, &ValidationReport, &mut EventContext)>;
pub(super) type ActionCallback = Arc<dyn Fn(&mut dyn Any, &dyn Any, &mut EventContext)>;
pub(super) type DragStartCallback =
    Arc<dyn Fn(&mut dyn Any, &DragStartEvent, &mut EventContext) -> AnyDrag>;
pub(super) type DropCallback = Arc<dyn Fn(&mut dyn Any, &dyn Any, &DropEvent, &mut EventContext)>;
pub(super) type EntityEventCallback =
    Rc<RefCell<dyn FnMut(&mut dyn Any, &dyn Any, &mut EventContext)>>;
pub(super) type GlobalObserverCallback = Rc<RefCell<dyn FnMut(&mut dyn Any, &mut EventContext)>>;
pub(super) type ChildWindowClosedCallback =
    Arc<dyn Fn(&mut dyn Any, WindowHandle, &mut EventContext)>;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum MouseListenerEvent {
    Down(MouseDownEvent),
    Up(MouseUpEvent),
    Move(MouseMoveEvent),
    Exit(MouseExitEvent),
    Hover(bool),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum KeyListenerEvent {
    Down(KeyDownEvent),
    Up(KeyUpEvent),
}

impl KeyListenerEvent {
    pub(super) fn kind(&self) -> KeyListenerKind {
        match self {
            Self::Down(_) => KeyListenerKind::Down,
            Self::Up(_) => KeyListenerKind::Up,
        }
    }
}

#[derive(Clone)]
pub(super) struct EntityEventSubscription {
    pub(super) state: Rc<SubscriptionState>,
    pub(super) callback: EntityEventCallback,
}

impl EntityEventSubscription {
    pub(super) fn is_active(&self) -> bool {
        self.state.is_active()
    }
}

#[derive(Clone)]
pub(super) struct GlobalObserverSubscription {
    pub(super) global_type: TypeId,
    pub(super) state: Rc<SubscriptionState>,
    pub(super) callback: GlobalObserverCallback,
}

impl GlobalObserverSubscription {
    pub(super) fn is_active(&self) -> bool {
        self.state.is_active()
    }
}

#[derive(Default)]
pub(super) struct ListenerRegistry {
    pub(super) current_scope: Option<ElementId>,
    pub(super) scope_listeners: HashMap<ElementId, Vec<OwnedListener>>,
    pub(super) scopes: component_scope::ComponentScopes,
    pub(super) clicks: HashMap<ElementId, ClickCallback>,
    pub(super) pointers: HashMap<ElementId, PointerCallback>,
    pub(super) mouse_listeners: ListenerSlots<MouseListenerCallback>,
    pub(super) key_listeners: ListenerSlots<KeyListenerCallback>,
    pub(super) scroll_wheels: HashMap<ElementId, ScrollWheelCallback>,
    pub(super) touches: HashMap<ElementId, TouchCallback>,
    pub(super) context_menus: HashMap<ElementId, ContextMenuCallback>,
    pub(super) mouse_pressures: HashMap<ElementId, MousePressureCallback>,
    pub(super) pinches: HashMap<ElementId, PinchCallback>,
    pub(super) rotations: HashMap<ElementId, RotationCallback>,
    pub(super) smart_magnifies: HashMap<ElementId, SmartMagnifyCallback>,
    pub(super) drag_sources: HashMap<ElementId, (TypeId, DragStartCallback)>,
    pub(super) drops: HashMap<(ElementId, TypeId), DropCallback>,
    pub(super) drop_order: Vec<(ElementId, TypeId)>,
    pub(super) inputs: HashMap<ElementId, InputCallback>,
    pub(super) submits: HashMap<ElementId, InputCallback>,
    pub(super) form_submits: HashMap<ElementId, FormSubmitCallback>,
    pub(super) form_invalids: HashMap<ElementId, FormInvalidCallback>,
    pub(super) dismisses: HashMap<ElementId, ClickCallback>,
    pub(super) actions: ListenerSlots<ActionCallback>,
    pub(super) observed_entities: HashSet<EntityId>,
    pub(super) observed_globals: HashSet<TypeId>,
    pub(super) observes_window_state: bool,
    pub(super) observes_viewport: bool,
    pub(super) observes_displays: bool,
    pub(super) observes_keyboard_layout: bool,
    pub(super) observes_system_preferences: bool,
    pub(super) entity_events: HashMap<(EntityId, TypeId), Vec<EntityEventSubscription>>,
    pub(super) entity_subscription_count: usize,
    pub(super) global_observers: Vec<GlobalObserverSubscription>,
    pub(super) child_window_closed: HashMap<WindowHandle, ChildWindowClosedCallback>,
    pub(super) any_child_window_closed: Vec<ChildWindowClosedCallback>,
}

impl ListenerRegistry {
    pub(super) fn requires_window_state_rebuild(&self, state_changed: bool) -> bool {
        state_changed && self.observes_window_state
    }

    pub(super) fn push_mouse_listener(
        &mut self,
        callback: MouseListenerCallback,
    ) -> MouseListenerKey {
        assert!(
            self.mouse_listeners.len() < MAX_MOUSE_LISTENERS_PER_WINDOW,
            "a window cannot declare more than {MAX_MOUSE_LISTENERS_PER_WINDOW} targeted desktop mouse listeners"
        );
        let key = MouseListenerKey(self.mouse_listeners.push(callback));
        self.own_listener(OwnedListener::Mouse(key.0));
        key
    }

    pub(super) fn mouse_listener(&self, key: MouseListenerKey) -> Option<MouseListenerCallback> {
        self.mouse_listeners.get(key.0).cloned()
    }

    pub(super) fn push_key_listener(&mut self, callback: KeyListenerCallback) -> KeyListenerKey {
        assert!(
            self.key_listeners.len() < MAX_KEY_LISTENERS_PER_WINDOW,
            "a window cannot declare more than {MAX_KEY_LISTENERS_PER_WINDOW} focused key listeners"
        );
        let key = KeyListenerKey(self.key_listeners.push(callback));
        self.own_listener(OwnedListener::Key(key.0));
        key
    }

    pub(super) fn key_listener(&self, key: KeyListenerKey) -> Option<KeyListenerCallback> {
        self.key_listeners.get(key.0).cloned()
    }

    pub(super) fn push_action_listener(&mut self, callback: ActionCallback) -> ActionListenerKey {
        assert!(
            self.actions.len() < MAX_ACTION_LISTENERS_PER_WINDOW,
            "a window cannot declare more than {MAX_ACTION_LISTENERS_PER_WINDOW} typed action listeners"
        );
        let key = ActionListenerKey(self.actions.push(callback));
        self.own_listener(OwnedListener::Action(key.0));
        key
    }

    pub(super) fn action_listener(&self, key: ActionListenerKey) -> Option<ActionCallback> {
        self.actions.get(key.0).cloned()
    }

    pub(super) fn observe_global(&mut self, global_type: TypeId) {
        if let Some(id) = self.current_scope {
            self.scopes.observe_global(id, global_type);
            return;
        }
        if self.observed_globals.contains(&global_type) {
            return;
        }
        assert!(
            self.observed_globals.len() < MAX_OBSERVED_GLOBALS_PER_WINDOW,
            "a window cannot observe more than {MAX_OBSERVED_GLOBALS_PER_WINDOW} global types"
        );
        self.observed_globals.insert(global_type);
    }

    pub(super) fn observes_global_change(&self, global_types: &[TypeId], all: bool) -> bool {
        all || global_types
            .iter()
            .any(|global_type| self.observed_globals.contains(global_type))
    }

    pub(super) fn subscribe_global(
        &mut self,
        global_type: TypeId,
        callback: GlobalObserverCallback,
    ) -> Subscription {
        if self.global_observers.len() >= MAX_GLOBAL_SUBSCRIPTIONS_PER_WINDOW {
            self.prune_global_subscriptions();
        }
        assert!(
            self.global_observers.len() < MAX_GLOBAL_SUBSCRIPTIONS_PER_WINDOW,
            "a window cannot retain more than {MAX_GLOBAL_SUBSCRIPTIONS_PER_WINDOW} global subscriptions"
        );
        let (subscription, state) = Subscription::new();
        self.own_listener(OwnedListener::Subscription(state.clone()));
        self.global_observers.push(GlobalObserverSubscription {
            global_type,
            state,
            callback,
        });
        subscription
    }

    pub(super) fn has_global_subscribers(&self, global_type: Option<TypeId>) -> bool {
        self.global_observers.iter().any(|subscription| {
            subscription.is_active()
                && global_type.is_none_or(|global_type| subscription.global_type == global_type)
        })
    }

    pub(super) fn global_subscriptions(
        &self,
        global_type: Option<TypeId>,
    ) -> Vec<GlobalObserverSubscription> {
        self.global_observers
            .iter()
            .filter(|subscription| {
                subscription.is_active()
                    && global_type.is_none_or(|global_type| subscription.global_type == global_type)
            })
            .cloned()
            .collect()
    }

    pub(super) fn prune_global_subscriptions(&mut self) {
        self.global_observers
            .retain(GlobalObserverSubscription::is_active);
    }

    pub(super) fn observe_entity(&mut self, entity: EntityId) {
        if let Some(id) = self.current_scope {
            self.scopes.observe_entity(id, entity);
            return;
        }
        if self.observed_entities.contains(&entity) {
            return;
        }
        assert!(
            self.observed_entities.len() < MAX_OBSERVED_ENTITIES_PER_WINDOW,
            "a window cannot observe more than {MAX_OBSERVED_ENTITIES_PER_WINDOW} entities"
        );
        self.observed_entities.insert(entity);
    }

    pub(super) fn observes_entity_change(&self, entities: &[EntityId], all: bool) -> bool {
        all || entities
            .iter()
            .any(|entity| self.observed_entities.contains(entity))
    }

    pub(super) fn subscribe_entity_event(
        &mut self,
        entity: EntityId,
        event_type: TypeId,
        callback: EntityEventCallback,
    ) -> Subscription {
        if self.entity_subscription_count >= MAX_ENTITY_SUBSCRIPTIONS_PER_WINDOW {
            self.prune_entity_event_subscriptions();
        }
        assert!(
            self.entity_subscription_count < MAX_ENTITY_SUBSCRIPTIONS_PER_WINDOW,
            "a window cannot retain more than {MAX_ENTITY_SUBSCRIPTIONS_PER_WINDOW} entity-event subscriptions"
        );
        let (subscription, state) = Subscription::new();
        self.own_listener(OwnedListener::Subscription(state.clone()));
        self.entity_events
            .entry((entity, event_type))
            .or_default()
            .push(EntityEventSubscription { state, callback });
        self.entity_subscription_count += 1;
        subscription
    }

    pub(super) fn has_entity_event_subscribers(
        &self,
        entity: EntityId,
        event_type: TypeId,
    ) -> bool {
        self.entity_events
            .get(&(entity, event_type))
            .is_some_and(|subscriptions| {
                subscriptions.iter().any(EntityEventSubscription::is_active)
            })
    }

    pub(super) fn entity_event_callbacks(
        &self,
        entity: EntityId,
        event_type: TypeId,
    ) -> Vec<EntityEventCallback> {
        self.entity_events
            .get(&(entity, event_type))
            .into_iter()
            .flatten()
            .filter(|subscription| subscription.is_active())
            .map(|subscription| subscription.callback.clone())
            .collect()
    }

    pub(super) fn prune_entity_event_subscriptions(&mut self) {
        self.entity_events.retain(|_, subscriptions| {
            subscriptions.retain(EntityEventSubscription::is_active);
            !subscriptions.is_empty()
        });
        self.entity_subscription_count = self.entity_events.values().map(Vec::len).sum();
    }

    pub(super) fn clear(&mut self) {
        for (_, listeners) in self.scope_listeners.drain() {
            for listener in listeners {
                if let OwnedListener::Subscription(state) = listener {
                    state.cancel();
                }
            }
        }
        self.current_scope = None;
        self.scopes = component_scope::ComponentScopes::default();
        self.clicks.clear();
        self.pointers.clear();
        self.mouse_listeners.clear();
        self.key_listeners.clear();
        self.scroll_wheels.clear();
        self.touches.clear();
        self.context_menus.clear();
        self.mouse_pressures.clear();
        self.pinches.clear();
        self.rotations.clear();
        self.smart_magnifies.clear();
        self.drag_sources.clear();
        self.drops.clear();
        self.drop_order.clear();
        self.inputs.clear();
        self.submits.clear();
        self.form_submits.clear();
        self.form_invalids.clear();
        self.dismisses.clear();
        self.actions.clear();
        self.observed_entities.clear();
        self.observed_globals.clear();
        self.observes_window_state = false;
        self.observes_viewport = false;
        self.observes_displays = false;
        self.observes_keyboard_layout = false;
        self.observes_system_preferences = false;
        self.child_window_closed.clear();
        self.any_child_window_closed.clear();
        self.prune_entity_event_subscriptions();
        self.prune_global_subscriptions();
    }
}

/// An opaque click binding returned by [`ViewContext::listener`].
pub struct ClickListener<V> {
    pub(super) id: ElementId,
    pub(super) marker: PhantomData<fn(&mut V)>,
}

/// An opaque captured-pointer binding returned by [`ViewContext::pointer_listener`].
pub struct PointerListener<V> {
    pub(super) id: ElementId,
    pub(super) marker: PhantomData<fn(&mut V)>,
}

/// An opaque targeted mouse-down binding returned by [`ViewContext::mouse_down_listener`].
pub struct MouseDownListener<V> {
    pub(super) id: ElementId,
    pub(super) key: MouseListenerKey,
    pub(super) marker: PhantomData<fn(&mut V)>,
}

/// An opaque targeted mouse-up binding returned by [`ViewContext::mouse_up_listener`].
pub struct MouseUpListener<V> {
    pub(super) id: ElementId,
    pub(super) key: MouseListenerKey,
    pub(super) marker: PhantomData<fn(&mut V)>,
}

/// An opaque targeted mouse-motion binding returned by [`ViewContext::mouse_move_listener`].
pub struct MouseMoveListener<V> {
    pub(super) id: ElementId,
    pub(super) key: MouseListenerKey,
    pub(super) marker: PhantomData<fn(&mut V)>,
}

/// An opaque targeted native-window mouse-exit binding.
pub struct MouseExitListener<V> {
    pub(super) id: ElementId,
    pub(super) key: MouseListenerKey,
    pub(super) marker: PhantomData<fn(&mut V)>,
}

/// An opaque web-style hover transition binding returned by [`ViewContext::hover_listener`].
pub struct HoverListener<V> {
    pub(super) id: ElementId,
    pub(super) key: MouseListenerKey,
    pub(super) marker: PhantomData<fn(&mut V)>,
}

/// An opaque focused key-down binding returned by [`ViewContext::key_down_listener`].
pub struct KeyDownListener<V> {
    pub(super) id: ElementId,
    pub(super) key: KeyListenerKey,
    pub(super) marker: PhantomData<fn(&mut V)>,
}

/// An opaque focused key-up binding returned by [`ViewContext::key_up_listener`].
pub struct KeyUpListener<V> {
    pub(super) id: ElementId,
    pub(super) key: KeyListenerKey,
    pub(super) marker: PhantomData<fn(&mut V)>,
}

/// An opaque scroll-wheel binding returned by [`ViewContext::scroll_wheel_listener`].
pub struct ScrollWheelListener<V> {
    pub(super) id: ElementId,
    pub(super) marker: PhantomData<fn(&mut V)>,
}

/// An opaque raw-touch binding returned by [`ViewContext::touch_listener`].
pub struct TouchListener<V> {
    pub(super) id: ElementId,
    pub(super) marker: PhantomData<fn(&mut V)>,
}

/// An opaque secondary-click binding returned by [`ViewContext::context_menu_listener`].
pub struct ContextMenuListener<V> {
    pub(super) id: ElementId,
    pub(super) marker: PhantomData<fn(&mut V)>,
}

/// An opaque Force Touch binding returned by [`ViewContext::mouse_pressure_listener`].
pub struct MousePressureListener<V> {
    pub(super) id: ElementId,
    pub(super) marker: PhantomData<fn(&mut V)>,
}

/// An opaque pinch binding returned by [`ViewContext::pinch_listener`].
pub struct PinchListener<V> {
    pub(super) id: ElementId,
    pub(super) marker: PhantomData<fn(&mut V)>,
}

/// An opaque rotation binding returned by [`ViewContext::rotation_listener`].
pub struct RotationListener<V> {
    pub(super) id: ElementId,
    pub(super) marker: PhantomData<fn(&mut V)>,
}

/// An opaque smart-magnify binding returned by [`ViewContext::smart_magnify_listener`].
pub struct SmartMagnifyListener<V> {
    pub(super) id: ElementId,
    pub(super) marker: PhantomData<fn(&mut V)>,
}

/// A typed payload and optional GPU preview produced when a drag starts.
pub struct Drag<T> {
    pub(super) value: T,
    pub(super) preview: Option<Element>,
    pub(super) cursor_offset: Option<Point>,
    pub(super) external_payload: Option<ExternalDragPayload>,
}

impl<T> Drag<T> {
    pub fn new(value: T) -> Self {
        Self {
            value,
            preview: None,
            cursor_offset: None,
            external_payload: None,
        }
    }

    /// Paint this element above the application while the pointer is dragging.
    pub fn preview(mut self, preview: impl IntoElement) -> Self {
        self.preview = Some(preview.into_element());
        self
    }

    /// Position the preview at this logical offset from the pointer.
    ///
    /// Without an explicit offset, QuickGUI preserves the pointer's location inside the source
    /// element, matching a native direct-manipulation drag.
    pub fn cursor_offset(mut self, offset: Point) -> Self {
        self.cursor_offset = Some(offset);
        self
    }

    /// Add a public native representation when this drag leaves its QuickGUI window.
    ///
    /// macOS always offers the original typed Rust value to other QuickGUI windows in this process.
    /// This optional representation additionally exposes existing files/directories, plain text,
    /// or an absolute URL to native applications. The retained GPU path remains active until the
    /// pointer exits, so attaching a representation adds no idle work.
    pub fn external_payload(mut self, payload: ExternalDragPayload) -> Self {
        self.external_payload = Some(payload);
        self
    }

    /// Offer existing files or directories when this drag leaves the window.
    pub fn external_files(self, paths: FileDragPaths) -> Self {
        self.external_payload(ExternalDragPayload::Files(paths))
    }

    /// Offer bounded plain text when this drag leaves the window.
    pub fn external_text(self, text: impl Into<ExternalDragText>) -> Self {
        self.external_payload(ExternalDragPayload::Text(text.into()))
    }

    /// Offer one validated absolute URL when this drag leaves the window.
    pub fn external_url(self, url: ExternalDragUrl) -> Self {
        self.external_payload(ExternalDragPayload::Url(url))
    }
}

pub(super) struct AnyDrag {
    pub(super) value: Arc<dyn Any>,
    pub(super) value_type: TypeId,
    pub(super) preview: Option<Element>,
    pub(super) cursor_offset: Option<Point>,
    pub(super) external_payload: Option<ExternalDragPayload>,
}

impl AnyDrag {
    pub(super) fn new<T: 'static>(drag: Drag<T>) -> Self {
        Self {
            value: Arc::new(drag.value),
            value_type: TypeId::of::<T>(),
            preview: drag.preview,
            cursor_offset: drag.cursor_offset,
            external_payload: drag.external_payload,
        }
    }
}

/// An opaque typed drag-source binding returned by [`ViewContext::drag_listener`].
pub struct DragListener<V, T> {
    pub(super) id: ElementId,
    pub(super) marker: PhantomData<fn(&mut V, T)>,
}

impl<V, T> DragListener<V, T> {
    pub(crate) fn id(&self) -> ElementId {
        self.id
    }
}

/// An opaque typed drop-target binding returned by [`ViewContext::drop_listener`].
pub struct DropListener<V, T> {
    pub(super) id: ElementId,
    pub(super) marker: PhantomData<fn(&mut V, T)>,
}

impl<V, T> DropListener<V, T> {
    pub(crate) fn id(&self) -> ElementId {
        self.id
    }
}

impl<V> PointerListener<V> {
    pub(crate) fn id(&self) -> ElementId {
        self.id
    }
}

macro_rules! impl_mouse_listener_handle {
    ($name:ident) => {
        impl<V> $name<V> {
            pub(crate) fn id(&self) -> ElementId {
                self.id
            }

            pub(crate) fn key(&self) -> MouseListenerKey {
                self.key
            }
        }
    };
}

impl_mouse_listener_handle!(MouseDownListener);
impl_mouse_listener_handle!(MouseUpListener);
impl_mouse_listener_handle!(MouseMoveListener);
impl_mouse_listener_handle!(MouseExitListener);
impl_mouse_listener_handle!(HoverListener);

macro_rules! impl_key_listener_handle {
    ($name:ident) => {
        impl<V> $name<V> {
            pub(crate) fn id(&self) -> ElementId {
                self.id
            }

            pub(crate) fn key(&self) -> KeyListenerKey {
                self.key
            }
        }
    };
}

impl_key_listener_handle!(KeyDownListener);
impl_key_listener_handle!(KeyUpListener);

impl<V> ScrollWheelListener<V> {
    pub(crate) fn id(&self) -> ElementId {
        self.id
    }
}

impl<V> TouchListener<V> {
    pub(crate) fn id(&self) -> ElementId {
        self.id
    }
}

impl<V> ContextMenuListener<V> {
    pub(crate) fn id(&self) -> ElementId {
        self.id
    }
}

impl<V> MousePressureListener<V> {
    pub(crate) fn id(&self) -> ElementId {
        self.id
    }
}

impl<V> PinchListener<V> {
    pub(crate) fn id(&self) -> ElementId {
        self.id
    }
}

impl<V> RotationListener<V> {
    pub(crate) fn id(&self) -> ElementId {
        self.id
    }
}

impl<V> SmartMagnifyListener<V> {
    pub(crate) fn id(&self) -> ElementId {
        self.id
    }
}

impl<V> ClickListener<V> {
    pub(crate) fn id(&self) -> ElementId {
        self.id
    }
}

/// An opaque text-change binding returned by [`ViewContext::input_listener`].
pub struct InputListener<V> {
    pub(super) id: ElementId,
    pub(super) marker: PhantomData<fn(&mut V)>,
}

/// An opaque single-line submit binding returned by [`ViewContext::submit_listener`].
pub struct SubmitListener<V> {
    pub(super) id: ElementId,
    pub(super) marker: PhantomData<fn(&mut V)>,
}

/// An opaque valid-form binding returned by [`ViewContext::form_submit_listener`].
pub struct FormSubmitListener<V> {
    pub(super) id: ElementId,
    pub(super) marker: PhantomData<fn(&mut V)>,
}

/// An opaque invalid-form binding returned by [`ViewContext::form_invalid_listener`].
pub struct FormInvalidListener<V> {
    pub(super) id: ElementId,
    pub(super) marker: PhantomData<fn(&mut V)>,
}

impl<V> InputListener<V> {
    pub(crate) fn id(&self) -> ElementId {
        self.id
    }
}

impl<V> SubmitListener<V> {
    pub(crate) fn id(&self) -> ElementId {
        self.id
    }
}

impl<V> FormSubmitListener<V> {
    pub(crate) fn id(&self) -> ElementId {
        self.id
    }
}

impl<V> FormInvalidListener<V> {
    pub(crate) fn id(&self) -> ElementId {
        self.id
    }
}

/// An opaque dismissal binding returned by [`ViewContext::dismiss_listener`].
pub struct DismissListener<V> {
    pub(super) id: ElementId,
    pub(super) marker: PhantomData<fn(&mut V)>,
}

impl<V> DismissListener<V> {
    pub(crate) fn id(&self) -> ElementId {
        self.id
    }
}

macro_rules! impl_copy_listener_handle {
    ($name:ident<$($parameter:ident),+>) => {
        impl<$($parameter),+> Copy for $name<$($parameter),+> {}

        impl<$($parameter),+> Clone for $name<$($parameter),+> {
            fn clone(&self) -> Self {
                *self
            }
        }
    };
}

impl_copy_listener_handle!(ClickListener<V>);
impl_copy_listener_handle!(PointerListener<V>);
impl_copy_listener_handle!(MouseDownListener<V>);
impl_copy_listener_handle!(MouseUpListener<V>);
impl_copy_listener_handle!(MouseMoveListener<V>);
impl_copy_listener_handle!(MouseExitListener<V>);
impl_copy_listener_handle!(HoverListener<V>);
impl_copy_listener_handle!(ScrollWheelListener<V>);
impl_copy_listener_handle!(TouchListener<V>);
impl_copy_listener_handle!(ContextMenuListener<V>);
impl_copy_listener_handle!(MousePressureListener<V>);
impl_copy_listener_handle!(PinchListener<V>);
impl_copy_listener_handle!(RotationListener<V>);
impl_copy_listener_handle!(SmartMagnifyListener<V>);
impl_copy_listener_handle!(DragListener<V, T>);
impl_copy_listener_handle!(DropListener<V, T>);
impl_copy_listener_handle!(InputListener<V>);
impl_copy_listener_handle!(SubmitListener<V>);
impl_copy_listener_handle!(FormSubmitListener<V>);
impl_copy_listener_handle!(FormInvalidListener<V>);
impl_copy_listener_handle!(DismissListener<V>);

use super::*;

impl Terminal {
    /// Start a PTY and terminal-emulation worker. Process-launch failures are published through
    /// [`TerminalSnapshot::status`] so a declarative terminal remains mounted and informative.
    #[cfg(any(feature = "terminal", quickgui_terminal_extension))]
    pub fn spawn(
        options: TerminalOptions,
        invalidator: WindowInvalidator,
    ) -> Result<Self, TerminalError> {
        options.validate()?;
        let initial_size = TerminalSize {
            cols: options.cols,
            rows: options.rows,
            cell_width_px: 0,
            cell_height_px: 0,
        };
        let snapshot = Arc::new(RwLock::new(Arc::new(TerminalSnapshot::starting(
            options.cols,
            options.rows,
        ))));
        let (messages, receiver) = sync_channel(MESSAGE_CAPACITY);
        let shutdown = Arc::new(AtomicBool::new(false));
        let worker_shutdown = Arc::clone(&shutdown);
        let worker_messages = messages.clone();
        let worker_snapshot = Arc::clone(&snapshot);
        thread::Builder::new()
            .name("quickgui-terminal".to_owned())
            .spawn(move || {
                run_terminal_worker(
                    options,
                    initial_size,
                    receiver,
                    worker_messages,
                    worker_snapshot,
                    invalidator,
                    worker_shutdown,
                );
            })
            .map_err(|error| TerminalError::Worker(error.to_string()))?;
        Ok(Self {
            inner: Arc::new(TerminalInner {
                messages,
                snapshot,
                #[cfg(not(quickgui_terminal_extension))]
                last_size: AtomicU64::new(pack_size(initial_size)),
                #[cfg(not(quickgui_terminal_extension))]
                viewport_height_bits: AtomicU32::new(0),
                #[cfg(not(quickgui_terminal_extension))]
                viewport_bounds: Mutex::new(Rect::ZERO),
                #[cfg(not(quickgui_terminal_extension))]
                selection_epoch: Instant::now(),
                #[cfg(not(quickgui_terminal_extension))]
                wheel_remainder: Mutex::new(0.0),
                #[cfg(not(quickgui_terminal_extension))]
                scrollbar_drag_remainder: Mutex::new(0.0),
                #[cfg(not(quickgui_terminal_extension))]
                scrollbar_interaction: Mutex::new(TerminalScrollbarInteraction::default()),
                #[cfg(not(quickgui_terminal_extension))]
                cursor_blink: Mutex::new(TerminalCursorBlink::new(Instant::now())),
                #[cfg(not(quickgui_terminal_extension))]
                theme: Mutex::new(None),
                shutdown,
            }),
        })
    }

    /// Read the latest complete terminal frame without locking the emulator or PTY.
    pub fn snapshot(&self) -> Arc<TerminalSnapshot> {
        #[cfg(any(feature = "terminal", quickgui_terminal_extension))]
        {
            self.inner
                .snapshot
                .read()
                .unwrap_or_else(|p| p.into_inner())
                .clone()
        }
    }

    /// Write raw bytes to the PTY. Prefer normal keyboard events or [`Self::paste`] for user input.
    #[cfg(not(quickgui_terminal_extension))]
    pub fn write(&self, bytes: impl AsRef<[u8]>) -> bool {
        let bytes = bytes.as_ref();
        if bytes.is_empty() || bytes.len() > MAX_TERMINAL_INPUT_BYTES {
            return false;
        }
        let sent = self.try_send(WorkerMessage::Input(bytes.to_vec()));
        if sent {
            self.reset_cursor_blink();
        }
        sent
    }

    /// Encode text using libghostty's bracketed-paste and control-byte rules, then write it.
    #[cfg(not(quickgui_terminal_extension))]
    pub fn paste(&self, value: impl Into<String>) -> bool {
        let value = value.into();
        if value.is_empty() || value.len() > MAX_TERMINAL_INPUT_BYTES {
            return false;
        }
        let sent = self.try_send(WorkerMessage::Paste(value));
        if sent {
            self.reset_cursor_blink();
        }
        sent
    }

    /// Scroll libghostty's retained viewport by terminal rows. Negative values move into history.
    #[cfg(not(quickgui_terminal_extension))]
    pub fn scroll_rows(&self, rows: isize) -> bool {
        let sent = rows != 0
            && self.try_send(WorkerMessage::Scroll(
                rows.clamp(-(MAX_ROWS as isize), MAX_ROWS as isize),
            ));
        if sent {
            self.reveal_scrollbar();
        }
        sent
    }

    #[cfg(not(quickgui_terminal_extension))]
    fn scroll_pixel_delta(&self, delta_y: f32, line_height: f32, phase: GesturePhase) -> bool {
        if matches!(phase, GesturePhase::Ended | GesturePhase::Cancelled) {
            *self
                .inner
                .wheel_remainder
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = 0.0;
            return false;
        }
        if !delta_y.is_finite() || delta_y == 0.0 {
            return false;
        }
        self.reveal_scrollbar();
        let mut remainder = self
            .inner
            .wheel_remainder
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let rows = accumulate_terminal_rows(&mut remainder, -delta_y / line_height.max(1.0));
        drop(remainder);
        let _ = self.scroll_rows(rows);
        true
    }

    #[cfg(not(quickgui_terminal_extension))]
    fn update_viewport_height(&self, height: f32) {
        let height = if height.is_finite() {
            height.max(0.0)
        } else {
            0.0
        };
        self.inner
            .viewport_height_bits
            .store(height.to_bits(), Ordering::Relaxed);
    }

    #[cfg(not(quickgui_terminal_extension))]
    fn update_viewport_bounds(&self, bounds: Rect) {
        self.update_viewport_height(bounds.height);
        *self
            .inner
            .viewport_bounds
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = bounds;
    }

    #[cfg(not(quickgui_terminal_extension))]
    fn send_selection_pointer(
        &self,
        event: &PointerEvent,
        cols: u16,
        rows: u16,
        cell_metrics: CellMetrics,
    ) -> bool {
        let bounds = *self
            .inner
            .viewport_bounds
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(input) = terminal_selection_input(
            event,
            bounds,
            cols,
            rows,
            cell_metrics.logical_width,
            cell_metrics.logical_height,
            cell_metrics.physical_width,
            cell_metrics.scale_factor,
            Instant::now().saturating_duration_since(self.inner.selection_epoch),
        ) else {
            return false;
        };
        self.try_send(WorkerMessage::Selection(input))
    }

    #[cfg(not(quickgui_terminal_extension))]
    fn viewport_height(&self) -> f32 {
        f32::from_bits(self.inner.viewport_height_bits.load(Ordering::Relaxed))
    }

    #[cfg(not(quickgui_terminal_extension))]
    fn begin_scrollbar_drag(&self) {
        *self
            .inner
            .scrollbar_drag_remainder
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = 0.0;
        self.inner
            .scrollbar_interaction
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .begin_drag(Instant::now());
    }

    #[cfg(not(quickgui_terminal_extension))]
    fn end_scrollbar_drag(&self) {
        *self
            .inner
            .scrollbar_drag_remainder
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = 0.0;
        self.inner
            .scrollbar_interaction
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .end_drag(Instant::now());
    }

    #[cfg(not(quickgui_terminal_extension))]
    fn set_scrollbar_hovered(&self, hovered: bool) {
        self.inner
            .scrollbar_interaction
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .set_hovered(hovered, Instant::now());
    }

    #[cfg(not(quickgui_terminal_extension))]
    fn reveal_scrollbar(&self) {
        self.inner
            .scrollbar_interaction
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .reveal(Instant::now());
    }

    #[cfg(not(quickgui_terminal_extension))]
    fn scrollbar_presentation(&self, now: Instant) -> TerminalScrollbarPresentation {
        self.inner
            .scrollbar_interaction
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .presentation(now)
    }

    #[cfg(not(quickgui_terminal_extension))]
    fn reset_cursor_blink(&self) {
        self.inner
            .cursor_blink
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .reset(Instant::now());
    }

    #[cfg(not(quickgui_terminal_extension))]
    fn cursor_blink_epoch_for_focus(&self, focused: bool, now: Instant) -> Instant {
        self.inner
            .cursor_blink
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .update_focus(focused, now)
    }

    #[cfg(not(quickgui_terminal_extension))]
    fn drag_scrollbar(&self, delta_y: f32) -> bool {
        if !delta_y.is_finite() || delta_y == 0.0 {
            return false;
        }
        let snapshot = self.snapshot();
        let Some(geometry) = terminal_scrollbar_geometry(snapshot.scroll, self.viewport_height())
        else {
            return false;
        };
        if geometry.travel <= 0.0 || geometry.max_offset_rows == 0 {
            return false;
        }
        let mut remainder = self
            .inner
            .scrollbar_drag_remainder
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let rows = accumulate_terminal_rows(
            &mut remainder,
            delta_y * geometry.max_offset_rows as f32 / geometry.travel,
        );
        drop(remainder);
        let _ = self.scroll_rows(rows);
        true
    }

    /// Resize both the PTY and libghostty viewport, coalescing identical measurements.
    #[cfg(not(quickgui_terminal_extension))]
    pub fn resize(&self, cols: u16, rows: u16, cell_width_px: u32, cell_height_px: u32) -> bool {
        let size = TerminalSize {
            cols: cols.clamp(1, MAX_COLS),
            rows: rows.clamp(1, MAX_ROWS),
            cell_width_px,
            cell_height_px,
        };
        let packed = pack_size(size);
        if self.inner.last_size.swap(packed, Ordering::Relaxed) == packed {
            return true;
        }
        self.try_send(WorkerMessage::Resize(size))
    }

    /// Build the complete retained terminal surface, including focus, keyboard, paste, scroll, and
    /// exact bounds-to-PTY resize behavior.
    #[cfg(not(quickgui_terminal_extension))]
    pub fn element<V: 'static>(
        &self,
        id: impl Into<ElementId>,
        style: TerminalStyle,
        cx: &mut ViewContext<'_, V>,
    ) -> Element {
        let id = id.into();
        let style = style.normalized();
        self.sync_theme(style.theme.clone());
        let snapshot = self.snapshot();
        let scale_factor = cx.scale_factor();
        let cell_metrics = CellMetrics::new(
            style.font_size,
            style.line_height,
            style.cell_width_ratio,
            scale_factor,
        );
        let cell_width = cell_metrics.logical_width;
        let line_height = cell_metrics.logical_height;
        let focus = cx.focus_handle(id);
        let focused = cx.window_state().focused && cx.is_focused(focus);
        let now = Instant::now();

        let keyboard = self.clone();
        let key_down = cx.key_down_listener(id, move |_view, event, event_cx| {
            if handle_terminal_copy(&keyboard, event, event_cx)
                || handle_terminal_select_all(&keyboard, event, event_cx)
            {
                return;
            }
            if handle_terminal_paste(&keyboard, event, event_cx) {
                return;
            }
            if event.modifiers.contains(Modifiers::SUPER) {
                return;
            }
            if keyboard.try_send(WorkerMessage::Key(TerminalKeyInput::down(event))) {
                keyboard.reset_cursor_blink();
                event_cx.clear_text_selection();
                event_cx.prevent_default();
                event_cx.stop_propagation();
                event_cx.invalidate();
            }
        });
        let keyboard = self.clone();
        let key_up = cx.key_up_listener(id, move |_view, event, event_cx| {
            if event.modifiers.contains(Modifiers::SUPER) {
                return;
            }
            if keyboard.try_send(WorkerMessage::Key(TerminalKeyInput::up(event))) {
                event_cx.prevent_default();
                event_cx.stop_propagation();
            }
        });
        let scroll_terminal = self.clone();
        let scroll = cx.scroll_wheel_listener(id, move |_view, event, event_cx| {
            let delta = event.delta.pixel_delta(line_height).y;
            if scroll_terminal.scroll_pixel_delta(delta, line_height, event.phase) {
                event_cx.prevent_default();
                event_cx.stop_propagation();
                event_cx.invalidate();
            }
        });

        let resize_terminal = self.clone();
        let resize_probe = canvas(move |_bounds, canvas| {
            let bounds = canvas.bounds();
            resize_terminal.update_viewport_bounds(bounds);
            let cols = (bounds.width / cell_width)
                .floor()
                .clamp(1.0, MAX_COLS as f32) as u16;
            let rows = (bounds.height / line_height)
                .floor()
                .clamp(1.0, MAX_ROWS as f32) as u16;
            let _ = resize_terminal.resize(
                cols,
                rows,
                cell_metrics.physical_width,
                cell_metrics.physical_height,
            );
        })
        .absolute()
        .inset_0()
        .size_full();

        let blink_epoch = self.cursor_blink_epoch_for_focus(focused, now);
        let cursor = snapshot
            .cursor
            .and_then(|cursor| terminal_cursor_presentation(cursor, focused, blink_epoch, now, cx));
        let cursor_color = if style.theme.is_some() {
            snapshot
                .cursor
                .map_or(snapshot.foreground, |cursor| cursor.color)
        } else {
            style.foreground.unwrap_or_else(|| {
                snapshot
                    .cursor
                    .map_or(snapshot.foreground, |cursor| cursor.color)
            })
        };
        let terminal_background = style.background.unwrap_or(snapshot.background);
        let padding_extension = (style.padding_color == TerminalPaddingColor::Extend).then(|| {
            let edges = snapshot.edge_backgrounds.clone();
            let padding_top = style.padding_top;
            let padding_left = style.padding_left;
            canvas(move |_bounds, canvas| {
                paint_padding_extension(canvas, &edges, cell_metrics, padding_top, padding_left);
            })
            .absolute()
            .inset_0()
            .size_full()
        });
        let block_cursor_graphic = cursor == Some(TerminalCursorStyle::Block)
            && snapshot.cursor.is_some_and(|cursor| {
                snapshot
                    .graphics
                    .iter()
                    .any(|graphic| graphic.column == cursor.column && graphic.row == cursor.row)
            });
        let highlights = if cursor == Some(TerminalCursorStyle::Block)
            && let Some(range) = snapshot.cursor_range.clone()
        {
            terminal_highlights_with_cursor(
                &snapshot.highlights,
                range,
                HighlightStyle::default()
                    .color(if block_cursor_graphic {
                        Color::TRANSPARENT
                    } else {
                        terminal_background
                    })
                    .background(cursor_color),
            )
        } else {
            snapshot.highlights.to_vec()
        };
        let styled = StyledText::new(snapshot.content.clone()).with_highlights(highlights);
        let terminal_text = styled
            .into_element()
            .id(derived_terminal_id(id, TERMINAL_TEXT_ID_TAG))
            .font_family(style.font_family)
            .text_size(style.font_size)
            .font_thicken(style.font_thicken)
            .line_height(line_height)
            .monospace_width(cell_width)
            .text_color(style.foreground.unwrap_or(snapshot.foreground))
            .whitespace_nowrap()
            .text_shaping_basic();

        let terminal_graphics = (!snapshot.graphics.is_empty()).then(|| {
            let graphics = snapshot.graphics.clone();
            let default_foreground = style.foreground.unwrap_or(snapshot.foreground);
            let block_cursor = (cursor == Some(TerminalCursorStyle::Block))
                .then_some(snapshot.cursor)
                .flatten();
            canvas(move |_bounds, canvas| {
                for graphic in graphics.iter() {
                    let color = if block_cursor.is_some_and(|cursor| {
                        cursor.column == graphic.column && cursor.row == graphic.row
                    }) {
                        terminal_background
                    } else {
                        graphic.foreground.unwrap_or(default_foreground)
                    };
                    paint_block(
                        canvas,
                        graphic.column,
                        graphic.row,
                        graphic.character,
                        cell_metrics,
                        color,
                    );
                }
            })
            .absolute()
            .inset_0()
            .size_full()
        });

        let cursor_element = cursor.zip(snapshot.cursor).map(|(cursor_style, cursor)| {
            terminal_cursor_element(cursor, cursor_style, cell_width, line_height, cursor_color)
        });
        let selection_id = derived_terminal_id(id, TERMINAL_SELECTION_ID_TAG);
        let selection_cols = snapshot.cols;
        let selection_rows = snapshot.rows;
        let selection_terminal = self.clone();
        let selection_pointer = cx.pointer_listener(selection_id, move |_view, event, event_cx| {
            if event.button != MouseButton::Left {
                return;
            }
            if event.phase == PointerPhase::Down {
                event_cx.focus(focus);
            }
            if selection_terminal.send_selection_pointer(
                event,
                selection_cols,
                selection_rows,
                cell_metrics,
            ) {
                event_cx.clear_text_selection();
                event_cx.prevent_default();
                event_cx.stop_propagation();
                event_cx.invalidate();
            }
        });
        let mut terminal_content = div()
            .id(selection_id)
            .absolute()
            .top(style.padding_top)
            .right(style.padding_right)
            .bottom(style.padding_bottom)
            .left(style.padding_left)
            .min_w(0.0)
            .min_h(0.0)
            .overflow_hidden()
            .on_pointer(selection_pointer)
            .child(resize_probe);
        if cursor == Some(TerminalCursorStyle::Block)
            && let Some(cursor_element) = cursor_element.clone()
        {
            terminal_content = terminal_content.child(cursor_element);
        }
        terminal_content = terminal_content.child(terminal_text);
        if let Some(terminal_graphics) = terminal_graphics {
            terminal_content = terminal_content.child(terminal_graphics);
        }
        if cursor != Some(TerminalCursorStyle::Block)
            && let Some(cursor_element) = cursor_element
        {
            terminal_content = terminal_content.child(cursor_element);
        }

        let mut terminal = div()
            .id(id)
            .relative()
            .min_w(0.0)
            .min_h(0.0)
            .overflow_hidden()
            .bg(terminal_background)
            .track_focus(focus)
            .accessibility_role(AccessibilityRole::Group)
            .accessibility_label("Terminal")
            .cursor_text();
        if let Some(padding_extension) = padding_extension {
            terminal = terminal.child(padding_extension);
        }
        terminal = terminal
            .child(terminal_content)
            .on_key_down(key_down)
            .on_key_up(key_up)
            .on_scroll_wheel(scroll);

        if let Some(geometry) = terminal_scrollbar_geometry(snapshot.scroll, self.viewport_height())
        {
            let scrollbar_id = derived_terminal_id(id, TERMINAL_SCROLLBAR_ID_TAG);
            let scrollbar = self.scrollbar_presentation(now);
            if let Some(deadline) = scrollbar.hide_at {
                cx.request_repaint_at(deadline);
            }
            let hover_terminal = self.clone();
            let hover = cx.hover_listener(scrollbar_id, move |_view, hovered, event_cx| {
                hover_terminal.set_scrollbar_hovered(*hovered);
                event_cx.invalidate();
            });
            let scrollbar_terminal = self.clone();
            let pointer = cx.pointer_listener(scrollbar_id, move |_view, event, event_cx| {
                if event.button != MouseButton::Left {
                    return;
                }
                match event.phase {
                    PointerPhase::Down => scrollbar_terminal.begin_scrollbar_drag(),
                    PointerPhase::Move => {
                        let _ = scrollbar_terminal.drag_scrollbar(event.delta.y);
                    }
                    PointerPhase::Up | PointerPhase::Cancel => {
                        scrollbar_terminal.end_scrollbar_drag();
                    }
                }
                event_cx.clear_text_selection();
                event_cx.prevent_default();
                event_cx.stop_propagation();
                event_cx.invalidate();
            });
            let thumb_width = if scrollbar.expanded {
                TERMINAL_SCROLLBAR_HOVER_WIDTH
            } else {
                TERMINAL_SCROLLBAR_IDLE_WIDTH
            };
            let mut track = div()
                .id(scrollbar_id)
                .absolute()
                .top(style.padding_top)
                .right(0.0)
                .bottom(style.padding_bottom)
                .w(TERMINAL_SCROLLBAR_HIT_WIDTH)
                .cursor_default()
                .on_hover(hover)
                .on_pointer(pointer);
            if scrollbar.visible {
                track = track.child(
                    div()
                        .absolute()
                        .top(geometry.thumb_top + TERMINAL_SCROLLBAR_VERTICAL_INSET)
                        .right(TERMINAL_SCROLLBAR_EDGE_INSET)
                        .w(thumb_width)
                        .h(
                            (geometry.thumb_height - TERMINAL_SCROLLBAR_VERTICAL_INSET * 2.0)
                                .max(thumb_width),
                        )
                        .rounded(thumb_width * 0.5)
                        .bg(if scrollbar.expanded {
                            Color::rgba8(142, 147, 160, 210)
                        } else {
                            Color::rgba8(142, 147, 160, 160)
                        }),
                );
            }
            terminal = terminal.child(track);
        }

        terminal
    }

    pub(super) fn try_send(&self, message: WorkerMessage) -> bool {
        #[cfg(any(feature = "terminal", quickgui_terminal_extension))]
        {
            self.inner.messages.try_send(message).is_ok()
        }
    }

    #[cfg(not(quickgui_terminal_extension))]
    fn sync_theme(&self, theme: Option<TerminalTheme>) {
        let mut current = self
            .inner
            .theme
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if *current == theme {
            return;
        }
        if self.try_send(WorkerMessage::Theme(theme.clone().map(Box::new))) {
            *current = theme;
        }
    }
}

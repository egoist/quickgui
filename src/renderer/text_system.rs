use super::*;

impl TextSystem {
    pub(super) fn upload_stats(&self) -> super::upload::UploadStats {
        let mut stats =
            self.batches
                .iter()
                .fold(super::upload::UploadStats::default(), |mut stats, batch| {
                    let renderer = &self.renderers[batch.renderer];
                    stats.bytes += renderer.last_upload_bytes() as u64;
                    stats.writes += renderer.last_upload_writes();
                    stats.reused += usize::from(renderer.last_upload_writes() == 0);
                    stats
                });
        stats.shadow_bytes = self
            .renderers
            .iter()
            .map(|renderer| renderer.retained_upload_bytes())
            .sum();
        stats
    }

    pub(super) fn new(
        device: &Device,
        queue: &Queue,
        format: TextureFormat,
        font_system: SharedFontSystem,
        shared_cache: Option<&Cache>,
    ) -> Self {
        let swash_cache = SwashCache::new();
        let cache = shared_cache.cloned().unwrap_or_else(|| Cache::new(device));
        let viewport = Viewport::new(device, &cache);
        let color_mode = if cfg!(any(target_os = "windows", target_os = "linux")) {
            glyphon::ColorMode::Platform
        } else {
            glyphon::ColorMode::Web
        };
        let mut atlas = TextAtlas::with_color_mode(device, queue, &cache, format, color_mode);
        let renderer = TextRenderer::new(&mut atlas, device, MultisampleState::default(), None);
        Self {
            font_system,
            cache,
            swash_cache,
            viewport,
            atlas,
            renderers: vec![renderer],
            buffers: HashMap::with_capacity(512),
            shared_buffers: HashMap::with_capacity(512),
            colors: HashMap::with_capacity(MAX_RETAINED_TEXT_COLORS),
            seen: HashSet::with_capacity(128),
            visible: Vec::with_capacity(128),
            batches: Vec::with_capacity(16),
            layer_batches: Vec::with_capacity(4),
            eviction_keys: Vec::new(),
            shared_eviction_keys: Vec::new(),
            frame: 0,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn prepare(
        &mut self,
        device: &Device,
        queue: &Queue,
        scene: &Scene,
        viewport_rect: Rect,
        physical_width: u32,
        physical_height: u32,
        scale: f32,
    ) -> Result<(usize, usize, usize), RendererError> {
        self.frame = self.frame.wrapping_add(1);
        self.seen.clear();
        self.visible.clear();
        self.batches.clear();
        self.layer_batches.clear();
        let mut text_count = 0;
        let mut reshaped = 0;

        for layer in scene.paint_layers() {
            let start = self.visible.len();
            for item in layer.paint() {
                let PrimitiveRef::Text(index) = item.primitive else {
                    continue;
                };
                let run = &layer.text_runs()[index];
                let clip = run.clip.unwrap_or(viewport_rect);
                let Some(clip) = clip.intersection(viewport_rect) else {
                    continue;
                };
                if !run.bounds.intersects(clip) {
                    continue;
                }
                if !self.seen.insert(run.id) {
                    return Err(RendererError::DuplicateTextId(run.id));
                }
                text_count += 1;
                let color_key = [
                    run.style.color.r.to_bits(),
                    run.style.color.g.to_bits(),
                    run.style.color.b.to_bits(),
                    run.style.color.a.to_bits(),
                ];
                let color = if let Some(color) = self.colors.get(&color_key) {
                    *color
                } else {
                    if self.colors.len() >= MAX_RETAINED_TEXT_COLORS {
                        self.colors.clear();
                    }
                    let rgba = run.style.color.to_srgba8();
                    let color = glyphon::Color::rgba(rgba[0], rgba[1], rgba[2], rgba[3]);
                    self.colors.insert(color_key, color);
                    color
                };
                let bounds = physical_text_bounds(clip, scale);
                let left = run.bounds.x * scale;
                // Glyphon truncates Y when hinting. Round the origin first so a downward
                // eased translation does not sit one pixel short until its exact endpoint.
                let top = (run.bounds.y * scale).round();
                let run_reshaped = if run.highlights.is_none()
                    && should_fragment_basic_text(&run.content, &run.style)
                {
                    let mut fragment_left = left;
                    let mut fragment_reshaped = false;
                    for fragment in BasicTextFragments::new(&run.content) {
                        let content = Arc::<str>::from(fragment);
                        let (buffer, _projection, was_reshaped) = self
                            .update_shared_text_entry(content, &run.style, None, scale, self.frame);
                        let width = text_buffer_width(&buffer);
                        self.visible.push(VisibleText {
                            order: item.order,
                            buffer,
                            left: fragment_left,
                            top,
                            bounds,
                            color,
                            opacity: run.opacity,
                        });
                        fragment_left += width;
                        fragment_reshaped |= was_reshaped;
                    }
                    fragment_reshaped
                } else {
                    let was_reshaped = self.update_text_entry(
                        run.id,
                        &run.content,
                        &run.style,
                        run.highlights.as_ref(),
                        Some(run.bounds.width),
                        scale,
                        self.frame,
                    );
                    self.visible.push(VisibleText {
                        order: item.order,
                        buffer: Arc::clone(&self.buffers[&run.id].buffer),
                        left,
                        top,
                        bounds,
                        color,
                        opacity: run.opacity,
                    });
                    was_reshaped
                };
                if run_reshaped {
                    reshaped += 1;
                }
            }
            self.visible[start..].sort_unstable_by_key(|text| text.order);
            let batch_start = self.batches.len();
            let mut visible_start = start;
            while visible_start < self.visible.len() {
                let order = self.visible[visible_start].order;
                let visible_end = self.visible[visible_start..]
                    .iter()
                    .position(|text| text.order != order)
                    .map_or(self.visible.len(), |offset| visible_start + offset);
                self.batches.push(TextBatch {
                    order,
                    visible: visible_start..visible_end,
                    renderer: self.batches.len(),
                });
                visible_start = visible_end;
            }
            self.layer_batches.push(batch_start..self.batches.len());
        }

        self.viewport.update(
            queue,
            Resolution {
                width: physical_width,
                height: physical_height,
            },
        );

        let draw_calls = self.batches.len();
        while self.renderers.len() < draw_calls.max(1) {
            self.renderers.push(TextRenderer::new(
                &mut self.atlas,
                device,
                MultisampleState::default(),
                None,
            ));
        }

        let font_system = self.font_system.clone();
        let Self {
            swash_cache,
            viewport,
            atlas,
            renderers,
            visible,
            batches,
            ..
        } = self;
        let mut font_system = font_system.borrow_mut();
        for batch in batches.iter() {
            let areas = visible[batch.visible.clone()].iter().map(|item| TextArea {
                buffer: item.buffer.as_ref(),
                left: item.left,
                top: item.top,
                scale: 1.0,
                bounds: item.bounds,
                default_color: item.color,
                opacity: item.opacity,
                custom_glyphs: &[],
            });
            renderers[batch.renderer].prepare(
                device,
                queue,
                &mut font_system,
                atlas,
                viewport,
                areas,
                swash_cache,
            )?;
        }
        Ok((text_count, reshaped, draw_calls))
    }

    pub(super) fn measure(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        highlights: Option<&Arc<[TextHighlight]>>,
        max_width: Option<f32>,
        scale: f32,
    ) -> Size {
        let width = canonical_text_width(
            style.wrap,
            style.align,
            style.text_overflow.is_some(),
            max_width,
        );
        let next_frame = self.frame.wrapping_add(1);
        self.update_text_entry(id, content, style, highlights, width, scale, next_frame);
        let buffer = &self.buffers[&id].buffer;
        let mut measured_width = 0.0_f32;
        let mut measured_height = 0.0_f32;
        for run in buffer
            .layout_runs()
            .take(style.line_clamp.unwrap_or(usize::MAX))
        {
            measured_width = measured_width.max(run.line_w);
            measured_height = measured_height.max(run.line_top + run.line_height);
        }
        if measured_height == 0.0 {
            measured_height = style.line_height * scale;
        }
        // Intrinsic layout uses logical pixels. Round after converting from device pixels
        // so Retina labels do not gain an extra physical guard pixel.
        let measured_width = (measured_width / scale).ceil();
        let measured_width = match width {
            Some(width) => measured_width.min(width),
            None => measured_width,
        };
        Size::new(measured_width, measured_height.ceil() / scale)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn caret_position(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        highlights: Option<&Arc<[TextHighlight]>>,
        width: f32,
        scale: f32,
        index: usize,
    ) -> Point {
        let next_frame = self.frame.wrapping_add(1);
        self.update_text_entry(
            id,
            content,
            style,
            highlights,
            Some(width),
            scale,
            next_frame,
        );
        let entry = &self.buffers[&id];
        let display_index = entry.projection.original_to_display(index);
        let cursor = text_cursor_for_byte_index(&entry.projection.content, display_index);
        entry
            .buffer
            .cursor_position(&cursor)
            .map(|(x, y)| Point::new(x / scale, y / scale))
            .unwrap_or(Point::ZERO)
    }

    pub(super) fn index_for_point(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        highlights: Option<&Arc<[TextHighlight]>>,
        hit: TextHitTest,
    ) -> usize {
        let next_frame = self.frame.wrapping_add(1);
        self.update_text_entry(
            id,
            content,
            style,
            highlights,
            Some(hit.width),
            hit.scale,
            next_frame,
        );
        let entry = &self.buffers[&id];
        entry
            .buffer
            .hit(
                hit.point.x.max(0.0) * hit.scale,
                hit.point.y.max(0.0) * hit.scale,
            )
            .map(|cursor| byte_index_for_text_cursor(&entry.projection.content, cursor))
            .map(|index| entry.projection.display_to_original(index))
            .unwrap_or(content.len())
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn selection_rects(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        highlights: Option<&Arc<[TextHighlight]>>,
        width: f32,
        scale: f32,
        visible_y: Range<f32>,
        start: usize,
        end: usize,
    ) -> Vec<Rect> {
        let next_frame = self.frame.wrapping_add(1);
        self.update_text_entry(
            id,
            content,
            style,
            highlights,
            Some(width),
            scale,
            next_frame,
        );
        let entry = &self.buffers[&id];
        let display_ranges = entry.projection.display_ranges_for_original(start..end);
        let mut rectangles = Vec::new();
        for display_range in display_ranges {
            let start = text_cursor_for_byte_index(&entry.projection.content, display_range.start);
            let end = text_cursor_for_byte_index(&entry.projection.content, display_range.end);
            for run in entry
                .buffer
                .layout_runs()
                .take(style.line_clamp.unwrap_or(usize::MAX))
            {
                let top = run.line_top / scale;
                let bottom = top + run.line_height / scale;
                if top >= visible_y.end || bottom <= visible_y.start {
                    continue;
                }
                let line_top = run.line_top;
                rectangles.extend(text_selection_spans(&run, start, end).into_iter().map(
                    |(x, width)| {
                        Rect::new(
                            x / scale,
                            line_top / scale,
                            width / scale,
                            run.line_height / scale,
                        )
                    },
                ));
            }
        }
        rectangles
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn text_geometry(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        highlights: Option<&Arc<[TextHighlight]>>,
        width: f32,
        scale: f32,
        visible_y: Range<f32>,
    ) -> StyledTextGeometry {
        let next_frame = self.frame.wrapping_add(1);
        self.update_text_entry(
            id,
            content,
            style,
            highlights,
            Some(width),
            scale,
            next_frame,
        );
        let entry = &self.buffers[&id];
        collect_styled_text_geometry(
            entry.buffer.as_ref(),
            entry.projection.highlights.as_deref().unwrap_or_default(),
            style,
            scale,
            visible_y,
            style.line_clamp,
        )
    }

    pub(crate) fn render_order<'pass>(
        &'pass self,
        pass: &mut wgpu::RenderPass<'pass>,
        layer: usize,
        order: u32,
    ) -> Result<(), RendererError> {
        let Some(batches) = self.layer_batches.get(layer) else {
            return Ok(());
        };
        let Some(batch) = self.batches[batches.clone()]
            .iter()
            .find(|batch| batch.order == order)
        else {
            return Ok(());
        };
        self.renderers[batch.renderer].render(&self.atlas, &self.viewport, pass)?;
        Ok(())
    }

    pub(super) fn finish_frame(&mut self) {
        // Glyphon's prepared renderers own all GPU state needed after command submission. Drop
        // the temporary CPU-buffer references now instead of retaining them until the next
        // `prepare`, which both lowers idle memory ownership and lets a stable text id reclaim its
        // uniquely cached Buffer for width-only reflow during the following layout pass.
        self.visible.clear();
        self.atlas.trim();
        if self.renderers.len() > MAX_RETAINED_TEXT_RENDERERS {
            self.renderers.truncate(MAX_RETAINED_TEXT_RENDERERS);
        }
        let oldest_allowed = self.frame.saturating_sub(TEXT_RETENTION_FRAMES);
        self.eviction_keys.clear();
        self.eviction_keys.extend(
            self.buffers
                .iter()
                .filter_map(|(id, entry)| (entry.last_used_frame < oldest_allowed).then_some(*id)),
        );
        for id in self.eviction_keys.drain(..) {
            self.buffers.remove(&id);
        }

        if self.buffers.len() > MAX_RETAINED_TEXT_AREAS {
            let mut by_age: Vec<_> = self
                .buffers
                .iter()
                .map(|(id, entry)| (*id, entry.last_used_frame))
                .collect();
            by_age.sort_unstable_by_key(|(_, age)| *age);
            let remove_count = self.buffers.len() - MAX_RETAINED_TEXT_AREAS;
            for (id, _) in by_age.into_iter().take(remove_count) {
                self.buffers.remove(&id);
            }
        }

        self.shared_eviction_keys.clear();
        self.shared_eviction_keys.extend(
            self.shared_buffers
                .iter()
                .filter(|(_, entry)| entry.last_used_frame < oldest_allowed)
                .map(|(key, _)| key.clone()),
        );
        for key in self.shared_eviction_keys.drain(..) {
            self.shared_buffers.remove(&key);
        }

        if self.shared_buffers.len() > MAX_RETAINED_TEXT_LAYOUTS {
            let mut by_age: Vec<_> = self
                .shared_buffers
                .iter()
                .map(|(key, entry)| (key.clone(), entry.last_used_frame))
                .collect();
            by_age.sort_unstable_by_key(|(_, age)| *age);
            let remove_count = self.shared_buffers.len() - MAX_RETAINED_TEXT_LAYOUTS;
            for (key, _) in by_age.into_iter().take(remove_count) {
                self.shared_buffers.remove(&key);
            }
        }
    }

    pub(super) fn retained_counts(&self) -> (usize, usize, usize) {
        (
            self.buffers.len(),
            self.shared_buffers.len(),
            self.renderers.len(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn update_text_entry(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        highlights: Option<&Arc<[TextHighlight]>>,
        width: Option<f32>,
        scale: f32,
        frame: u64,
    ) -> bool {
        // Left-aligned unwrapped glyph layout is independent of paint bounds. Other alignments need
        // the assigned width so shaping, hit testing, selection, and rendering share exact x data.
        let width = canonical_text_width(
            style.wrap,
            style.align,
            style.text_overflow.is_some(),
            width,
        );
        let key = TextLayoutKey {
            content: content.clone(),
            highlights: highlights.cloned(),
            width,
            font_size: style.font_size,
            line_height: style.line_height,
            monospace_width: style.monospace_width,
            family: style.family.clone(),
            features: style.features.clone(),
            fallbacks: normalize_fallbacks(style.fallbacks.clone()),
            weight: style.weight,
            font_style: style.font_style,
            font_thicken: style.font_thicken,
            underline: style.underline,
            underline_color: style.underline_color,
            underline_wavy: style.underline_wavy,
            underline_thickness: style.underline_thickness,
            strikethrough: style.strikethrough,
            strikethrough_color: style.strikethrough_color,
            align: style.align,
            wrap: style.wrap,
            text_overflow: style.text_overflow.clone(),
            line_clamp: style.line_clamp,
            shaping: style.shaping,
            extras: TextShapingExtras::from_style(style),
            scale,
        };

        if let Some(buffer) = self.buffers.get_mut(&id).and_then(|entry| {
            entry.last_used_frame = frame;
            (entry.key == key).then(|| Arc::clone(&entry.buffer))
        }) {
            self.shared_buffers
                .entry(key)
                .and_modify(|entry| entry.last_used_frame = frame)
                .or_insert(SharedTextEntry {
                    buffer,
                    projection: Arc::clone(&self.buffers[&id].projection),
                    last_used_frame: frame,
                });
            return false;
        }

        let previous = self.buffers.remove(&id);

        // Prefer a layout already shared by another text id or retained from a recent frame.
        if let Some(entry) = self.shared_buffers.get_mut(&key) {
            entry.last_used_frame = frame;
            self.buffers.insert(
                id,
                TextEntry {
                    key,
                    buffer: Arc::clone(&entry.buffer),
                    projection: Arc::clone(&entry.projection),
                    last_used_frame: frame,
                },
            );
            return false;
        }

        // A stable text id commonly changes only its width while a window is being resized. The
        // per-id entry and shared-layout entry are normally the only owners after `finish_frame`.
        // Reclaim that Buffer and ask Cosmic Text for a relayout; its shaped line cache remains
        // intact. If another id genuinely shares the old layout, keep it immutable and fall back
        // to the normal shared/new-buffer path.
        let reusable = previous.and_then(|entry| {
            let shared = self.shared_buffers.get(&entry.key)?;
            if !Arc::ptr_eq(&entry.buffer, &shared.buffer) || Arc::strong_count(&entry.buffer) != 2
            {
                return None;
            }
            self.shared_buffers.remove(&entry.key);
            if entry.key.same_except_width(&key) {
                Arc::try_unwrap(entry.buffer)
                    .ok()
                    .map(|buffer| (buffer, entry.projection))
            } else {
                // A stable id with a projected/truncated layout cannot reflow the shortened
                // buffer, but its superseded width must not linger for the retention window.
                // Removing the sole shared owner drops that CPU buffer immediately while layouts
                // genuinely shared by another id remain cached.
                None
            }
        });

        let (buffer, projection, reshaped) = if let Some((mut buffer, projection)) = reusable {
            let mut font_system = self.font_system.borrow_mut();
            reflow_text_buffer(&mut buffer, &mut font_system, key.width, scale);
            drop(font_system);
            let buffer = Arc::new(buffer);
            self.shared_buffers.insert(
                key.clone(),
                SharedTextEntry {
                    buffer: Arc::clone(&buffer),
                    projection: Arc::clone(&projection),
                    last_used_frame: frame,
                },
            );
            (buffer, projection, true)
        } else {
            self.update_shared_text_entry_for_key(key.clone(), style, scale, frame)
        };
        self.buffers.insert(
            id,
            TextEntry {
                key,
                buffer,
                projection,
                last_used_frame: frame,
            },
        );
        reshaped
    }

    pub(super) fn update_shared_text_entry(
        &mut self,
        content: Arc<str>,
        style: &TextStyle,
        width: Option<f32>,
        scale: f32,
        frame: u64,
    ) -> (Arc<Buffer>, Arc<ProjectedText>, bool) {
        let key = TextLayoutKey {
            content,
            highlights: None,
            width: canonical_text_width(
                style.wrap,
                style.align,
                style.text_overflow.is_some(),
                width,
            ),
            font_size: style.font_size,
            line_height: style.line_height,
            monospace_width: style.monospace_width,
            family: style.family.clone(),
            features: style.features.clone(),
            fallbacks: normalize_fallbacks(style.fallbacks.clone()),
            weight: style.weight,
            font_style: style.font_style,
            font_thicken: style.font_thicken,
            underline: style.underline,
            underline_color: style.underline_color,
            underline_wavy: style.underline_wavy,
            underline_thickness: style.underline_thickness,
            strikethrough: style.strikethrough,
            strikethrough_color: style.strikethrough_color,
            align: style.align,
            wrap: style.wrap,
            text_overflow: style.text_overflow.clone(),
            line_clamp: style.line_clamp,
            shaping: style.shaping,
            extras: TextShapingExtras::from_style(style),
            scale,
        };
        self.update_shared_text_entry_for_key(key, style, scale, frame)
    }

    pub(super) fn update_shared_text_entry_for_key(
        &mut self,
        key: TextLayoutKey,
        style: &TextStyle,
        scale: f32,
        frame: u64,
    ) -> (Arc<Buffer>, Arc<ProjectedText>, bool) {
        if let Some(entry) = self.shared_buffers.get_mut(&key) {
            entry.last_used_frame = frame;
            (
                Arc::clone(&entry.buffer),
                Arc::clone(&entry.projection),
                false,
            )
        } else {
            let (buffer, projection) = {
                let mut font_system = self.font_system.borrow_mut();
                let (projection, buffer) = prepare_text_buffer(
                    &mut font_system,
                    &key.content,
                    key.highlights.as_ref(),
                    style,
                    key.width,
                    scale,
                );
                (Arc::new(buffer), Arc::new(projection))
            };
            self.shared_buffers.insert(
                key.clone(),
                SharedTextEntry {
                    buffer: Arc::clone(&buffer),
                    projection: Arc::clone(&projection),
                    last_used_frame: frame,
                },
            );
            (buffer, projection, true)
        }
    }
}

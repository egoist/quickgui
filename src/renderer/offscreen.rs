use super::*;

/// Lazily constructed WGPU target for deterministic visual tests.
///
/// It owns the same bounded pipeline and text-cache implementations as a native renderer, but no
/// Winit window, surface, event loop, timer, or presentation loop.
#[cfg(any(test, feature = "test-support"))]
pub(crate) struct OffscreenRenderer {
    _instance: Instance,
    _adapter: Adapter,
    device: Device,
    queue: Queue,
    format: TextureFormat,
    shapes: ShapeRenderer,
    path: PathRenderer,
    custom_shader: CustomShaderRenderer,
    image: ImageRenderer,
    svg: SvgRenderer,
    text: TextSystem,
    compositor: Compositor,
    #[cfg(test)]
    last_composite: CompositeStats,
    #[cfg(test)]
    last_reshaped_text_areas: usize,
}

#[cfg(any(test, feature = "test-support"))]
impl OffscreenRenderer {
    pub(crate) fn validate_snapshot_size(
        logical_size: Size,
        scale_factor: f32,
    ) -> Result<(), crate::VisualTestError> {
        visual_physical_size(logical_size, scale_factor).map(|_| ())
    }

    pub(crate) async fn new(
        profile: PerformanceProfile,
        font_system: SharedFontSystem,
    ) -> Result<Self, crate::VisualTestError> {
        let instance = Instance::new(InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: profile.into(),
                compatible_surface: None,
                ..Default::default()
            })
            .await
            .map_err(|error| crate::VisualTestError::Adapter(error.to_string()))?;
        let (device, queue) = adapter
            .request_device(&renderer_device_descriptor())
            .await
            .map_err(|error| crate::VisualTestError::Device(error.to_string()))?;
        let format = TextureFormat::Rgba8Unorm;
        let shapes = ShapeRenderer::new(&device, format, None);
        let path = PathRenderer::new(&device, format);
        let custom_shader = CustomShaderRenderer::new(&device, format);
        let image = ImageRenderer::new(&device, format);
        let svg = SvgRenderer::new(&device, format);
        let text = TextSystem::new(&device, &queue, format, font_system, None);
        Ok(Self {
            _instance: instance,
            _adapter: adapter,
            device,
            queue,
            format,
            shapes,
            path,
            custom_shader,
            image,
            svg,
            text,
            compositor: Compositor::default(),
            #[cfg(test)]
            last_composite: CompositeStats::default(),
            #[cfg(test)]
            last_reshaped_text_areas: 0,
        })
    }

    /// The device and queue behind this renderer, for tests of individual passes.
    #[cfg(test)]
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    pub(crate) fn gpu(&self) -> (&Device, &Queue) {
        (&self.device, &self.queue)
    }

    #[cfg(test)]
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    pub(crate) fn last_reshaped_text_areas(&self) -> usize {
        self.last_reshaped_text_areas
    }

    /// Compositing telemetry for the most recent [`Self::render_to_snapshot`].
    #[cfg(test)]
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    pub(crate) fn last_composite(&self) -> CompositeStats {
        self.last_composite
    }

    #[cfg(test)]
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    pub(crate) fn compositor(&self) -> &Compositor {
        &self.compositor
    }

    pub(crate) fn render_to_snapshot(
        &mut self,
        scene: &Scene,
        logical_size: Size,
        scale_factor: f32,
    ) -> Result<crate::VisualSnapshot, crate::VisualTestError> {
        let target_size = visual_physical_size(logical_size, scale_factor)?;
        self.render_to_snapshot_with_target(scene, logical_size, scale_factor, target_size)
    }

    pub(super) fn render_to_snapshot_with_target(
        &mut self,
        scene: &Scene,
        logical_size: Size,
        scale_factor: f32,
        target_size: (u32, u32),
    ) -> Result<crate::VisualSnapshot, crate::VisualTestError> {
        let (physical_width, physical_height) = visual_physical_size(logical_size, scale_factor)?;
        let (target_width, target_height) = target_size;
        crate::visual_test::validate_snapshot_dimensions(target_width, target_height)?;
        let logical_viewport = Rect::new(0.0, 0.0, logical_size.width, logical_size.height);
        self.shapes.prepare(
            &self.device,
            &self.queue,
            scene,
            logical_viewport,
            physical_width,
            physical_height,
            scale_factor,
        );
        self.path.prepare(
            &self.device,
            &self.queue,
            scene,
            logical_viewport,
            physical_width,
            physical_height,
            scale_factor,
        );
        self.custom_shader
            .prepare(
                &self.device,
                &self.queue,
                scene,
                logical_viewport,
                physical_width,
                physical_height,
                scale_factor,
            )
            .map_err(crate::VisualTestError::Render)?;
        self.image.prepare(
            &self.device,
            &self.queue,
            scene,
            logical_viewport,
            physical_width,
            physical_height,
            scale_factor,
        );
        self.svg.prepare(
            &self.device,
            &self.queue,
            scene,
            logical_viewport,
            physical_width,
            physical_height,
            scale_factor,
        );
        let (_, _reshaped, _) = self
            .text
            .prepare(
                &self.device,
                &self.queue,
                scene,
                logical_viewport,
                physical_width,
                physical_height,
                scale_factor,
            )
            .map_err(|error| crate::VisualTestError::Render(error.to_string()))?;
        #[cfg(test)]
        {
            self.last_reshaped_text_areas = _reshaped;
        }

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("quickgui visual-test target"),
            size: wgpu::Extent3d {
                width: target_width,
                height: target_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&TextureViewDescriptor::default());
        let unpadded_bytes_per_row = target_width
            .checked_mul(4)
            .ok_or(crate::VisualTestError::TooLarge { bytes: u64::MAX })?;
        let padded_bytes_per_row = unpadded_bytes_per_row
            .checked_add(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT - 1)
            .map(|value| value / wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            .and_then(|rows| rows.checked_mul(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT))
            .ok_or(crate::VisualTestError::TooLarge { bytes: u64::MAX })?;
        let readback_bytes = u64::from(padded_bytes_per_row)
            .checked_mul(u64::from(target_height))
            .ok_or(crate::VisualTestError::TooLarge { bytes: u64::MAX })?;
        if readback_bytes > crate::MAX_VISUAL_TEST_BYTES {
            return Err(crate::VisualTestError::TooLarge {
                bytes: readback_bytes,
            });
        }
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("quickgui visual-test readback"),
            size: readback_bytes,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("quickgui visual-test encoder"),
            });
        {
            let renderers = SceneRenderers {
                shapes: &self.shapes,
                path: Some(&self.path),
                custom_shader: Some(&self.custom_shader),
                image: Some(&self.image),
                svg: Some(&self.svg),
                text: &self.text,
            };
            let _composite = self
                .compositor
                .render_scene(
                    &self.device,
                    &self.queue,
                    &mut encoder,
                    scene,
                    &renderers,
                    &view,
                    Some(&texture),
                    Some(scene.background()),
                    CompositeFrame {
                        width: target_width,
                        height: target_height,
                        scale: scale_factor,
                        format: self.format,
                        target_copyable: true,
                        plane: None,
                    },
                )
                .map_err(|error| crate::VisualTestError::Render(error.to_string()))?;
            #[cfg(test)]
            {
                self.last_composite = _composite;
            }
        }
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(target_height),
                },
            },
            wgpu::Extent3d {
                width: target_width,
                height: target_height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(encoder.finish()));
        self.text.finish_frame();

        let slice = readback.slice(..);
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|error| crate::VisualTestError::Readback(error.to_string()))?;
        receiver
            .recv()
            .map_err(|error| crate::VisualTestError::Readback(error.to_string()))?
            .map_err(|error| crate::VisualTestError::Readback(error.to_string()))?;
        let mapped = slice
            .get_mapped_range()
            .map_err(|error| crate::VisualTestError::Readback(error.to_string()))?;
        let tight_row = unpadded_bytes_per_row as usize;
        let padded_row = padded_bytes_per_row as usize;
        let tight_bytes = tight_row
            .checked_mul(target_height as usize)
            .ok_or(crate::VisualTestError::TooLarge { bytes: u64::MAX })?;
        let mut rgba = Vec::with_capacity(tight_bytes);
        for row in mapped.chunks_exact(padded_row).take(target_height as usize) {
            rgba.extend_from_slice(&row[..tight_row]);
        }
        drop(mapped);
        readback.unmap();
        crate::VisualSnapshot::from_rgba(target_width, target_height, rgba)
    }
}

#[cfg(any(test, feature = "test-support"))]
impl TextLayoutEngine for OffscreenRenderer {
    fn measure_text(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        max_width: Option<f32>,
        scale_factor: f32,
    ) -> Size {
        self.text
            .measure(id, content, style, None, max_width, scale_factor)
    }

    fn measure_styled_text(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        highlights: &Arc<[TextHighlight]>,
        max_width: Option<f32>,
        scale_factor: f32,
    ) -> Size {
        self.text.measure(
            id,
            content,
            style,
            Some(highlights),
            max_width,
            scale_factor,
        )
    }

    fn text_geometry(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        highlights: Option<&Arc<[TextHighlight]>>,
        width: f32,
        scale_factor: f32,
        visible_y: Range<f32>,
    ) -> StyledTextGeometry {
        self.text.text_geometry(
            id,
            content,
            style,
            highlights,
            width,
            scale_factor,
            visible_y,
        )
    }

    fn text_caret_position_with_highlights(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        highlights: Option<&Arc<[TextHighlight]>>,
        width: f32,
        scale_factor: f32,
        index: usize,
    ) -> Point {
        self.text
            .caret_position(id, content, style, highlights, width, scale_factor, index)
    }

    fn text_index_for_point_with_highlights(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        highlights: Option<&Arc<[TextHighlight]>>,
        width: f32,
        scale_factor: f32,
        point: Point,
    ) -> usize {
        self.text.index_for_point(
            id,
            content,
            style,
            highlights,
            TextHitTest {
                width,
                scale: scale_factor,
                point,
            },
        )
    }

    fn text_selection_rects_with_highlights(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        highlights: Option<&Arc<[TextHighlight]>>,
        width: f32,
        scale_factor: f32,
        visible_y: Range<f32>,
        start: usize,
        end: usize,
    ) -> Vec<Rect> {
        self.text.selection_rects(
            id,
            content,
            style,
            highlights,
            width,
            scale_factor,
            visible_y,
            start,
            end,
        )
    }
}

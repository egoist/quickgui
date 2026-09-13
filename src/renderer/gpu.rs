use super::*;

#[cfg(target_arch = "wasm32")]
impl GpuContext {
    pub(crate) async fn for_canvas(
        mut canvas: web_sys::HtmlCanvasElement,
        profile: PerformanceProfile,
    ) -> Result<(Self, web_sys::HtmlCanvasElement), RendererInitError> {
        // Prefer WebGPU, then WebGL2 on a fresh canvas. A canvas can only bind one of
        // `webgpu` or `webgl2`, and wgpu will not use GL while `BROWSER_WEBGPU` is set
        // and `navigator.gpu` exists — including browsers that expose that object without
        // a usable adapter.
        let mut webgpu_error = None;
        if wgpu::util::is_browser_webgpu_supported().await {
            match Self::for_canvas_with_backends(&canvas, profile, wgpu::Backends::BROWSER_WEBGPU)
                .await
            {
                Ok(gpu) => return Ok((gpu, canvas)),
                Err(error) => {
                    tracing::warn!(
                        error = %error,
                        "WebGPU initialization failed; falling back to WebGL2"
                    );
                    webgpu_error = Some(error);
                    canvas = replace_canvas(&canvas)?;
                }
            }
        } else {
            tracing::warn!("browser WebGPU is unavailable; using WebGL2");
        }
        match Self::for_canvas_with_backends(&canvas, profile, wgpu::Backends::GL).await {
            Ok(gpu) => Ok((gpu, canvas)),
            Err(webgl_error) => Err(RendererInitError::WebCanvas(match webgpu_error {
                Some(webgpu_error) => format!(
                    "no browser graphics backend could be initialized. \
                     WebGPU failed: {webgpu_error}. WebGL2 failed: {webgl_error}"
                ),
                None => format!("WebGL2 initialization failed: {webgl_error}"),
            })),
        }
    }

    async fn for_canvas_with_backends(
        canvas: &web_sys::HtmlCanvasElement,
        profile: PerformanceProfile,
        backends: wgpu::Backends,
    ) -> Result<Self, RendererInitError> {
        let mut descriptor = InstanceDescriptor::new_without_display_handle();
        descriptor.backends = backends;
        let instance = Instance::new(descriptor);
        let surface = instance.create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))?;
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: profile.into(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                ..Default::default()
            })
            .await?;
        let (device, queue) = adapter
            .request_device(&DeviceDescriptor {
                required_limits: adapter.limits(),
                ..DeviceDescriptor::default()
            })
            .await?;
        device.on_uncaptured_error(Arc::new(|error| {
            web_sys::console::error_1(&error.to_string().into());
            if let Some(window) = web_sys::window() {
                if let Ok(event) = web_sys::CustomEvent::new("quickgui:error") {
                    let _ = window.dispatch_event(&event);
                }
            }
        }));
        let format = preferred_surface_format(&surface.get_capabilities(&adapter).formats)
            .ok_or(RendererInitError::IncompatibleSurface)?;
        let shape_pipeline = ShapePipeline::new(&device, format);
        let text_cache = Cache::new(&device);
        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            shape_pipeline,
            text_cache,
        })
    }
}

#[cfg(target_arch = "wasm32")]
fn replace_canvas(
    old: &web_sys::HtmlCanvasElement,
) -> Result<web_sys::HtmlCanvasElement, RendererInitError> {
    use wasm_bindgen::JsCast;

    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| RendererInitError::WebCanvas("no document".to_owned()))?;
    let new: web_sys::HtmlCanvasElement = document
        .create_element("canvas")
        .map_err(web_canvas_error)?
        .dyn_into()
        .map_err(|_| RendererInitError::WebCanvas("created element is not a canvas".to_owned()))?;
    if !old.id().is_empty() {
        new.set_id(&old.id());
    }
    if !old.class_name().is_empty() {
        new.set_class_name(&old.class_name());
    }
    new.set_width(old.width());
    new.set_height(old.height());
    new.set_tab_index(old.tab_index());
    if let Some(label) = old.get_attribute("aria-label") {
        new.set_attribute("aria-label", &label)
            .map_err(web_canvas_error)?;
    }
    new.style().set_css_text(&old.style().css_text());
    let parent = old
        .parent_node()
        .ok_or_else(|| RendererInitError::WebCanvas("canvas has no parent".to_owned()))?;
    parent
        .insert_before(new.as_ref(), Some(old.as_ref()))
        .map_err(web_canvas_error)?;
    old.remove();
    Ok(new)
}

#[cfg(target_arch = "wasm32")]
fn web_canvas_error(error: wasm_bindgen::JsValue) -> RendererInitError {
    RendererInitError::WebCanvas(
        error
            .as_string()
            .filter(|text| !text.is_empty())
            .unwrap_or_else(|| format!("{error:?}")),
    )
}

impl GpuRenderer {
    pub async fn new(
        window: Arc<Window>,
        event_loop: &ActiveEventLoop,
        profile: PerformanceProfile,
        transparent: bool,
        font_system: SharedFontSystem,
        shared: Option<&GpuContext>,
    ) -> Result<Self, RendererInitError> {
        let instance = shared.map_or_else(
            || {
                Instance::new(InstanceDescriptor::new_with_display_handle(Box::new(
                    event_loop.owned_display_handle(),
                )))
            },
            |context| context.instance.clone(),
        );
        #[cfg(target_os = "macos")]
        let (surface, metal_layer, appkit_view) = create_macos_window_surface(&instance, &window)?;
        #[cfg(not(target_os = "macos"))]
        let surface = instance.create_surface(window.clone())?;
        let (adapter, device, queue) = if let Some(context) =
            shared.filter(|context| context.adapter.is_surface_supported(&surface))
        {
            (
                context.adapter.clone(),
                context.device.clone(),
                context.queue.clone(),
            )
        } else {
            let adapter = instance
                .request_adapter(&RequestAdapterOptions {
                    power_preference: profile.into(),
                    compatible_surface: Some(&surface),
                    ..Default::default()
                })
                .await?;
            let (device, queue) = adapter.request_device(&DeviceDescriptor::default()).await?;
            (adapter, device, queue)
        };
        let capabilities = surface.get_capabilities(&adapter);
        let format = preferred_surface_format(&capabilities.formats)
            .ok_or(RendererInitError::IncompatibleSurface)?;
        let opaque_alpha_mode = opaque_surface_alpha_mode(&capabilities.alpha_modes)
            .ok_or(RendererInitError::IncompatibleSurface)?;
        let transparent_alpha_mode = transparent_surface_alpha_mode(&capabilities.alpha_modes);
        let alpha_mode = if transparent {
            transparent_alpha_mode.ok_or(RendererInitError::TransparentSurfaceUnsupported)?
        } else {
            opaque_alpha_mode
        };
        let physical_size = window.inner_size();
        // Backdrop filters and destination-reading blend modes copy the presented image back into
        // a bounded scratch texture. Ask for `COPY_SRC` when the surface advertises it; when it
        // does not, those effects degrade to painting without the effect.
        let surface_usage = if capabilities
            .usages
            .contains(TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC)
        {
            TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC
        } else {
            TextureUsages::RENDER_ATTACHMENT
        };
        let config = SurfaceConfiguration {
            usage: surface_usage,
            format,
            width: physical_size.width.max(1),
            height: physical_size.height.max(1),
            present_mode: PresentMode::Fifo,
            alpha_mode,
            view_formats: Vec::new(),
            desired_maximum_frame_latency: 2,
            color_space: SurfaceColorSpace::Auto,
        };
        let configured_surface_size = if physical_size.width > 0 && physical_size.height > 0 {
            surface.configure(&device, &config);
            Some((config.width, config.height))
        } else {
            None
        };
        let desired_surface_size = (config.width, config.height);

        let shapes = ShapeRenderer::new(
            &device,
            format,
            shared.map(|context| &context.shape_pipeline),
        );
        let text = TextSystem::new(
            &device,
            &queue,
            format,
            font_system,
            shared.map(|context| &context.text_cache),
        );
        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            surface,
            config,
            opaque_alpha_mode,
            transparent_alpha_mode,
            desired_surface_size,
            configured_surface_size,
            #[cfg(target_os = "macos")]
            metal_layer,
            #[cfg(target_os = "macos")]
            metal_drawable_size: configured_surface_size,
            #[cfg(target_os = "macos")]
            appkit_view,
            #[cfg(target_os = "macos")]
            live_resize_transaction: false,
            shapes,
            path: None,
            custom_shader: None,
            image: None,
            svg: None,
            text,
            #[cfg(target_os = "macos")]
            overlay_surface: None,
            #[cfg(target_os = "macos")]
            overlay_view: None,
            #[cfg(target_os = "macos")]
            composition_active: false,
            #[cfg(target_os = "macos")]
            overlay_active: false,
            compositor: Compositor::default(),
            present: None,
            window,
        })
    }

    pub(crate) fn context(&self) -> GpuContext {
        GpuContext {
            instance: self.instance.clone(),
            adapter: self.adapter.clone(),
            device: self.device.clone(),
            queue: self.queue.clone(),
            shape_pipeline: self.shapes.pipeline.clone(),
            text_cache: self.text.cache.clone(),
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.desired_surface_size = (width.max(1), height.max(1));
    }

    /// Switch only the surface compositor mode; retained GPU resources and pipelines are reused.
    pub(crate) fn set_transparent(&mut self, transparent: bool) -> Result<bool, RendererError> {
        let alpha_mode = if transparent {
            self.transparent_alpha_mode
                .ok_or(RendererError::TransparentSurfaceUnsupported)?
        } else {
            self.opaque_alpha_mode
        };
        if self.config.alpha_mode == alpha_mode {
            return Ok(false);
        }
        self.config.alpha_mode = alpha_mode;
        #[cfg(target_os = "macos")]
        set_metal_layer_opaque(&self.metal_layer, !transparent);
        #[cfg(not(target_os = "macos"))]
        {
            // Other backends apply the compositor policy through surface configuration. On
            // macOS the policy is solely CAMetalLayer.opaque, and reconfiguring an active layer
            // can strand its last presented drawable in the window server. Mutating the layer
            // directly mirrors native Metal renderers and keeps the drawable chain alive.
            self.configured_surface_size = None;
        }
        Ok(true)
    }

    /// Apply only the latest queued size immediately before drawable acquisition.
    fn configure_surface_for_frame(&mut self) {
        #[cfg(target_os = "macos")]
        {
            self.configure_macos_surface_for_frame();
        }
        #[cfg(not(target_os = "macos"))]
        {
            let size = self.desired_surface_size;
            if self.configured_surface_size == Some(size) {
                return;
            }
            self.config.width = size.0;
            self.config.height = size.1;
            self.surface.configure(&self.device, &self.config);
            self.configured_surface_size = Some(size);
        }
    }

    fn reconfigure_base_surface_after_error(&mut self) {
        self.surface.configure(&self.device, &self.config);
        let configured = (self.config.width, self.config.height);
        self.configured_surface_size = Some(configured);
        #[cfg(target_os = "macos")]
        {
            // `Surface::configure` restores CAMetalLayer's drawable size to WGPU's validated
            // extent. Keep our tracking honest so the next stable frame can snap it back to the
            // exact window size when the capacity is intentionally larger.
            set_metal_drawable_size(&self.metal_layer, configured);
            self.metal_drawable_size = Some(configured);
        }
    }

    /// Resize a Metal-backed GUI surface without making WGPU wait for the entire device on every
    /// AppKit frame change.
    ///
    /// WGPU recreates a configured surface only after `Device::maintain(Wait)`, even though a
    /// CAMetalLayer drawable size can change independently of the configured WGPU extent. Keep
    /// WGPU's validated extent as a bounded grow-only capacity, but keep the actual layer drawable
    /// exact-sized on every frame. That avoids both the blocking surface reconfiguration path and
    /// Core Animation resampling text while a resize gesture is in progress. A full configure is
    /// needed only when the window exceeds the validated capacity.
    #[cfg(target_os = "macos")]
    fn configure_macos_surface_for_frame(&mut self) {
        let desired = self.desired_surface_size;
        let maximum = self.device.limits().max_texture_dimension_2d;
        let capacity = self.configured_surface_size.map_or(desired, |current| {
            grow_surface_capacity(current, desired, maximum)
        });
        if self.configured_surface_size != Some(capacity) {
            self.config.width = capacity.0;
            self.config.height = capacity.1;
            self.surface.configure(&self.device, &self.config);
            if let Some(overlay) = &mut self.overlay_surface {
                overlay.config.width = capacity.0;
                overlay.config.height = capacity.1;
                overlay.surface.configure(&self.device, &overlay.config);
            }
            self.configured_surface_size = Some(capacity);
            self.metal_drawable_size = Some(capacity);
        }
        // `Surface::configure` resets the layer to the capacity extent. Restore the exact current
        // drawable immediately, and update it on subsequent frames without reconfiguring WGPU.
        // Glyph quads then land on native backing pixels instead of passing through a changing
        // Core Animation scale that makes text stems shimmer between thick and thin.
        if self.metal_drawable_size != Some(desired) {
            set_metal_drawable_size(&self.metal_layer, desired);
            if let Some(overlay) = &self.overlay_surface {
                set_metal_drawable_size(&overlay.metal_layer, desired);
            }
            self.metal_drawable_size = Some(desired);
        }
    }

    #[cfg(target_os = "macos")]
    fn update_live_resize_transaction(&mut self) {
        let resize_transaction = unsafe { self.appkit_view.inLiveResize() };
        if self.live_resize_transaction != resize_transaction {
            set_presents_with_transaction(&self.metal_layer, resize_transaction);
            set_metal_display_sync(&self.metal_layer, !resize_transaction);
            if let Some(overlay) = &self.overlay_surface {
                set_presents_with_transaction(&overlay.metal_layer, resize_transaction);
                set_metal_display_sync(&overlay.metal_layer, !resize_transaction);
            }
            self.live_resize_transaction = resize_transaction;
        }
        // AppKit already paces a real live-resize transaction at its composition boundary. FIFO
        // display sync inside `nextDrawable` would wait for a second boundary and halve visible
        // resize throughput, so that one native interaction presents transactionally without the
        // additional wait. Programmatic resizes retain display sync: they are driven by presented
        // frames and must not enqueue work faster than the display can retire its drawable
        // storage. Ordinary scrolling and animation use the same bounded FIFO policy.
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn enable_native_composition(
        &mut self,
        view: NonNull<c_void>,
    ) -> Result<(), RendererError> {
        if self.overlay_view != Some(view) {
            self.overlay_surface = None;
            self.overlay_view = Some(view);
        }
        self.composition_active = true;
        Ok(())
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn set_native_composition_active(&mut self, active: bool) {
        self.composition_active = active && self.overlay_view.is_some();
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn set_native_overlay_active(&mut self, active: bool) -> Result<(), RendererError> {
        if !active {
            self.overlay_active = false;
            return Ok(());
        }
        if self.overlay_surface.is_none() {
            let view = self
                .overlay_view
                .ok_or_else(|| RendererError::NativeComposition("missing overlay view".into()))?;
            let overlay = create_overlay_surface(
                &self.instance,
                &self.adapter,
                &self.device,
                &self.config,
                view,
            )?;
            set_presents_with_transaction(&overlay.metal_layer, self.live_resize_transaction);
            set_metal_display_sync(&overlay.metal_layer, !self.live_resize_transaction);
            set_metal_drawable_size(
                &overlay.metal_layer,
                self.metal_drawable_size
                    .unwrap_or(self.desired_surface_size),
            );
            self.overlay_surface = Some(overlay);
        }
        self.overlay_active = true;
        Ok(())
    }

    pub(crate) fn measure_text(
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

    pub(crate) fn measure_styled_text(
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

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn text_geometry(
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

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn text_caret_position_with_highlights(
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

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn text_index_for_point_with_highlights(
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

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn text_selection_rects_with_highlights(
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

    /// Wait for the latest submitted frame before removing a platform launch cover.
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    pub(crate) fn wait_for_submitted_work(&self) -> Result<(), RendererError> {
        self.device.poll(wgpu::PollType::wait_indefinitely())?;
        Ok(())
    }

    pub fn render(
        &mut self,
        scene: &Scene,
        scale_factor: f32,
    ) -> Result<RenderOutcome, RendererError> {
        let physical_size = self.window.inner_size();
        if physical_size.width == 0 || physical_size.height == 0 {
            return Ok(RenderOutcome::Occluded);
        }
        self.resize(physical_size.width, physical_size.height);
        self.configure_surface_for_frame();
        #[cfg(target_os = "macos")]
        self.update_live_resize_transaction();

        let logical_viewport = Rect::new(
            0.0,
            0.0,
            physical_size.width as f32 / scale_factor,
            physical_size.height as f32 / scale_factor,
        );
        let (quad_count, shadow_count, shape_draw_calls) = self.shapes.prepare(
            &self.device,
            &self.queue,
            scene,
            logical_viewport,
            physical_size.width,
            physical_size.height,
            scale_factor,
        );
        if self.path.is_none()
            && scene
                .paint_layers()
                .iter()
                .any(|layer| !layer.paths().is_empty())
        {
            self.path = Some(PathRenderer::new(&self.device, self.config.format));
        }
        let path_stats = if let Some(path) = &mut self.path {
            path.prepare(
                &self.device,
                &self.queue,
                scene,
                logical_viewport,
                physical_size.width,
                physical_size.height,
                scale_factor,
            )
        } else {
            Default::default()
        };
        if self.custom_shader.is_none()
            && scene
                .paint_layers()
                .iter()
                .any(|layer| !layer.custom_shaders().is_empty())
        {
            self.custom_shader = Some(CustomShaderRenderer::new(&self.device, self.config.format));
        }
        let custom_shader_stats = if let Some(custom_shader) = &mut self.custom_shader {
            custom_shader
                .prepare(
                    &self.device,
                    &self.queue,
                    scene,
                    logical_viewport,
                    physical_size.width,
                    physical_size.height,
                    scale_factor,
                )
                .map_err(RendererError::CustomShader)?
        } else {
            Default::default()
        };
        if self.image.is_none()
            && scene
                .paint_layers()
                .iter()
                .any(|layer| !layer.images().is_empty())
        {
            self.image = Some(ImageRenderer::new(&self.device, self.config.format));
        }
        let image_stats = if let Some(image) = &mut self.image {
            image.prepare(
                &self.device,
                &self.queue,
                scene,
                logical_viewport,
                physical_size.width,
                physical_size.height,
                scale_factor,
            )
        } else {
            Default::default()
        };
        if self.svg.is_none()
            && scene
                .paint_layers()
                .iter()
                .any(|layer| !layer.svgs().is_empty())
        {
            self.svg = Some(SvgRenderer::new(&self.device, self.config.format));
        }
        let svg_stats = if let Some(svg) = &mut self.svg {
            svg.prepare(
                &self.device,
                &self.queue,
                scene,
                logical_viewport,
                physical_size.width,
                physical_size.height,
                scale_factor,
            )
        } else {
            Default::default()
        };
        let (text_count, reshaped, text_draw_calls) = self.text.prepare(
            &self.device,
            &self.queue,
            scene,
            logical_viewport,
            physical_size.width,
            physical_size.height,
            scale_factor,
        )?;

        #[cfg(target_os = "macos")]
        let composed = self.composition_active;
        #[cfg(not(target_os = "macos"))]
        let composed = false;

        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                // A surface texture must not outlive reconfiguration. In particular, Metal can
                // otherwise wait on the drawable we still own while rebuilding its pool.
                drop(frame);
                self.reconfigure_base_surface_after_error();
                return Ok(RenderOutcome::Retry);
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Outdated => {
                self.reconfigure_base_surface_after_error();
                return Ok(RenderOutcome::Retry);
            }
            wgpu::CurrentSurfaceTexture::Occluded => return Ok(RenderOutcome::Occluded),
            wgpu::CurrentSurfaceTexture::Lost => {
                #[cfg(target_os = "macos")]
                let surface = create_surface_from_metal_layer(&self.instance, &self.metal_layer);
                #[cfg(not(target_os = "macos"))]
                let surface = self.instance.create_surface(self.window.clone());
                self.surface =
                    surface.map_err(|error| RendererError::RecreateSurface(error.to_string()))?;
                self.reconfigure_base_surface_after_error();
                return Ok(RenderOutcome::Retry);
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                panic!("WGPU reported a surface validation error")
            }
        };

        #[cfg(target_os = "macos")]
        let overlay_frame = if composed && self.overlay_active {
            let overlay = self
                .overlay_surface
                .as_mut()
                .expect("active composition owns an overlay surface");
            match overlay.surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(frame) => Some(frame),
                wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                    drop(frame);
                    overlay.surface.configure(&self.device, &overlay.config);
                    set_metal_drawable_size(
                        &overlay.metal_layer,
                        self.metal_drawable_size
                            .unwrap_or(self.desired_surface_size),
                    );
                    return Ok(RenderOutcome::Retry);
                }
                wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Outdated => {
                    overlay.surface.configure(&self.device, &overlay.config);
                    set_metal_drawable_size(
                        &overlay.metal_layer,
                        self.metal_drawable_size
                            .unwrap_or(self.desired_surface_size),
                    );
                    return Ok(RenderOutcome::Retry);
                }
                wgpu::CurrentSurfaceTexture::Occluded => return Ok(RenderOutcome::Occluded),
                wgpu::CurrentSurfaceTexture::Lost => {
                    let view = overlay.view;
                    let overlay = create_overlay_surface(
                        &self.instance,
                        &self.adapter,
                        &self.device,
                        &self.config,
                        view,
                    )?;
                    set_presents_with_transaction(
                        &overlay.metal_layer,
                        self.live_resize_transaction,
                    );
                    set_metal_display_sync(&overlay.metal_layer, !self.live_resize_transaction);
                    set_metal_drawable_size(
                        &overlay.metal_layer,
                        self.metal_drawable_size
                            .unwrap_or(self.desired_surface_size),
                    );
                    self.overlay_surface = Some(overlay);
                    return Ok(RenderOutcome::Retry);
                }
                wgpu::CurrentSurfaceTexture::Validation => {
                    panic!("WGPU reported an overlay surface validation error")
                }
            }
        } else {
            None
        };
        #[cfg(not(target_os = "macos"))]
        let overlay_frame: Option<wgpu::SurfaceTexture> = None;

        let surface_view = frame.texture.create_view(&TextureViewDescriptor::default());
        let overlay_surface_view = overlay_frame
            .as_ref()
            .map(|frame| frame.texture.create_view(&TextureViewDescriptor::default()));
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("quickgui frame encoder"),
            });
        let surface_copyable = self.config.usage.contains(TextureUsages::COPY_SRC);
        // A transparent sRGB frame renders into an intermediate and is re-premultiplied into the
        // surface afterwards; see `present::TransparentPresent`. The overlay surface is always
        // transparent, so it takes the same route whenever the base surface would.
        let re_premultiply = present::needs_present_pass(&self.config);
        let mut intermediate: Option<(wgpu::Texture, wgpu::TextureView)> = None;
        let mut overlay_intermediate: Option<(wgpu::Texture, wgpu::TextureView)> = None;
        if re_premultiply {
            let present = self.present.get_or_insert_with(|| {
                present::TransparentPresent::new(&self.device, self.config.format)
            });
            intermediate = Some(present.target(
                &self.device,
                self.config.format,
                0,
                physical_size.width,
                physical_size.height,
            ));
            if overlay_surface_view.is_some() {
                overlay_intermediate = Some(present.target(
                    &self.device,
                    self.config.format,
                    1,
                    physical_size.width,
                    physical_size.height,
                ));
            }
        } else if let Some(present) = &mut self.present {
            present.release();
        }
        let view = intermediate
            .as_ref()
            .map_or_else(|| surface_view.clone(), |(_, view)| view.clone());
        let target_copyable = intermediate.is_some() || surface_copyable;
        let overlay_view = overlay_surface_view.as_ref().map(|surface| {
            overlay_intermediate
                .as_ref()
                .map_or_else(|| surface.clone(), |(_, view)| view.clone())
        });
        let mut composite_stats = {
            let renderers = SceneRenderers {
                shapes: &self.shapes,
                path: self.path.as_ref(),
                custom_shader: self.custom_shader.as_ref(),
                image: self.image.as_ref(),
                svg: self.svg.as_ref(),
                text: &self.text,
            };
            self.compositor.render_scene(
                &self.device,
                &self.queue,
                &mut encoder,
                scene,
                &renderers,
                &view,
                Some(
                    intermediate
                        .as_ref()
                        .map_or(&frame.texture, |(texture, _)| texture),
                ),
                Some(scene.background()),
                CompositeFrame {
                    width: physical_size.width,
                    height: physical_size.height,
                    scale: scale_factor,
                    format: self.config.format,
                    target_copyable,
                    plane: composed.then_some(ScenePlane::Base),
                },
            )?
        };
        if intermediate.is_some()
            && let Some(present) = &self.present
        {
            present.present(&self.device, &mut encoder, 0, &surface_view);
        }
        if let Some(overlay) = &overlay_view {
            let renderers = SceneRenderers {
                shapes: &self.shapes,
                path: self.path.as_ref(),
                custom_shader: self.custom_shader.as_ref(),
                image: self.image.as_ref(),
                svg: self.svg.as_ref(),
                text: &self.text,
            };
            let overlay_stats = self.compositor.render_scene(
                &self.device,
                &self.queue,
                &mut encoder,
                scene,
                &renderers,
                overlay,
                overlay_intermediate
                    .as_ref()
                    .map(|(texture, _)| texture)
                    .or(overlay_frame.as_ref().map(|frame| &frame.texture)),
                Some(crate::Color::TRANSPARENT),
                CompositeFrame {
                    width: physical_size.width,
                    height: physical_size.height,
                    scale: scale_factor,
                    format: self.config.format,
                    target_copyable,
                    plane: Some(ScenePlane::Overlay),
                },
            )?;
            composite_stats.layers += overlay_stats.layers;
            composite_stats.layer_passes += overlay_stats.layer_passes;
            composite_stats.reused_layers += overlay_stats.reused_layers;
            composite_stats.blur_passes += overlay_stats.blur_passes;
            composite_stats.layer_texture_bytes = overlay_stats.layer_texture_bytes;
            if overlay_intermediate.is_some()
                && let (Some(present), Some(surface)) =
                    (&self.present, overlay_surface_view.as_ref())
            {
                present.present(&self.device, &mut encoder, 1, surface);
            }
        }

        self.window.pre_present_notify();
        self.queue.submit(Some(encoder.finish()));
        self.queue.present(frame);
        if let Some(frame) = overlay_frame {
            self.queue.present(frame);
        }
        self.text.finish_frame();
        let (retained_text_areas, retained_text_layouts, retained_text_renderers) =
            self.text.retained_counts();

        let uploads = std::iter::once(self.shapes.uploads.stats())
            .chain(self.path.as_ref().map(|renderer| renderer.uploads.stats()))
            .chain(
                self.custom_shader
                    .as_ref()
                    .map(|renderer| renderer.uploads.stats()),
            )
            .chain(self.image.as_ref().map(|renderer| renderer.uploads.stats()))
            .chain(self.svg.as_ref().map(|renderer| renderer.uploads.stats()))
            .chain(std::iter::once(self.text.upload_stats()))
            .fold(super::upload::UploadStats::default(), |mut total, stats| {
                total.bytes += stats.bytes;
                total.writes += stats.writes;
                total.reused += stats.reused;
                total.shadow_bytes += stats.shadow_bytes;
                total
            });
        Ok(RenderOutcome::Presented(RenderStats {
            uploaded_buffer_bytes: uploads.bytes,
            buffer_write_calls: uploads.writes,
            reused_buffers: uploads.reused,
            upload_shadow_bytes: uploads.shadow_bytes,
            quads: quad_count,
            shadows: shadow_count,
            images: image_stats.images,
            image_uploads: image_stats.uploads,
            gpu_image_cache_bytes: image_stats.cache_bytes,
            cpu_image_cache_bytes: 0,
            image_resource_entries: 0,
            image_resources_loading: 0,
            image_resources_failed: 0,
            animated_images: 0,
            active_animations: 0,
            svgs: svg_stats.svgs,
            svg_rasterizations: svg_stats.rasterizations,
            gpu_svg_cache_bytes: svg_stats.cache_bytes,
            paths: path_stats.paths,
            path_vertices: path_stats.vertices,
            skipped_paths: path_stats.skipped_paths,
            custom_shader_instances: custom_shader_stats.instances,
            custom_shader_compilations: custom_shader_stats.compiled_pipelines,
            cached_custom_shader_pipelines: custom_shader_stats.cached_pipelines,
            skipped_custom_shader_instances: custom_shader_stats.skipped_instances,
            text_areas: text_count,
            draw_calls: shape_draw_calls
                + path_stats.draw_calls
                + custom_shader_stats.draw_calls
                + image_stats.draw_calls
                + svg_stats.draw_calls
                + text_draw_calls
                + composite_stats.layers
                + composite_stats.blur_passes,
            reshaped_text_areas: reshaped,
            retained_text_areas,
            retained_text_layouts,
            retained_text_renderers,
            cached_text_areas: text_count.saturating_sub(reshaped),
            compositing_layers: composite_stats.layers,
            layer_passes: composite_stats.layer_passes,
            reused_compositing_layers: composite_stats.reused_layers,
            blur_passes: composite_stats.blur_passes,
            layer_texture_bytes: composite_stats.layer_texture_bytes,
            skipped_layer_effects: composite_stats.skipped_layer_effects,
        }))
    }

    #[allow(dead_code)]
    pub fn adapter_info(&self) -> wgpu::AdapterInfo {
        self.adapter.get_info()
    }
}

impl TextLayoutEngine for GpuRenderer {
    fn measure_text(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        max_width: Option<f32>,
        scale_factor: f32,
    ) -> Size {
        GpuRenderer::measure_text(self, id, content, style, max_width, scale_factor)
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
        GpuRenderer::measure_styled_text(
            self,
            id,
            content,
            style,
            highlights,
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
        GpuRenderer::text_geometry(
            self,
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
        GpuRenderer::text_caret_position_with_highlights(
            self,
            id,
            content,
            style,
            highlights,
            width,
            scale_factor,
            index,
        )
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
        GpuRenderer::text_index_for_point_with_highlights(
            self,
            id,
            content,
            style,
            highlights,
            width,
            scale_factor,
            point,
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
        GpuRenderer::text_selection_rects_with_highlights(
            self,
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

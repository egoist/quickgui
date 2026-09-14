use std::{
    borrow::Cow,
    cell::RefCell,
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
    mem,
    ops::Range,
    rc::Rc,
    sync::Arc,
};

use bytemuck::{Pod, Zeroable};
use glyphon::{
    Attrs, Buffer, Cache, Color as GlyphColor, Cursor, FontSystem, Metrics, Resolution,
    Shaping as GlyphShaping, Style as GlyphStyle, SwashCache, TextArea, TextAtlas, TextBounds,
    TextRenderer, Viewport, Wrap,
    cosmic_text::{
        Align as GlyphAlign, BaseDirection, CacheKeyFlags, DecorationSpan, Ellipsize,
        EllipsizeHeightLimit, LayoutGlyph, LayoutRun, UnderlineStyle as GlyphUnderlineStyle,
    },
};
use thiserror::Error;
use unicode_segmentation::UnicodeSegmentation;
use wgpu::{
    Adapter, BindGroup, BufferAddress, ColorTargetState, CommandEncoderDescriptor,
    CompositeAlphaMode, Device, DeviceDescriptor, FragmentState, Instance, InstanceDescriptor,
    MultisampleState, PipelineCompilationOptions, PresentMode, PrimitiveState, PrimitiveTopology,
    Queue, RenderPipeline, RenderPipelineDescriptor, RequestAdapterOptions, ShaderStages, Surface,
    SurfaceColorSpace, SurfaceConfiguration, TextureFormat, TextureUsages, TextureViewDescriptor,
    VertexAttribute, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
    util::DeviceExt,
};
use winit::{event_loop::ActiveEventLoop, window::Window};

use crate::{
    AssetError, Assets, Color as UiColor, FontFallbacks, FontFamily, FontFeatures, FontSource,
    Gradient, Hyphens, MAX_GRADIENT_STOPS, MAX_TEXT_HIGHLIGHTS, OverflowWrap, PerformanceProfile,
    Point, Quad, Rect, RenderStats, Scene, ScenePlane, Size, TextAlign, TextDirection,
    TextHighlight, TextId, TextOverflow, TextShaping, TextStyle, TextTransform, TextUnderline,
    TextWrap, WordBreak,
    assets::resolve_fonts,
    custom_shader_renderer::CustomShaderRenderer,
    font::{glyph_family, normalize_fallbacks},
    image_renderer::ImageRenderer,
    path::GradientData,
    path_renderer::PathRenderer,
    scene::{EdgeQuad, PrimitiveRef, Shadow, ShapeRef, WavyUnderline},
    svg_renderer::SvgRenderer,
};

pub(crate) struct StyledTextGeometry {
    pub backgrounds: Vec<TextPaintRect>,
    pub decorations: Vec<TextPaintRect>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum TextPaintKind {
    Solid,
    WavyUnderline {
        baseline: f32,
        amplitude: f32,
        thickness: f32,
        wavelength: f32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TextPaintRect {
    pub rect: Rect,
    pub color: UiColor,
    pub kind: TextPaintKind,
}

/// CPU text-layout surface shared by native and headless WGPU renderers.
///
/// Keeping this narrow lets deterministic visual tests exercise the production Taffy and paint
/// paths without introducing a native window. Generic callers remain statically dispatched in
/// the native hot path.
pub(crate) trait TextLayoutEngine {
    fn measure_text(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        max_width: Option<f32>,
        scale_factor: f32,
    ) -> Size;

    fn measure_styled_text(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        highlights: &Arc<[TextHighlight]>,
        max_width: Option<f32>,
        scale_factor: f32,
    ) -> Size;

    #[allow(clippy::too_many_arguments)]
    fn text_geometry(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        highlights: Option<&Arc<[TextHighlight]>>,
        width: f32,
        scale_factor: f32,
        visible_y: Range<f32>,
    ) -> StyledTextGeometry;

    #[allow(clippy::too_many_arguments)]
    fn text_caret_position_with_highlights(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        highlights: Option<&Arc<[TextHighlight]>>,
        width: f32,
        scale_factor: f32,
        index: usize,
    ) -> Point;

    #[allow(clippy::too_many_arguments)]
    fn text_index_for_point_with_highlights(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        highlights: Option<&Arc<[TextHighlight]>>,
        width: f32,
        scale_factor: f32,
        point: Point,
    ) -> usize;

    #[allow(clippy::too_many_arguments)]
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
    ) -> Vec<Rect>;
}

#[cfg(target_os = "macos")]
use objc2::rc::Retained;
#[cfg(target_os = "macos")]
use objc2_app_kit::NSView;
#[cfg(target_os = "macos")]
use objc2_foundation::CGSize;
#[cfg(target_os = "macos")]
use objc2_quartz_core::CAMetalLayer;
#[cfg(target_os = "macos")]
use raw_window_metal::Layer as MetalLayer;
#[cfg(target_os = "macos")]
use std::{ffi::c_void, ptr::NonNull};
#[cfg(target_os = "macos")]
use wgpu::SurfaceTargetUnsafe;
#[cfg(target_os = "macos")]
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

const INITIAL_SHAPE_CAPACITY: usize = 256;
const BUFFERED_FRAMES: usize = 3;
/// WebGL2 has no storage buffers. Gradient and path-paint tables then live in a
/// fixed uniform array that must fit the 16 KiB WebGL2 UBO minimum.
pub(crate) const UNIFORM_TABLE_LEN: usize = 64;
pub(crate) const QUAD_WGSL: &str = include_str!("quad.wgsl");
pub(crate) const QUAD_STORAGE_BINDING: &str =
    "@group(1) @binding(0)\nvar<storage, read> gradients: array<GradientRecord>;";
const MAX_RETAINED_TEXT_AREAS: usize = 256;
const MAX_RETAINED_TEXT_LAYOUTS: usize = 256;
// Text separated by intersecting paint primitives needs independent glyph vertex buffers to
// preserve painter's order. Keep enough for a layered application scene without retaining every
// pathological overlap forever. Each Glyphon renderer starts with a 4 KiB buffer.
const MAX_RETAINED_TEXT_RENDERERS: usize = 32;
const MAX_RETAINED_TEXT_COLORS: usize = 64;
// Keep recently visited text alive across an ordinary back-and-forth scroll gesture. The two
// caches remain hard-capped below, so extending the age window trades no unbounded memory for the
// ability to reuse shaped Markdown after it has been offscreen for a few seconds.
const TEXT_RETENTION_FRAMES: u64 = 600;
const BASIC_FRAGMENT_MIN_BYTES: usize = 24;

pub(crate) fn uses_read_only_storage_buffers(device: &Device) -> bool {
    device.limits().max_storage_buffers_per_shader_stage > 0
}

pub(crate) fn read_only_table_binding(storage: bool) -> wgpu::BufferBindingType {
    if storage {
        wgpu::BufferBindingType::Storage { read_only: true }
    } else {
        wgpu::BufferBindingType::Uniform
    }
}

pub(crate) fn table_buffer_usages(storage: bool) -> wgpu::BufferUsages {
    let kind = if storage {
        wgpu::BufferUsages::STORAGE
    } else {
        wgpu::BufferUsages::UNIFORM
    };
    kind | wgpu::BufferUsages::COPY_DST
}

pub(crate) fn rewrite_storage_array_as_uniform(
    source: &str,
    storage_binding: &str,
    array_name: &str,
    table_name: &str,
    record_type: &str,
) -> String {
    assert!(
        source.contains(storage_binding),
        "shader storage binding drifted; update rewrite_storage_array_as_uniform"
    );
    source.replace(
        storage_binding,
        &format!(
            "struct {record_type}Table {{\n    records: array<{record_type}, {UNIFORM_TABLE_LEN}>,\n}}\n@group(1) @binding(0)\nvar<uniform> {table_name}: {record_type}Table;"
        ),
    )
    .replace(&format!("{array_name}["), &format!("{table_name}.records["))
}

pub(crate) fn shader_module_from_wgsl(
    device: &Device,
    label: &str,
    source: Cow<'static, str>,
) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(source),
    })
}

#[derive(Debug, Error)]
pub(crate) enum RendererInitError {
    #[error("could not create a WGPU surface: {0}")]
    CreateSurface(#[from] wgpu::CreateSurfaceError),
    #[error("no compatible graphics adapter was found: {0}")]
    RequestAdapter(#[from] wgpu::RequestAdapterError),
    #[error("could not create a graphics device: {0}")]
    RequestDevice(#[from] wgpu::RequestDeviceError),
    #[error("the selected graphics adapter cannot render to this window surface")]
    IncompatibleSurface,
    #[error("the selected graphics adapter does not expose an alpha-capable window surface")]
    TransparentSurfaceUnsupported,
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    #[error("could not create the macOS Metal surface: {0}")]
    PlatformSurface(String),
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    #[error("could not initialize browser graphics: {0}")]
    WebCanvas(String),
}

#[derive(Debug, Error)]
pub(crate) enum RendererError {
    #[error("text id {0:?} was submitted more than once in one frame")]
    DuplicateTextId(TextId),
    #[error("text preparation failed: {0}")]
    PrepareText(#[from] glyphon::PrepareError),
    #[error("text rendering failed: {0}")]
    RenderText(#[from] glyphon::RenderError),
    #[error("could not recreate a lost window surface: {0}")]
    RecreateSurface(String),
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    #[error("could not initialize native-view composition: {0}")]
    NativeComposition(String),
    #[error("this window surface cannot switch to alpha compositing")]
    TransparentSurfaceUnsupported,
    #[error("GPU submission did not complete: {0}")]
    DevicePoll(#[from] wgpu::PollError),
    #[error("custom shader pipeline creation failed: {0}")]
    CustomShader(String),
}

// RenderStats intentionally travels inline once per frame. Boxing it would add a heap allocation
// to the hottest renderer return path merely to shrink the two exceptional unit variants.
#[allow(clippy::large_enum_variant)]
pub(crate) enum RenderOutcome {
    Presented(RenderStats),
    Retry,
    Occluded,
}

/// Share the heavyweight WGPU context and immutable device pipelines across compatible windows.
/// Surface state, uniforms, upload buffers, glyph atlases, text layouts, and bounded asset caches
/// remain window-local.
#[derive(Clone)]
pub(crate) struct GpuContext {
    instance: Instance,
    adapter: Adapter,
    device: Device,
    queue: Queue,
    shape_pipeline: ShapePipeline,
    text_cache: Cache,
}

pub(crate) struct GpuRenderer {
    instance: Instance,
    adapter: Adapter,
    device: Device,
    queue: Queue,
    surface: Surface<'static>,
    config: SurfaceConfiguration,
    opaque_alpha_mode: CompositeAlphaMode,
    transparent_alpha_mode: Option<CompositeAlphaMode>,
    desired_surface_size: (u32, u32),
    /// The extent WGPU validates surface operations against. On macOS this is a grow-only
    /// capacity during resize bursts; the CAMetalLayer drawable itself remains exact-sized.
    configured_surface_size: Option<(u32, u32)>,
    #[cfg(target_os = "macos")]
    metal_layer: MetalLayer,
    #[cfg(target_os = "macos")]
    metal_drawable_size: Option<(u32, u32)>,
    #[cfg(target_os = "macos")]
    appkit_view: Retained<NSView>,
    #[cfg(target_os = "macos")]
    live_resize_transaction: bool,
    shapes: ShapeRenderer,
    // These primitive families are absent from ordinary shape-and-text windows. Construct their
    // window-local pipelines and caches only after the first scene that actually uses them.
    path: Option<PathRenderer>,
    custom_shader: Option<CustomShaderRenderer>,
    image: Option<ImageRenderer>,
    svg: Option<SvgRenderer>,
    text: TextSystem,
    #[cfg(target_os = "macos")]
    overlay_surface: Option<OverlaySurface>,
    #[cfg(target_os = "macos")]
    overlay_view: Option<NonNull<c_void>>,
    #[cfg(target_os = "macos")]
    composition_active: bool,
    #[cfg(target_os = "macos")]
    overlay_active: bool,
    compositor: Compositor,
    /// The re-premultiplying presentation of transparent frames, created on first use.
    present: Option<present::TransparentPresent>,
    window: Arc<Window>,
}

#[cfg(target_os = "macos")]
struct OverlaySurface {
    surface: Surface<'static>,
    config: SurfaceConfiguration,
    view: NonNull<c_void>,
    metal_layer: MetalLayer,
}

mod compositor;
mod gpu;
#[cfg(any(test, feature = "test-support"))]
mod offscreen;
mod present;
mod text_layout;
mod text_system;
pub(crate) mod upload;

#[cfg(test)]
pub(crate) use compositor::CompositeStats;
pub(crate) use compositor::{CompositeFrame, Compositor, SceneRenderers};
#[cfg(any(test, feature = "test-support"))]
pub(crate) use offscreen::OffscreenRenderer;
use text_layout::*;
pub(crate) use text_layout::{SharedFontSystem, create_shared_font_system};

#[cfg(any(test, feature = "test-support"))]
fn visual_physical_size(
    logical_size: Size,
    scale_factor: f32,
) -> Result<(u32, u32), crate::VisualTestError> {
    if !logical_size.width.is_finite()
        || !logical_size.height.is_finite()
        || logical_size.width <= 0.0
        || logical_size.height <= 0.0
        || !scale_factor.is_finite()
        || scale_factor <= 0.0
    {
        return Err(crate::VisualTestError::InvalidDimensions);
    }
    let physical_width = (logical_size.width * scale_factor).round();
    let physical_height = (logical_size.height * scale_factor).round();
    if physical_width < 1.0
        || physical_height < 1.0
        || physical_width > crate::MAX_VISUAL_TEST_DIMENSION as f32
        || physical_height > crate::MAX_VISUAL_TEST_DIMENSION as f32
    {
        return Err(crate::VisualTestError::InvalidDimensions);
    }
    let physical_width = physical_width as u32;
    let physical_height = physical_height as u32;
    crate::visual_test::validate_snapshot_dimensions(physical_width, physical_height)?;
    Ok((physical_width, physical_height))
}

#[cfg(target_os = "macos")]
fn create_macos_window_surface(
    instance: &Instance,
    window: &Window,
) -> Result<(Surface<'static>, MetalLayer, Retained<NSView>), RendererInitError> {
    let handle = window
        .window_handle()
        .map_err(|error| RendererInitError::PlatformSurface(error.to_string()))?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return Err(RendererInitError::PlatformSurface(
            "the window does not expose an AppKit view".to_owned(),
        ));
    };
    let appkit_view = unsafe { Retained::retain(handle.ns_view.as_ptr().cast::<NSView>()) }
        .ok_or_else(|| {
            RendererInitError::PlatformSurface("the AppKit view pointer is null".to_owned())
        })?;
    let metal_layer =
        unsafe { MetalLayer::from_ns_view(NonNull::from(appkit_view.as_ref()).cast::<c_void>()) };
    let surface = create_surface_from_metal_layer(instance, &metal_layer)?;
    Ok((surface, metal_layer, appkit_view))
}

#[cfg(target_os = "macos")]
fn create_surface_from_metal_layer(
    instance: &Instance,
    layer: &MetalLayer,
) -> Result<Surface<'static>, wgpu::CreateSurfaceError> {
    unsafe {
        instance.create_surface_unsafe(SurfaceTargetUnsafe::CoreAnimationLayer(
            layer.as_ptr().as_ptr(),
        ))
    }
}

#[cfg(target_os = "macos")]
fn set_presents_with_transaction(layer: &MetalLayer, enabled: bool) {
    let layer = unsafe { layer.as_ptr().cast::<CAMetalLayer>().as_ref() };
    unsafe { layer.setPresentsWithTransaction(enabled) };
}

#[cfg(target_os = "macos")]
fn set_metal_display_sync(layer: &MetalLayer, enabled: bool) {
    let layer = unsafe { layer.as_ptr().cast::<CAMetalLayer>().as_ref() };
    // SAFETY: the renderer owns this CAMetalLayer and updates its presentation policy on the
    // AppKit application thread before acquiring the frame's drawable.
    unsafe { layer.setDisplaySyncEnabled(enabled) };
}

#[cfg(target_os = "macos")]
fn set_metal_drawable_size(layer: &MetalLayer, size: (u32, u32)) {
    let layer = unsafe { layer.as_ptr().cast::<CAMetalLayer>().as_ref() };
    // SAFETY: the renderer owns this CAMetalLayer and calls this on AppKit's application thread.
    // Both dimensions have already been clamped to at least one physical pixel.
    unsafe { layer.setDrawableSize(CGSize::new(f64::from(size.0), f64::from(size.1))) };
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn grow_surface_capacity(current: (u32, u32), required: (u32, u32), maximum: u32) -> (u32, u32) {
    fn grow(current: u32, required: u32, maximum: u32) -> u32 {
        if required <= current {
            return current;
        }
        current
            .saturating_add(current / 2)
            .max(required)
            .min(maximum)
    }

    (
        grow(current.0, required.0, maximum),
        grow(current.1, required.1, maximum),
    )
}

#[cfg(target_os = "macos")]
fn set_metal_layer_opaque(layer: &MetalLayer, opaque: bool) {
    let layer = unsafe { layer.as_ptr().cast::<CAMetalLayer>().as_ref() };
    layer.setOpaque(opaque);
}

#[cfg(target_os = "macos")]
fn create_overlay_surface(
    instance: &Instance,
    adapter: &Adapter,
    device: &Device,
    base_config: &SurfaceConfiguration,
    view: NonNull<c_void>,
) -> Result<OverlaySurface, RendererError> {
    let metal_layer = unsafe { MetalLayer::from_ns_view(view) };
    let surface = create_surface_from_metal_layer(instance, &metal_layer)
        .map_err(|error| RendererError::NativeComposition(error.to_string()))?;
    let capabilities = surface.get_capabilities(adapter);
    if !capabilities.formats.contains(&base_config.format) {
        return Err(RendererError::NativeComposition(
            "the overlay surface does not support the base surface format".to_owned(),
        ));
    }
    let alpha_mode = capabilities
        .alpha_modes
        .iter()
        .copied()
        .find(|mode| *mode == CompositeAlphaMode::PreMultiplied)
        .or_else(|| {
            capabilities
                .alpha_modes
                .iter()
                .copied()
                .find(|mode| *mode == CompositeAlphaMode::PostMultiplied)
        })
        .or_else(|| {
            capabilities
                .alpha_modes
                .iter()
                .copied()
                .find(|mode| *mode == CompositeAlphaMode::Auto)
        })
        .ok_or_else(|| {
            RendererError::NativeComposition(
                "the overlay surface does not expose a transparent alpha mode".to_owned(),
            )
        })?;
    let mut config = base_config.clone();
    config.alpha_mode = alpha_mode;
    surface.configure(device, &config);
    Ok(OverlaySurface {
        surface,
        config,
        view,
        metal_layer,
    })
}

fn preferred_surface_format(formats: &[TextureFormat]) -> Option<TextureFormat> {
    // Raster masks and premultiplied UI colors blend in encoded sRGB, matching CoreText/GPUI.
    // An sRGB attachment would linearize the destination and wash out dark glyph edges.
    [TextureFormat::Bgra8Unorm, TextureFormat::Rgba8Unorm]
        .into_iter()
        .find(|format| formats.contains(format))
}

fn opaque_surface_alpha_mode(modes: &[CompositeAlphaMode]) -> Option<CompositeAlphaMode> {
    modes
        .iter()
        .copied()
        .find(|mode| *mode == CompositeAlphaMode::Opaque)
        .or_else(|| modes.first().copied())
}

fn transparent_surface_alpha_mode(modes: &[CompositeAlphaMode]) -> Option<CompositeAlphaMode> {
    [
        CompositeAlphaMode::PreMultiplied,
        CompositeAlphaMode::PostMultiplied,
        CompositeAlphaMode::Inherit,
    ]
    .into_iter()
    .find(|preferred| modes.contains(preferred))
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ViewUniform {
    viewport: [f32; 2],
    scale: f32,
    _padding: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ShapeInstance {
    geometry: [f32; 4],
    primary: [f32; 4],
    secondary: [f32; 4],
    clip: [f32; 4],
    params: [f32; 4],
    subject: [f32; 4],
    /// Corner radii ordered top-left, top-right, bottom-right, bottom-left.
    corners: [f32; 4],
    /// Gradient index (negative when absent), border style code, unused, unused.
    effects: [f32; 4],
}

/// One GPU gradient record, shared by every gradient-capable shader family.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuGradient {
    header: [f32; 4],
    geometry: [f32; 4],
    positions: [[f32; 4]; 2],
    colors: [[f32; 4]; MAX_GRADIENT_STOPS],
}

impl From<GradientData> for GpuGradient {
    fn from(data: GradientData) -> Self {
        Self {
            header: data.header,
            geometry: data.geometry,
            positions: data.positions,
            colors: data.colors,
        }
    }
}

const _: () = assert!(UNIFORM_TABLE_LEN * mem::size_of::<GpuGradient>() <= 16 * 1024);

/// Largest number of resolved gradients uploaded for one window frame.
///
/// Admission follows scene order. A shape whose gradient does not fit falls back to its solid
/// fill instead of growing the per-frame upload without a bound.
pub const MAX_GRADIENTS_PER_FRAME: usize = 4_096;
const INITIAL_GRADIENT_CAPACITY: usize = 16;
const NO_GRADIENT: f32 = -1.0;

const SHAPE_MODE_QUAD: f32 = 0.0;
const SHAPE_MODE_DROP_SHADOW: f32 = 1.0;
const SHAPE_MODE_INSET_SHADOW: f32 = 2.0;
const SHAPE_MODE_WAVY_UNDERLINE: f32 = 3.0;
const SHAPE_MODE_EDGE_QUAD: f32 = 4.0;
const SHADOW_SIGMA_PER_BLUR_RADIUS: f32 = 0.5;
const SHADOW_MARGIN_SIGMAS: f32 = 3.0;

#[derive(Clone)]
pub(crate) struct ShapePipeline {
    pipeline: Arc<RenderPipeline>,
    bind_group_layout: Arc<wgpu::BindGroupLayout>,
    gradient_bind_group_layout: Arc<wgpu::BindGroupLayout>,
    format: TextureFormat,
    uses_storage: bool,
}

pub(crate) struct ShapeRenderer {
    pipeline: ShapePipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group: BindGroup,
    instance_buffers: Vec<wgpu::Buffer>,
    instance_capacities: Vec<usize>,
    gradient_buffers: Vec<wgpu::Buffer>,
    gradient_capacities: Vec<usize>,
    gradient_bind_groups: Vec<BindGroup>,
    active_buffer: usize,
    pub(crate) uploads: crate::renderer::upload::BufferUploads,
    batches: Vec<ShapeBatch>,
    layer_batches: Vec<Range<usize>>,
    pending: Vec<OrderedShape>,
    instances: Vec<ShapeInstance>,
    gradients: Vec<GpuGradient>,
}

#[derive(Clone, Copy)]
struct OrderedShape {
    order: u32,
    instance: ShapeInstance,
}

struct ShapeBatch {
    order: u32,
    instances: Range<u32>,
}

impl ShapePipeline {
    fn new(device: &Device, format: TextureFormat) -> Self {
        let uses_storage = uses_read_only_storage_buffers(device);
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("quickgui view bind group layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let gradient_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("quickgui shape gradient bind group layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: read_only_table_binding(uses_storage),
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("quickgui shape pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout), Some(&gradient_bind_group_layout)],
            immediate_size: 0,
        });
        let shader = if uses_storage {
            shader_module_from_wgsl(device, "quickgui shape pipeline", Cow::Borrowed(QUAD_WGSL))
        } else {
            shader_module_from_wgsl(
                device,
                "quickgui shape pipeline",
                Cow::Owned(rewrite_storage_array_as_uniform(
                    QUAD_WGSL,
                    QUAD_STORAGE_BINDING,
                    "gradients",
                    "gradient_table",
                    "GradientRecord",
                )),
            )
        };
        let attributes = [
            VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: 0,
                shader_location: 0,
            },
            VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: 16,
                shader_location: 1,
            },
            VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: 32,
                shader_location: 2,
            },
            VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: 48,
                shader_location: 3,
            },
            VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: 64,
                shader_location: 4,
            },
            VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: 80,
                shader_location: 5,
            },
            VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: 96,
                shader_location: 6,
            },
            VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: 112,
                shader_location: 7,
            },
        ];
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("quickgui shape pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[Some(VertexBufferLayout {
                    array_stride: mem::size_of::<ShapeInstance>() as BufferAddress,
                    step_mode: VertexStepMode::Instance,
                    attributes: &attributes,
                })],
            },
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        Self {
            pipeline: Arc::new(pipeline),
            bind_group_layout: Arc::new(bind_group_layout),
            gradient_bind_group_layout: Arc::new(gradient_bind_group_layout),
            format,
            uses_storage,
        }
    }
}

impl ShapeRenderer {
    fn new(device: &Device, format: TextureFormat, shared: Option<&ShapePipeline>) -> Self {
        let pipeline = shared
            .filter(|pipeline| pipeline.format == format)
            .cloned()
            .unwrap_or_else(|| ShapePipeline::new(device, format));
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quickgui view uniform"),
            contents: bytemuck::bytes_of(&ViewUniform::zeroed()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("quickgui view bind group"),
            layout: &pipeline.bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let instance_buffers = (0..BUFFERED_FRAMES)
            .map(|_| create_shape_instance_buffer(device, INITIAL_SHAPE_CAPACITY))
            .collect();
        let gradient_capacity = if pipeline.uses_storage {
            INITIAL_GRADIENT_CAPACITY
        } else {
            UNIFORM_TABLE_LEN
        };
        let gradient_buffers: Vec<_> = (0..BUFFERED_FRAMES)
            .map(|_| create_gradient_buffer(device, gradient_capacity, pipeline.uses_storage))
            .collect();
        let gradient_bind_groups = gradient_buffers
            .iter()
            .map(|buffer| {
                create_gradient_bind_group(device, &pipeline.gradient_bind_group_layout, buffer)
            })
            .collect();
        Self {
            pipeline,
            uniform_buffer,
            bind_group,
            instance_buffers,
            instance_capacities: vec![INITIAL_SHAPE_CAPACITY; BUFFERED_FRAMES],
            gradient_buffers,
            gradient_capacities: vec![gradient_capacity; BUFFERED_FRAMES],
            gradient_bind_groups,
            active_buffer: 0,
            uploads: crate::renderer::upload::BufferUploads::default(),
            batches: Vec::with_capacity(16),
            layer_batches: Vec::with_capacity(4),
            pending: Vec::with_capacity(INITIAL_SHAPE_CAPACITY),
            instances: Vec::with_capacity(INITIAL_SHAPE_CAPACITY),
            gradients: Vec::with_capacity(INITIAL_GRADIENT_CAPACITY),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn prepare(
        &mut self,
        device: &Device,
        queue: &Queue,
        scene: &Scene,
        viewport: Rect,
        physical_width: u32,
        physical_height: u32,
        scale: f32,
    ) -> (usize, usize, usize) {
        self.uploads.begin_frame();
        self.instances.clear();
        self.batches.clear();
        self.layer_batches.clear();
        self.pending.clear();
        self.gradients.clear();
        let gradient_limit = self.gradient_limit();
        let mut quads = 0;
        let mut shadows = 0;
        for layer in scene.paint_layers() {
            let batch_start = self.batches.len();
            self.pending.clear();
            for item in layer.paint() {
                let PrimitiveRef::Shape(shape) = item.primitive else {
                    continue;
                };
                let instance = match shape {
                    ShapeRef::Quad(index) => {
                        let quad = &layer.quads()[index];
                        let clip = quad.clip.unwrap_or(viewport);
                        let Some(clip) = clip.intersection(viewport) else {
                            continue;
                        };
                        if !quad.rect.intersects(clip) {
                            continue;
                        }
                        quads += 1;
                        let gradient = admit_gradient(
                            &mut self.gradients,
                            quad.background.as_ref(),
                            quad.rect,
                            gradient_limit,
                        );
                        quad_instance(quad, clip, gradient)
                    }
                    ShapeRef::EdgeQuad(index) => {
                        let quad = &layer.edge_quads()[index];
                        let clip = quad.clip.unwrap_or(viewport);
                        let Some(clip) = clip.intersection(viewport) else {
                            continue;
                        };
                        if !quad.rect.intersects(clip) {
                            continue;
                        }
                        quads += 1;
                        let gradient = admit_gradient(
                            &mut self.gradients,
                            quad.background.as_ref(),
                            quad.rect,
                            gradient_limit,
                        );
                        edge_quad_instance(quad, clip, gradient)
                    }
                    ShapeRef::WavyUnderline(index) => {
                        let underline = &layer.wavy_underlines()[index];
                        let clip = underline.clip.unwrap_or(viewport);
                        let Some(clip) = clip.intersection(viewport) else {
                            continue;
                        };
                        if !underline.rect.intersects(clip) {
                            continue;
                        }
                        quads += 1;
                        wavy_underline_instance(underline, clip)
                    }
                    ShapeRef::Shadow(index) => {
                        let shadow = &layer.shadows()[index];
                        let clip = shadow.clip.unwrap_or(viewport);
                        let Some(clip) = clip.intersection(viewport) else {
                            continue;
                        };
                        let Some(instance) = shadow_instance(shadow, clip) else {
                            continue;
                        };
                        shadows += 1;
                        instance
                    }
                };
                self.pending.push(OrderedShape {
                    order: item.order,
                    instance,
                });
            }
            self.pending.sort_unstable_by_key(|shape| shape.order);
            for shape in &self.pending {
                let index = self.instances.len() as u32;
                self.instances.push(shape.instance);
                if self.batches.len() > batch_start
                    && self
                        .batches
                        .last()
                        .is_some_and(|batch| batch.order == shape.order)
                {
                    self.batches
                        .last_mut()
                        .expect("the previous shape batch exists")
                        .instances
                        .end = index + 1;
                } else {
                    self.batches.push(ShapeBatch {
                        order: shape.order,
                        instances: index..index + 1,
                    });
                }
            }
            self.layer_batches.push(batch_start..self.batches.len());
        }

        self.uploads.write(
            0,
            queue,
            &self.uniform_buffer,
            bytemuck::bytes_of(&ViewUniform {
                viewport: [physical_width as f32, physical_height as f32],
                scale,
                _padding: 0.0,
            }),
        );
        self.active_buffer = (self.active_buffer + 1) % BUFFERED_FRAMES;
        let required = self.instances.len().max(1);
        if required > self.instance_capacities[self.active_buffer] {
            let capacity = required.next_power_of_two();
            self.instance_buffers[self.active_buffer] =
                create_shape_instance_buffer(device, capacity);
            self.instance_capacities[self.active_buffer] = capacity;
            self.uploads.reset(1 + self.active_buffer);
        }
        if !self.instances.is_empty() {
            self.uploads.write(
                1 + self.active_buffer,
                queue,
                &self.instance_buffers[self.active_buffer],
                bytemuck::cast_slice(&self.instances),
            );
        }
        let required_gradients = self.gradients.len().max(1);
        if self.pipeline.uses_storage
            && required_gradients > self.gradient_capacities[self.active_buffer]
        {
            let capacity = required_gradients
                .next_power_of_two()
                .min(MAX_GRADIENTS_PER_FRAME);
            self.gradient_buffers[self.active_buffer] =
                create_gradient_buffer(device, capacity, true);
            self.gradient_bind_groups[self.active_buffer] = create_gradient_bind_group(
                device,
                &self.pipeline.gradient_bind_group_layout,
                &self.gradient_buffers[self.active_buffer],
            );
            self.gradient_capacities[self.active_buffer] = capacity;
            self.uploads.reset(1 + BUFFERED_FRAMES + self.active_buffer);
        }
        if !self.gradients.is_empty() {
            self.uploads.write(
                1 + BUFFERED_FRAMES + self.active_buffer,
                queue,
                &self.gradient_buffers[self.active_buffer],
                bytemuck::cast_slice(&self.gradients),
            );
        }
        (quads, shadows, self.batches.len())
    }

    pub(crate) fn render_order<'pass>(
        &'pass self,
        pass: &mut wgpu::RenderPass<'pass>,
        layer: usize,
        order: u32,
    ) {
        let Some(batches) = self.layer_batches.get(layer) else {
            return;
        };
        let Some(batch) = self.batches[batches.clone()]
            .iter()
            .find(|batch| batch.order == order)
        else {
            return;
        };
        pass.set_pipeline(&self.pipeline.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_bind_group(1, &self.gradient_bind_groups[self.active_buffer], &[]);
        pass.set_vertex_buffer(0, self.instance_buffers[self.active_buffer].slice(..));
        pass.draw(0..6, batch.instances.clone());
    }

    fn gradient_limit(&self) -> usize {
        if self.pipeline.uses_storage {
            MAX_GRADIENTS_PER_FRAME
        } else {
            UNIFORM_TABLE_LEN
        }
    }
}

/// Upload one resolved gradient and return its instance index, or [`NO_GRADIENT`].
fn admit_gradient(
    gradients: &mut Vec<GpuGradient>,
    gradient: Option<&Gradient>,
    bounds: Rect,
    max: usize,
) -> f32 {
    let Some(gradient) = gradient else {
        return NO_GRADIENT;
    };
    if gradients.len() >= max {
        return NO_GRADIENT;
    }
    let index = gradients.len() as f32;
    gradients.push(GradientData::new(gradient, bounds).into());
    index
}

fn quad_instance(quad: &Quad, clip: Rect, gradient: f32) -> ShapeInstance {
    let corners = quad.radius.resolve(quad.rect.width, quad.rect.height);
    ShapeInstance {
        geometry: rect_array(quad.rect),
        primary: quad.fill.as_array(),
        secondary: quad.border_color.as_array(),
        clip: clip_array(clip),
        params: [
            SHAPE_MODE_QUAD,
            corners.maximum(),
            quad.border_width.max(0.0),
            0.0,
        ],
        subject: rect_array(quad.rect),
        corners: corners.as_array(),
        effects: [gradient, 0.0, 0.0, 0.0],
    }
}

fn edge_quad_instance(quad: &EdgeQuad, clip: Rect, gradient: f32) -> ShapeInstance {
    let corners = quad.radius.resolve(quad.rect.width, quad.rect.height);
    ShapeInstance {
        geometry: rect_array(quad.rect),
        primary: quad.fill.as_array(),
        secondary: quad.border_color.as_array(),
        clip: clip_array(clip),
        params: [SHAPE_MODE_EDGE_QUAD, corners.maximum(), 0.0, 0.0],
        subject: [
            quad.border_widths.top.max(0.0),
            quad.border_widths.right.max(0.0),
            quad.border_widths.bottom.max(0.0),
            quad.border_widths.left.max(0.0),
        ],
        corners: corners.as_array(),
        effects: [gradient, quad.border_style.code(), 0.0, 0.0],
    }
}

fn wavy_underline_instance(underline: &WavyUnderline, clip: Rect) -> ShapeInstance {
    ShapeInstance {
        geometry: rect_array(underline.rect),
        primary: underline.color.as_array(),
        secondary: [0.0; 4],
        clip: clip_array(clip),
        params: [
            SHAPE_MODE_WAVY_UNDERLINE,
            underline.baseline,
            underline.thickness,
            underline.wavelength,
        ],
        subject: [underline.amplitude, 0.0, 0.0, 0.0],
        corners: [0.0; 4],
        effects: [NO_GRADIENT, 0.0, 0.0, 0.0],
    }
}

fn shadow_instance(shadow: &Shadow, clip: Rect) -> Option<ShapeInstance> {
    let style = shadow.style;
    let blur = style.blur();
    let element_rect = shadow.element_rect;
    let element_corners = shadow
        .radius
        .resolve(element_rect.width, element_rect.height);
    let (geometry, subject, mode, corners, spread) = if style.is_inset() {
        (
            element_rect,
            dilate_rect(element_rect.translate(style.offset()), -style.spread()),
            SHAPE_MODE_INSET_SHADOW,
            element_corners,
            style.spread(),
        )
    } else {
        let subject = dilate_rect(element_rect.translate(style.offset()), style.spread());
        if subject.is_empty() {
            return None;
        }
        let margin = blur * SHADOW_SIGMA_PER_BLUR_RADIUS * SHADOW_MARGIN_SIGMAS + 1.0;
        (
            dilate_rect(subject, margin),
            subject,
            SHAPE_MODE_DROP_SHADOW,
            element_corners.resolve(subject.width, subject.height),
            0.0,
        )
    };
    if geometry.is_empty() || !geometry.intersects(clip) {
        return None;
    }
    Some(ShapeInstance {
        geometry: rect_array(geometry),
        primary: style.color().as_array(),
        secondary: [0.0; 4],
        clip: clip_array(clip),
        params: [mode, spread, 0.0, blur],
        subject: rect_array(subject),
        corners: corners.as_array(),
        effects: [NO_GRADIENT, 0.0, 0.0, 0.0],
    })
}

fn dilate_rect(rect: Rect, amount: f32) -> Rect {
    let left = rect.x - amount;
    let top = rect.y - amount;
    let right = rect.right() + amount;
    let bottom = rect.bottom() + amount;
    let width = (right - left).max(0.0);
    let height = (bottom - top).max(0.0);
    Rect::new(
        if width > 0.0 {
            left
        } else {
            (left + right) * 0.5
        },
        if height > 0.0 {
            top
        } else {
            (top + bottom) * 0.5
        },
        width,
        height,
    )
}

fn rect_array(rect: Rect) -> [f32; 4] {
    [rect.x, rect.y, rect.width, rect.height]
}

fn clip_array(rect: Rect) -> [f32; 4] {
    [rect.x, rect.y, rect.right(), rect.bottom()]
}

fn create_shape_instance_buffer(device: &Device, capacity: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("quickgui shape instance buffer"),
        size: (capacity * mem::size_of::<ShapeInstance>()) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn create_gradient_buffer(device: &Device, capacity: usize, storage: bool) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("quickgui shape gradient buffer"),
        size: (capacity * mem::size_of::<GpuGradient>()) as u64,
        usage: table_buffer_usages(storage),
        mapped_at_creation: false,
    })
}

fn create_gradient_bind_group(
    device: &Device,
    layout: &wgpu::BindGroupLayout,
    buffer: &wgpu::Buffer,
) -> BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("quickgui shape gradient bind group"),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }],
    })
}

#[derive(Clone, Debug)]
struct TextLayoutKey {
    content: Arc<str>,
    highlights: Option<Arc<[TextHighlight]>>,
    width: Option<f32>,
    font_size: f32,
    line_height: f32,
    monospace_width: Option<f32>,
    family: FontFamily,
    features: FontFeatures,
    fallbacks: Option<FontFallbacks>,
    weight: glyphon::Weight,
    font_style: GlyphStyle,
    font_thicken: bool,
    underline: TextUnderline,
    underline_color: Option<UiColor>,
    underline_wavy: bool,
    underline_thickness: f32,
    strikethrough: bool,
    strikethrough_color: Option<UiColor>,
    align: TextAlign,
    wrap: TextWrap,
    text_overflow: Option<TextOverflow>,
    line_clamp: Option<usize>,
    shaping: TextShaping,
    /// Shaping-relevant properties added by the extended text-styling API.
    extras: TextShapingExtras,
    scale: f32,
}

/// The extended text-styling properties that change shaped output.
///
/// These are part of the retained shaping key so a cached layout is only reused for a run whose
/// spacing, direction, case mapping, and break behavior are all identical. Purely visual
/// properties, such as a text shadow, deliberately stay out of it.
#[derive(Clone, Copy, Debug)]
struct TextShapingExtras {
    overline: bool,
    overline_color: Option<UiColor>,
    direction: TextDirection,
    letter_spacing: f32,
    word_spacing: f32,
    transform: Option<TextTransform>,
    word_break: WordBreak,
    overflow_wrap: OverflowWrap,
    hyphens: Hyphens,
}

impl TextShapingExtras {
    fn from_style(style: &TextStyle) -> Self {
        Self {
            overline: style.overline,
            overline_color: style.overline_color,
            direction: style.direction,
            letter_spacing: style.letter_spacing,
            word_spacing: style.word_spacing,
            transform: style.transform,
            word_break: style.word_break,
            overflow_wrap: style.overflow_wrap,
            hyphens: style.hyphens,
        }
    }

    #[allow(clippy::type_complexity)]
    fn canonical(
        self,
    ) -> (
        bool,
        Option<[u32; 4]>,
        TextDirection,
        u32,
        u32,
        Option<TextTransform>,
        WordBreak,
        OverflowWrap,
        Hyphens,
    ) {
        (
            self.overline,
            optional_color_bits(self.overline_color),
            self.direction,
            (self.letter_spacing + 0.0).to_bits(),
            (self.word_spacing + 0.0).to_bits(),
            self.transform,
            self.word_break,
            self.overflow_wrap,
            self.hyphens,
        )
    }
}

impl PartialEq for TextShapingExtras {
    fn eq(&self, other: &Self) -> bool {
        self.canonical() == other.canonical()
    }
}

impl Eq for TextShapingExtras {}

impl Hash for TextShapingExtras {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.canonical().hash(state);
    }
}

impl PartialEq for TextLayoutKey {
    fn eq(&self, other: &Self) -> bool {
        self.content == other.content
            && highlights_equal(&self.highlights, &other.highlights)
            && self.width.map(f32::to_bits) == other.width.map(f32::to_bits)
            && self.font_size.to_bits() == other.font_size.to_bits()
            && self.line_height.to_bits() == other.line_height.to_bits()
            && self.monospace_width.map(f32::to_bits) == other.monospace_width.map(f32::to_bits)
            && self.family == other.family
            && self.features == other.features
            && self.fallbacks == other.fallbacks
            && self.weight == other.weight
            && self.font_style == other.font_style
            && self.font_thicken == other.font_thicken
            && self.underline == other.underline
            && optional_color_bits(self.underline_color)
                == optional_color_bits(other.underline_color)
            && self.underline_wavy == other.underline_wavy
            && self.underline_thickness.to_bits() == other.underline_thickness.to_bits()
            && self.strikethrough == other.strikethrough
            && optional_color_bits(self.strikethrough_color)
                == optional_color_bits(other.strikethrough_color)
            && self.align == other.align
            && self.wrap == other.wrap
            && self.text_overflow == other.text_overflow
            && self.line_clamp == other.line_clamp
            && self.shaping == other.shaping
            && self.extras == other.extras
            && self.scale.to_bits() == other.scale.to_bits()
    }
}

impl Eq for TextLayoutKey {}

impl TextLayoutKey {
    /// Whether two entries have identical shaping input and differ, if at all, only in the
    /// available layout width.
    ///
    /// Cosmic Text keeps shaped glyph runs when `Buffer::set_size` invalidates layout, so a
    /// stable text id can cheaply reflow during window resizing without rebuilding its content,
    /// attributes, font matches, and glyph shaping.
    fn same_except_width(&self, other: &Self) -> bool {
        self.text_overflow.as_ref().is_none_or(uses_cosmic_ellipsis)
            && other
                .text_overflow
                .as_ref()
                .is_none_or(uses_cosmic_ellipsis)
            && self.text_overflow == other.text_overflow
            && self.content == other.content
            && highlights_equal(&self.highlights, &other.highlights)
            && self.font_size.to_bits() == other.font_size.to_bits()
            && self.line_height.to_bits() == other.line_height.to_bits()
            && self.monospace_width.map(f32::to_bits) == other.monospace_width.map(f32::to_bits)
            && self.family == other.family
            && self.features == other.features
            && self.fallbacks == other.fallbacks
            && self.weight == other.weight
            && self.font_style == other.font_style
            && self.font_thicken == other.font_thicken
            && self.underline == other.underline
            && optional_color_bits(self.underline_color)
                == optional_color_bits(other.underline_color)
            && self.underline_wavy == other.underline_wavy
            && self.underline_thickness.to_bits() == other.underline_thickness.to_bits()
            && self.strikethrough == other.strikethrough
            && optional_color_bits(self.strikethrough_color)
                == optional_color_bits(other.strikethrough_color)
            && self.align == other.align
            && self.wrap == other.wrap
            && self.line_clamp == other.line_clamp
            && self.shaping == other.shaping
            && self.extras == other.extras
            && self.scale.to_bits() == other.scale.to_bits()
    }
}

impl Hash for TextLayoutKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.content.hash(state);
        hash_highlights(&self.highlights, state);
        self.width.map(f32::to_bits).hash(state);
        self.font_size.to_bits().hash(state);
        self.line_height.to_bits().hash(state);
        self.monospace_width.map(f32::to_bits).hash(state);
        self.family.hash(state);
        self.features.hash(state);
        self.fallbacks.hash(state);
        self.weight.hash(state);
        self.font_style.hash(state);
        self.font_thicken.hash(state);
        self.underline.hash(state);
        optional_color_bits(self.underline_color).hash(state);
        self.underline_wavy.hash(state);
        self.underline_thickness.to_bits().hash(state);
        self.strikethrough.hash(state);
        optional_color_bits(self.strikethrough_color).hash(state);
        self.align.hash(state);
        self.wrap.hash(state);
        self.text_overflow.hash(state);
        self.line_clamp.hash(state);
        self.shaping.hash(state);
        self.extras.hash(state);
        self.scale.to_bits().hash(state);
    }
}

fn highlights_equal(
    left: &Option<Arc<[TextHighlight]>>,
    right: &Option<Arc<[TextHighlight]>>,
) -> bool {
    let left = left.as_deref().unwrap_or_default();
    let right = right.as_deref().unwrap_or_default();
    left.len() == right.len()
        && left.iter().zip(right).all(|(left, right)| {
            left.range == right.range
                && optional_color_bits(left.style.color) == optional_color_bits(right.style.color)
                && left.style.family == right.style.family
                && left.style.features == right.style.features
                && left.style.fallbacks == right.style.fallbacks
                && left.style.weight == right.style.weight
                && left.style.glyph_style == right.style.glyph_style
                && left.style.underline == right.style.underline
                && optional_color_bits(left.style.underline_color)
                    == optional_color_bits(right.style.underline_color)
                && left.style.underline_wavy == right.style.underline_wavy
                && left.style.underline_thickness.map(f32::to_bits)
                    == right.style.underline_thickness.map(f32::to_bits)
                && left.style.strikethrough == right.style.strikethrough
                && optional_color_bits(left.style.strikethrough_color)
                    == optional_color_bits(right.style.strikethrough_color)
        })
}

fn hash_highlights<H: Hasher>(highlights: &Option<Arc<[TextHighlight]>>, state: &mut H) {
    let highlights = highlights.as_deref().unwrap_or_default();
    highlights.len().hash(state);
    for highlight in highlights {
        highlight.range.start.hash(state);
        highlight.range.end.hash(state);
        optional_color_bits(highlight.style.color).hash(state);
        highlight.style.family.hash(state);
        highlight.style.features.hash(state);
        highlight.style.fallbacks.hash(state);
        highlight.style.weight.hash(state);
        highlight.style.glyph_style.hash(state);
        highlight.style.underline.hash(state);
        optional_color_bits(highlight.style.underline_color).hash(state);
        highlight.style.underline_wavy.hash(state);
        highlight
            .style
            .underline_thickness
            .map(f32::to_bits)
            .hash(state);
        highlight.style.strikethrough.hash(state);
        optional_color_bits(highlight.style.strikethrough_color).hash(state);
    }
}

fn optional_color_bits(color: Option<UiColor>) -> Option<[u32; 4]> {
    color.map(|color| {
        [
            color.r.to_bits(),
            color.g.to_bits(),
            color.b.to_bits(),
            color.a.to_bits(),
        ]
    })
}

#[derive(Clone, Debug)]
struct ProjectedText {
    content: Arc<str>,
    highlights: Option<Arc<[TextHighlight]>>,
    mapping: TextProjection,
}

#[derive(Clone, Debug)]
enum TextProjection {
    Identity,
    /// A one-to-one content rewrite such as a case mapping or soft-hyphen removal.
    ///
    /// `spans` covers the whole display and original strings in order. A span whose display and
    /// original lengths agree maps linearly; one that does not (a case mapping that changes the
    /// UTF-8 length, or a removed character) snaps to its nearest edge, which keeps selection and
    /// copy anchored to real boundaries of the source string.
    Mapped {
        original_len: usize,
        spans: Arc<[ProjectionSpan]>,
    },
    Truncated {
        original_len: usize,
        retained: Arc<[ProjectionSpan]>,
        insertion_display: Range<usize>,
        omitted_original: Range<usize>,
    },
}

#[derive(Clone, Debug)]
struct ProjectionSpan {
    display: Range<usize>,
    original: Range<usize>,
}

impl ProjectedText {
    fn identity(content: Arc<str>, highlights: Option<Arc<[TextHighlight]>>) -> Self {
        Self {
            content,
            highlights,
            mapping: TextProjection::Identity,
        }
    }

    fn display_to_original(&self, index: usize) -> usize {
        let mut index = index.min(self.content.len());
        while !self.content.is_char_boundary(index) {
            index -= 1;
        }
        if let TextProjection::Mapped {
            original_len,
            spans,
        } = &self.mapping
        {
            for span in spans.iter() {
                if index >= span.display.start && index <= span.display.end {
                    if span.display.len() == span.original.len() {
                        return span.original.start + (index - span.display.start);
                    }
                    return if index * 2 <= span.display.start + span.display.end {
                        span.original.start
                    } else {
                        span.original.end
                    };
                }
            }
            return *original_len;
        }
        let TextProjection::Truncated {
            original_len,
            retained,
            insertion_display,
            omitted_original,
        } = &self.mapping
        else {
            return index;
        };
        for span in retained.iter() {
            if index >= span.display.start && index <= span.display.end {
                return (span.original.start + index.saturating_sub(span.display.start))
                    .min(span.original.end);
            }
        }
        if index <= insertion_display.start {
            omitted_original.start
        } else if index >= insertion_display.end {
            omitted_original.end.min(*original_len)
        } else {
            let midpoint =
                insertion_display.start + (insertion_display.end - insertion_display.start) / 2;
            if index <= midpoint {
                omitted_original.start
            } else {
                omitted_original.end.min(*original_len)
            }
        }
    }

    fn original_to_display(&self, index: usize) -> usize {
        if let TextProjection::Mapped {
            original_len,
            spans,
        } = &self.mapping
        {
            let index = index.min(*original_len);
            for span in spans.iter() {
                if index >= span.original.start && index <= span.original.end {
                    if span.display.len() == span.original.len() {
                        return span.display.start + (index - span.original.start);
                    }
                    return if index * 2 <= span.original.start + span.original.end {
                        span.display.start
                    } else {
                        span.display.end
                    };
                }
            }
            return self.content.len();
        }
        let TextProjection::Truncated {
            original_len,
            retained,
            insertion_display,
            omitted_original,
        } = &self.mapping
        else {
            return index.min(self.content.len());
        };
        let index = index.min(*original_len);
        for span in retained.iter() {
            if index >= span.original.start && index <= span.original.end {
                return (span.display.start + index.saturating_sub(span.original.start))
                    .min(span.display.end);
            }
        }
        let distance_to_start = index.saturating_sub(omitted_original.start);
        let distance_to_end = omitted_original.end.saturating_sub(index);
        if distance_to_start <= distance_to_end {
            insertion_display.start
        } else {
            insertion_display.end
        }
    }

    fn display_ranges_for_original(&self, range: Range<usize>) -> Vec<Range<usize>> {
        if let TextProjection::Mapped {
            original_len,
            spans,
        } = &self.mapping
        {
            let start = range.start.min(*original_len);
            let end = range.end.min(*original_len).max(start);
            if start == end {
                return Vec::new();
            }
            let mut merged: Vec<Range<usize>> = Vec::with_capacity(2);
            for span in spans.iter() {
                let overlap_start = start.max(span.original.start);
                let overlap_end = end.min(span.original.end);
                if overlap_start >= overlap_end {
                    continue;
                }
                let projected = if span.display.len() == span.original.len() {
                    span.display.start + overlap_start - span.original.start
                        ..span.display.start + overlap_end - span.original.start
                } else {
                    span.display.clone()
                };
                if let Some(previous) = merged.last_mut()
                    && projected.start <= previous.end
                {
                    previous.end = previous.end.max(projected.end);
                } else {
                    merged.push(projected);
                }
            }
            return merged;
        }
        let TextProjection::Truncated {
            original_len,
            retained,
            insertion_display,
            omitted_original,
        } = &self.mapping
        else {
            let start = range.start.min(self.content.len());
            let end = range.end.min(self.content.len()).max(start);
            return (start < end).then_some(start..end).into_iter().collect();
        };
        let start = range.start.min(*original_len);
        let end = range.end.min(*original_len).max(start);
        if start == end {
            return Vec::new();
        }

        let mut projected = Vec::with_capacity(retained.len() + 1);
        for span in retained.iter() {
            let intersection_start = start.max(span.original.start);
            let intersection_end = end.min(span.original.end);
            if intersection_start < intersection_end {
                projected.push(
                    span.display.start + intersection_start - span.original.start
                        ..span.display.start + intersection_end - span.original.start,
                );
            }
        }
        if start < omitted_original.end
            && end > omitted_original.start
            && !insertion_display.is_empty()
        {
            projected.push(insertion_display.clone());
        }
        projected.sort_unstable_by_key(|range| range.start);
        let mut merged: Vec<Range<usize>> = Vec::with_capacity(projected.len());
        for range in projected {
            if let Some(previous) = merged.last_mut()
                && range.start <= previous.end
            {
                previous.end = previous.end.max(range.end);
            } else {
                merged.push(range);
            }
        }
        merged
    }
}

struct TextEntry {
    key: TextLayoutKey,
    buffer: Arc<Buffer>,
    projection: Arc<ProjectedText>,
    last_used_frame: u64,
}

struct SharedTextEntry {
    buffer: Arc<Buffer>,
    projection: Arc<ProjectedText>,
    last_used_frame: u64,
}

pub(crate) struct TextSystem {
    font_system: SharedFontSystem,
    cache: Cache,
    swash_cache: SwashCache,
    viewport: Viewport,
    atlas: TextAtlas,
    renderers: Vec<TextRenderer>,
    buffers: HashMap<TextId, TextEntry>,
    shared_buffers: HashMap<TextLayoutKey, SharedTextEntry>,
    colors: HashMap<[u32; 4], glyphon::Color>,
    seen: HashSet<TextId>,
    visible: Vec<VisibleText>,
    batches: Vec<TextBatch>,
    layer_batches: Vec<Range<usize>>,
    eviction_keys: Vec<TextId>,
    shared_eviction_keys: Vec<TextLayoutKey>,
    frame: u64,
}

#[derive(Clone)]
struct VisibleText {
    order: u32,
    buffer: Arc<Buffer>,
    left: f32,
    top: f32,
    bounds: TextBounds,
    color: glyphon::Color,
    opacity: f32,
    mask: Option<glyphon::TextMask>,
}

struct TextBatch {
    order: u32,
    visible: Range<usize>,
    renderer: usize,
}

#[derive(Clone, Copy, Debug)]
struct TextHitTest {
    width: f32,
    scale: f32,
    point: Point,
}

impl From<PerformanceProfile> for wgpu::PowerPreference {
    fn from(value: PerformanceProfile) -> Self {
        match value {
            PerformanceProfile::Balanced => wgpu::PowerPreference::None,
            PerformanceProfile::LowPower => wgpu::PowerPreference::LowPower,
            PerformanceProfile::HighPerformance => wgpu::PowerPreference::HighPerformance,
        }
    }
}

#[cfg(test)]
mod tests;

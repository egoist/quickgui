use std::{
    collections::{HashMap, HashSet},
    mem,
    ops::Range,
};

use bytemuck::{Pod, Zeroable};
use wgpu::{
    BindGroup, BufferAddress, ColorTargetState, Device, Extent3d, FilterMode, FragmentState,
    MultisampleState, Origin3d, PipelineCompilationOptions, PrimitiveState, PrimitiveTopology,
    Queue, RenderPipeline, RenderPipelineDescriptor, Sampler, ShaderStages, TexelCopyBufferLayout,
    TexelCopyTextureInfo, TextureAspect, TextureDimension, TextureFormat, TextureUsages,
    TextureViewDescriptor, VertexAttribute, VertexBufferLayout, VertexFormat, VertexState,
    VertexStepMode, util::DeviceExt,
};

use crate::{
    Rect, Scene, Svg,
    scene::{PrimitiveRef, SvgPrimitive},
    svg::{MAX_SVG_RASTER_DIMENSION, MAX_SVG_RASTER_PIXELS, SvgId, SvgMask},
};

/// Maximum one-channel SVG mask bytes retained by one renderer.
pub const MAX_GPU_SVG_CACHE_BYTES: u64 = 32 * 1024 * 1024;
/// Maximum distinct SVG identity-and-size mask textures retained by one renderer.
pub const MAX_GPU_SVG_CACHE_ENTRIES: usize = 512;

const INITIAL_SVG_CAPACITY: usize = 64;
const BUFFERED_FRAMES: usize = 3;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SvgPrepareStats {
    pub svgs: usize,
    pub rasterizations: usize,
    pub draw_calls: usize,
    pub cache_bytes: u64,
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
struct SvgInstance {
    rect: [f32; 4],
    uv: [f32; 4],
    clip: [f32; 4],
    mask: [f32; 4],
    color: [f32; 4],
    scale_and_translation: [f32; 4],
    radius_and_rotation: [f32; 2],
    _padding: [f32; 2],
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct SvgRasterKey {
    svg: SvgId,
    width: u32,
    height: u32,
}

struct AdmittedSvg {
    key: SvgRasterKey,
    svg: Svg,
}

struct CachedSvg {
    _texture: wgpu::Texture,
    _view: wgpu::TextureView,
    bind_group: BindGroup,
    bytes: u64,
    last_used_frame: u64,
}

struct SvgBatch {
    order: u32,
    key: SvgRasterKey,
    instances: Range<u32>,
}

#[derive(Clone, Copy)]
struct OrderedSvg {
    order: u32,
    key: SvgRasterKey,
    instance: SvgInstance,
}

pub(crate) struct SvgRenderer {
    pipeline: RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    view_bind_group: BindGroup,
    svg_bind_group_layout: wgpu::BindGroupLayout,
    sampler: Sampler,
    instance_buffers: Vec<wgpu::Buffer>,
    instance_capacities: Vec<usize>,
    active_buffer: usize,
    pub(crate) uploads: crate::renderer::upload::BufferUploads,
    instances: Vec<SvgInstance>,
    pending: Vec<OrderedSvg>,
    batches: Vec<SvgBatch>,
    layer_batches: Vec<Range<usize>>,
    admitted_svgs: Vec<AdmittedSvg>,
    admitted_keys: HashSet<SvgRasterKey>,
    cache: HashMap<SvgRasterKey, CachedSvg>,
    resident_bytes: u64,
    frame: u64,
}

impl SvgRenderer {
    pub(crate) fn new(device: &Device, format: TextureFormat) -> Self {
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quickgui SVG view uniform"),
            contents: bytemuck::bytes_of(&ViewUniform::zeroed()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let view_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("quickgui SVG view bind group layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let view_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("quickgui SVG view bind group"),
            layout: &view_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });
        let svg_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("quickgui SVG mask bind group layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("quickgui SVG mask sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..Default::default()
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("quickgui SVG pipeline layout"),
            bind_group_layouts: &[Some(&view_bind_group_layout), Some(&svg_bind_group_layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::include_wgsl!("svg.wgsl"));
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
                format: VertexFormat::Float32x2,
                offset: 96,
                shader_location: 6,
            },
        ];
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("quickgui SVG pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[Some(VertexBufferLayout {
                    array_stride: mem::size_of::<SvgInstance>() as BufferAddress,
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
            pipeline,
            uniform_buffer,
            view_bind_group,
            svg_bind_group_layout,
            sampler,
            instance_buffers: (0..BUFFERED_FRAMES)
                .map(|_| create_instance_buffer(device, INITIAL_SVG_CAPACITY))
                .collect(),
            instance_capacities: vec![INITIAL_SVG_CAPACITY; BUFFERED_FRAMES],
            active_buffer: 0,
            uploads: crate::renderer::upload::BufferUploads::default(),
            instances: Vec::with_capacity(INITIAL_SVG_CAPACITY),
            pending: Vec::with_capacity(INITIAL_SVG_CAPACITY),
            batches: Vec::with_capacity(16),
            layer_batches: Vec::with_capacity(4),
            admitted_svgs: Vec::with_capacity(16),
            admitted_keys: HashSet::with_capacity(16),
            cache: HashMap::with_capacity(32),
            resident_bytes: 0,
            frame: 0,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn prepare(
        &mut self,
        device: &Device,
        queue: &Queue,
        scene: &Scene,
        viewport: Rect,
        physical_width: u32,
        physical_height: u32,
        scale: f32,
    ) -> SvgPrepareStats {
        self.uploads.begin_frame();
        self.frame = self.frame.wrapping_add(1);
        self.instances.clear();
        self.pending.clear();
        self.batches.clear();
        self.layer_batches.clear();
        self.admitted_svgs.clear();
        self.admitted_keys.clear();

        let mut admitted_bytes = 0_u64;
        for layer in scene.paint_layers() {
            for primitive in layer.svgs() {
                if !visible_svg(primitive, viewport) {
                    continue;
                }
                let Some(key) = raster_key(primitive, scale) else {
                    continue;
                };
                if self.admitted_keys.contains(&key) {
                    continue;
                }
                let bytes = raster_bytes(key);
                if self.admitted_svgs.len() == MAX_GPU_SVG_CACHE_ENTRIES
                    || admitted_bytes.saturating_add(bytes) > MAX_GPU_SVG_CACHE_BYTES
                {
                    continue;
                }
                admitted_bytes += bytes;
                self.admitted_keys.insert(key);
                self.admitted_svgs.push(AdmittedSvg {
                    key,
                    svg: primitive.svg.clone(),
                });
            }
        }

        let mut rasterizations = 0;
        for index in 0..self.admitted_svgs.len() {
            let key = self.admitted_svgs[index].key;
            if let Some(cached) = self.cache.get_mut(&key) {
                cached.last_used_frame = self.frame;
                continue;
            }
            let svg = self.admitted_svgs[index].svg.clone();
            self.evict_until_fits(raster_bytes(key));
            match svg.rasterize_mask(key.width, key.height) {
                Ok(mask) => {
                    let cached = upload_svg_mask(
                        device,
                        queue,
                        &self.svg_bind_group_layout,
                        &self.sampler,
                        mask,
                        self.frame,
                    );
                    self.resident_bytes += cached.bytes;
                    self.cache.insert(key, cached);
                    rasterizations += 1;
                }
                Err(error) => {
                    tracing::warn!(%error, width = key.width, height = key.height, "could not rasterize SVG");
                }
            }
        }

        for layer in scene.paint_layers() {
            let batch_start = self.batches.len();
            self.pending.clear();
            for item in layer.paint() {
                let PrimitiveRef::Svg(index) = item.primitive else {
                    continue;
                };
                let primitive = &layer.svgs()[index];
                let Some(key) = raster_key(primitive, scale) else {
                    continue;
                };
                if !self.admitted_keys.contains(&key) || !self.cache.contains_key(&key) {
                    continue;
                }
                let clip = primitive.clip.unwrap_or(viewport);
                let Some(clip) = clip.intersection(viewport) else {
                    continue;
                };
                if !primitive.render_bounds().intersects(clip) {
                    continue;
                }
                let transform_scale = primitive.transform.scale_factors();
                let translation = primitive.transform.translation();
                let destination = crate::renderer::snap_paint_rect(primitive.destination, scale);
                self.pending.push(OrderedSvg {
                    order: item.order,
                    key,
                    instance: SvgInstance {
                        rect: [
                            destination.x,
                            destination.y,
                            destination.width,
                            destination.height,
                        ],
                        uv: [
                            primitive.source_uv.x,
                            primitive.source_uv.y,
                            primitive.source_uv.width,
                            primitive.source_uv.height,
                        ],
                        clip: [clip.x, clip.y, clip.right(), clip.bottom()],
                        mask: [
                            primitive.mask.x,
                            primitive.mask.y,
                            primitive.mask.width,
                            primitive.mask.height,
                        ],
                        color: primitive.color.as_array(),
                        scale_and_translation: [
                            transform_scale[0],
                            transform_scale[1],
                            translation.x,
                            translation.y,
                        ],
                        radius_and_rotation: [primitive.radius, primitive.transform.rotation()],
                        _padding: [0.0; 2],
                    },
                });
            }
            self.pending
                .sort_unstable_by_key(|svg| (svg.order, svg.key));
            for svg in &self.pending {
                let index = self.instances.len() as u32;
                self.instances.push(svg.instance);
                let extends_last = self.batches.len() > batch_start
                    && self
                        .batches
                        .last()
                        .is_some_and(|batch| batch.order == svg.order && batch.key == svg.key);
                if extends_last {
                    self.batches
                        .last_mut()
                        .expect("the previous SVG batch exists")
                        .instances
                        .end = index + 1;
                } else {
                    self.batches.push(SvgBatch {
                        order: svg.order,
                        key: svg.key,
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
            self.instance_buffers[self.active_buffer] = create_instance_buffer(device, capacity);
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

        debug_assert!(self.cache.len() <= MAX_GPU_SVG_CACHE_ENTRIES);
        debug_assert!(self.resident_bytes <= MAX_GPU_SVG_CACHE_BYTES);
        SvgPrepareStats {
            svgs: self.instances.len(),
            rasterizations,
            draw_calls: self.batches.len(),
            cache_bytes: self.resident_bytes,
        }
    }

    pub(crate) fn render_order<'pass>(
        &'pass self,
        pass: &mut wgpu::RenderPass<'pass>,
        layer: usize,
        order: u32,
    ) {
        let Some(layer_batches) = self.layer_batches.get(layer) else {
            return;
        };
        let mut batches = self.batches[layer_batches.clone()]
            .iter()
            .filter(|batch| batch.order == order);
        let Some(first) = batches.next() else {
            return;
        };
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.view_bind_group, &[]);
        pass.set_vertex_buffer(0, self.instance_buffers[self.active_buffer].slice(..));
        for batch in std::iter::once(first).chain(batches) {
            let cached = self
                .cache
                .get(&batch.key)
                .expect("visible SVG mask is resident");
            pass.set_bind_group(1, &cached.bind_group, &[]);
            pass.draw(0..6, batch.instances.clone());
        }
    }

    fn evict_until_fits(&mut self, incoming_bytes: u64) {
        while self.cache.len() >= MAX_GPU_SVG_CACHE_ENTRIES
            || self.resident_bytes + incoming_bytes > MAX_GPU_SVG_CACHE_BYTES
        {
            let Some(key) = self
                .cache
                .iter()
                .filter(|(key, _)| !self.admitted_keys.contains(key))
                .min_by_key(|(_, cached)| cached.last_used_frame)
                .map(|(key, _)| *key)
            else {
                debug_assert!(false, "admitted SVG set must fit the cache limits");
                break;
            };
            if let Some(removed) = self.cache.remove(&key) {
                self.resident_bytes = self.resident_bytes.saturating_sub(removed.bytes);
            }
        }
    }
}

fn visible_svg(primitive: &SvgPrimitive, viewport: Rect) -> bool {
    primitive
        .clip
        .unwrap_or(viewport)
        .intersection(viewport)
        .is_some_and(|clip| primitive.render_bounds().intersects(clip))
}

fn raster_key(primitive: &SvgPrimitive, scale: f32) -> Option<SvgRasterKey> {
    if !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    let transform_scale = primitive.transform.scale_factors();
    let width = f64::from(primitive.destination.width) / f64::from(primitive.source_uv.width.abs())
        * f64::from(scale)
        * f64::from(transform_scale[0].abs());
    let height = f64::from(primitive.destination.height)
        / f64::from(primitive.source_uv.height.abs())
        * f64::from(scale)
        * f64::from(transform_scale[1].abs());
    // Supersample retained masks so thin strokes survive minification and
    // fractional placement. The existing byte and dimension budgets still apply.
    let (width, height) = bounded_raster_size(width * 2.0, height * 2.0)?;
    Some(SvgRasterKey {
        svg: primitive.svg.id(),
        width,
        height,
    })
}

fn bounded_raster_size(width: f64, height: f64) -> Option<(u32, u32)> {
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return None;
    }
    let mut width = width.ceil().max(1.0);
    let mut height = height.ceil().max(1.0);
    let dimension_scale = (f64::from(MAX_SVG_RASTER_DIMENSION) / width)
        .min(f64::from(MAX_SVG_RASTER_DIMENSION) / height)
        .min(1.0);
    width *= dimension_scale;
    height *= dimension_scale;
    let pixels = width * height;
    if pixels > MAX_SVG_RASTER_PIXELS as f64 {
        let pixel_scale = (MAX_SVG_RASTER_PIXELS as f64 / pixels).sqrt();
        width *= pixel_scale;
        height *= pixel_scale;
    }
    let width = width.floor().max(1.0) as u32;
    let height = height.floor().max(1.0) as u32;
    debug_assert!(width <= MAX_SVG_RASTER_DIMENSION);
    debug_assert!(height <= MAX_SVG_RASTER_DIMENSION);
    debug_assert!(u64::from(width) * u64::from(height) <= MAX_SVG_RASTER_PIXELS);
    Some((width, height))
}

fn raster_bytes(key: SvgRasterKey) -> u64 {
    u64::from(key.width) * u64::from(key.height)
}

fn upload_svg_mask(
    device: &Device,
    queue: &Queue,
    layout: &wgpu::BindGroupLayout,
    sampler: &Sampler,
    mask: SvgMask,
    frame: u64,
) -> CachedSvg {
    let size = Extent3d {
        width: mask.width,
        height: mask.height,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("quickgui cached SVG mask"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::R8Unorm,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        },
        &mask.alpha,
        TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(mask.width),
            rows_per_image: Some(mask.height),
        },
        size,
    );
    let view = texture.create_view(&TextureViewDescriptor::default());
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("quickgui cached SVG bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    });
    CachedSvg {
        _texture: texture,
        _view: view,
        bind_group,
        bytes: mask.alpha.len() as u64,
        last_used_frame: frame,
    }
}

fn create_instance_buffer(device: &Device, capacity: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("quickgui SVG instance buffer"),
        size: (capacity * mem::size_of::<SvgInstance>()) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, SvgTransform};

    fn icon() -> Svg {
        Svg::from_svg(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="12"><path d="M0 0h24v12H0z"/></svg>"#,
        )
        .unwrap()
    }

    #[test]
    fn raster_size_is_density_aware_and_bounded() {
        let primitive = SvgPrimitive::new(icon(), Rect::new(0.0, 0.0, 48.0, 24.0), Color::WHITE);
        let key = raster_key(&primitive, 2.0).unwrap();
        assert_eq!((key.width, key.height), (192, 96));

        let huge = bounded_raster_size(1_000_000.0, 1_000_000.0).unwrap();
        assert!(huge.0 <= MAX_SVG_RASTER_DIMENSION);
        assert!(huge.1 <= MAX_SVG_RASTER_DIMENSION);
        assert!(u64::from(huge.0) * u64::from(huge.1) <= MAX_SVG_RASTER_PIXELS);
    }

    #[test]
    fn cover_crop_rasterizes_the_full_source_at_visible_density() {
        let primitive = SvgPrimitive::new(icon(), Rect::new(0.0, 0.0, 100.0, 100.0), Color::WHITE)
            .source_uv(Rect::new(0.25, 0.0, 0.5, 1.0));
        let key = raster_key(&primitive, 2.0).unwrap();
        assert_eq!((key.width, key.height), (800, 400));
    }

    #[test]
    fn color_and_translation_do_not_invalidate_the_mask() {
        let asset = icon();
        let first = SvgPrimitive::new(asset.clone(), Rect::new(0.0, 0.0, 24.0, 12.0), Color::WHITE);
        let second = SvgPrimitive::new(asset, Rect::new(0.0, 0.0, 24.0, 12.0), Color::BLACK)
            .transform(SvgTransform::new().translate(50.0, 20.0));
        assert_eq!(raster_key(&first, 2.0), raster_key(&second, 2.0));
    }

    #[test]
    fn transformed_visibility_uses_rendered_bounds() {
        let primitive =
            SvgPrimitive::new(icon(), Rect::new(-100.0, 10.0, 20.0, 20.0), Color::WHITE)
                .transform(SvgTransform::new().translate(100.0, 0.0));
        assert!(visible_svg(&primitive, Rect::new(0.0, 0.0, 100.0, 100.0)));
    }
}

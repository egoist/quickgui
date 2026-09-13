use std::{borrow::Cow, mem, ops::Range};

use bytemuck::{Pod, Zeroable};
use wgpu::{
    BindGroup, BufferAddress, ColorTargetState, Device, FragmentState, MultisampleState,
    PipelineCompilationOptions, PrimitiveState, PrimitiveTopology, Queue, RenderPipeline,
    RenderPipelineDescriptor, ShaderStages, TextureFormat, VertexAttribute, VertexBufferLayout,
    VertexFormat, VertexState, VertexStepMode, util::DeviceExt,
};

use crate::{
    MAX_GRADIENT_STOPS, Rect, Scene,
    path::GradientData,
    renderer::{
        UNIFORM_TABLE_LEN, read_only_table_binding, rewrite_storage_array_as_uniform,
        shader_module_from_wgsl, table_buffer_usages, uses_read_only_storage_buffers,
    },
    scene::{PathPrimitive, PrimitiveRef},
};

const PATH_WGSL: &str = include_str!("path.wgsl");
const PATH_STORAGE_BINDING: &str =
    "@group(1) @binding(0) var<storage, read> paints: array<PathPaint>;";

/// Maximum tessellated path vertices uploaded in one frame.
///
/// Admission follows scene order. Once the budget is exhausted, later paths are skipped rather
/// than allowing an unbounded transient allocation.
pub const MAX_GPU_PATH_VERTICES: usize = 262_144;
/// Maximum visible path paints uploaded in one frame.
pub const MAX_GPU_PATHS_PER_FRAME: usize = 16_384;

const INITIAL_VERTEX_CAPACITY: usize = 4_096;
const INITIAL_PAINT_CAPACITY: usize = 64;
const BUFFERED_FRAMES: usize = 3;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct PathPrepareStats {
    pub paths: usize,
    pub vertices: usize,
    pub draw_calls: usize,
    pub skipped_paths: usize,
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
struct GpuPathVertex {
    position: [f32; 2],
    barycentric: [f32; 3],
    edge_mask: [f32; 3],
    paint_index: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuPathPaint {
    /// Scale XY followed by translation XY.
    transform: [f32; 4],
    /// Logical left, top, right, bottom.
    clip: [f32; 4],
    /// Solid fill, used when `header.w` is zero.
    color: [f32; 4],
    /// Gradient kind, interpolation space, stop count, and gradient flag.
    header: [f32; 4],
    /// Linear start/end XY, radial center and radii, or conic center and start angle.
    geometry: [f32; 4],
    /// Stop positions 0..4 followed by 4..8.
    positions: [[f32; 4]; 2],
    colors: [[f32; 4]; MAX_GRADIENT_STOPS],
}

const _: () = assert!(UNIFORM_TABLE_LEN * mem::size_of::<GpuPathPaint>() <= 16 * 1024);

#[derive(Clone, Copy)]
struct PendingPath {
    order: u32,
    primitive: usize,
}

struct PathBatch {
    order: u32,
    vertices: Range<u32>,
}

pub(crate) struct PathRenderer {
    pipeline: RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    view_bind_group: BindGroup,
    paint_bind_group_layout: wgpu::BindGroupLayout,
    vertex_buffers: Vec<wgpu::Buffer>,
    vertex_capacities: Vec<usize>,
    paint_buffers: Vec<wgpu::Buffer>,
    paint_capacities: Vec<usize>,
    paint_bind_groups: Vec<BindGroup>,
    uses_storage: bool,
    active_buffer: usize,
    pub(crate) uploads: crate::renderer::upload::BufferUploads,
    vertices: Vec<GpuPathVertex>,
    paints: Vec<GpuPathPaint>,
    pending: Vec<PendingPath>,
    batches: Vec<PathBatch>,
    layer_batches: Vec<Range<usize>>,
}

impl PathRenderer {
    pub(crate) fn new(device: &Device, format: TextureFormat) -> Self {
        let uses_storage = uses_read_only_storage_buffers(device);
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quickgui path view uniform"),
            contents: bytemuck::bytes_of(&ViewUniform::zeroed()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let view_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("quickgui path view bind group layout"),
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
        let view_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("quickgui path view bind group"),
            layout: &view_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });
        let paint_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("quickgui path paint bind group layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: read_only_table_binding(uses_storage),
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("quickgui path pipeline layout"),
            bind_group_layouts: &[
                Some(&view_bind_group_layout),
                Some(&paint_bind_group_layout),
            ],
            immediate_size: 0,
        });
        let shader = if uses_storage {
            shader_module_from_wgsl(device, "quickgui path pipeline", Cow::Borrowed(PATH_WGSL))
        } else {
            shader_module_from_wgsl(
                device,
                "quickgui path pipeline",
                Cow::Owned(rewrite_storage_array_as_uniform(
                    PATH_WGSL,
                    PATH_STORAGE_BINDING,
                    "paints",
                    "paint_table",
                    "PathPaint",
                )),
            )
        };
        let attributes = [
            VertexAttribute {
                format: VertexFormat::Float32x2,
                offset: 0,
                shader_location: 0,
            },
            VertexAttribute {
                format: VertexFormat::Float32x3,
                offset: 8,
                shader_location: 1,
            },
            VertexAttribute {
                format: VertexFormat::Float32x3,
                offset: 20,
                shader_location: 2,
            },
            VertexAttribute {
                format: VertexFormat::Uint32,
                offset: 32,
                shader_location: 3,
            },
        ];
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("quickgui path pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[Some(VertexBufferLayout {
                    array_stride: mem::size_of::<GpuPathVertex>() as BufferAddress,
                    step_mode: VertexStepMode::Vertex,
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

        let vertex_buffers = (0..BUFFERED_FRAMES)
            .map(|_| create_vertex_buffer(device, INITIAL_VERTEX_CAPACITY))
            .collect();
        let paint_capacity = if uses_storage {
            INITIAL_PAINT_CAPACITY
        } else {
            UNIFORM_TABLE_LEN
        };
        let paint_buffers: Vec<_> = (0..BUFFERED_FRAMES)
            .map(|_| create_paint_buffer(device, paint_capacity, uses_storage))
            .collect();
        let paint_bind_groups = paint_buffers
            .iter()
            .map(|buffer| create_paint_bind_group(device, &paint_bind_group_layout, buffer))
            .collect();
        Self {
            pipeline,
            uniform_buffer,
            view_bind_group,
            paint_bind_group_layout,
            vertex_buffers,
            vertex_capacities: vec![INITIAL_VERTEX_CAPACITY; BUFFERED_FRAMES],
            paint_buffers,
            paint_capacities: vec![paint_capacity; BUFFERED_FRAMES],
            paint_bind_groups,
            uses_storage,
            active_buffer: 0,
            uploads: crate::renderer::upload::BufferUploads::default(),
            vertices: Vec::with_capacity(INITIAL_VERTEX_CAPACITY),
            paints: Vec::with_capacity(INITIAL_PAINT_CAPACITY),
            pending: Vec::with_capacity(INITIAL_PAINT_CAPACITY),
            batches: Vec::with_capacity(16),
            layer_batches: Vec::with_capacity(4),
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
    ) -> PathPrepareStats {
        self.uploads.begin_frame();
        self.vertices.clear();
        self.paints.clear();
        self.pending.clear();
        self.batches.clear();
        self.layer_batches.clear();
        let mut skipped_paths = 0;
        let mut admitted_paths = 0_usize;
        let mut admitted_vertices = 0_usize;
        let max_paths = self.max_paths();

        for layer in scene.paint_layers() {
            let batch_start = self.batches.len();
            self.pending.clear();
            for item in layer.paint() {
                let PrimitiveRef::Path(index) = item.primitive else {
                    continue;
                };
                let primitive = &layer.paths()[index];
                if !visible_path(primitive, viewport) {
                    continue;
                }
                let vertex_count = primitive.path.vertex_count();
                if !admit_path(
                    &mut admitted_paths,
                    &mut admitted_vertices,
                    vertex_count,
                    max_paths,
                ) {
                    skipped_paths += 1;
                    continue;
                }
                self.pending.push(PendingPath {
                    order: item.order,
                    primitive: index,
                });
            }
            self.pending.sort_unstable_by_key(|path| path.order);
            for pending in &self.pending {
                let primitive = &layer.paths()[pending.primitive];
                let clip = primitive
                    .clip
                    .unwrap_or(viewport)
                    .intersection(viewport)
                    .expect("visible paths have a viewport intersection");
                let paint_index = self.paints.len() as u32;
                self.paints.push(path_paint(primitive, clip));
                let vertex_start = self.vertices.len() as u32;
                append_path_vertices(&mut self.vertices, primitive, paint_index);
                let vertex_end = self.vertices.len() as u32;
                if self.batches.len() > batch_start
                    && self
                        .batches
                        .last()
                        .is_some_and(|batch| batch.order == pending.order)
                {
                    self.batches
                        .last_mut()
                        .expect("the previous path batch exists")
                        .vertices
                        .end = vertex_end;
                } else {
                    self.batches.push(PathBatch {
                        order: pending.order,
                        vertices: vertex_start..vertex_end,
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
        self.ensure_active_capacity(device);
        if !self.vertices.is_empty() {
            self.uploads.write(
                1 + self.active_buffer,
                queue,
                &self.vertex_buffers[self.active_buffer],
                bytemuck::cast_slice(&self.vertices),
            );
            self.uploads.write(
                1 + BUFFERED_FRAMES + self.active_buffer,
                queue,
                &self.paint_buffers[self.active_buffer],
                bytemuck::cast_slice(&self.paints),
            );
        }
        debug_assert!(self.vertices.len() <= MAX_GPU_PATH_VERTICES);
        debug_assert!(self.paints.len() <= max_paths);
        PathPrepareStats {
            paths: self.paints.len(),
            vertices: self.vertices.len(),
            draw_calls: self.batches.len(),
            skipped_paths,
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
        let Some(batch) = self.batches[layer_batches.clone()]
            .iter()
            .find(|batch| batch.order == order)
        else {
            return;
        };
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.view_bind_group, &[]);
        pass.set_bind_group(1, &self.paint_bind_groups[self.active_buffer], &[]);
        pass.set_vertex_buffer(0, self.vertex_buffers[self.active_buffer].slice(..));
        pass.draw(batch.vertices.clone(), 0..1);
    }

    fn max_paths(&self) -> usize {
        if self.uses_storage {
            MAX_GPU_PATHS_PER_FRAME
        } else {
            UNIFORM_TABLE_LEN
        }
    }

    fn ensure_active_capacity(&mut self, device: &Device) {
        let vertex_required = self.vertices.len().max(1);
        if vertex_required > self.vertex_capacities[self.active_buffer] {
            let capacity = vertex_required
                .next_power_of_two()
                .min(MAX_GPU_PATH_VERTICES);
            self.vertex_buffers[self.active_buffer] = create_vertex_buffer(device, capacity);
            self.vertex_capacities[self.active_buffer] = capacity;
            self.uploads.reset(1 + self.active_buffer);
        }

        let paint_required = self.paints.len().max(1);
        if self.uses_storage && paint_required > self.paint_capacities[self.active_buffer] {
            let capacity = paint_required
                .next_power_of_two()
                .min(MAX_GPU_PATHS_PER_FRAME);
            self.paint_buffers[self.active_buffer] = create_paint_buffer(device, capacity, true);
            self.paint_bind_groups[self.active_buffer] = create_paint_bind_group(
                device,
                &self.paint_bind_group_layout,
                &self.paint_buffers[self.active_buffer],
            );
            self.paint_capacities[self.active_buffer] = capacity;
            self.uploads.reset(1 + BUFFERED_FRAMES + self.active_buffer);
        }
    }
}

fn admit_path(
    paths: &mut usize,
    vertices: &mut usize,
    vertex_count: usize,
    max_paths: usize,
) -> bool {
    if *paths == max_paths || vertices.saturating_add(vertex_count) > MAX_GPU_PATH_VERTICES {
        return false;
    }
    *paths += 1;
    *vertices += vertex_count;
    true
}

fn visible_path(primitive: &PathPrimitive, viewport: Rect) -> bool {
    primitive
        .clip
        .unwrap_or(viewport)
        .intersection(viewport)
        .is_some_and(|clip| primitive.render_bounds().intersects(clip))
}

fn append_path_vertices(
    vertices: &mut Vec<GpuPathVertex>,
    primitive: &PathPrimitive,
    paint_index: u32,
) {
    const BARYCENTRICS: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    for (triangle, boundary_mask) in primitive
        .path
        .positions()
        .as_chunks::<3>()
        .0
        .iter()
        .zip(primitive.path.boundary_masks())
    {
        let edge_mask = [
            f32::from(boundary_mask & 0b001 != 0),
            f32::from(boundary_mask & 0b010 != 0),
            f32::from(boundary_mask & 0b100 != 0),
        ];
        for vertex in 0..3 {
            vertices.push(GpuPathVertex {
                position: triangle[vertex],
                barycentric: BARYCENTRICS[vertex],
                edge_mask,
                paint_index,
            });
        }
    }
}

fn path_paint(primitive: &PathPrimitive, clip: Rect) -> GpuPathPaint {
    let scale = primitive.scale_factors();
    let translation = primitive.translation();
    let mut paint = GpuPathPaint {
        transform: [scale[0], scale[1], translation.x, translation.y],
        clip: [clip.x, clip.y, clip.right(), clip.bottom()],
        color: [0.0; 4],
        header: [0.0; 4],
        geometry: [0.0; 4],
        positions: [[0.0; 4]; 2],
        colors: [[0.0; 4]; MAX_GRADIENT_STOPS],
    };
    match primitive.background.as_gradient() {
        None => {
            let crate::Background::Solid(color) = primitive.background else {
                unreachable!("a non-gradient background is always solid");
            };
            paint.color = color.as_array();
        }
        Some(gradient) => {
            let data = GradientData::new(&gradient, primitive.render_bounds());
            paint.header = [data.header[0], data.header[1], data.header[2], 1.0];
            paint.geometry = data.geometry;
            paint.positions = data.positions;
            paint.colors = data.colors;
        }
    }
    paint
}

fn create_vertex_buffer(device: &Device, capacity: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("quickgui path vertex buffer"),
        size: (capacity * mem::size_of::<GpuPathVertex>()) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn create_paint_buffer(device: &Device, capacity: usize, storage: bool) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("quickgui path paint buffer"),
        size: (capacity * mem::size_of::<GpuPathPaint>()) as u64,
        usage: table_buffer_usages(storage),
        mapped_at_creation: false,
    })
}

fn create_paint_bind_group(
    device: &Device,
    layout: &wgpu::BindGroupLayout,
    buffer: &wgpu::Buffer,
) -> BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("quickgui path paint bind group"),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, PathBuilder, Point, linear_color_stop, linear_gradient};

    fn triangle() -> crate::Path {
        let mut builder = PathBuilder::fill();
        builder.move_to(Point::new(0.0, 0.0));
        builder.line_to(Point::new(20.0, 0.0));
        builder.line_to(Point::new(10.0, 10.0));
        builder.close();
        builder.build().unwrap()
    }

    #[test]
    fn expands_boundary_masks_and_paint_indices() {
        let path = triangle();
        let primitive = PathPrimitive::new(path.clone(), Color::WHITE);
        let mut vertices = Vec::new();
        append_path_vertices(&mut vertices, &primitive, 7);
        assert_eq!(vertices.len(), path.vertex_count());
        assert!(vertices.iter().all(|vertex| vertex.paint_index == 7));
        assert_eq!(vertices[0].barycentric, [1.0, 0.0, 0.0]);
        assert_eq!(vertices[1].barycentric, [0.0, 1.0, 0.0]);
        assert_eq!(vertices[2].barycentric, [0.0, 0.0, 1.0]);
        assert_eq!(vertices[0].edge_mask, [1.0, 1.0, 1.0]);
    }

    #[test]
    fn paint_records_preserve_gradient_stops_and_transform() {
        let background = linear_gradient(
            180.0,
            linear_color_stop(Color::BLACK, 0.2),
            linear_color_stop(Color::WHITE, 0.8),
        );
        let primitive = PathPrimitive::new(triangle(), background)
            .scale_xy(2.0, -3.0)
            .translate(4.0, 5.0);
        let paint = path_paint(&primitive, Rect::new(1.0, 2.0, 3.0, 4.0));
        assert_eq!(paint.transform, [2.0, -3.0, 4.0, 5.0]);
        assert_eq!(paint.clip, [1.0, 2.0, 4.0, 6.0]);
        assert_eq!(paint.header, [0.0, 0.0, 2.0, 1.0]);
        assert_eq!(paint.positions[0], [0.2, 0.8, 0.0, 0.0]);
        assert_eq!(paint.colors[0], Color::BLACK.as_array());
        assert_eq!(paint.colors[1], Color::WHITE.as_array());
    }

    #[test]
    fn multi_stop_path_gradients_upload_every_stop() {
        let background = crate::Gradient::radial([Color::BLACK, Color::WHITE, Color::TRANSPARENT])
            .shape(crate::RadialGradientShape::Circle)
            .color_space(crate::GradientColorSpace::Oklab);
        let primitive = PathPrimitive::new(triangle(), background);
        let paint = path_paint(&primitive, Rect::new(0.0, 0.0, 20.0, 10.0));
        assert_eq!(paint.header, [1.0, 2.0, 3.0, 1.0]);
        assert_eq!(paint.positions[0], [0.0, 0.5, 1.0, 0.0]);
        // A circle uses one radius on both axes.
        assert_eq!(paint.geometry[2], paint.geometry[3]);
    }

    #[test]
    fn frame_admission_accumulates_every_path_and_never_exceeds_the_caps() {
        let mut paths = 0;
        let mut vertices = 0;
        assert!(admit_path(
            &mut paths,
            &mut vertices,
            100_000,
            MAX_GPU_PATHS_PER_FRAME
        ));
        assert!(admit_path(
            &mut paths,
            &mut vertices,
            100_000,
            MAX_GPU_PATHS_PER_FRAME
        ));
        assert!(!admit_path(
            &mut paths,
            &mut vertices,
            100_000,
            MAX_GPU_PATHS_PER_FRAME
        ));
        assert_eq!(paths, 2);
        assert_eq!(vertices, 200_000);

        paths = MAX_GPU_PATHS_PER_FRAME;
        vertices = 0;
        assert!(!admit_path(
            &mut paths,
            &mut vertices,
            3,
            MAX_GPU_PATHS_PER_FRAME
        ));
        assert_eq!(vertices, 0);

        paths = UNIFORM_TABLE_LEN;
        assert!(!admit_path(&mut paths, &mut vertices, 3, UNIFORM_TABLE_LEN));
    }

    #[test]
    fn path_shader_parses_and_validates() {
        let module = wgpu::naga::front::wgsl::parse_str(PATH_WGSL)
            .expect("the retained path shader must parse");
        wgpu::naga::valid::Validator::new(
            wgpu::naga::valid::ValidationFlags::all(),
            wgpu::naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .expect("the retained path shader must validate");
    }

    #[test]
    fn rewritten_uniform_path_shader_parses_and_validates() {
        let source = rewrite_storage_array_as_uniform(
            PATH_WGSL,
            PATH_STORAGE_BINDING,
            "paints",
            "paint_table",
            "PathPaint",
        );
        assert!(source.contains("paint_table.records["));
        assert!(!source.contains("paints["));
        let module =
            wgpu::naga::front::wgsl::parse_str(&source).expect("the WebGL path shader must parse");
        wgpu::naga::valid::Validator::new(
            wgpu::naga::valid::ValidationFlags::all(),
            wgpu::naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .expect("the WebGL path shader must validate");
    }
}

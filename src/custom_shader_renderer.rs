use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    mem,
    ops::Range,
};

use bytemuck::{Pod, Zeroable};
use wgpu::{
    BindGroup, BufferAddress, ColorTargetState, Device, ErrorFilter, FragmentState,
    MultisampleState, PipelineCompilationOptions, PipelineLayout, PrimitiveState,
    PrimitiveTopology, Queue, RenderPipeline, RenderPipelineDescriptor, ShaderModule, ShaderStages,
    TextureFormat, VertexAttribute, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
    util::DeviceExt,
};

use crate::{
    CustomShader, Rect, Scene,
    scene::{CustomShaderPrimitive, PrimitiveRef},
};

/// Maximum distinct application shader pipelines retained by one window.
pub const MAX_CUSTOM_SHADER_PIPELINES_PER_WINDOW: usize = 32;
/// Maximum visible custom shader rectangles uploaded in one frame.
pub const MAX_CUSTOM_SHADER_INSTANCES_PER_FRAME: usize = 4_096;

const INITIAL_INSTANCE_CAPACITY: usize = 64;
const BUFFERED_FRAMES: usize = 3;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct CustomShaderPrepareStats {
    pub instances: usize,
    pub draw_calls: usize,
    pub compiled_pipelines: usize,
    pub skipped_instances: usize,
    pub cached_pipelines: usize,
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
struct CustomShaderInstance {
    rect: [f32; 4],
    clip: [f32; 4],
    params: [[f32; 4]; 4],
    opacity_and_padding: [f32; 4],
}

#[derive(Clone)]
struct PendingShader {
    order: u32,
    primitive: usize,
    shader: CustomShader,
}

struct ShaderBatch {
    order: u32,
    shader: CustomShader,
    instances: Range<u32>,
}

struct CachedPipeline {
    pipeline: RenderPipeline,
    last_used_frame: u64,
}

pub(crate) struct CustomShaderRenderer {
    vertex_shader: ShaderModule,
    pipeline_layout: PipelineLayout,
    uniform_buffer: wgpu::Buffer,
    view_bind_group: BindGroup,
    instance_buffers: Vec<wgpu::Buffer>,
    instance_capacities: Vec<usize>,
    active_buffer: usize,
    pub(crate) uploads: crate::renderer::upload::BufferUploads,
    instances: Vec<CustomShaderInstance>,
    pending: Vec<PendingShader>,
    batches: Vec<ShaderBatch>,
    layer_batches: Vec<Range<usize>>,
    pipelines: HashMap<CustomShader, CachedPipeline>,
    admitted_shaders: Vec<CustomShader>,
    admitted_shader_set: HashSet<CustomShader>,
    frame: u64,
    format: TextureFormat,
}

impl CustomShaderRenderer {
    pub(crate) fn new(device: &Device, format: TextureFormat) -> Self {
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quickgui custom shader view uniform"),
            contents: bytemuck::bytes_of(&ViewUniform::zeroed()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let view_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("quickgui custom shader view bind group layout"),
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
            label: Some("quickgui custom shader view bind group"),
            layout: &view_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("quickgui custom shader pipeline layout"),
            bind_group_layouts: &[Some(&view_bind_group_layout)],
            immediate_size: 0,
        });
        let vertex_shader = device.create_shader_module(wgpu::include_wgsl!("custom_shader.wgsl"));

        Self {
            vertex_shader,
            pipeline_layout,
            uniform_buffer,
            view_bind_group,
            instance_buffers: (0..BUFFERED_FRAMES)
                .map(|_| create_instance_buffer(device, INITIAL_INSTANCE_CAPACITY))
                .collect(),
            instance_capacities: vec![INITIAL_INSTANCE_CAPACITY; BUFFERED_FRAMES],
            active_buffer: 0,
            uploads: crate::renderer::upload::BufferUploads::default(),
            instances: Vec::with_capacity(INITIAL_INSTANCE_CAPACITY),
            pending: Vec::with_capacity(INITIAL_INSTANCE_CAPACITY),
            batches: Vec::with_capacity(16),
            layer_batches: Vec::with_capacity(4),
            pipelines: HashMap::with_capacity(MAX_CUSTOM_SHADER_PIPELINES_PER_WINDOW),
            admitted_shaders: Vec::with_capacity(MAX_CUSTOM_SHADER_PIPELINES_PER_WINDOW),
            admitted_shader_set: HashSet::with_capacity(MAX_CUSTOM_SHADER_PIPELINES_PER_WINDOW),
            frame: 0,
            format,
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
    ) -> Result<CustomShaderPrepareStats, String> {
        self.uploads.begin_frame();
        self.frame = self.frame.wrapping_add(1);
        self.instances.clear();
        self.pending.clear();
        self.batches.clear();
        self.layer_batches.clear();
        self.admitted_shaders.clear();
        self.admitted_shader_set.clear();

        let mut admitted_instances = 0_usize;
        let mut skipped_instances = 0_usize;
        for layer in scene.paint_layers() {
            for primitive in layer.custom_shaders() {
                if !visible_shader(primitive, viewport) {
                    continue;
                }
                if admitted_instances == MAX_CUSTOM_SHADER_INSTANCES_PER_FRAME {
                    skipped_instances += 1;
                    continue;
                }
                if !self.admitted_shader_set.contains(&primitive.shader) {
                    if self.admitted_shaders.len() == MAX_CUSTOM_SHADER_PIPELINES_PER_WINDOW {
                        skipped_instances += 1;
                        continue;
                    }
                    self.admitted_shader_set.insert(primitive.shader.clone());
                    self.admitted_shaders.push(primitive.shader.clone());
                }
                admitted_instances += 1;
            }
        }

        let compiled_pipelines = self.ensure_pipelines(device)?;
        let mut retained_instances = 0_usize;
        for layer in scene.paint_layers() {
            let batch_start = self.batches.len();
            self.pending.clear();
            for item in layer.paint() {
                let PrimitiveRef::CustomShader(index) = item.primitive else {
                    continue;
                };
                let primitive = &layer.custom_shaders()[index];
                if retained_instances == MAX_CUSTOM_SHADER_INSTANCES_PER_FRAME
                    || !self.admitted_shader_set.contains(&primitive.shader)
                    || !visible_shader(primitive, viewport)
                {
                    continue;
                }
                retained_instances += 1;
                self.pending.push(PendingShader {
                    order: item.order,
                    primitive: index,
                    shader: primitive.shader.clone(),
                });
            }
            self.pending
                .sort_unstable_by_key(|pending| (pending.order, pending.shader.id()));
            for pending in &self.pending {
                let primitive = &layer.custom_shaders()[pending.primitive];
                let clip = primitive
                    .clip
                    .unwrap_or(viewport)
                    .intersection(viewport)
                    .expect("visible custom shaders have a viewport intersection");
                let instance_start = self.instances.len() as u32;
                self.instances.push(shader_instance(primitive, clip));
                let instance_end = self.instances.len() as u32;
                if self.batches.len() > batch_start
                    && self.batches.last().is_some_and(|batch| {
                        batch.order == pending.order && batch.shader == pending.shader
                    })
                {
                    self.batches
                        .last_mut()
                        .expect("the previous custom shader batch exists")
                        .instances
                        .end = instance_end;
                } else {
                    self.batches.push(ShaderBatch {
                        order: pending.order,
                        shader: pending.shader.clone(),
                        instances: instance_start..instance_end,
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
        if !self.instances.is_empty() {
            self.uploads.write(
                1 + self.active_buffer,
                queue,
                &self.instance_buffers[self.active_buffer],
                bytemuck::cast_slice(&self.instances),
            );
        }

        Ok(CustomShaderPrepareStats {
            instances: self.instances.len(),
            draw_calls: self.batches.len(),
            compiled_pipelines,
            skipped_instances,
            cached_pipelines: self.pipelines.len(),
        })
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
        for batch in self.batches[layer_batches.clone()]
            .iter()
            .filter(|batch| batch.order == order)
        {
            let pipeline = &self
                .pipelines
                .get(&batch.shader)
                .expect("admitted custom shader owns a cached pipeline")
                .pipeline;
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &self.view_bind_group, &[]);
            pass.set_vertex_buffer(0, self.instance_buffers[self.active_buffer].slice(..));
            pass.draw(0..6, batch.instances.clone());
        }
    }

    fn ensure_pipelines(&mut self, device: &Device) -> Result<usize, String> {
        for shader in &self.admitted_shaders {
            if let Some(pipeline) = self.pipelines.get_mut(shader) {
                pipeline.last_used_frame = self.frame;
            }
        }
        let missing = self
            .admitted_shaders
            .iter()
            .filter(|shader| !self.pipelines.contains_key(*shader))
            .count();
        while self.pipelines.len() + missing > MAX_CUSTOM_SHADER_PIPELINES_PER_WINDOW {
            let Some(evict) = self
                .pipelines
                .iter()
                .filter(|(shader, _)| !self.admitted_shader_set.contains(*shader))
                .min_by_key(|(_, pipeline)| pipeline.last_used_frame)
                .map(|(shader, _)| shader.clone())
            else {
                break;
            };
            self.pipelines.remove(&evict);
        }

        let mut compiled = 0;
        for shader in &self.admitted_shaders {
            if self.pipelines.contains_key(shader) {
                continue;
            }
            let pipeline = create_pipeline(
                device,
                self.format,
                &self.pipeline_layout,
                &self.vertex_shader,
                shader,
            )?;
            self.pipelines.insert(
                shader.clone(),
                CachedPipeline {
                    pipeline,
                    last_used_frame: self.frame,
                },
            );
            compiled += 1;
        }
        Ok(compiled)
    }

    fn ensure_active_capacity(&mut self, device: &Device) {
        let required = self.instances.len().max(1);
        if required <= self.instance_capacities[self.active_buffer] {
            return;
        }
        let capacity = required
            .next_power_of_two()
            .min(MAX_CUSTOM_SHADER_INSTANCES_PER_FRAME);
        self.instance_buffers[self.active_buffer] = create_instance_buffer(device, capacity);
        self.instance_capacities[self.active_buffer] = capacity;
        self.uploads.reset(1 + self.active_buffer);
    }
}

fn create_pipeline(
    device: &Device,
    format: TextureFormat,
    layout: &PipelineLayout,
    vertex_shader: &ShaderModule,
    shader: &CustomShader,
) -> Result<RenderPipeline, String> {
    let scope = device.push_error_scope(ErrorFilter::Validation);
    let fragment_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("quickgui application fragment shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Owned(encoded_shader_source(shader.source()))),
    });
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
    ];
    let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("quickgui application shader pipeline"),
        layout: Some(layout),
        vertex: VertexState {
            module: vertex_shader,
            entry_point: Some("vs_main"),
            compilation_options: PipelineCompilationOptions::default(),
            buffers: &[Some(VertexBufferLayout {
                array_stride: mem::size_of::<CustomShaderInstance>() as BufferAddress,
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
            module: &fragment_shader,
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
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(error) = pollster::block_on(scope.pop()) {
        return Err(error.to_string());
    }
    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_futures::spawn_local(async move {
        if let Some(error) = scope.pop().await {
            web_sys::console::error_1(&error.to_string().into());
            if let Some(window) = web_sys::window() {
                if let Ok(event) = web_sys::CustomEvent::new("quickgui:error") {
                    let _ = window.dispatch_event(&event);
                }
            }
        }
    });
    Ok(pipeline)
}

fn visible_shader(primitive: &CustomShaderPrimitive, viewport: Rect) -> bool {
    primitive
        .clip
        .unwrap_or(viewport)
        .intersection(viewport)
        .is_some_and(|clip| primitive.rect.intersects(clip))
}

fn shader_instance(primitive: &CustomShaderPrimitive, clip: Rect) -> CustomShaderInstance {
    CustomShaderInstance {
        rect: [
            primitive.rect.x,
            primitive.rect.y,
            primitive.rect.width,
            primitive.rect.height,
        ],
        clip: [clip.x, clip.y, clip.right(), clip.bottom()],
        params: primitive.parameters.vectors(),
        opacity_and_padding: [primitive.opacity, 0.0, 0.0, 0.0],
    }
}

fn create_instance_buffer(device: &Device, capacity: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("quickgui custom shader instance buffer"),
        size: (capacity * mem::size_of::<CustomShaderInstance>()) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn encoded_shader_source(source: &str) -> String {
    let mut source = source.replace(
        "@fragment\nfn fs_main(input: QuickGuiFragmentInput) -> @location(0) vec4<f32>",
        "fn quickgui_shade_linear(input: QuickGuiFragmentInput) -> vec4<f32>",
    );
    source.push_str(r#"
// Public paint colors stay linear; UI targets blend encoded sRGB, as native UI toolkits do.
fn quickgui_encode_component(v: f32) -> f32 {
    if v <= 0.0031308 { return 12.92 * v; }
    return 1.055 * pow(max(v, 0.0), 1.0 / 2.4) - 0.055;
}
fn quickgui_encode_output(color: vec4<f32>) -> vec4<f32> {
    if color.a <= 0.0 { return vec4<f32>(0.0); }
    let straight = color.rgb / color.a;
    return vec4<f32>(vec3<f32>(quickgui_encode_component(straight.r), quickgui_encode_component(straight.g), quickgui_encode_component(straight.b)) * color.a, color.a);
}
@fragment
fn fs_main(input: QuickGuiFragmentInput) -> @location(0) vec4<f32> {
    return quickgui_encode_output(quickgui_shade_linear(input));
}
"#);
    source
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CustomShaderPrimitive, ShaderParameters};

    fn test_shader() -> CustomShader {
        CustomShader::new(
            r#"
fn quickgui_fragment(input: QuickGuiShaderInput) -> vec4<f32> {
    return vec4<f32>(input.uv, input.params[0].x, 1.0);
}
"#,
        )
        .unwrap()
    }

    #[test]
    fn instance_layout_matches_the_fixed_vertex_contract() {
        let primitive =
            CustomShaderPrimitive::new(test_shader(), Rect::new(10.0, 20.0, 30.0, 40.0))
                .parameters(ShaderParameters::new().float(0, 0.5))
                .opacity(0.25);
        let instance = shader_instance(&primitive, Rect::new(12.0, 22.0, 20.0, 30.0));

        assert_eq!(mem::size_of::<CustomShaderInstance>(), 112);
        assert_eq!(instance.rect, [10.0, 20.0, 30.0, 40.0]);
        assert_eq!(instance.clip, [12.0, 22.0, 32.0, 52.0]);
        assert_eq!(instance.params[0][0], 0.5);
        assert_eq!(instance.opacity_and_padding, [0.25, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn visibility_requires_both_geometry_and_clip() {
        let shader = test_shader();
        let viewport = Rect::new(0.0, 0.0, 100.0, 100.0);

        assert!(visible_shader(
            &CustomShaderPrimitive::new(shader.clone(), Rect::new(10.0, 10.0, 20.0, 20.0)),
            viewport
        ));
        assert!(!visible_shader(
            &CustomShaderPrimitive::new(shader, Rect::new(110.0, 10.0, 20.0, 20.0)),
            viewport
        ));
    }
}

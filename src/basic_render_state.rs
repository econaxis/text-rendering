use wgpu::{BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, BlendState, Buffer, BufferAddress, BufferBindingType, BufferDescriptor, BufferUsages, ColorTargetState, ColorWrites, Extent3d, FragmentState, FrontFace, PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology, RenderPipeline, RenderPipelineDescriptor, Sampler, SamplerBindingType, ShaderStages, Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureViewDescriptor, VertexBufferLayout, VertexState};

use crate::gpu_device::{device, shader};

pub struct BasicRenderState {
    pub(crate) render_pipeline: RenderPipeline,
    pub(crate) bind_group: BindGroup,
    pub(crate) texture: Texture,

    #[allow(unused)]
    sampler: Sampler,
    pub(crate) uniform_buffer: Buffer,
}

impl BasicRenderState {
    pub(crate) fn new(shader_prefix: &'static str, uniform_size: usize, texture_size: Extent3d, vert_layout: VertexBufferLayout, blend: BlendState) -> Self {
        let device = device();
        let shader = shader();
        let bindgrouplayout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: Default::default(),
                    view_dimension: Default::default(),
                    multisampled: false,
                },
                count: None,
            }, BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                ty: BindingType::Sampler {
                    0: SamplerBindingType::Filtering
                },
                count: None,
            }, BindGroupLayoutEntry {
                binding: 2,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let texture = device.create_texture(&TextureDescriptor {
            label: None,
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8Unorm,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        });

        let uniform_buffer = device.create_buffer(
            &BufferDescriptor {
                label: None,
                size: uniform_size as BufferAddress,
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }
        );

        let view = texture.create_view(&TextureViewDescriptor::default());
        let sampler = crate::create_sampler(device);
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &bindgrouplayout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&view),
            }, BindGroupEntry {
                binding: 1,
                resource: BindingResource::Sampler(&sampler),
            }, BindGroupEntry {
                binding: 2,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });
        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[&bindgrouplayout],
            push_constant_ranges: &[],
        });
        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: Some(&layout),
            vertex: VertexState {
                module: shader,
                entry_point: &*format!("{}_{}", shader_prefix, "vs_main"),
                buffers: &[vert_layout]
            },
            fragment: Some(FragmentState {
                module: shader,
                entry_point: &*format!("{}_{}", shader_prefix, "fs_main"),
                targets: &[ColorTargetState {
                    format: TextureFormat::Bgra8Unorm,
                    blend: Some(blend),
                    write_mask: ColorWrites::ALL,
                }],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: Default::default(),
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: Default::default(),
                conservative: false,
            },
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
        });
        Self {
            render_pipeline,
            bind_group,
            texture,
            sampler,
            uniform_buffer,
        }
    }
}

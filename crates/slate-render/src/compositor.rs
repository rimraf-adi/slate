use crate::context::GpuContext;
use bytemuck::{Pod, Zeroable};
use slate_core::traits::compositor::{Compositor, FrameRequest, RenderError, RenderedFrame};
use slate_core::traits::decoder::VideoFrame;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct Vertex {
    position: [f32; 2],
    uv: [f32; 2],
}

const QUAD_VERTICES: &[Vertex] = &[
    Vertex { position: [-1.0, -1.0], uv: [0.0, 1.0] },
    Vertex { position: [1.0, -1.0], uv: [1.0, 1.0] },
    Vertex { position: [1.0, 1.0], uv: [1.0, 0.0] },
    Vertex { position: [-1.0, -1.0], uv: [0.0, 1.0] },
    Vertex { position: [1.0, 1.0], uv: [1.0, 0.0] },
    Vertex { position: [-1.0, 1.0], uv: [0.0, 0.0] },
];

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct Uniforms {
    matrix: [f32; 16],
    opacity: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct TransitionUniforms {
    progress: f32,
    mode: u32,
    _pad0: f32,
    _pad1: f32,
}

pub struct GpuCompositor {
    context: GpuContext,
    vertex_buffer: wgpu::Buffer,
    sampler: wgpu::Sampler,
    composite_pipeline: wgpu::RenderPipeline,
    composite_bind_group_layout: wgpu::BindGroupLayout,
    transition_pipeline: wgpu::RenderPipeline,
    transition_bind_group_layout: wgpu::BindGroupLayout,
}

impl GpuCompositor {
    pub fn new(context: GpuContext) -> Result<Self, RenderError> {
        let device = &context.device;

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Quad Vertex Buffer"),
            contents: bytemuck::cast_slice(QUAD_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Texture Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // 1. Composite Pipeline
        let composite_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Composite Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/composite.wgsl").into()),
        });

        let composite_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Composite Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let composite_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Composite Pipeline Layout"),
                bind_group_layouts: &[&composite_bind_group_layout],
                push_constant_ranges: &[],
            });

        let composite_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Composite Pipeline"),
            layout: Some(&composite_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &composite_shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        wgpu::VertexAttribute {
                            offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &composite_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // 2. Transition Pipeline
        let transition_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Transition Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/transitions.wgsl").into()),
        });

        let transition_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Transition Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let transition_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Transition Pipeline Layout"),
                bind_group_layouts: &[&transition_bind_group_layout],
                push_constant_ranges: &[],
            });

        let transition_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Transition Pipeline"),
            layout: Some(&transition_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &transition_shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        wgpu::VertexAttribute {
                            offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &transition_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Ok(Self {
            context,
            vertex_buffer,
            sampler,
            composite_pipeline,
            composite_bind_group_layout,
            transition_pipeline,
            transition_bind_group_layout,
        })
    }

    fn upload_texture(&self, frame: &VideoFrame) -> wgpu::Texture {
        let device = &self.context.device;
        let queue = &self.context.queue;

        let texture_desc = wgpu::TextureDescriptor {
            label: Some("Frame Video Texture"),
            size: wgpu::Extent3d {
                width: frame.width,
                height: frame.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        };

        let texture = device.create_texture(&texture_desc);

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &frame.data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(frame.width * 4),
                rows_per_image: Some(frame.height),
            },
            texture_desc.size,
        );

        texture
    }
}

impl Compositor for GpuCompositor {
    fn composite_frame(&self, req: &FrameRequest) -> Result<RenderedFrame, RenderError> {
        let device = &self.context.device;
        let queue = &self.context.queue;

        let width = req.canvas_width;
        let height = req.canvas_height;
        let timeline_time = req.timeline_time;

        // Create canvas render target
        let target_desc = wgpu::TextureDescriptor {
            label: Some("Canvas Render Target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        };
        let target_texture = device.create_texture(&target_desc);
        let target_view = target_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Frame Composite Encoder"),
        });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Composite Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

            // Render visible clips
            for (clip, frame) in &req.visible_clips {
                let frame_texture = self.upload_texture(frame);
                let frame_view = frame_texture.create_view(&wgpu::TextureViewDescriptor::default());

                // Affine transformation matrix
                let transform = &clip.transform;
                let sx = transform.scale_x.evaluate_at(timeline_time, 1.0) as f32;
                let sy = transform.scale_y.evaluate_at(timeline_time, 1.0) as f32;
                let rad = transform.rotation_degrees.evaluate_at(timeline_time, 0.0).to_radians() as f32;
                let cos_r = rad.cos();
                let sin_r = rad.sin();

                // NDC translation
                let pos_x = transform.position_x.evaluate_at(timeline_time, 0.0) as f32;
                let pos_y = transform.position_y.evaluate_at(timeline_time, 0.0) as f32;
                let tx = (pos_x / width as f32) * 2.0;
                let ty = -(pos_y / height as f32) * 2.0;

                let matrix: [f32; 16] = [
                    sx * cos_r, -sx * sin_r, 0.0, 0.0,
                    sy * sin_r,  sy * cos_r, 0.0, 0.0,
                    0.0,        0.0,         1.0, 0.0,
                    tx,         ty,          0.0, 1.0,
                ];

                let uniforms = Uniforms {
                    matrix,
                    opacity: transform.opacity.evaluate_at(timeline_time, 1.0) as f32,
                    _pad0: 0.0,
                    _pad1: 0.0,
                    _pad2: 0.0,
                };

                let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Clip Uniform Buffer"),
                    contents: bytemuck::cast_slice(&[uniforms]),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

                let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Clip Bind Group"),
                    layout: &self.composite_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: uniform_buf.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&frame_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::Sampler(&self.sampler),
                        },
                    ],
                });

                rpass.set_pipeline(&self.composite_pipeline);
                rpass.set_bind_group(0, &bind_group, &[]);
                rpass.draw(0..6, 0..1);
            }

            // Render active transitions
            for (transition, from_frame, to_frame, progress) in &req.active_transitions {
                let from_tex = self.upload_texture(from_frame);
                let from_view = from_tex.create_view(&wgpu::TextureViewDescriptor::default());

                let to_tex = self.upload_texture(to_frame);
                let to_view = to_tex.create_view(&wgpu::TextureViewDescriptor::default());

                let mode = match transition.transition_type.to_lowercase().as_str() {
                    "cross_dissolve" | "crossdissolve" => 0u32,
                    "dip_to_color" | "dip_to_black" | "dip" => 1u32,
                    _ => 0u32,
                };

                let tuniforms = TransitionUniforms {
                    progress: *progress,
                    mode,
                    _pad0: 0.0,
                    _pad1: 0.0,
                };

                let tuniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Transition Uniform Buffer"),
                    contents: bytemuck::cast_slice(&[tuniforms]),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

                let t_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Transition Bind Group"),
                    layout: &self.transition_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: tuniform_buf.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&from_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::Sampler(&self.sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: wgpu::BindingResource::TextureView(&to_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 4,
                            resource: wgpu::BindingResource::Sampler(&self.sampler),
                        },
                    ],
                });

                rpass.set_pipeline(&self.transition_pipeline);
                rpass.set_bind_group(0, &t_bind_group, &[]);
                rpass.draw(0..6, 0..1);
            }
        }

        // WebGPU alignment requirement: bytes_per_row must be a multiple of 256
        let unpadded_bytes_per_row = width * 4;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(align) * align;
        let staging_buffer_size = (padded_bytes_per_row * height) as u64;

        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Readback Staging Buffer"),
            size: staging_buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &target_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &staging_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            target_desc.size,
        );

        queue.submit(std::iter::once(encoder.finish()));

        // Map buffer and read back RGBA pixels
        let buffer_slice = staging_buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });

        device.poll(wgpu::Maintain::Wait);
        rx.recv()
            .map_err(|e| RenderError::Internal(e.to_string()))?
            .map_err(|e| RenderError::Internal(format!("Buffer map error: {:?}", e)))?;

        let mapped = buffer_slice.get_mapped_range();
        let mut rgba_pixels = Vec::with_capacity((width * height * 4) as usize);

        for row in 0..height {
            let start = (row * padded_bytes_per_row) as usize;
            let end = start + (width * 4) as usize;
            rgba_pixels.extend_from_slice(&mapped[start..end]);
        }

        drop(mapped);
        staging_buffer.unmap();

        Ok(RenderedFrame {
            width,
            height,
            rgba_pixels,
        })
    }
}

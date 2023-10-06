use crate::objparse::{Model, Vertex};
use nalgebra::Matrix4;
use std::borrow::Cow;
use std::mem::size_of;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    Buffer, BufferUsages, Color, ColorTargetState, ColorWrites, CommandEncoderDescriptor,
    CompareFunction, DepthStencilState, Device, DeviceDescriptor, Features, FragmentState,
    Instance, Limits, LoadOp, MultisampleState, Operations, PipelineLayoutDescriptor,
    PowerPreference, PresentMode, PrimitiveState, PushConstantRange, Queue,
    RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor,
    RenderPipeline, RenderPipelineDescriptor, RequestAdapterOptions, ShaderModule, ShaderSource,
    ShaderStages, Surface, SurfaceConfiguration, TextureDescriptor, TextureFormat, TextureUsages,
    TextureView, VertexAttribute, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
};
use winit::window::Window;

fn depth(device: &Device, width: u32, height: u32) -> TextureView {
    let depth = device.create_texture(&TextureDescriptor {
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        label: None,
    });
    let view = depth.create_view(&wgpu::TextureViewDescriptor::default());
    view
}

fn projgen(width: u32, height: u32) -> Matrix4<f32> {
    let proj = nalgebra::Perspective3::new(
        width as f32 / height as f32,
        70.0f32.to_radians(),
        0.01,
        1000.0,
    );
    proj.to_homogeneous()
}

pub struct RenderState {
    // surface/instance
    _instance: Instance,
    surface: Surface,
    surface_cfg: SurfaceConfiguration,
    // device/queue
    device: Device,
    queue: Queue,
    // rendering infrastructure
    _shader: ShaderModule,
    render_pipeline: RenderPipeline,
    // model buffer
    model_buffer: Buffer,
    idx_offset: u64,
    idx_count: u32,
    // model depth texture
    depth_view: TextureView,
    // matrices
    pub proj: Matrix4<f32>,
    pub projview: Matrix4<f32>,
}

impl RenderState {
    pub fn create(window: &Window, model: &Model) -> Self {
        let instance = Instance::new(wgpu::Backends::all());
        let surface = unsafe { instance.create_surface(&window) };
        let adapter = futures::executor::block_on(async {
            instance
                .request_adapter(&RequestAdapterOptions {
                    power_preference: PowerPreference::HighPerformance,
                    force_fallback_adapter: false,
                    compatible_surface: Some(&surface),
                })
                .await
        })
        .unwrap();
        log::info!("Chose the adapter: {:?}", adapter.get_info());
        let format = surface.get_supported_formats(&adapter)[0];
        let (width, height) = window.inner_size().into();
        let surface_cfg = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: PresentMode::Mailbox,
        };
        let (device, queue) = futures::executor::block_on(async {
            adapter
                .request_device(
                    &DeviceDescriptor {
                        label: None,
                        features: Features::PUSH_CONSTANTS,
                        // we don't care about OpenGL or DX11 here
                        // can always change in future if it turns out we do
                        limits: Limits {
                            // supported on basically every platform
                            max_push_constant_size: 128,
                            ..Default::default()
                        },
                    },
                    None,
                )
                .await
        })
        .unwrap();
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
        });
        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::VERTEX,
                range: 0..64,
            }],
        });
        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: Some(&render_pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[VertexBufferLayout {
                    array_stride: size_of::<Vertex>() as u64,
                    step_mode: VertexStepMode::Vertex,
                    attributes: &[
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        },
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: 12,
                            shader_location: 1,
                        },
                    ],
                }],
            },
            // triangles, no culling, no special features
            primitive: PrimitiveState::default(),
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
                // no stencil
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: MultisampleState::default(),
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format,
                    blend: None,
                    write_mask: ColorWrites::COLOR,
                })],
            }),
            multiview: None,
        });
        let vtxln = model.vertices.len() * size_of::<Vertex>();
        let idxln = model.indices.len() * size_of::<u32>();
        let idx_count = model.indices.len() as u32;
        let mut model_bytes = vec![0u8; vtxln + idxln];
        // copy the model into a buffer to be passed as one into the shader
        // the first part of the buffer for vertices, latter part indices
        unsafe {
            std::ptr::copy(
                model.vertices.as_ptr() as *const u8,
                model_bytes.as_mut_ptr(),
                vtxln,
            );
            std::ptr::copy(
                model.indices.as_ptr() as *const u8,
                model_bytes.as_mut_ptr().offset(vtxln as isize),
                idxln,
            );
        }
        let model_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: &model_bytes,
            usage: BufferUsages::VERTEX | BufferUsages::INDEX,
        });
        let (w, h) = window.inner_size().into();
        let depth_view = depth(&device, w, h);
        let proj = projgen(w, h);
        Self {
            _instance: instance,
            surface,
            surface_cfg,
            device,
            queue,
            _shader: shader,
            render_pipeline,
            model_buffer,
            idx_offset: vtxln as u64,
            idx_count,
            depth_view,
            proj,
            projview: Matrix4::identity(),
        }
    }

    pub fn resize(&mut self, (width, height): (u32, u32)) {
        self.surface_cfg.width = width;
        self.surface_cfg.height = height;
        self.surface.configure(&self.device, &self.surface_cfg);
        self.depth_view = depth(&self.device, width, height);
        self.proj = projgen(width, height);
    }

    pub fn render(&self) {
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: None });
        let frame = self.surface.get_current_texture().unwrap();
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        {
            let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color {
                            r: 0.3,
                            g: 0.3,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });
            rpass.set_pipeline(&self.render_pipeline);
            rpass.set_push_constants(ShaderStages::VERTEX, 0, unsafe {
                std::slice::from_raw_parts(self.projview.as_ptr() as *const u8, 64)
            });
            rpass.set_index_buffer(
                self.model_buffer.slice(self.idx_offset..),
                wgpu::IndexFormat::Uint32,
            );
            rpass.set_vertex_buffer(0, self.model_buffer.slice(..self.idx_offset));
            rpass.draw_indexed(0..self.idx_count, 0, 0..1);
        }
        self.queue.submit(Some(encoder.finish()));
        frame.present();
    }
}

use nalgebra::Matrix4;
use std::borrow::Cow;
use std::mem::size_of;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    vertex_attr_array, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, Buffer,
    BufferBinding, BufferBindingType, BufferDescriptor, BufferUsages, Color, ColorTargetState,
    ColorWrites, CommandEncoder, CommandEncoderDescriptor, CompareFunction, DepthStencilState,
    Device, DeviceDescriptor, Features, FragmentState, IndexFormat, Instance, Limits, LoadOp,
    Maintain, MapMode, MultisampleState, Operations, PipelineLayoutDescriptor, PolygonMode,
    PowerPreference, PresentMode, PrimitiveState, PushConstantRange, Queue,
    RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor,
    RenderPipeline, RenderPipelineDescriptor, RequestAdapterOptions, ShaderModule, ShaderSource,
    ShaderStages, Surface, SurfaceConfiguration, TextureDescriptor, TextureFormat, TextureUsages,
    TextureView, VertexBufferLayout, VertexState, VertexStepMode,
};
use winit::window::Window;

use crate::model::{SkeletalModel, Vertex};

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

// similar note to MAX_WEIGHTS in model.rs
// this value is a compromise and hardcoded into the shader
// as the maximum length of the uniform array
// cause uniforms have to be sized :/
const MAX_JOINT_TRANSFORMS: u64 = 64;

// ok so in theory i could have one buffer
// and then multiple bind groups for each model
// and do something similar for vertices/indices
// but i'm not gonna do that cause why would i do that
pub struct JointTransformsState {
    staging_buffer: Buffer,
    uniform_buffer: Buffer,
    bind_group: BindGroup,

    verticesindices: Buffer,
    vtx_offset: u64,
    idx_count: u32,
}

impl RenderState {
    pub fn new_jts(&self, skel: &SkeletalModel) -> JointTransformsState {
        let staging_buffer = self.device.create_buffer(&BufferDescriptor {
            label: None,
            size: MAX_JOINT_TRANSFORMS * 64,
            usage: BufferUsages::MAP_WRITE | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let main_buffer = self.device.create_buffer(&BufferDescriptor {
            label: None,
            size: MAX_JOINT_TRANSFORMS * 64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &self.matrices_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: &main_buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        let mesh = match &skel.mesh {
            Some(mesh) => mesh,
            None => unimplemented!(), // TODO
        };
        let offset = mesh.vertices.len() * size_of::<Vertex>();
        let mut contents = vec![0u8; offset + mesh.indices.len() * 4];
        unsafe {
            std::ptr::copy(
                mesh.vertices.as_ptr() as *const u8,
                contents.as_mut_ptr(),
                offset,
            );
            std::ptr::copy(
                mesh.indices.as_ptr() as *const u8,
                contents.as_mut_ptr().add(offset),
                mesh.indices.len() * 4,
            );
        }

        let verticesindices = self.device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: &contents,
            usage: BufferUsages::VERTEX | BufferUsages::INDEX,
        });
        JointTransformsState {
            staging_buffer,
            uniform_buffer: main_buffer,
            bind_group,
            verticesindices,
            vtx_offset: offset as u64,
            idx_count: mesh.indices.len() as u32,
        }
    }

    pub fn update_jts_staging(&self, state: &JointTransformsState, matrices: &[Matrix4<f32>]) {
        // ok so the way wgpu does synchronization bothers me
        // maybe in future i should consider wgpu-hal
        // ok probably not it's basically Vulkan and this works perfectly fine i just find it very displeasing
        state
            .staging_buffer
            .slice(..)
            .map_async(MapMode::Write, |e| e.unwrap());
        self.device.poll(Maintain::Wait);
        let mut buf = state.staging_buffer.slice(..).get_mapped_range_mut();
        unsafe {
            std::ptr::copy(
                matrices.as_ptr() as *const u8,
                (&mut *buf).as_mut_ptr() as *mut u8,
                matrices.len() * 64,
            )
        }
        drop(buf);
        state.staging_buffer.unmap();
    }

    pub fn schedule_jts_updates<'a, I: Iterator<Item = &'a JointTransformsState>>(
        &self,
        states: I,
        encoder: &mut CommandEncoder,
    ) {
        for state in states {
            encoder.copy_buffer_to_buffer(
                &state.staging_buffer,
                0,
                &state.uniform_buffer,
                0,
                MAX_JOINT_TRANSFORMS * 64,
            );
        }
    }
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
    matrices_bind_group_layout: BindGroupLayout,
    render_pipeline: RenderPipeline,
    // depth texture
    depth_view: TextureView,

    // matrices
    pub proj: Matrix4<f32>,
    pub projview: Matrix4<f32>,
}

impl RenderState {
    pub fn create(window: &Window) -> Self {
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
                        features: Features::PUSH_CONSTANTS | Features::POLYGON_MODE_LINE,
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
        let matrices_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: None,
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        // uniform buffers apparently require *sized* arrays
                        // so may as well perform the check here
                        // can't do the new().unwrap() in const annoyingly so i have to do this
                        min_binding_size: Some(MAX_JOINT_TRANSFORMS.try_into().unwrap()),
                    },
                    count: None,
                }],
            });
        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&matrices_bind_group_layout],
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
                    attributes: &vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x3, 3 => Float32x3],
                }],
            },
            primitive: PrimitiveState {
                // when we don't have normals it's more helpful to view as a wireframe
                // but we do now so it's all good
                polygon_mode: PolygonMode::Fill,
                ..Default::default()
            },
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
            matrices_bind_group_layout,
            render_pipeline,
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

    pub fn render(&self, jtss: &[&JointTransformsState]) {
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: None });
        self.schedule_jts_updates(jtss.iter().map(|a| *a), &mut encoder);
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
            for jts in jtss {
                rpass.set_bind_group(0, &jts.bind_group, &[]);
                rpass.set_vertex_buffer(0, jts.verticesindices.slice(..jts.vtx_offset));
                rpass.set_index_buffer(
                    jts.verticesindices.slice(jts.vtx_offset..),
                    IndexFormat::Uint32,
                );
                rpass.draw_indexed(0..jts.idx_count, 0, 0..1);
            }
            // rpass.set_index_buffer(
            //     self.model_buffer.slice(self.idx_offset..),
            //     wgpu::IndexFormat::Uint32,
            // );
            // rpass.set_vertex_buffer(0, self.model_buffer.slice(..self.idx_offset));
            // rpass.draw_indexed(0..self.idx_count, 0, 0..1);
        }
        self.queue.submit(Some(encoder.finish()));
        frame.present();
    }
}

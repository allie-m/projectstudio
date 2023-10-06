use nalgebra::Matrix4;
use std::borrow::Cow;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    Backends, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBinding, BufferBindingType,
    BufferDescriptor, BufferUsages, Color, ColorTargetState, ColorWrites, CommandEncoderDescriptor,
    CompareFunction, ComputePassDescriptor, ComputePipeline, ComputePipelineDescriptor,
    DepthStencilState, Device, DeviceDescriptor, Extent3d, Features, FragmentState, IndexFormat,
    Instance, Limits, LoadOp, MultisampleState, Operations, PipelineLayoutDescriptor, PolygonMode,
    PowerPreference, PresentMode, PrimitiveState, PushConstantRange, Queue,
    RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor,
    RenderPipeline, RenderPipelineDescriptor, RequestAdapterOptions, ShaderModule,
    ShaderModuleDescriptor, ShaderSource, ShaderStages, Surface, SurfaceConfiguration,
    TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView,
    TextureViewDescriptor, VertexState,
};
use winit::window::Window;

fn depth(device: &Device, width: u32, height: u32) -> TextureView {
    let depth = device.create_texture(&TextureDescriptor {
        size: Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Depth32Float,
        usage: TextureUsages::RENDER_ATTACHMENT,
        label: None,
    });
    let view = depth.create_view(&TextureViewDescriptor::default());
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

struct RK4Compute {
    c1: ComputePipeline,
    c2: ComputePipeline,
    c3: ComputePipeline,
    c4: ComputePipeline,
    c5: ComputePipeline,
    pbg1: BindGroup,
    pbg2: BindGroup,
    nbg1: BindGroup,
    nbg2: BindGroup,
}

// must be a power of 2
const CLOTH_SIZE: u64 = 128;

pub struct RenderState {
    // surface/instance
    _instance: Instance,
    surface: Surface,
    surface_cfg: SurfaceConfiguration,
    device: Device,
    queue: Queue,
    _shader: ShaderModule,
    render_pipeline: RenderPipeline,
    #[allow(unused)]
    euler_pipeline: ComputePipeline,
    depth_view: TextureView,
    // particle_buf: Buffer,
    particles_bg: BindGroup,
    render_bg: BindGroup,
    rk4_compute: RK4Compute,
    index_buf: Buffer,
    idx_count: u32,
    // matrices
    pub proj: Matrix4<f32>,
    pub projview: Matrix4<f32>,
}

impl RenderState {
    pub fn create(window: &Window) -> Self {
        let instance = Instance::new(Backends::all());
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
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: None,
            source: ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
        });
        let particles_bg_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let render_bg_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&render_bg_layout],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::VERTEX_FRAGMENT,
                range: 0..72,
            }],
        });
        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: Some(&render_pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            // triangles, no culling, no special features
            primitive: PrimitiveState {
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
        let euler_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&particles_bg_layout],
            push_constant_ranges: &[],
        });
        let euler_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: None,
            layout: Some(&euler_pipeline_layout),
            module: &shader,
            entry_point: "cloth_euler",
        });
        let particles = (0..CLOTH_SIZE * CLOTH_SIZE)
            .map(|a| {
                [
                    (a % CLOTH_SIZE) as f32,
                    (a / CLOTH_SIZE) as f32,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                ]
            })
            .collect::<Vec<_>>();
        let particle_buf = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: unsafe {
                std::slice::from_raw_parts(
                    particles.as_ptr() as *const u8,
                    particles.len() * 12 * 4,
                )
            },
            usage: BufferUsages::STORAGE | BufferUsages::VERTEX,
        });
        let particles_bg = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &particles_bg_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: &particle_buf,
                    offset: 0,
                    size: None,
                }),
            }],
        });
        let render_bg = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &render_bg_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: &particle_buf,
                    offset: 0,
                    size: None,
                }),
            }],
        });
        let rk4_compute = {
            let bg_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
            let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&particles_bg_layout, &bg_layout],
                push_constant_ranges: &[],
            });
            let mut clt = ComputePipelineDescriptor {
                label: None,
                layout: Some(&layout),
                module: &shader,
                entry_point: "rk4_1",
            };
            let c1 = device.create_compute_pipeline(&clt);
            clt.entry_point = "rk4_2";
            let c2 = device.create_compute_pipeline(&clt);
            clt.entry_point = "rk4_3";
            let c3 = device.create_compute_pipeline(&clt);
            clt.entry_point = "rk4_4";
            let c4 = device.create_compute_pipeline(&clt);
            clt.entry_point = "rk4_5";
            let c5 = device.create_compute_pipeline(&clt);
            let n1 = device.create_buffer(&BufferDescriptor {
                label: None,
                size: CLOTH_SIZE * CLOTH_SIZE * 48,
                usage: BufferUsages::STORAGE,
                mapped_at_creation: false,
            });
            let n2 = device.create_buffer(&BufferDescriptor {
                label: None,
                size: CLOTH_SIZE * CLOTH_SIZE * 48,
                usage: BufferUsages::STORAGE,
                mapped_at_creation: false,
            });
            let forcesum = device.create_buffer(&BufferDescriptor {
                label: None,
                size: CLOTH_SIZE * CLOTH_SIZE * 24,
                usage: BufferUsages::STORAGE,
                mapped_at_creation: false,
            });
            let pbg1 = device.create_bind_group(&BindGroupDescriptor {
                label: None,
                layout: &particles_bg_layout,
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &n1,
                        offset: 0,
                        size: None,
                    }),
                }],
            });
            let pbg2 = device.create_bind_group(&BindGroupDescriptor {
                label: None,
                layout: &particles_bg_layout,
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &n2,
                        offset: 0,
                        size: None,
                    }),
                }],
            });
            let nbg1 = device.create_bind_group(&BindGroupDescriptor {
                label: None,
                layout: &bg_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::Buffer(BufferBinding {
                            buffer: &n1,
                            offset: 0,
                            size: None,
                        }),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Buffer(BufferBinding {
                            buffer: &forcesum,
                            offset: 0,
                            size: None,
                        }),
                    },
                ],
            });
            let nbg2 = device.create_bind_group(&BindGroupDescriptor {
                label: None,
                layout: &bg_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::Buffer(BufferBinding {
                            buffer: &n2,
                            offset: 0,
                            size: None,
                        }),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Buffer(BufferBinding {
                            buffer: &forcesum,
                            offset: 0,
                            size: None,
                        }),
                    },
                ],
            });
            RK4Compute {
                c1,
                c2,
                c3,
                c4,
                c5,
                pbg1,
                pbg2,
                nbg1,
                nbg2,
            }
        };
        let index_cts = (0..(CLOTH_SIZE - 1) * (CLOTH_SIZE - 1) * 2 * 3)
            .map(|a| {
                let i = a % 6;
                let d = (a - i) / 6;
                let x = d % (CLOTH_SIZE - 1);
                let y = d / (CLOTH_SIZE - 1);
                let (x, y) = match i {
                    0 => (x, y),
                    1 => (x + 1, y),
                    2 => (x, y + 1),
                    3 => (x + 1, y),
                    4 => (x + 1, y + 1),
                    5 => (x, y + 1),
                    _ => unreachable!(),
                };
                (x + y * CLOTH_SIZE) as u32
            })
            .collect::<Vec<_>>();
        let index_buf = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: unsafe {
                std::slice::from_raw_parts(index_cts.as_ptr() as *const u8, index_cts.len() * 4)
            },
            usage: BufferUsages::INDEX,
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
            euler_pipeline,
            // particle_buf,
            particles_bg,
            render_bg,
            rk4_compute,
            index_buf,
            idx_count: index_cts.len() as u32,
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

    pub fn render(&self, times: u32, flag: u32) {
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: None });
        let frame = self.surface.get_current_texture().unwrap();
        let view = frame.texture.create_view(&TextureViewDescriptor::default());
        // ok so the number of compute shader invocations
        // and therefore speed of the cloth's movement
        // is in fact tied to the fps
        // so like bear that in mind
        for _ in 0..times {
            let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor { label: None });

            // rk4 method
            const DIMS: u32 = CLOTH_SIZE as u32 / 16;
            cpass.set_pipeline(&self.rk4_compute.c1);
            cpass.set_bind_group(0, &self.particles_bg, &[]);
            cpass.set_bind_group(1, &self.rk4_compute.nbg1, &[]);
            cpass.dispatch_workgroups(DIMS, DIMS, 1);
            cpass.set_pipeline(&self.rk4_compute.c2);
            cpass.set_bind_group(0, &self.rk4_compute.pbg1, &[]);
            cpass.set_bind_group(1, &self.rk4_compute.nbg2, &[]);
            cpass.dispatch_workgroups(DIMS, DIMS, 1);
            cpass.set_pipeline(&self.rk4_compute.c3);
            cpass.set_bind_group(0, &self.rk4_compute.pbg2, &[]);
            cpass.set_bind_group(1, &self.rk4_compute.nbg1, &[]);
            cpass.dispatch_workgroups(DIMS, DIMS, 1);
            cpass.set_pipeline(&self.rk4_compute.c4);
            cpass.set_bind_group(0, &self.rk4_compute.pbg1, &[]);
            cpass.set_bind_group(1, &self.rk4_compute.nbg2, &[]);
            cpass.dispatch_workgroups(DIMS, DIMS, 1);
            cpass.set_pipeline(&self.rk4_compute.c5);
            cpass.set_bind_group(0, &self.particles_bg, &[]);
            cpass.set_bind_group(1, &self.rk4_compute.nbg2, &[]); // doesn't matter what this bg is
            cpass.dispatch_workgroups(DIMS, DIMS, 1);

            // euler method
            // cpass.set_pipeline(&self.euler_pipeline);
            // cpass.set_bind_group(0, &self.particles_bg, &[]);
            // cpass.dispatch_workgroups(DIMS, DIMS, 1);
        }
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
            rpass.set_bind_group(0, &self.render_bg, &[]);
            rpass.set_index_buffer(self.index_buf.slice(..), IndexFormat::Uint32);
            rpass.set_push_constants(ShaderStages::VERTEX_FRAGMENT, 0, unsafe {
                std::slice::from_raw_parts(self.projview.as_ptr() as *const u8, 64)
            });
            rpass.set_push_constants(ShaderStages::VERTEX_FRAGMENT, 64, &flag.to_ne_bytes());
            rpass.draw_indexed(0..self.idx_count, 0, 0..1);
        }
        self.queue.submit(Some(encoder.finish()));
        frame.present();
    }
}

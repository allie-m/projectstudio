use std::borrow::Cow;

use nalgebra::{Matrix4, Perspective3};
use wgpu::{
    vertex_attr_array, Backends, BindGroup, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Color, ColorTargetState, ColorWrites,
    CommandEncoderDescriptor, CompareFunction, DepthStencilState, Device, DeviceDescriptor,
    Extent3d, Features, FragmentState, Instance, InstanceDescriptor, Limits, LoadOp, Operations,
    PipelineLayoutDescriptor, PolygonMode, PowerPreference, PrimitiveState, PushConstantRange,
    Queue, RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor,
    RenderPipeline, RenderPipelineDescriptor, RequestAdapterOptions, SamplerBindingType,
    ShaderModule, ShaderSource, ShaderStages, Surface, SurfaceConfiguration, TextureDescriptor,
    TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureView,
    TextureViewDescriptor, TextureViewDimension, VertexBufferLayout, VertexState, VertexStepMode,
};
use winit::window::Window;

use crate::{
    chunk::{generate_normal_map, Chunk, ChunkLOD},
    heightmap::TriangulatedHeightmap,
};

fn depth(device: &Device, width: u32, height: u32, usage: TextureUsages) -> TextureView {
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
        usage,
        view_formats: &[TextureFormat::Depth32Float],
        label: None,
    });
    let view = depth.create_view(&wgpu::TextureViewDescriptor::default());
    view
}

const FOV: f32 = 1.2217304763960306; // 70 degrees, in radians

fn projgen(width: u32, height: u32) -> Matrix4<f32> {
    let proj = Perspective3::new(width as f32 / height as f32, FOV, 0.01, 10000.0);
    proj.to_homogeneous()
}

pub struct RenderState {
    // base
    _instance: Instance,
    surface: Surface,
    surface_cfg: SurfaceConfiguration,
    device: Device,
    queue: Queue,
    _shader: ShaderModule,
    // pipelines & layouts
    normal_map_layout: BindGroupLayout,
    terrain_pipeline: RenderPipeline,
    #[allow(unused)]
    terrain_pipeline_bake: RenderPipeline,
    // depth textures
    second_pass_depth: TextureView,
    // matrices
    proj: Matrix4<f32>,
    pub projview: Matrix4<f32>,
}

impl RenderState {
    pub fn create(window: &Window) -> Self {
        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::PRIMARY,
            dx12_shader_compiler: Default::default(),
        });
        let surface = unsafe { instance.create_surface(&window) }.unwrap();
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
        let (width, height) = window.inner_size().into();
        let surface_cfg = surface.get_default_config(&adapter, width, height).unwrap();
        let (device, queue) = futures::executor::block_on(async {
            adapter
                .request_device(
                    &DeviceDescriptor {
                        label: None,
                        features: Features::PUSH_CONSTANTS | Features::POLYGON_MODE_LINE,
                        limits: Limits {
                            // need this for the large models
                            max_texture_array_layers: 512,
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
        let normal_map_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let terrain_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::VERTEX_FRAGMENT,
                range: 0..72,
            }],
        });
        let terrain_pipeline_bake = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: Some(&terrain_pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "terrain_vertex_bake",
                buffers: &[VertexBufferLayout {
                    array_stride: 24,
                    step_mode: VertexStepMode::Vertex,
                    attributes: &vertex_attr_array![0 => Float32x3, 1 => Float32x3],
                }],
            },
            primitive: PrimitiveState::default(),
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            fragment: None,
            multiview: None,
        });
        let terrain_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&normal_map_layout],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::VERTEX_FRAGMENT,
                range: 0..72,
            }],
        });
        let terrain_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: Some(&terrain_pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "terrain_vertex",
                buffers: &[VertexBufferLayout {
                    array_stride: 24,
                    step_mode: VertexStepMode::Vertex,
                    attributes: &vertex_attr_array![0 => Float32x3, 1 => Float32x3],
                }],
            },
            primitive: PrimitiveState {
                polygon_mode: PolygonMode::Fill,
                ..Default::default()
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "terrain_fragment",
                targets: &[Some(ColorTargetState {
                    format: TextureFormat::Bgra8UnormSrgb,
                    blend: None,
                    write_mask: ColorWrites::COLOR,
                })],
            }),
            multiview: None,
        });
        let second_pass_depth = depth(&device, width, height, TextureUsages::RENDER_ATTACHMENT);
        Self {
            _instance: instance,
            surface,
            surface_cfg,
            device,
            queue,
            _shader: shader,
            normal_map_layout,
            terrain_pipeline,
            terrain_pipeline_bake,
            second_pass_depth,
            proj: Matrix4::identity(),
            projview: Matrix4::identity(),
        }
    }

    pub fn upload_heightmap(&self, heightmap: &TriangulatedHeightmap) -> Chunk {
        Chunk {
            lods: heightmap
                .meshes
                .iter()
                .map(|mesh| ChunkLOD::create(&self.device, mesh, heightmap.index as i32))
                .collect(),
            bl_pos: (heightmap.pos.0 as f32, heightmap.pos.1 as f32),
        }
    }

    pub fn normal_map(&self, chunks: &[TriangulatedHeightmap], chunk_size: u32) -> BindGroup {
        generate_normal_map(
            &self.device,
            &self.queue,
            &self.normal_map_layout,
            chunks,
            chunk_size,
        )
    }

    pub fn resize(&mut self, (width, height): (u32, u32)) {
        self.proj = projgen(width, height);
        self.surface_cfg.width = width;
        self.surface_cfg.height = height;
        self.surface.configure(&self.device, &self.surface_cfg);
        self.second_pass_depth = depth(
            &self.device,
            width,
            height,
            TextureUsages::RENDER_ATTACHMENT,
        );
    }

    pub fn update_view(&mut self, view: Matrix4<f32>) {
        self.projview = self.proj * view;
    }

    pub fn render(
        &self,
        chunks: &[Chunk],
        normal_map: &BindGroup,
        chunk_size: u32,
        // camera: &Camera3D,
        lod: usize,
    ) {
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: None });
        let frame = self.surface.get_current_texture().unwrap();
        let view = frame.texture.create_view(&TextureViewDescriptor::default());

        // second rpass
        {
            let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color {
                            r: 0.53,
                            g: 0.8,
                            b: 0.92,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &self.second_pass_depth,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });
            rpass.set_pipeline(&self.terrain_pipeline);
            rpass.set_bind_group(0, normal_map, &[]);
            rpass.set_push_constants(ShaderStages::VERTEX_FRAGMENT, 0, unsafe {
                std::slice::from_raw_parts(self.projview.as_ptr() as *const u8, 64)
            });
            rpass.set_push_constants(ShaderStages::VERTEX_FRAGMENT, 64, &chunk_size.to_ne_bytes());
            for chunk in chunks.iter() {
                // let xdist = ((chunk.bl_pos.0 * chunk_size as f32 + chunk_size as f32 / 2.0)
                //     - camera.position.x)
                //     .abs();
                // let zdist = ((chunk.bl_pos.1 * chunk_size as f32 + chunk_size as f32 / 2.0)
                //     - camera.position.z)
                //     .abs();

                // // TODO don't render offscreen chunks
                // // let v1 = nalgebra::Vector3::new(chunk.bl_pos.0, 0.0, chunk.bl_pos.1);
                // // let d = (v1 - camera.position).normalize();
                // // let angle = camera.rotation.normalize().dot(&d);
                // // if angle.abs() > FOV {
                // //     println!("not rendering a chunk! {}, {}", angle, FOV);
                // //     continue;
                // // }

                // if xdist < chunk_size as f32 * 1.5 && zdist < chunk_size as f32 * 1.5 {
                //     chunk.lods[0].draw(&mut rpass);
                // } else if xdist < chunk_size as f32 * 3.0 && zdist < chunk_size as f32 * 3.0 {
                //     chunk.lods[1].draw(&mut rpass);
                // } else {
                //     chunk.lods[2].draw(&mut rpass);
                // }
                chunk.lods[lod].draw(&mut rpass);
            }
        }
        self.queue.submit(Some(encoder.finish()));
        frame.present();
    }
}

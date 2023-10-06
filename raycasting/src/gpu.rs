use std::{borrow::Cow, mem::size_of, num::NonZeroU32};

use nalgebra::{Matrix4, Vector3};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    Backends, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, BufferBindingType, BufferDescriptor,
    BufferUsages, CommandEncoderDescriptor, ComputePassDescriptor, ComputePipeline,
    ComputePipelineDescriptor, Device, DeviceDescriptor, Extent3d, Features, ImageCopyBuffer,
    ImageCopyTexture, ImageDataLayout, Instance, InstanceDescriptor, Limits, Maintain, MapMode,
    Origin3d, PipelineLayoutDescriptor, PowerPreference, PushConstantRange, Queue,
    RequestAdapterOptions, SamplerBindingType, SamplerDescriptor, ShaderModule, ShaderSource,
    ShaderStages, StorageTextureAccess, Texture, TextureAspect, TextureDescriptor,
    TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureViewDescriptor,
    TextureViewDimension,
};

use crate::{
    perspective::Perspective,
    scene::{transform_index, SceneConfig},
};

#[repr(C)]
struct PushConstants {
    bg_color: [f32; 4],
    screensize: [f32; 2],
    sphere_cts: u32,
    plane_cts: u32,
    mesh_cts: u32,
    light_cts: u32,
    max_bounces: u32,
    has_cubemap: u32,
    ambient_light: [f32; 4],
}

pub struct GpuState {
    _instance: Instance,
    device: Device,
    queue: Queue,
    _shader: ShaderModule,

    pipeline: ComputePipeline,

    target: Texture,

    bg: BindGroup,

    pcs: PushConstants,

    width: u32,
    height: u32,
}

impl GpuState {
    pub fn create(width: u32, height: u32, sc: &SceneConfig, max_bounces: u32) -> Self {
        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::all(),
            ..Default::default()
        });
        let adapter = futures::executor::block_on(async {
            instance
                .request_adapter(&RequestAdapterOptions {
                    power_preference: PowerPreference::HighPerformance,
                    force_fallback_adapter: false,
                    compatible_surface: None,
                })
                .await
        })
        .unwrap();
        log::info!("Chose the adapter: {:?}", adapter.get_info());
        let (device, queue) = futures::executor::block_on(async {
            adapter
                .request_device(
                    &DeviceDescriptor {
                        label: None,
                        // unfortunately this feature set restricts me to DX12/Vulkan1.2+/MSL2+
                        features: Features::PUSH_CONSTANTS | Features::TEXTURE_BINDING_ARRAY | Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING,
                        // we don't care about OpenGL or DX11 here
                        // can always change in future if it turns out we do
                        limits: Limits {
                            // supported on basically every platform
                            max_push_constant_size: 128,
                            max_sampled_textures_per_shader_stage: sc.num_of_textures().max(16) as u32,
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

        let bgl = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::StorageTexture {
                        access: StorageTextureAccess::WriteOnly,
                        format: TextureFormat::Rgba8Unorm,
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 4,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 5,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 6,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 7,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 8,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 9,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 10,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 11,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 12,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: NonZeroU32::new(sc.num_of_textures().max(1) as u32),
                },
                BindGroupLayoutEntry {
                    binding: 13,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 14,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::Cube,
                        multisampled: false,
                    },
                    count: NonZeroU32::new(sc.num_of_textures().max(1) as u32),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::COMPUTE,
                range: 0..size_of::<PushConstants>() as u32,
            }],
        });
        let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "render",
        });

        let target = device.create_texture(&TextureDescriptor {
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::COPY_SRC | TextureUsages::STORAGE_BINDING,
            view_formats: &[],
            label: None,
        });
        let target_view = target.create_view(&TextureViewDescriptor::default());

        let mut transformations = vec![Matrix4::identity()];

        let mats = Perspective::generate(&sc.perspective, width as f32 / height as f32);
        let b1 = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: unsafe {
                std::slice::from_raw_parts(
                    &mats as *const Perspective as *const u8,
                    size_of::<Perspective>(),
                )
            },
            usage: BufferUsages::UNIFORM,
        });

        // why no anonymous structs with defined layouts
        // ok maybe it's a hint that i should be doing this all in a more structured less cobbled together sloppily way
        // i should have probably sketched out the structure of what i'm sending to the shader ahead of time
        // but anonymous + heterogenous + defined layout
        // isn't that much to ask for right
        #[repr(C)]
        struct Sphere {
            stuff: [f32; 4],
            idx: u32,
            material: u32,
            _pad: [u32; 2],
        }
        let spheres = sc
            .objects
            .iter()
            .filter_map(|a| match a.kind {
                crate::scene::ROKind::Sphere { center, radius } => Some(Sphere {
                    stuff: [center.x, center.y, center.z, radius],
                    idx: transform_index(a.transform, &mut transformations, true),
                    material: a.material as u32,
                    _pad: [0; 2],
                }),
                _ => None,
            })
            .collect::<Vec<_>>();
        let b2 = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: if spheres.len() > 0 {
                unsafe {
                    std::slice::from_raw_parts(
                        spheres.as_ptr() as *const u8,
                        size_of::<Sphere>() * spheres.len(),
                    )
                }
            } else {
                &[0; 32]
            },
            usage: BufferUsages::UNIFORM,
        });
        let planes = sc
            .objects
            .iter()
            .filter_map(|a| match a.kind {
                crate::scene::ROKind::Plane { normal, offset } => Some([
                    normal.x,
                    normal.y,
                    normal.z,
                    offset,
                    f32::from_bits(a.material as u32),
                    0.0,
                    0.0,
                ]),
                _ => None,
            })
            .collect::<Vec<_>>();
        let b3 = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: if planes.len() > 0 {
                unsafe {
                    std::slice::from_raw_parts(planes.as_ptr() as *const u8, 8 * 4 * planes.len())
                }
            } else {
                &[0; 32]
            },
            usage: BufferUsages::UNIFORM,
        });
        let mut vertices = vec![];
        let mut indices = vec![];
        let mut tcoords = vec![];
        let mut meshes = vec![];
        let mut end_normals = vec![];
        let mut texs = vec![];
        let mut tcount = 0;
        for a in sc.objects.iter() {
            match &a.kind {
                crate::scene::ROKind::Mesh { obj_file } => {
                    let vo = vertices.len() as u32;
                    let io = indices.len() as u32;
                    let mesh =
                        &tobj::load_obj(sc.path_to_scene.join(obj_file), &tobj::GPU_LOAD_OPTIONS)
                            .unwrap()
                            .0[0]
                            .mesh;
                    // ok a brief note on how I'm handling textures
                    // so there are several ways I could approach this:
                    // a texture binding array, a 2d texture array, or a mipmapped/3d texture
                    //
                    // the only one that can handle variable size textures is a texture binding array
                    // so that's what i'm going with
                    // it's not necessary for the raycaster half but is necessary
                    // for the more complex raytraced scenes
                    //
                    // pour one out for the 2d texture array implementation i sadly had to axe
                    // when i realized it didn't fit the requirements of the raytraced scenes
                    // i preferred texture arrays cause it was a more cross-platform take but
                    // so it goes
                    if let Some(tex) = &sc.materials[a.material].texture {
                        let tex = image::open(sc.path_to_scene.join(tex)).unwrap();
                        tcount += 1;
                        let s = (tex.width(), tex.height());
                        let contents = tex.into_rgba32f().to_vec();
                        texs.push((s, contents));
                    }
                    let new_positions = mesh
                        .positions
                        .chunks(3)
                        .map(|b| [b[0], b[1], b[2], 0.0])
                        .collect::<Vec<_>>();
                    let tcs = if mesh.texcoords.len() != 0 {
                        mesh.texcoords
                            .chunks(2)
                            .map(|a| [a[0], 1.0 - a[1]])
                            .collect::<Vec<_>>()
                    } else {
                        vec![]
                    };
                    let (normals, flat) = if mesh.normals.len() == 0 {
                        // https://computergraphics.stackexchange.com/questions/4031/programmatically-generating-vertex-normals
                        // https://stackoverflow.com/questions/13205226/most-efficient-algorithm-to-calculate-vertex-normals-from-set-of-triangles-for-g
                        let mut normals = vec![Vector3::new(0.0, 0.0, 0.0); new_positions.len()];
                        for pos in mesh.indices.chunks(3) {
                            let a = new_positions[pos[0] as usize];
                            let a = Vector3::new(a[0], a[1], a[2]);
                            let b = new_positions[pos[1] as usize];
                            let b = Vector3::new(b[0], b[1], b[2]);
                            let c = new_positions[pos[2] as usize];
                            let c = Vector3::new(c[0], c[1], c[2]);

                            let v = (b - a).cross(&(c - a));
                            normals[pos[0] as usize] += v;
                            normals[pos[1] as usize] += v;
                            normals[pos[2] as usize] += v;
                        }
                        let normals = normals
                            .into_iter()
                            .map(|n| {
                                let n = n.normalize();
                                [n.x, n.y, n.z, 0.0]
                            })
                            .collect::<Vec<_>>();
                        (normals, false)
                    } else {
                        (
                            mesh.normals
                                .chunks(3)
                                .map(|a| [a[0], a[1], a[2], 0.0])
                                .collect::<Vec<_>>(),
                            false,
                        )
                    };

                    end_normals.extend_from_slice(&normals);
                    vertices.extend_from_slice(&new_positions);
                    tcoords.extend_from_slice(&tcs);
                    indices.extend_from_slice(&mesh.indices);
                    meshes.push([
                        vo,
                        io,
                        mesh.indices.len() as u32,
                        transform_index(a.transform, &mut transformations, false),
                        a.material as u32,
                        flat as u32,
                        if tcs.len() == 0 { 0 } else { tcount },
                        0,
                    ]);
                }
                _ => continue,
            }
        }
        let mut textures = texs
            .iter()
            .map(|(size, data)| {
                let texture = device.create_texture_with_data(
                    &queue,
                    &TextureDescriptor {
                        label: None,
                        size: Extent3d {
                            width: size.0,
                            height: size.1,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: TextureDimension::D2,
                        format: TextureFormat::Rgba32Float,
                        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                        view_formats: &[],
                    },
                    if !data.is_empty() {
                        unsafe {
                            std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 4)
                        }
                    } else {
                        &[0u8; 16]
                    },
                );
                let tex_view = texture.create_view(&TextureViewDescriptor {
                    dimension: Some(TextureViewDimension::D2),
                    ..Default::default()
                });
                tex_view
            })
            .collect::<Vec<_>>();
        // if the scene has no textures we still need a stub
        while textures.len() < 1 {
            let texture = device.create_texture(&TextureDescriptor {
                label: None,
                size: Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba32Float,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let tex_view = texture.create_view(&TextureViewDescriptor {
                dimension: Some(TextureViewDimension::D2),
                ..Default::default()
            });
            textures.push(tex_view);
        }
        let textures_borrowed = textures.iter().map(|a| a).collect::<Vec<_>>();
        // let texture = device.create_texture_with_data(
        //     &queue,
        //     &TextureDescriptor {
        //         label: None,
        //         size: if tsize != (0, 0) {
        //             Extent3d {
        //                 width: tsize.0,
        //                 height: tsize.1,
        //                 depth_or_array_layers: tcount,
        //             }
        //         } else {
        //             Extent3d {
        //                 width: 1,
        //                 height: 1,
        //                 depth_or_array_layers: 1,
        //             }
        //         },
        //         mip_level_count: 1,
        //         sample_count: 1,
        //         dimension: TextureDimension::D2,
        //         format: TextureFormat::Rgba32Float,
        //         usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        //         view_formats: &[],
        //     },
        //     if !texs.is_empty() {
        //         unsafe { std::slice::from_raw_parts(texs.as_ptr() as *const u8, texs.len() * 4) }
        //     } else {
        //         &[0u8; 16]
        //     },
        // );
        let cubemap = {
            let (buf, size) = match &sc.background.cube_map {
                Some(cubemap) => {
                    let mut buf = vec![];
                    buf.extend_from_slice(&cubemap.right);
                    buf.extend_from_slice(&cubemap.left);
                    buf.extend_from_slice(&cubemap.up);
                    buf.extend_from_slice(&cubemap.down);
                    buf.extend_from_slice(&cubemap.back);
                    buf.extend_from_slice(&cubemap.front);
                    (buf, cubemap.size)
                }
                None => (vec![0.0; 6 * 4], (1, 1)),
            };
            let texture = device.create_texture_with_data(
                &queue,
                &TextureDescriptor {
                    label: None,
                    size: Extent3d {
                        width: size.0,
                        height: size.1,
                        depth_or_array_layers: 6,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format: TextureFormat::Rgba32Float,
                    usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                    view_formats: &[],
                },
                unsafe { std::slice::from_raw_parts(buf.as_ptr() as *const u8, buf.len() * 4) },
            );
            let tex_view = texture.create_view(&TextureViewDescriptor {
                dimension: Some(TextureViewDimension::Cube),
                ..Default::default()
            });
            tex_view
        };
        let b10 = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: if end_normals.len() > 0 {
                unsafe {
                    std::slice::from_raw_parts(
                        end_normals.as_ptr() as *const u8,
                        end_normals.len() * 4 * 4,
                    )
                }
            } else {
                &[0; 16]
            },
            usage: BufferUsages::STORAGE,
        });
        let b11 = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: if tcoords.len() > 0 {
                unsafe {
                    std::slice::from_raw_parts(tcoords.as_ptr() as *const u8, tcoords.len() * 4 * 2)
                }
            } else {
                &[0; 8]
            },
            usage: BufferUsages::STORAGE,
        });
        let b4 = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: if vertices.len() > 0 {
                unsafe {
                    std::slice::from_raw_parts(
                        vertices.as_ptr() as *const u8,
                        vertices.len() * 4 * 4,
                    )
                }
            } else {
                &[0; 16]
            },
            usage: BufferUsages::STORAGE,
        });
        let b5 = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: if indices.len() > 0 {
                unsafe {
                    std::slice::from_raw_parts(indices.as_ptr() as *const u8, indices.len() * 4)
                }
            } else {
                &[0; 16]
            },
            usage: BufferUsages::STORAGE,
        });
        let b6 = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: if meshes.len() > 0 {
                unsafe {
                    std::slice::from_raw_parts(meshes.as_ptr() as *const u8, 32 * meshes.len())
                }
            } else {
                &[0; 32]
            },
            usage: BufferUsages::UNIFORM,
        });
        let b7 = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: unsafe {
                std::slice::from_raw_parts(
                    transformations.as_ptr() as *const u8,
                    transformations.len() * 64,
                )
            },
            usage: BufferUsages::UNIFORM,
        });

        #[repr(C)]
        #[derive(Debug)]
        struct Material {
            diffuse: [f32; 3],
            tex: i32,
            specular: [f32; 3],
            shininess: f32,
            refractive_index: f32,
            _a: [f32; 3],
        }
        let materials = sc
            .materials
            .iter()
            .map(|m| Material {
                diffuse: [m.diffuse_color.x, m.diffuse_color.y, m.diffuse_color.z],
                tex: -1,
                specular: m
                    .specular
                    .map(|a| [a.color.x, a.color.y, a.color.z])
                    .unwrap_or([0.0, 0.0, 0.0]),
                shininess: m.specular.map(|a| a.shininess).unwrap_or(0.0),
                refractive_index: m.refractive_index,
                _a: [0.0; 3],
            })
            .collect::<Vec<_>>();
        let b8 = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: unsafe {
                std::slice::from_raw_parts(
                    materials.as_ptr() as *const u8,
                    materials.len() * size_of::<Material>(),
                )
            },
            usage: BufferUsages::UNIFORM,
        });
        #[repr(C)]
        struct Light {
            dp: Vector3<f32>,
            falloff: f32,
            color: Vector3<f32>,
            dorp: u32,
        }
        let lights = sc
            .lights
            .iter()
            .map(|light| {
                use crate::scene::Light::*;
                match *light {
                    Directional {
                        direction,
                        color,
                        falloff,
                    } => Light {
                        dp: direction,
                        falloff,
                        color,
                        dorp: 0,
                    },
                    Point {
                        position,
                        color,
                        falloff,
                    } => Light {
                        dp: position,
                        falloff,
                        color,
                        dorp: 1,
                    },
                }
            })
            .collect::<Vec<_>>();
        let b9 = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: unsafe {
                std::slice::from_raw_parts(
                    lights.as_ptr() as *const u8,
                    lights.len() * size_of::<Light>(),
                )
            },
            usage: BufferUsages::UNIFORM,
        });

        let sampler = device.create_sampler(&SamplerDescriptor {
            ..Default::default()
        });

        let bg = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &bgl,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&target_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: b1.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: b2.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: b3.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: b4.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: b5.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: b6.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 7,
                    resource: b7.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 8,
                    resource: b8.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 9,
                    resource: b9.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 10,
                    resource: b10.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 11,
                    resource: b11.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 12,
                    resource: BindingResource::TextureViewArray(&textures_borrowed),
                },
                BindGroupEntry {
                    binding: 13,
                    resource: BindingResource::Sampler(&sampler),
                },
                BindGroupEntry {
                    binding: 14,
                    resource: BindingResource::TextureView(&cubemap),
                },
            ],
        });

        let pcs = PushConstants {
            bg_color: [
                sc.background.color.x,
                sc.background.color.y,
                sc.background.color.z,
                1.0,
            ],
            screensize: [width as f32, height as f32],
            sphere_cts: spheres.len() as u32,
            plane_cts: planes.len() as u32,
            mesh_cts: meshes.len() as u32,
            light_cts: lights.len() as u32,
            max_bounces,
            has_cubemap: sc.background.cube_map.is_some() as u32,
            ambient_light: [
                sc.background.ambient_light.x,
                sc.background.ambient_light.y,
                sc.background.ambient_light.z,
                1.0,
            ],
        };

        Self {
            _instance: instance,
            device,
            queue,
            _shader: shader,
            pipeline,
            target,
            bg,
            pcs,
            width,
            height,
        }
    }

    pub fn render(&self) {
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: None });
        {
            let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor { label: None });
            cpass.set_pipeline(&self.pipeline);
            cpass.set_push_constants(0, unsafe {
                std::slice::from_raw_parts(
                    &self.pcs as *const PushConstants as *const u8,
                    size_of::<PushConstants>(),
                )
            });
            cpass.set_bind_group(0, &self.bg, &[]);
            // workgroup size is 16x16x1
            cpass.dispatch_workgroups(self.width / 16, self.height / 16, 1);
        }
        self.queue.submit(Some(encoder.finish()));
    }

    pub fn export_image<W>(&self, writer: W)
    where
        W: std::io::Write,
    {
        let out_buf = self.device.create_buffer(&BufferDescriptor {
            label: None,
            size: (self.width * self.height * 4) as u64,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: None });
        encoder.copy_texture_to_buffer(
            ImageCopyTexture {
                texture: &self.target,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            ImageCopyBuffer {
                buffer: &out_buf,
                layout: ImageDataLayout {
                    offset: 0,
                    bytes_per_row: (self.width * 4).try_into().ok(),
                    rows_per_image: (self.height).try_into().ok(),
                },
            },
            Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(encoder.finish()));
        let bf = out_buf.slice(..);
        bf.map_async(MapMode::Read, |e| e.unwrap());
        assert!(self.device.poll(Maintain::Wait));
        let contents = bf.get_mapped_range();

        let mut encoder = png::Encoder::new(writer, self.width, self.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&*contents).unwrap();

        drop(contents);
        out_buf.unmap();
    }
}

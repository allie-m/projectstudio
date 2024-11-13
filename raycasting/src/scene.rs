use std::{
    fs::{read_dir, read_to_string},
    path::PathBuf,
};

use nalgebra::{Matrix4, Point3, Vector3};

pub fn load_scene(mut path_to_scene: PathBuf) -> SceneConfig {
    let contents = read_to_string(&path_to_scene).unwrap();
    let mut perspective = PerspectiveCamera::default();
    let mut lights = vec![];
    let mut background = Background::default();
    let mut materials = vec![];
    let mut objects: Vec<RenderObject> = vec![];

    let contents = contents.replace("}", "\n}");
    let contents = contents.replace("{", "{\n");
    let contents = contents.replace("  ", " ");

    path_to_scene.pop();

    let mut mode = arrayvec::ArrayVec::<&str, 32>::new();

    let mut current_material: usize = 0;
    let mut current_transform: Option<Transform> = None;
    for line in contents.lines() {
        let line = line.trim();

        if line.is_empty() || line.starts_with("//") {
            continue;
        }

        match line.split(" ").next().unwrap().trim() {
            a @ ("PerspectiveCamera" | "Lights" | "Materials" | "Background" | "Group") => {
                mode.push(a);
                continue;
            }
            _ => {}
        }

        if line.starts_with("}") {
            mode.pop();
            continue;
        }

        match mode[0] {
            "PerspectiveCamera" => {
                let mut parts = line.split(" ");
                match parts.next().unwrap() {
                    "center" => {
                        perspective.center = Point3::new(
                            parts.next().unwrap().parse().unwrap(),
                            parts.next().unwrap().parse().unwrap(),
                            parts.next().unwrap().parse().unwrap(),
                        );
                    }
                    "direction" => {
                        perspective.direction = Vector3::new(
                            parts.next().unwrap().parse().unwrap(),
                            parts.next().unwrap().parse().unwrap(),
                            parts.next().unwrap().parse().unwrap(),
                        );
                    }
                    "up" => {
                        perspective.up = Vector3::new(
                            parts.next().unwrap().parse().unwrap(),
                            parts.next().unwrap().parse().unwrap(),
                            parts.next().unwrap().parse().unwrap(),
                        );
                    }
                    "angle" => {
                        perspective.angle = parts.next().unwrap().parse().unwrap();
                    }
                    _ => unimplemented!(),
                }
            }
            "Lights" => {
                let mut parts = line.split(" ");
                match parts.next().unwrap() {
                    a @ ("DirectionalLight" | "PointLight") => {
                        mode.push(a);
                        match a {
                            "DirectionalLight" => lights.push(Light::Directional {
                                direction: Vector3::new(0.0, 0.0, 0.0),
                                color: Vector3::new(1.0, 1.0, 1.0),
                                falloff: 0.0,
                            }),
                            "PointLight" => lights.push(Light::Point {
                                position: Vector3::new(0.0, 0.0, 0.0),
                                color: Vector3::new(1.0, 1.0, 1.0),
                                falloff: 0.0,
                            }),
                            _ => unreachable!(),
                        }
                    }
                    "position" => {
                        let l = lights.last_mut().unwrap();
                        match l {
                            Light::Point { position, .. } => {
                                *position = Vector3::new(
                                    parts.next().unwrap().parse().unwrap(),
                                    parts.next().unwrap().parse().unwrap(),
                                    parts.next().unwrap().parse().unwrap(),
                                )
                            }
                            _ => unimplemented!(),
                        }
                    }
                    "direction" => {
                        let l = lights.last_mut().unwrap();
                        match l {
                            Light::Directional { direction, .. } => {
                                *direction = Vector3::new(
                                    parts.next().unwrap().parse().unwrap(),
                                    parts.next().unwrap().parse().unwrap(),
                                    parts.next().unwrap().parse().unwrap(),
                                )
                            }
                            _ => unimplemented!(),
                        }
                    }
                    "color" => {
                        let l = lights.last_mut().unwrap();
                        match l {
                            Light::Directional { color, .. } => {
                                *color = Vector3::new(
                                    parts.next().unwrap().parse().unwrap(),
                                    parts.next().unwrap().parse().unwrap(),
                                    parts.next().unwrap().parse().unwrap(),
                                )
                            }
                            Light::Point { color, .. } => {
                                *color = Vector3::new(
                                    parts.next().unwrap().parse().unwrap(),
                                    parts.next().unwrap().parse().unwrap(),
                                    parts.next().unwrap().parse().unwrap(),
                                )
                            }
                        }
                    }
                    "falloff" => {
                        let l = lights.last_mut().unwrap();
                        match l {
                            Light::Directional { falloff, .. } => {
                                *falloff = parts.next().unwrap().parse().unwrap();
                            }
                            Light::Point { falloff, .. } => {
                                *falloff = parts.next().unwrap().parse().unwrap();
                            }
                        }
                        unimplemented!() // until i implement falloff in the shader leave this like this
                    }
                    "numLights" => {}
                    _ => unimplemented!(),
                }
            }
            "Background" => {
                let mut parts = line.split(" ");
                match parts.next().unwrap() {
                    "color" => {
                        background.color = Vector3::new(
                            parts.next().unwrap().parse().unwrap(),
                            parts.next().unwrap().parse().unwrap(),
                            parts.next().unwrap().parse().unwrap(),
                        )
                    }
                    "ambientLight" => {
                        background.ambient_light = Vector3::new(
                            parts.next().unwrap().parse().unwrap(),
                            parts.next().unwrap().parse().unwrap(),
                            parts.next().unwrap().parse().unwrap(),
                        )
                    }
                    "cubeMap" => {
                        let path = parts.next().unwrap();
                        background.cube_map = Some(CubeMap {
                            size: (0, 0),
                            front: vec![],
                            back: vec![],
                            down: vec![],
                            up: vec![],
                            left: vec![],
                            right: vec![],
                        });
                        for item in read_dir(path_to_scene.join(path)).unwrap() {
                            if let Ok(item) = item {
                                let p = item.path();
                                if p.extension().unwrap().to_str().unwrap() != "bmp" {
                                    continue;
                                }
                                let tex = image::open(item.path()).unwrap();
                                background.cube_map.as_mut().unwrap().size =
                                    (tex.width(), tex.height());
                                let contents = tex.into_rgba32f().to_vec();
                                *match item.file_name().to_str().unwrap() {
                                    "back.bmp" => &mut background.cube_map.as_mut().unwrap().back,
                                    "down.bmp" => &mut background.cube_map.as_mut().unwrap().down,
                                    "front.bmp" => &mut background.cube_map.as_mut().unwrap().front,
                                    "left.bmp" => &mut background.cube_map.as_mut().unwrap().left,
                                    "right.bmp" => &mut background.cube_map.as_mut().unwrap().right,
                                    "up.bmp" => &mut background.cube_map.as_mut().unwrap().up,
                                    _ => continue,
                                } = contents;
                            }
                        }
                    }
                    _ => unimplemented!(),
                }
            }
            "Materials" => {
                let mut parts = line.split(" ");
                match parts.next().unwrap() {
                    a @ ("PhongMaterial" | "Material") => {
                        mode.push(a);
                        materials.push(Material::default())
                    }
                    "refractionIndex" => {
                        materials.last_mut().unwrap().refractive_index =
                            parts.next().unwrap().parse().unwrap();
                    }
                    "diffuseColor" => {
                        materials.last_mut().unwrap().diffuse_color = Vector3::new(
                            parts.next().unwrap().parse().unwrap(),
                            parts.next().unwrap().parse().unwrap(),
                            parts.next().unwrap().parse().unwrap(),
                        )
                    }
                    a @ ("specularColor" | "shininess") => {
                        if let None = materials.last().unwrap().specular {
                            materials.last_mut().unwrap().specular = Some(Specular::default());
                        }
                        let spec = materials.last_mut().unwrap().specular.as_mut().unwrap();

                        match a {
                            "specularColor" => {
                                spec.color = Vector3::new(
                                    parts.next().unwrap().parse().unwrap(),
                                    parts.next().unwrap().parse().unwrap(),
                                    parts.next().unwrap().parse().unwrap(),
                                )
                            }
                            "shininess" => spec.shininess = parts.next().unwrap().parse().unwrap(),
                            _ => unreachable!(),
                        }
                    }
                    "texture" => {
                        materials.last_mut().unwrap().texture =
                            Some(parts.next().unwrap().to_string());
                    }
                    "numMaterials" => {}
                    "Noise" | "color" | "octaves" | "frequency" | "amplitude" => {} // TODO
                    _ => unimplemented!(),
                }
            }
            "Group" => {
                let mut parts = line.split(" ");
                match parts.next().unwrap() {
                    a @ ("MaterialIndex" | "Transform" | "Sphere" | "Plane" | "TriangleMesh") => {
                        mode.push(a);

                        match a {
                            "MaterialIndex" => {
                                current_material = parts.next().unwrap().parse().unwrap();
                            }
                            "Transform" => {
                                current_transform = Some(Transform::default());
                            }
                            a @ ("Sphere" | "Plane" | "TriangleMesh") => {
                                objects.push(RenderObject {
                                    material: current_material,
                                    kind: match a {
                                        "Sphere" => ROKind::Sphere {
                                            center: Vector3::new(0.0, 0.0, 0.0),
                                            radius: 0.0,
                                        },
                                        "Plane" => ROKind::Plane {
                                            normal: Vector3::new(0.0, 1.0, 0.0),
                                            offset: 0.0,
                                        },
                                        "TriangleMesh" => ROKind::Mesh {
                                            obj_file: String::new(),
                                        },
                                        _ => unreachable!(),
                                    },
                                    transform: current_transform,
                                })
                            }
                            _ => unreachable!(),
                        }
                    }
                    "numObjects" => {}
                    other => {
                        if let Some(transform) = current_transform.as_mut() {
                            match other {
                                "Translate" => {
                                    transform.translation = Vector3::new(
                                        parts.next().unwrap().parse().unwrap(),
                                        parts.next().unwrap().parse().unwrap(),
                                        parts.next().unwrap().parse().unwrap(),
                                    );
                                    continue;
                                }
                                "XRotate" => {
                                    transform.rotation.x = parts.next().unwrap().parse().unwrap();
                                    transform.rotation.x = transform.rotation.x.to_radians();
                                    continue;
                                }
                                "YRotate" => {
                                    transform.rotation.y = parts.next().unwrap().parse().unwrap();
                                    transform.rotation.y = transform.rotation.y.to_radians();
                                    continue;
                                }
                                "ZRotate" => {
                                    transform.rotation.z = parts.next().unwrap().parse().unwrap();
                                    transform.rotation.z = transform.rotation.z.to_radians();
                                    continue;
                                }
                                "Scale" => {
                                    transform.scale = Vector3::new(
                                        parts.next().unwrap().parse().unwrap(),
                                        parts.next().unwrap().parse().unwrap(),
                                        parts.next().unwrap().parse().unwrap(),
                                    );
                                    continue;
                                }
                                _ => {}
                            }
                        }

                        if let ROKind::Sphere { center, radius } =
                            &mut objects.last_mut().unwrap().kind
                        {
                            match other {
                                "center" => {
                                    *center = Vector3::new(
                                        parts.next().unwrap().parse().unwrap(),
                                        parts.next().unwrap().parse().unwrap(),
                                        parts.next().unwrap().parse().unwrap(),
                                    )
                                }
                                "radius" => *radius = parts.next().unwrap().parse().unwrap(),
                                _ => unimplemented!(),
                            }
                        }

                        if let ROKind::Plane { normal, offset } =
                            &mut objects.last_mut().unwrap().kind
                        {
                            match other {
                                "normal" => {
                                    *normal = Vector3::new(
                                        parts.next().unwrap().parse().unwrap(),
                                        parts.next().unwrap().parse().unwrap(),
                                        parts.next().unwrap().parse().unwrap(),
                                    )
                                }
                                "offset" => *offset = parts.next().unwrap().parse().unwrap(),
                                _ => unimplemented!(),
                            }
                        }

                        if let ROKind::Mesh { obj_file } = &mut objects.last_mut().unwrap().kind {
                            match other {
                                "obj_file" => *obj_file = parts.next().unwrap().to_string(),
                                _ => unimplemented!(),
                            }
                        }
                    }
                }
            }
            _ => unimplemented!(),
        }
    }

    SceneConfig {
        perspective,
        lights,
        background,
        materials,
        objects,
        path_to_scene,
    }
}

#[derive(Debug)]
pub struct SceneConfig {
    pub perspective: PerspectiveCamera,
    pub lights: Vec<Light>,
    pub background: Background,
    pub materials: Vec<Material>,
    pub objects: Vec<RenderObject>,
    pub path_to_scene: PathBuf,
}

impl SceneConfig {
    pub fn num_of_textures(&self) -> usize {
        self.materials
            .iter()
            .filter(|a| a.texture.is_some())
            .count()
    }
}

#[derive(Debug)]
pub struct PerspectiveCamera {
    pub center: Point3<f32>,
    pub direction: Vector3<f32>,
    pub up: Vector3<f32>,
    pub angle: f32,
}

impl Default for PerspectiveCamera {
    fn default() -> Self {
        Self {
            center: Point3::new(0.0, 0.0, 5.0),
            direction: Vector3::new(0.0, 0.0, -1.0),
            up: Vector3::new(0.0, 1.0, 0.0),
            angle: 0.0,
        }
    }
}

#[derive(Debug)]
pub enum Light {
    Directional {
        direction: Vector3<f32>,
        color: Vector3<f32>,
        falloff: f32,
    },
    Point {
        position: Vector3<f32>,
        color: Vector3<f32>,
        falloff: f32,
    },
}

#[derive(Debug)]
pub struct Background {
    pub color: Vector3<f32>,
    pub ambient_light: Vector3<f32>,
    pub cube_map: Option<CubeMap>,
}

impl Default for Background {
    fn default() -> Self {
        Self {
            color: Vector3::new(0.2, 0.2, 0.2),
            ambient_light: Vector3::new(0.0, 0.0, 0.0),
            cube_map: None,
        }
    }
}

#[derive(Debug)]
pub struct Material {
    pub diffuse_color: Vector3<f32>,
    pub specular: Option<Specular>,
    pub texture: Option<String>,
    pub refractive_index: f32,
}

impl Default for Material {
    fn default() -> Self {
        Self {
            diffuse_color: Vector3::new(1.0, 1.0, 1.0),
            specular: None,
            texture: None,
            refractive_index: 0.0,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Specular {
    pub color: Vector3<f32>,
    pub shininess: f32,
}

impl Default for Specular {
    fn default() -> Self {
        Self {
            color: Vector3::new(1.0, 1.0, 1.0),
            shininess: 0.0,
        }
    }
}

#[derive(Debug)]
pub struct RenderObject {
    pub material: usize,
    pub kind: ROKind,
    pub transform: Option<Transform>,
}

#[derive(Copy, Clone, Debug)]
pub struct Transform {
    pub translation: Vector3<f32>,
    pub rotation: Vector3<f32>,
    pub scale: Vector3<f32>,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Vector3::new(0.0, 0.0, 0.0),
            rotation: Vector3::new(0.0, 0.0, 0.0),
            scale: Vector3::new(1.0, 1.0, 1.0),
        }
    }
}

pub fn transform_index(trans: Option<Transform>, list: &mut Vec<Matrix4<f32>>, inv: bool) -> u32 {
    trans.map(|t| t.index(list, inv)).unwrap_or(0)
}

impl Transform {
    fn index(&self, list: &mut Vec<Matrix4<f32>>, inv: bool) -> u32 {
        let mat = Matrix4::new_translation(&self.translation)
            * Matrix4::from_euler_angles(self.rotation.x, self.rotation.y, self.rotation.z)
            * Matrix4::new_nonuniform_scaling(&self.scale);
        let mat = if inv { mat.try_inverse().unwrap() } else { mat };
        list.iter().position(|a| *a == mat).unwrap_or_else(|| {
            list.push(mat);
            list.len() - 1
        }) as u32
    }
}

#[derive(Debug)]
pub enum ROKind {
    Sphere { center: Vector3<f32>, radius: f32 },
    Plane { normal: Vector3<f32>, offset: f32 },
    Mesh { obj_file: String },
}

#[derive(Debug)]
pub struct CubeMap {
    pub size: (u32, u32),
    pub front: Vec<f32>,
    pub back: Vec<f32>,
    pub down: Vec<f32>,
    pub up: Vec<f32>,
    pub left: Vec<f32>,
    pub right: Vec<f32>,
}

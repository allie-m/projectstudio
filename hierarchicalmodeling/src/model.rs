use nalgebra::{Matrix4, UnitQuaternion, Vector3};
use std::path::Path;
use std::rc::{Rc, Weak};
use std::sync::Mutex;

// ok so I'm throwing away most of their approach to organizing this
// it's based pretty completely on OpenGL 2-isms and is wildly inefficient;
// instead of regenerating the vertices every frame and reuploading them
// we'll upload all the vertices and normals at the START into a long-lasting immutable buffer
// (normals will be computed at model loading time cause they DON'T GIVE US THEM!!)
// and then upload all the matrices into a mutable uniform buffer
// each joint will have an index used to index into the matrix array
// these matrices will be cumulative so each matrix includes all its parents transformations
// the stack will be regenerated whenever a matrix changes but the joints and vertices unaltered;
// so still kinda a CPUside matrix stack

// pretty inefficient recursive function
// good enough for our models' bone complexities
pub fn generate_matrices(joint: &Joint) -> Vec<(Matrix4<f32>, usize)> {
    if joint.children.is_empty() {
        vec![(joint.transform, joint.idx)]
    } else {
        // vec![joint.transform]
        let mut v: Vec<_> = joint
            .children
            .iter()
            .map(|c| generate_matrices(&c.lock().unwrap()))
            .flatten()
            .map(|(m, i)| (m * joint.transform, i))
            .collect();
        v.push((joint.transform, joint.idx));
        v.sort_by(|(_, i), (_, j)| i.cmp(&j));
        v
    }
}

pub struct Animation {
    pub start_position: Vector3<f32>,
    pub end_position: Vector3<f32>,
    pub start_rotation: UnitQuaternion<f32>,
    pub end_rotation: UnitQuaternion<f32>,
    pub progress: f32,
}

impl Animation {
    fn frame(&self) -> (Vector3<f32>, UnitQuaternion<f32>) {
        (
            self.start_position + (self.end_position - self.start_position) * self.progress,
            self.start_rotation.nlerp(&self.end_rotation, self.progress),
        )
    }
}

#[derive(Clone, Debug)]
pub struct Joint {
    // is MUTABLE
    // initialized to the identity matrix
    // will be modified by any animation
    pub transform: Matrix4<f32>,
    // owns its children;
    // each rc is guaranteed to be unique
    // TODO: find a way to turn this into not using Rcs
    pub children: Vec<Rc<Mutex<Joint>>>,
    pub abs_pos: (f32, f32, f32),
    idx: usize,
}

impl Joint {
    pub fn apply_animation(&mut self, animation: &Animation) {
        let inv_bind = Matrix4::new_translation(&Vector3::new(
            self.abs_pos.0,
            self.abs_pos.1,
            self.abs_pos.2,
        ))
        .try_inverse()
        .unwrap();
        let (position, rotation) = animation.frame();
        let rot = *rotation.to_rotation_matrix().matrix();
        let rot = Matrix4::new(
            rot.m11, rot.m12, rot.m13, 0.0, rot.m21, rot.m22, rot.m23, 0.0, rot.m31, rot.m32,
            rot.m33, 0.0, 0.0, 0.0, 0.0, 1.0,
        );
        let pose = Matrix4::new_translation(&position) * rot;
        self.transform = pose * inv_bind;
    }

    // pub fn mat_to(&self, other: (f32, f32, f32), rotation: (f32, f32, f32)) -> Matrix4<f32> {
    //     let inv_bind = Matrix4::new_translation(&Vector3::new(
    //         self.abs_pos.0,
    //         self.abs_pos.1,
    //         self.abs_pos.2,
    //     ))
    //     .try_inverse()
    //     .unwrap();
    //     let pose = Matrix4::new_translation(&Vector3::new(other.0, other.1, other.2))
    //         * Matrix4::from_euler_angles(rotation.0, rotation.1, rotation.2);
    //     pose * inv_bind
    // }
}

// the const value here is arbitrary
// i pick 3 because i don't want to deal with 18 values that are mostly 0
// to make it 100% clear to my future self, MAX_WEIGHTS
// probably can't be greater than 4 without great inconvenience
// and the value of 3 for MAX_WEIGHTS is hardcoded in the shading and render pipeline creation
// so like, don't change this value lightly
const MAX_WEIGHTS: usize = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normals: [f32; 3],
    // can't pass integers in a vertex buffer; have to use not-flat stuff
    pub joints: [f32; MAX_WEIGHTS],
    pub weights: [f32; MAX_WEIGHTS],
}

#[derive(Debug)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

#[derive(Debug)]
pub struct SkeletalModel {
    pub root: Joint,
    pub mesh: Option<Mesh>,
}

// afaik both attach and skel are hand-rolled formats
// thanks Eugene
// and it's only specified in the assignment pdf, fun
pub fn skel_model(skel: &str, obj: Option<(&Path, &str)>) -> SkeletalModel {
    let iter = skel.lines().map(|a| a.split(" "));
    let mut joints: Vec<Weak<Mutex<Joint>>> = vec![];
    let mut root = None;
    for (idx, mut item) in iter.enumerate() {
        let rel_x: f32 = item.next().unwrap().parse().unwrap();
        let rel_y: f32 = item.next().unwrap().parse().unwrap();
        let rel_z: f32 = item.next().unwrap().parse().unwrap();
        let parent_id: i32 = item.next().unwrap().parse().unwrap();
        let jt = Rc::new(Mutex::new(Joint {
            transform: Matrix4::identity(),
            children: vec![],
            abs_pos: (rel_x, rel_y, rel_z),
            idx,
        }));
        if parent_id > -1 {
            let par = joints[parent_id as usize].upgrade().unwrap();
            let mut par = par.lock().unwrap();
            let mut l = jt.lock().unwrap();
            l.abs_pos.0 += par.abs_pos.0;
            l.abs_pos.1 += par.abs_pos.1;
            l.abs_pos.2 += par.abs_pos.2;
            drop(l);
            par.children.push(jt.clone())
        } else {
            root = Some(jt.clone())
        }
        joints.push(Rc::downgrade(&jt))
    }
    let mesh = obj.map(|(path, attach)| {
        // could load directly from a bufreader
        // but like why do that when i'm reading from a file anyways
        let models = tobj::load_obj(path, &tobj::GPU_LOAD_OPTIONS).unwrap().0;
        let model = models.into_iter().next().unwrap();
        let items = {
            attach
                .lines()
                .map(|v| {
                    let mut w = v
                        .trim()
                        .split(" ")
                        .enumerate()
                        // add 1 because the weight applied to the root is assumed to be 0
                        .map(|(idx, a)| (idx as i32 + 1, a.parse::<f32>().unwrap()))
                        .collect::<Vec<_>>();
                    w.sort_by(|(_, a), (_, b)| b.partial_cmp(&a).unwrap());
                    let (joints, weights): (Vec<i32>, Vec<f32>) = w.iter().cloned().unzip();
                    (
                        <[i32; MAX_WEIGHTS]>::try_from(&joints[..MAX_WEIGHTS]).unwrap(),
                        <[f32; MAX_WEIGHTS]>::try_from(&weights[..MAX_WEIGHTS]).unwrap(),
                    )
                })
                .collect::<Vec<_>>()
        };
        // the models here... don't have normals
        // why they felt the need to not give us them
        // i do know but like, it's not a good reason
        let mut mesh = Mesh {
            vertices: model
                .mesh
                .positions
                .chunks(3)
                .enumerate()
                .map(|(index, pos)| Vertex {
                    position: [pos[0], pos[1], pos[2]],
                    normals: [0.0, 0.0, 0.0],
                    // can't pass integers in a vertex buffer; have to use not-flat stuff
                    joints: items[index].0.map(|a| a as f32),
                    weights: items[index].1,
                })
                .collect(),
            indices: model.mesh.indices,
        };
        for triangle in mesh.indices.chunks(3) {
            let v1: Vector3<f32> = mesh.vertices[triangle[0] as usize].position.into();
            let v2: Vector3<f32> = mesh.vertices[triangle[1] as usize].position.into();
            let v3: Vector3<f32> = mesh.vertices[triangle[2] as usize].position.into();
            let v = (v2 - v1).cross(&(v3 - v1));
            mesh.vertices[triangle[0] as usize].normals[0] += v.x;
            mesh.vertices[triangle[0] as usize].normals[1] += v.y;
            mesh.vertices[triangle[0] as usize].normals[2] += v.z;
            mesh.vertices[triangle[1] as usize].normals[0] += v.x;
            mesh.vertices[triangle[1] as usize].normals[1] += v.y;
            mesh.vertices[triangle[1] as usize].normals[2] += v.z;
            mesh.vertices[triangle[2] as usize].normals[0] += v.x;
            mesh.vertices[triangle[2] as usize].normals[1] += v.y;
            mesh.vertices[triangle[2] as usize].normals[2] += v.z;
        }
        for v in mesh.vertices.iter_mut() {
            let normals: Vector3<f32> = v.normals.into();
            let normals = normals.normalize();
            v.normals = normals.into();
        }
        mesh
    });
    drop(joints);
    SkeletalModel {
        root: Rc::try_unwrap(root.unwrap()).unwrap().into_inner().unwrap(),
        mesh,
    }
}

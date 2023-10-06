// ok, so, the assignment instructs that the position and normal (& texture) coords
// of each vertex be extracted separately and the face's multiple indices for each
// specified vertex index their respective array of position and normal (& texture);
// this is a terrible terrible way to do it but since this was written before
// OpenGL 3 it was the most ergonomic way then
// I will process objs differently, use faces to define vertices which have a
// unique combo of position and normal (& texture) that are indexed with a unified
// index buffer, as has been standard for like >10 years
// I will also use f32s and u32s cause those are standard for this kind of thing

use std::collections::HashMap;

#[derive(Debug, PartialEq, PartialOrd)]
pub struct Model {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

// no texcoords cause the assignment only wants position and normals;
// should be raw-ly passable to the shader
#[derive(Debug, PartialEq, PartialOrd)]
#[repr(C)]
pub struct Vertex {
    position: [f32; 3],
    normals: [f32; 3],
}

// not particularly efficient
pub fn process(obj: &str) -> Model {
    let mut vertices = vec![];
    let mut indices = vec![];

    let a = |a: &str| -> Option<[f32; 3]> {
        let mut i = a.split(" ");
        i.next();
        Some([
            i.next()?.parse().ok()?,
            i.next()?.parse().ok()?,
            i.next()?.parse().ok()?,
        ])
    };
    let ps = obj
        .lines()
        .filter(|line| line.starts_with("v "))
        .map(|f| a(f).expect(f))
        .collect::<Vec<_>>();
    let ns = obj
        .lines()
        .filter(|line| line.starts_with("vn"))
        .map(|f| a(f).expect(f))
        .collect::<Vec<_>>();

    let mut definedindices = HashMap::<[u32; 2], u32>::new();
    for vtx in obj
        .lines()
        .filter(|line| line.starts_with("f"))
        .map(|a| a.split(" "))
        .flatten()
        .filter(|a| !a.starts_with("f"))
    {
        let mut parts = vtx.split("/");
        let p = parts.next().expect(vtx).parse::<u32>().expect(vtx) - 1;
        parts.next().expect(vtx);
        let n = parts.next().expect(vtx).parse::<u32>().expect(vtx) - 1;
        let x = [p, n];
        match definedindices.get(&x) {
            Some(item) => indices.push(*item),
            None => {
                definedindices.insert(x, vertices.len() as u32);
                indices.push(vertices.len() as u32);
                vertices.push(Vertex {
                    position: ps[p as usize],
                    normals: ns[n as usize],
                });
            }
        }
    }

    Model { vertices, indices }
}

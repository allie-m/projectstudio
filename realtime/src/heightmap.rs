// use std::{fs::File, path::PathBuf, time::Instant};
use std::path::PathBuf;

// use nalgebra::Vector3;
// use png::Decoder;

use nalgebra::Vector3;
use rayon::prelude::*;

pub struct TriangulatedHeightmap {
    pub normals: Vec<[f32; 4]>,
    pub meshes: Vec<TriangulatedMesh>,
    pub index: usize,
    pub pos: (u32, u32),
}

pub struct TriangulatedMesh {
    pub vertices: Vec<[[f32; 3]; 2]>,
    pub indices: Vec<u32>,
}

pub const CHUNK_SIZE: u32 = 512;
const LOD_LAYERS: u32 = 3;

pub fn get_heightmap(
    path: PathBuf,
    x_start: u32,
    x_end: u32,
    z_start: u32,
    z_end: u32,
) -> Vec<TriangulatedHeightmap> {
    // let b = Instant::now();

    // let decoder = Decoder::new(File::open(&path.join("tiled").with_extension("png")).unwrap());
    // let mut reader = decoder.read_info().unwrap();
    // let mut buf = vec![0; reader.output_buffer_size()];
    // let info = reader.next_frame(&mut buf).unwrap();

    // log::info!("Took {:?} to load the heightmap png", b.elapsed());
    // let b = Instant::now();
    // let heights = buf.iter().map(|&b| b as f32).collect::<Vec<_>>();
    // let get_height = |x, z| heights[(x + z * info.width) as usize];
    // log::info!("Took {:?} to convert the heightmap format", b.elapsed());

    (x_start..x_end)
        .into_par_iter()
        .map(move |x_i| {
            let path = path.clone();
            (z_start..z_end).into_par_iter().map(move |z_i| {
                let path = path.clone();
                let x_offset = x_i * CHUNK_SIZE;
                let z_offset = z_i * CHUNK_SIZE;
                let mut meshes = (0..LOD_LAYERS)
                    .map(|i| {
                        let mesh = tobj::load_obj(
                            path.join(&format!("tiled_{}_{}_lod_{}", x_offset, z_offset, i))
                                .with_extension("obj"),
                            &tobj::GPU_LOAD_OPTIONS,
                        )
                        .unwrap()
                        .0
                        .into_iter()
                        .next()
                        .unwrap()
                        .mesh;
                        TriangulatedMesh {
                            vertices: mesh
                                .positions
                                .chunks(3)
                                .map(|a| {
                                    [
                                        [
                                            a[0] + 6.0 - (x_start * CHUNK_SIZE) as f32,
                                            a[1],
                                            -a[2] + 6.0 - (z_start * CHUNK_SIZE) as f32,
                                        ],
                                        [0.0, 0.0, 0.0],
                                    ]
                                })
                                .collect(),
                            indices: mesh.indices,
                        }
                    })
                    .collect::<Vec<_>>();
                meshes.par_iter_mut().for_each(|mesh| {
                    let mut flagged = vec![];
                    for (idx_idx, index) in mesh.indices.chunks(3).enumerate() {
                        // let v = &mut mesh.vertices[*index as usize];
                        let v1: Vector3<f32> = mesh.vertices[index[0] as usize][0].into();
                        let v2: Vector3<f32> = mesh.vertices[index[1] as usize][0].into();
                        let v3: Vector3<f32> = mesh.vertices[index[2] as usize][0].into();
                        let v = (v2 - v1).cross(&(v3 - v1));
                        if v.normalize().y < 0.1 {
                            log::debug!(
                                "Removing for being too horizontal: {:?}, {:?}, {:?}, {:?}",
                                v1,
                                v2,
                                v3,
                                v.normalize()
                            );
                            flagged.push(idx_idx);
                            continue;
                        }
                        mesh.vertices[index[0] as usize][1][0] += v.x;
                        mesh.vertices[index[0] as usize][1][1] += v.y;
                        mesh.vertices[index[0] as usize][1][2] += v.z;
                        mesh.vertices[index[1] as usize][1][0] += v.x;
                        mesh.vertices[index[1] as usize][1][1] += v.y;
                        mesh.vertices[index[1] as usize][1][2] += v.z;
                        mesh.vertices[index[2] as usize][1][0] += v.x;
                        mesh.vertices[index[2] as usize][1][1] += v.y;
                        mesh.vertices[index[2] as usize][1][2] += v.z;
                    }
                    for idx_idx in flagged.iter() {
                        mesh.indices[idx_idx * 3] = mesh.indices[0];
                        mesh.indices[idx_idx * 3 + 1] = mesh.indices[1];
                        mesh.indices[idx_idx * 3 + 2] = mesh.indices[2];
                    }
                    for vtx in 0..mesh.vertices.len() {
                        let normals: Vector3<f32> = mesh.vertices[vtx][1].into();
                        mesh.vertices[vtx][1] = normals.normalize().into();
                    }
                });
                let normals = (0..CHUNK_SIZE * CHUNK_SIZE)
                    .into_par_iter()
                    .map(|_i| {
                        [0.0, 1.0, 0.0, 1.0]
                        // let real_x = i % CHUNK_SIZE + x_offset;
                        // let real_z = i / CHUNK_SIZE + z_offset;

                        // // https://stackoverflow.com/a/40540028
                        // let a = if real_x as u32 != 0 {
                        //     Vector3::new(
                        //         0.0,
                        //         get_height(real_x as u32 - 1, real_z as u32),
                        //         -1.0,
                        //     )
                        // } else {
                        //     Vector3::zeros()
                        // };
                        // let b = if real_x as u32 != info.width - 1 {
                        //     Vector3::new(0.0, get_height(real_x as u32 + 1, real_z as u32), 1.0)
                        // } else {
                        //     Vector3::zeros()
                        // };
                        // let c = if real_z as u32 != 0 {
                        //     Vector3::new(
                        //         -1.0,
                        //         get_height(real_x as u32, real_z as u32 - 1),
                        //         0.0,
                        //     )
                        // } else {
                        //     Vector3::zeros()
                        // };
                        // let d = if real_z as u32 != info.height - 1 {
                        //     Vector3::new(1.0, get_height(real_x as u32, real_z as u32 + 1), 0.0)
                        // } else {
                        //     Vector3::zeros()
                        // };
                        // let out: [f32; 3] = (b - a).cross(&(d - c)).normalize().into();
                        // [out[0], out[1], out[2], 1.0]
                    })
                    .collect();
                log::info!(
                    "Loaded/calculated the meshes and normals for chunk {:?}, {:?}",
                    x_offset,
                    z_offset,
                );
                TriangulatedHeightmap {
                    normals,
                    meshes,
                    index: 0, // to be set in main.rs; TODO figure out why the below equation does not work
                    // index: (x_i * info.height / CHUNK_SIZE + z_i) as usize,
                    pos: (x_i - x_start, z_i - z_start),
                }
            })
        })
        .flatten()
        .collect::<Vec<_>>()
}

// // initialize the 4 corners of the mesh
// // find the point with the highest error (interpolated height vs actual height)
// // add it
// // repeat until every point has less error than the acceptable threshold

// fn naive_triangulator(
//     lod: f32,
//     chunk_size: u32,
//     xoffset: u32,
//     zoffset: u32,
//     heightmap: &Heightmap,
// ) -> TriangulatedMesh {
//     use spade::Triangulation;

//     let then = Instant::now();

//     let mut triangulation: spade::DelaunayTriangulation<spade::Point2<f32>> =
//         spade::DelaunayTriangulation::new();
//     triangulation
//         .insert(spade::Point2::new(xoffset as f32, zoffset as f32))
//         .unwrap();
//     triangulation
//         .insert(spade::Point2::new(
//             (xoffset + chunk_size - 1) as f32,
//             zoffset as f32,
//         ))
//         .unwrap();
//     triangulation
//         .insert(spade::Point2::new(
//             xoffset as f32,
//             (zoffset + chunk_size - 1) as f32,
//         ))
//         .unwrap();
//     triangulation
//         .insert(spade::Point2::new(
//             (xoffset + chunk_size - 1) as f32,
//             (zoffset + chunk_size - 1) as f32,
//         ))
//         .unwrap();
//     // println!("{:?}", triangulation.vertices().map(|v| *v.data()).collect::<Vec<_>>());

//     let mut flag = false;
//     while !flag {
//         flag = true;
//         let mut pos = None;
//         let mut reqd_edges = None;
//         let mut worst_error = (lod, 0.0, 0.0);
//         for i in 0..chunk_size * chunk_size {
//             let x = i % chunk_size;
//             let z = i / chunk_size;
//             let pt = spade::Point2::new((x + xoffset) as f32, (z + zoffset) as f32);

//             let (interpolated_h, edges) = match triangulation.locate(pt) {
//                 spade::PositionInTriangulation::OnFace(face) => {
//                     let face = triangulation.face(face);
//                     let [a, b, c] = face.barycentric_interpolation(pt);
//                     let [(j, q), (k, r), (l, s)] = face.vertices().map(|v| {
//                         let d = *v.data();
//                         (heightmap.get_height(d.x as u32, d.y as u32), d)
//                     });
//                     (a * j + b * k + c * l, (q, r, s))
//                 }
//                 spade::PositionInTriangulation::OnEdge(edge) => {
//                     let [(v1, h1), (v2, h2)] =
//                         triangulation.directed_edge(edge).vertices().map(|v| {
//                             let d = *v.data();
//                             (d, heightmap.get_height(d.x as u32, d.y as u32))
//                         });
//                     let d = v1.distance_2(v2);
//                     // println!("{:?}, {:?}, {:?}, {:?}, {:?}", d, v1.distance_2(pt), v2.distance_2(pt), h1, h2);
//                     (
//                         (1.0 - v1.distance_2(pt) / d) * h1 + (1.0 - v2.distance_2(pt) / d) * h2,
//                         (v1, v2, spade::Point2::new(-1.0, -1.0)),
//                     )
//                 }
//                 spade::PositionInTriangulation::OnVertex(_) => continue,
//                 spade::PositionInTriangulation::OutsideOfConvexHull(_edge) => {
//                     log::info!("Outside of convex hull; {:?} -- {:?}, {:?}", i, x, z);
//                     panic!()
//                 }
//                 other => panic!("Should not be reached; {:?}", other),
//             };
//             let err = (interpolated_h - heightmap.get_height(pt.x as u32, pt.y as u32)).abs();
//             if err > worst_error.0 {
//                 worst_error = (
//                     err,
//                     interpolated_h,
//                     heightmap.get_height(pt.x as u32, pt.y as u32),
//                 );
//                 flag = false;
//                 pos = Some(pt);
//                 reqd_edges = Some(edges);
//             }
//         }
//         if let Some(pos) = pos {
//             let e = reqd_edges.take().unwrap();
//             log::info!("{:?} at {:?}, neighbors {:?}", worst_error, pos, e);
//             triangulation.insert(pos).unwrap();
//             // if e.0 != spade::Point2::new(-1.0, -1.0) {
//             //     triangulation.add_constraint_edge(pos, e.0).unwrap();
//             // }
//             // if e.1 != spade::Point2::new(-1.0, -1.0) {
//             //     triangulation.add_constraint_edge(pos, e.1).unwrap();
//             // }
//             // if e.2 != spade::Point2::new(-1.0, -1.0) {
//             //     triangulation.add_constraint_edge(pos, e.2).unwrap();
//             // }
//         }
//     }

//     log::info!(
//         "Took {:?} to triangulate chunk {:?} LOD {:?}",
//         then,
//         (xoffset, zoffset),
//         lod
//     );
//     let then = Instant::now();
//     let mut vertex_cache = HashMap::new();
//     let mut vertices = vec![];
//     let mut indices = vec![];
//     for face in triangulation.inner_faces() {
//         let vs = face.vertices().map(|v| {
//             let data = *v.data();
//             (
//                 [
//                     data.x,
//                     heightmap.get_height(data.x as u32, data.y as u32),
//                     data.y,
//                 ],
//                 v.fix(),
//             )
//         });
//         for (v, key) in vs {
//             if vertex_cache.contains_key(&key) {
//                 indices.push(*vertex_cache.get(&key).unwrap())
//             } else {
//                 vertex_cache.insert(key, vertices.len() as u32);
//                 indices.push(vertices.len() as u32);
//                 vertices.push(v);
//             }
//         }
//     }
//     log::info!(
//         "Took {:?} to create vertices and indices for LOD {:?}",
//         then.elapsed(),
//         lod,
//     );

//     TriangulatedMesh { vertices, indices }
//     // TriangulatedMesh {
//     //     vertices: vec![
//     //         [
//     //             xoffset as f32,
//     //             heightmap.get_height(xoffset, zoffset),
//     //             zoffset as f32,
//     //         ],
//     //         [
//     //             xoffset as f32,
//     //             heightmap.get_height(xoffset, zoffset + chunk_size - 1),
//     //             (zoffset + chunk_size - 1) as f32,
//     //         ],
//     //         [
//     //             (xoffset + chunk_size) as f32,
//     //             heightmap.get_height(xoffset + chunk_size - 1, zoffset),
//     //             zoffset as f32,
//     //         ],
//     //         [
//     //             (xoffset + chunk_size) as f32,
//     //             heightmap.get_height(xoffset + chunk_size - 1, zoffset + chunk_size - 1),
//     //             (zoffset + chunk_size) as f32,
//     //         ],
//     //     ],
//     //     indices: vec![0, 1, 3, 0, 2, 3],
//     // }
// }

// impl Heightmap {
//     fn normals(&self, chunk_size: u32, x_offset: u32, z_offset: u32) -> Vec<[f32; 4]> {
//         (0..chunk_size * chunk_size)
//             .into_par_iter()
//             .map(|i| {
//                 let real_x = i % chunk_size + x_offset;
//                 let real_z = i / chunk_size + z_offset;

//                 // https://stackoverflow.com/a/40540028
//                 let a = if real_x as u32 != 0 {
//                     Vector3::new(0.0, self.get_height(real_x as u32 - 1, real_z as u32), -1.0)
//                 } else {
//                     Vector3::zeros()
//                 };
//                 let b = if real_x as u32 != self.width - 1 {
//                     Vector3::new(0.0, self.get_height(real_x as u32 + 1, real_z as u32), 1.0)
//                 } else {
//                     Vector3::zeros()
//                 };
//                 let c = if real_z as u32 != 0 {
//                     Vector3::new(-1.0, self.get_height(real_x as u32, real_z as u32 - 1), 0.0)
//                 } else {
//                     Vector3::zeros()
//                 };
//                 let d = if real_z as u32 != self.length - 1 {
//                     Vector3::new(1.0, self.get_height(real_x as u32, real_z as u32 + 1), 0.0)
//                 } else {
//                     Vector3::zeros()
//                 };
//                 let out: [f32; 3] = (b - a).cross(&(d - c)).normalize().into();
//                 [out[0], out[1], out[2], 1.0]
//             })
//             .collect()
//     }

//     pub fn triangulate(&self, lods: &[f32], chunk_size: u32) -> Vec<TriangulatedHeightmap> {
//         (0..self.width / chunk_size)
//             .into_par_iter()
//             .map(|xoffset| {
//                 (0..self.length / chunk_size)
//                     .into_par_iter()
//                     .map(move |zoffset| TriangulatedHeightmap {
//                         normals: self.normals(
//                             chunk_size,
//                             xoffset * chunk_size,
//                             zoffset * chunk_size,
//                         ),
//                         meshes: lods
//                             .into_par_iter()
//                             .map(|lod| {
//                                 naive_triangulator(
//                                     *lod,
//                                     chunk_size,
//                                     xoffset * chunk_size,
//                                     zoffset * chunk_size,
//                                     &self,
//                                 )
//                             })
//                             .collect(),
//                         index: (xoffset * (self.length / chunk_size) + zoffset) as usize,
//                         pos: (xoffset, zoffset),
//                     })
//             })
//             .flatten()
//             .collect()
//         // use spade::Triangulation;
//         // let then = Instant::now();
//         // let pruned = (0..self.width / chunk_size)
//         //     .into_par_iter()
//         //     .map(|x_offset| {
//         //         (0..self.length / chunk_size)
//         //             .into_par_iter()
//         //             .map(|z_offset| {
//         //                 let x_offset = x_offset * chunk_size;
//         //                 let z_offset = z_offset * chunk_size;
//         //                 (
//         //                     lods.into_par_iter()
//         //                         .map(|&error_threshold| {
//         //                             (0..chunk_size * chunk_size)
//         //                                 .into_par_iter()
//         //                                 .filter_map(move |i| {
//         //                                     let local_x = i % chunk_size;
//         //                                     let local_z = i / chunk_size;
//         //                                     let real_x = i % chunk_size + x_offset;
//         //                                     let real_z = i / chunk_size + z_offset;
//         //                                     let a = if local_x == 0 {
//         //                                         f32::MAX
//         //                                     } else {
//         //                                         self.get_height(real_x - 1, real_z)
//         //                                     };
//         //                                     let b = if local_x == chunk_size - 1 {
//         //                                         f32::MAX
//         //                                     } else {
//         //                                         self.get_height(real_x + 1, real_z)
//         //                                     };
//         //                                     let c = if local_z == 0 {
//         //                                         f32::MAX
//         //                                     } else {
//         //                                         self.get_height(real_x, real_z - 1)
//         //                                     };
//         //                                     let d = if local_z == chunk_size - 1 {
//         //                                         f32::MAX
//         //                                     } else {
//         //                                         self.get_height(real_x, real_z + 1)
//         //                                     };
//         //                                     let h = self.get_height(real_x, real_z);
//         //                                     let avg = (a + b + c + d) / 4.0;
//         //                                     ((h - avg).abs() >= error_threshold).then(|| {
//         //                                         spade::Point2::new(real_x as f32, real_z as f32)
//         //                                     })
//         //                                 })
//         //                                 .collect::<Vec<_>>()
//         //                         })
//         //                         .collect::<Vec<_>>(),
//         //                     (0..chunk_size * chunk_size)
//         //                         .into_par_iter()
//         //                         .map(|i| {
//         //                             let real_x = i % chunk_size + x_offset;
//         //                             let real_z = i / chunk_size + z_offset;

//         //                             // https://stackoverflow.com/a/40540028
//         //                             let a = if real_x as u32 != 0 {
//         //                                 Vector3::new(
//         //                                     0.0,
//         //                                     self.get_height(real_x as u32 - 1, real_z as u32),
//         //                                     -1.0,
//         //                                 )
//         //                             } else {
//         //                                 Vector3::zeros()
//         //                             };
//         //                             let b = if real_x as u32 != self.width - 1 {
//         //                                 Vector3::new(
//         //                                     0.0,
//         //                                     self.get_height(real_x as u32 + 1, real_z as u32),
//         //                                     1.0,
//         //                                 )
//         //                             } else {
//         //                                 Vector3::zeros()
//         //                             };
//         //                             let c = if real_z as u32 != 0 {
//         //                                 Vector3::new(
//         //                                     -1.0,
//         //                                     self.get_height(real_x as u32, real_z as u32 - 1),
//         //                                     0.0,
//         //                                 )
//         //                             } else {
//         //                                 Vector3::zeros()
//         //                             };
//         //                             let d = if real_z as u32 != self.length - 1 {
//         //                                 Vector3::new(
//         //                                     1.0,
//         //                                     self.get_height(real_x as u32, real_z as u32 + 1),
//         //                                     0.0,
//         //                                 )
//         //                             } else {
//         //                                 Vector3::zeros()
//         //                             };
//         //                             let out: [f32; 3] = (b - a).cross(&(d - c)).normalize().into();
//         //                             [out[0], out[1], out[2], 1.0]
//         //                         })
//         //                         .collect(),
//         //                 )
//         //             })
//         //             .collect::<Vec<_>>()
//         //     })
//         //     .flatten()
//         //     .collect::<Vec<_>>();
//         // // let pruned = (0..self.width * self.length).into_par_iter().filter_map(|i| {
//         // //     let a = if i % self.width == 0 { f32::MAX } else { self.heights[i as usize - 1] as f32 };
//         // //     let b = if i % self.width == self.width - 1 { f32::MAX } else { self.heights[i as usize + 1] as f32 };
//         // //     let c = if i / self.width == 0 { f32::MAX } else { self.heights[i as usize - self.width as usize] as f32 };
//         // //     let d = if i / self.width == self.length - 1 { f32::MAX } else { self.heights[i as usize + self.width as usize] as f32 };
//         // //     let avg = a + b + c + d / 4.0;
//         // //     ((self.heights[i as usize] as f32 - avg).abs() >= error_threshold).then(|| spade::Point2::new((i % self.width) as f32, (i / self.width) as f32))
//         // // }).collect::<Vec<_>>();
//         // log::info!(
//         //     "Took {:?} to prune the heightmap and generate the normals",
//         //     then.elapsed()
//         // );
//         // let then = Instant::now();
//         // pruned
//         //     .into_par_iter()
//         //     .enumerate()
//         //     .map(|(index, (pruned, normals))| {
//         //         let meshes = pruned
//         //             .into_par_iter()
//         //             .enumerate()
//         //             .map(|(lod, pruned)| {
//         //                 let triangulation =
//         //                     spade::DelaunayTriangulation::<spade::Point2<f32>>::bulk_load(pruned)
//         //                         .unwrap();
//         //                 log::info!(
//         //                     "Took {:?} to triangulate the heightmap for LOD {:?} for chunk {:?}",
//         //                     then.elapsed(),
//         //                     lods[lod],
//         //                     index
//         //                 );
//         //                 let then = Instant::now();
//         //                 let mut vertex_cache = HashMap::new();
//         //                 let mut vertices = vec![];
//         //                 let mut indices = vec![];
//         //                 for face in triangulation.inner_faces() {
//         //                     let vs = face.vertices().map(|v| {
//         //                         let data = *v.data();
//         //                         (
//         //                             [
//         //                                 data.x,
//         //                                 self.heights[data.x as usize
//         //                                     + data.y as usize * self.width as usize]
//         //                                     as f32,
//         //                                 data.y,
//         //                             ],
//         //                             v.fix(),
//         //                         )
//         //                     });
//         //                     for (v, key) in vs {
//         //                         if vertex_cache.contains_key(&key) {
//         //                             indices.push(*vertex_cache.get(&key).unwrap())
//         //                         } else {
//         //                             vertex_cache.insert(key, vertices.len() as u32);
//         //                             indices.push(vertices.len() as u32);
//         //                             vertices.push(v);
//         //                         }
//         //                     }
//         //                 }
//         //                 log::info!(
//         //                     "Took {:?} to create vertices and indices for LOD {:?} for chunk {:?}",
//         //                     then.elapsed(),
//         //                     lods[lod],
//         //                     index,
//         //                 );

//         //                 TriangulatedMesh { vertices, indices }
//         //             })
//         //             .collect::<Vec<_>>();
//         //         TriangulatedHeightmap {
//         //             normals,
//         //             meshes,
//         //             index,
//         //             pos: (
//         //                 index as u32 % (self.width / chunk_size),
//         //                 index as u32 / (self.width / chunk_size),
//         //             ),
//         //         }
//         //     })
//         //     .collect()
//     }

//     fn get_height(&self, x: u32, z: u32) -> f32 {
//         self.heights[(x + z * self.width) as usize]
//     }
// }

// pub fn get_heightmap(override_cache: bool, loc: PathBuf) -> Heightmap {
//     if !loc.with_extension("cache").exists() || override_cache {
//         let hm = process_heightmap(&loc);
//         let b = Instant::now();
//         cache_raw_heightmap(&hm, loc.with_extension("cache"));
//         log::info!("Took {:?} to cache the raw heightmap", b.elapsed());
//         hm
//     } else {
//         let b = Instant::now();
//         let hm = load_raw_heightmap(loc.with_extension("cache"));
//         log::info!("Took {:?} to load the raw heightmap", b.elapsed());
//         hm
//     }
// }

// pub struct Heightmap {
//     // [[f32; width]; height]
//     pub heights: Vec<f32>,
//     pub width: u32,
//     pub length: u32,
// }

// pub fn cache_raw_heightmap(heightmap: &Heightmap, loc: PathBuf) {
//     let mut file = OpenOptions::new()
//         .create(true)
//         .write(true)
//         .open(loc)
//         .unwrap();
//     file.write(&heightmap.width.to_ne_bytes()).unwrap();
//     file.write(&heightmap.length.to_ne_bytes()).unwrap();
//     // TODO: endianness
//     file.write_all(unsafe {
//         std::slice::from_raw_parts(
//             heightmap.heights.as_ptr() as *const u8,
//             heightmap.heights.len() * 4,
//         )
//     })
//     .unwrap();
// }

// pub fn load_raw_heightmap(loc: PathBuf) -> Heightmap {
//     let contents = std::fs::read(loc).unwrap();
//     let width = u32::from_ne_bytes(contents[0..4].try_into().unwrap());
//     let length = u32::from_ne_bytes(contents[4..8].try_into().unwrap());
//     let ptr = contents[8..].as_ptr() as *const f32;
//     // TODO: endianness
//     let heights = unsafe { std::slice::from_raw_parts(ptr, contents[8..].len() / 4) }.to_vec();
//     Heightmap {
//         heights,
//         width,
//         length,
//     }
// }

// // #[derive(Default)]
// // struct HeightmapSpec {
// //     min: f32,
// //     max: f32,
// //     width: u32,
// //     length: u32,
// // }

// fn process_heightmap(name: &PathBuf) -> Heightmap {
//     // let spec = name.with_extension("txt");

//     // let spec = heightmap_spec(&read_to_string(spec).unwrap());

//     let b = Instant::now();

//     let decoder = Decoder::new(File::open(&name).unwrap());
//     let mut reader = decoder.read_info().unwrap();
//     let mut buf = vec![0; reader.output_buffer_size()];
//     let info = reader.next_frame(&mut buf).unwrap();

//     log::info!("Took {:?} to load the heightmap png", b.elapsed());
//     let b = Instant::now();
//     let heights = buf.iter().map(|&b| b as f32).collect();
//     log::info!("Took {:?} to convert the heightmap format", b.elapsed());

//     Heightmap {
//         heights,
//         width: info.width,
//         length: info.height,
//     }
// }

// // fn convert_to_raw(spec: HeightmapSpec, (pixels, info): (Vec<u8>, OutputInfo)) -> Heightmap {
// //     let xfactor = info.width as f32 / spec.width as f32;
// //     let yfactor = (spec.max - spec.min) as f32 / 255.0;
// //     let zfactor = info.height as f32 / spec.length as f32;
// //     let heights = (0..spec.length * spec.width)
// //         .into_par_iter()
// //         .map(|i| {
// //             let x = i % spec.width;
// //             let z = i / spec.width;
// //             // TODO: interpolation
// //             let x = (x as f32 * xfactor) as u32;
// //             let z = (z as f32 * zfactor) as u32;
// //             let p = pixels[(z * info.width * 4 + x * 4) as usize] as f32 * yfactor;
// //             p
// //         })
// //         .collect();
// //     Heightmap {
// //         heights,
// //         width: spec.width,
// //         length: spec.length,
// //     }
// // }

// // fn heightmap_spec(spec: &str) -> HeightmapSpec {
// //     let mut s = HeightmapSpec::default();
// //     for l in spec.lines().filter(|a| !a.is_empty()) {
// //         let l = l.trim();
// //         if l.starts_with("//") {
// //             continue;
// //         }
// //         let mut parts = l.split(" ");
// //         match parts.next().unwrap() {
// //             "min" => s.min = parts.next().unwrap().parse().unwrap(),
// //             "max" => s.max = parts.next().unwrap().parse().unwrap(),
// //             "width" => s.width = parts.next().unwrap().parse().unwrap(),
// //             "length" => s.length = parts.next().unwrap().parse().unwrap(),
// //             a => panic!("Invalid parameter {}", a),
// //         }
// //     }
// //     s
// // }

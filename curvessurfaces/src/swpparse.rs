// THE BELOW COMMENT IS COPY PASTED FROM parser.h
// WHICH IS A FILE GIVEN BY THE ASSIGNMENT AND SPECIFIES
// THE SWP FORMAT

use crate::render::Vertex;
use nalgebra::{Rotation3, Vector3};

// may be 2D
// in that case we discard the Z coordinate
// NOT a control point, but an interpolated one
#[derive(Debug)]
pub struct CurvePoint {
    // absolute
    pub position: Vector3<f32>,
    // unit
    pub tangent: Vector3<f32>,
    pub normal: Vector3<f32>,
    pub binormal: Vector3<f32>,
}

pub struct Curve {
    pub points: Vec<CurvePoint>,
    pub name: Option<String>,
    // would be an enum if the boilerplate justified it
    is_2d: bool,
}

pub struct SurfaceSpec {
    pub name: String,
    pub curve: usize,
    pub sweep: Sweep,
}

pub enum Sweep {
    // anonymous value is # of steps
    Axis(u32),
    Curve(usize),
}

pub type Model = (Vec<Vertex>, Vec<u32>);

pub fn triangulate_surface(spec: &SurfaceSpec, curves: &[Curve]) -> Model {
    let profile = &curves[spec.curve];
    let steps = match spec.sweep {
        Sweep::Axis(s) => s,
        Sweep::Curve(_) => profile.points.len() as u32,
    };

    let mut vertices = vec![];
    let mut indices = vec![];
    let n = match spec.sweep {
        Sweep::Axis(_) => &profile.points,
        Sweep::Curve(c) => &curves[c].points,
    };
    // println!("{:?}", n.iter().map(|a| (a.position.x, a.position.y)).collect::<Vec<_>>());
    for (i, cvpt) in n.iter().enumerate() {
        for iter in 0..steps {
            match spec.sweep {
                Sweep::Axis(_) => {
                    let prog = (iter as f32 / steps as f32) * std::f32::consts::TAU;
                    let rot = Rotation3::from_axis_angle(&Vector3::y_axis(), prog);
                    let npos = rot * cvpt.position;
                    // inverse transpose of a rotation matrix is literally just the matrix
                    // done here for clarity
                    let nnorm = rot.transpose().inverse() * cvpt.normal;
                    vertices.push(Vertex {
                        position: [npos.x, npos.y, npos.z],
                        normals: [nnorm.x, nnorm.y, nnorm.z],
                    });
                }
                Sweep::Curve(_) => {
                    let profpt = &profile.points[iter as usize];
                    let mat = Rotation3::face_towards(&cvpt.tangent, &cvpt.normal);
                    let npos = (mat * profpt.position) + cvpt.position;
                    let nnorm = mat * profpt.normal;
                    vertices.push(Vertex {
                        position: [npos.x, npos.y, npos.z],
                        normals: [nnorm.x, nnorm.y, nnorm.z],
                    });
                }
            }
        }
        // the last set of curve points shouldn't have stuff
        if i < n.len() - 1 {
            let offset = i as u32 * steps;
            for index in 0..steps - 1 {
                // triangle 1
                indices.push(index + offset);
                indices.push(index + steps + offset);
                indices.push(index + steps + 1 + offset);
                // triangle 2
                indices.push(index + 1 + offset);
                indices.push(index + offset);
                indices.push(index + steps + 1 + offset);
            }
            // wrap around
            indices.push(0 + offset);
            indices.push(steps - 1 + offset);
            indices.push(steps + offset);
            indices.push(steps - 1 + offset);
            indices.push(steps + offset);
            indices.push(steps * 2 - 1 + offset);
        }
    }

    (vertices, indices)
}

pub fn parse(contents: &str) -> (Vec<Curve>, Vec<SurfaceSpec>) {
    let mut iter = contents.split_whitespace();
    let mut curves = vec![];
    let mut surfaces = vec![];
    loop {
        let token = iter.next();
        if token.is_none() {
            break;
        }
        let token = token.unwrap();

        match token {
            "circ" => {
                let name = match iter.next().unwrap() {
                    "." => None,
                    other => Some(other.to_string()),
                };
                let steps: u32 = iter.next().unwrap().parse().unwrap();
                let radius: f32 = iter.next().unwrap().parse().unwrap();
                let mut points = vec![];
                for iter in 0..steps {
                    let prog = (iter as f32 / steps as f32) * std::f32::consts::TAU;
                    let position = Vector3::new(prog.cos() * radius, prog.sin() * radius, 0.0);
                    let tangent = Vector3::new(-prog.sin(), prog.cos(), 0.0);
                    let normal = Vector3::new(prog.cos(), prog.sin(), 0.0);
                    let binormal = tangent.cross(&normal);
                    points.push(CurvePoint {
                        position,
                        tangent,
                        normal,
                        binormal,
                    })
                }
                // add the first point again
                points.push(CurvePoint {
                    position: Vector3::new(radius, 0.0, 0.0),
                    tangent: Vector3::new(0.0, 1.0, 0.0),
                    normal: Vector3::new(1.0, 0.0, 0.0),
                    binormal: Vector3::new(0.0, 0.0, -1.0),
                });
                curves.push(Curve {
                    points,
                    name,
                    is_2d: true,
                });
            }
            "srev" => {
                let name = iter.next().unwrap().to_string();
                let steps: u32 = iter.next().unwrap().parse().unwrap();
                let profile = iter.next().unwrap();
                surfaces.push(SurfaceSpec {
                    name,
                    curve: curves
                        .iter()
                        .enumerate()
                        .find(|(_, Curve { name, is_2d, .. })| {
                            name.is_some() && name.as_ref().unwrap() == profile && *is_2d
                        })
                        .expect("Curve not found")
                        .0,
                    sweep: Sweep::Axis(steps),
                })
            }
            "gcyl" => {
                let name = iter.next().unwrap().to_string();
                let profile = iter.next().unwrap();
                let sweep = iter.next().unwrap();

                let (profile_index, _) = curves
                    .iter()
                    .enumerate()
                    .find(|(_, Curve { name, is_2d, .. })| {
                        name.is_some() && name.as_ref().unwrap() == profile && *is_2d
                    })
                    .expect("Profile curve not found");
                let (sweep_index, _) = curves
                    .iter()
                    .enumerate()
                    .find(|(_, Curve { name, .. })| {
                        name.is_some() && name.as_ref().unwrap() == sweep
                    })
                    .expect("Sweep curve not found");
                surfaces.push(SurfaceSpec {
                    name,
                    curve: profile_index,
                    sweep: Sweep::Curve(sweep_index),
                })
            }
            mode @ ("bez2" | "bez3" | "bsp2" | "bsp3") => {
                let name = iter.next().unwrap();
                let steps: u32 = iter.next().unwrap().parse().unwrap();
                let numpoints: u32 = iter.next().unwrap().parse().unwrap();
                let mut ctrlpoints = vec![];
                for _ in 0..numpoints {
                    let first = iter.next().unwrap()[1..].parse().unwrap();
                    let n = iter.next().unwrap();
                    let (second, third) = match &n[n.len() - 1..] {
                        "]" => (n[..n.len() - 1].parse().unwrap(), 0.0),
                        _ => (n.parse().unwrap(), {
                            let next = iter.next().unwrap();
                            next[..next.len() - 1].parse().unwrap()
                        }),
                    };
                    ctrlpoints.push([first, second, third]);
                }
                let mut points = Vec::<CurvePoint>::new();
                let mut poss = vec![];
                for cindex in 0..ctrlpoints.len() - 3 {
                    let c0: Vector3<f32> = ctrlpoints[cindex].into();
                    let c1: Vector3<f32> = ctrlpoints[cindex + 1].into();
                    let c2: Vector3<f32> = ctrlpoints[cindex + 2].into();
                    let c3: Vector3<f32> = ctrlpoints[cindex + 3].into();
                    for index in 0..steps {
                        let t = index as f32 / steps as f32;
                        let interpolated = match mode {
                            "bez2" | "bez3" => {
                                // Cubic bezier curve formula
                                // (1 - t)^3 P_0 +
                                // 3(1 - t)^2t P_1 +
                                // 3(1 - t)t^2 P_2 +
                                // t^3 P_3
                                (1.0 - t).powi(3) * c0
                                    + 3.0 * (1.0 - t).powi(2) * t * c1
                                    + 3.0 * (1.0 - t) * t * t * c2
                                    + t * t * t * c3
                            }
                            "bsp2" | "bsp3" => {
                                // Cubic B-spline formula
                                // thanks course lecture notes :)
                                // (1 - t)                 * 1/6 P_0
                                // (3t^3 - 6t^2 + 4)       * 1/6 P_1
                                // (-3t^3 + 3t^3 + 3t + 1) * 1/6 P_2
                                // (t^3)                   * 1/6 P_3
                                let s = 1.0 / 6.0;
                                s * (1.0 - t) * c0
                                    + s * (3.0 * t * t * t - 6.0 * t * t + 4.0) * c1
                                    + s * (-3.0 * t * t * t + 3.0 * t * t + 3.0 * t + 1.0) * c2
                                    + s * t * t * t * c3
                            }
                            _ => unreachable!(),
                        };
                        poss.push(interpolated);
                    }
                }
                for (index, pvec) in poss.iter().enumerate() {
                    let (ppvec, bbvec) = {
                        if index == 0 {
                            (
                                pvec - (poss[index + 1] - pvec),
                                // I'm told that the B(0) initialization is arbitrary
                                // but that it cannot be parallel to T(0)
                                // hence I take T(0) and use a vector that isn't it
                                {
                                    let par = (poss[index + 1] - pvec).normalize();
                                    Vector3::new(par.x, -par.y, par.z).normalize()
                                },
                            )
                        } else {
                            let p = &points[points.len() - 1];
                            (p.position, p.binormal)
                        }
                    };
                    // q'(t) (normalized)
                    let tvec: Vector3<f32> = (pvec - ppvec).normalize();
                    // T' (normalized)
                    let nvec: Vector3<f32> = bbvec.cross(&tvec).normalize();
                    // T' * q'(t) (both normalized then normalized again)
                    let bvec: Vector3<f32> = tvec.cross(&nvec).normalize();

                    points.push(CurvePoint {
                        position: *pvec,
                        tangent: tvec,
                        normal: nvec,
                        binormal: bvec,
                    })
                }
                curves.push(Curve {
                    points,
                    name: match name {
                        "." => None,
                        other => Some(other.to_string()),
                    },
                    is_2d: mode.ends_with("2"),
                })
            }
            _ => {
                //
            }
        }
    }
    (curves, surfaces)
}

// thanks Eugene

/* This function implements a parser for the "SWP" file format.  It's
   something Eugene came up with specifically for this assigment.

   A SWP file allows you to describe spline curves and swept surfaces.
   To specify a curve, you use the following syntax:

   TYPE NAME STEPS NUMPOINTS
   [ CONTROLPOINT ]
   [ CONTROLPOINT ]
   ...

   ---CURVES---

   TYPE can be Bez2, Bez3, Bsp2, Bsp3 which specify Bezier/Bspline
   curves in 2/3 dimensions.

   NAME is just a term that can be used later to refer to the curve.
   You can create an anonymous curve by giving '.' (period) as the
   name.

   STEPS controls how finely the curve is discretized.  Specifically,
   each cubic piece (not the whole curve) will be discretized into
   STEPS segments.

   NUMPOINTS indicates the number of control points.

   Each CONTROLPOINT is given as [ x y ] for 2D curves, and [ x y z ]
   for 3D curves.  Note that the square braces are required.

   In addition to these curves, you can specify circles as follows:

   circ NAME STEPS RADIUS

   The variables are self-explanatory.

   ---SURFACES---

   Surfaces of revolution are defined as follows:

   srev NAME STEPS PROFILE

   PROFILE is the name of a curve that previously occurred in the
   file.  This name *must* refer to a 2D curve (an error will be
   returned if a 3D curve is provided).

   Finally, generalized cylinders are defined as follows:

   gcyl NAME PROFILE SWEEP

   As with surfaces of revolution, PROFILE is the name of a 2D curve.
   SWEEP is the name of a 2D *or* 3D curve.
*/

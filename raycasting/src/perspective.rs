use nalgebra::{Point3, Vector3};

use crate::scene::PerspectiveCamera;

#[repr(C)]
#[derive(Debug)]
pub struct Perspective {
    origin: Point3<f32>,
    _a: f32,
    horizontal: Vector3<f32>,
    _b: f32,
    up: Vector3<f32>,
    aspect: f32,
    direction: Vector3<f32>,
    angle: f32,
}

impl Perspective {
    pub fn generate(cam: &PerspectiveCamera, aspect: f32) -> Self {
        let angle = cam.angle.to_radians();

        // I spent so long
        // trying to implement this myself
        // without having taken linear algebra formally
        // or seen the lectures for this course
        // so I finally gave in and referenced
        // https://github.com/dj-lesson/mit-6.837/blob/master/Answers/assn-4/Camera.h
        let horizontal = cam.direction.cross(&cam.up).normalize();
        let up = horizontal.cross(&cam.direction).normalize();
        let direction = (0.5 / (angle / 2.0).tan()) * cam.direction;

        Self {
            origin: cam.center,
            _a: 0.0,
            horizontal,
            _b: 0.0,
            up,
            aspect,
            direction,
            angle,
        }
    }
}

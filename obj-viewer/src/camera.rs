use nalgebra::{Matrix4, Vector3};
use winit::event::{ElementState, VirtualKeyCode};

// basically all this code I copied from another project;
// very hasty/not good camera controller

pub struct Camera3D {
    pub position: Vector3<f32>,
    // roll then pitch then yaw
    pub rotation: Vector3<f32>,
}

impl Camera3D {
    pub fn matrix(&self) -> Matrix4<f32> {
        Matrix4::from_euler_angles(self.rotation.x, self.rotation.y, self.rotation.z)
            * Matrix4::new_translation(&-self.position)
    }
}

pub struct SimpleCamera3DController {
    wasd: [bool; 4],
    space: bool,
    lshift: bool,
    arrows: [bool; 4],
}

impl SimpleCamera3DController {
    pub fn create() -> Self {
        Self {
            wasd: [false; 4],
            space: false,
            lshift: false,
            arrows: [false; 4],
        }
    }

    pub fn input(&mut self, state: ElementState, code: VirtualKeyCode) {
        let c = match state {
            ElementState::Pressed => true,
            ElementState::Released => false,
        };
        match code {
            VirtualKeyCode::W => self.wasd[0] = c,
            VirtualKeyCode::A => self.wasd[1] = c,
            VirtualKeyCode::S => self.wasd[2] = c,
            VirtualKeyCode::D => self.wasd[3] = c,
            VirtualKeyCode::Up => self.arrows[0] = c,
            VirtualKeyCode::Down => self.arrows[1] = c,
            VirtualKeyCode::Left => self.arrows[2] = c,
            VirtualKeyCode::Right => self.arrows[3] = c,
            VirtualKeyCode::Space => self.space = c,
            VirtualKeyCode::LShift => self.lshift = c,
            _ => {}
        }
    }

    // THIS IS FPS TIED!!
    pub fn camera_update(&self, camera: &mut Camera3D) {
        if self.wasd[0] {
            camera.position.z -= camera.rotation.y.cos() * 0.1;
            camera.position.x += camera.rotation.y.sin() * 0.1;
        }
        if self.wasd[1] {
            camera.position.z -= camera.rotation.y.sin() * 0.1;
            camera.position.x -= camera.rotation.y.cos() * 0.1;
        }
        if self.wasd[2] {
            camera.position.z += camera.rotation.y.cos() * 0.1;
            camera.position.x -= camera.rotation.y.sin() * 0.1;
        }
        if self.wasd[3] {
            camera.position.z += camera.rotation.y.sin() * 0.1;
            camera.position.x += camera.rotation.y.cos() * 0.1;
        }

        camera.position.y += (self.space as u8 as f32) * 0.1;
        camera.position.y -= (self.lshift as u8 as f32) * 0.1;

        camera.rotation.y -= (self.arrows[2] as u8 as f32) * 0.023;
        camera.rotation.y += (self.arrows[3] as u8 as f32) * 0.023;
    }
}

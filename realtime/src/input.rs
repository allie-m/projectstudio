use nalgebra::{Matrix4, Rotation3, Vector3};
use winit::event::{ElementState, VirtualKeyCode};

pub struct Camera3D {
    pub position: Vector3<f32>,
    // roll then pitch then yaw
    pub rotation: Vector3<f32>,
    pub o_mat: Option<Matrix4<f32>>,
}

impl Camera3D {
    pub fn matrix(&mut self) -> Matrix4<f32> {
        if let None = self.o_mat {
            self.o_mat = Some(Matrix4::from_euler_angles(
                self.rotation.x,
                self.rotation.y,
                self.rotation.z,
            ));
        }
        self.o_mat.unwrap() * Matrix4::new_translation(&-self.position)
    }
}

pub trait CameraController {
    fn create() -> Self;
    fn input(&mut self, state: ElementState, code: VirtualKeyCode);
    fn camera_update(&mut self, camera: &mut Camera3D);
}

pub struct AutoCamera3DController {
    t: u64,
    zoom: f32,
    z_v: f32,
}

impl CameraController for AutoCamera3DController {
    fn create() -> Self {
        Self {
            t: 0,
            zoom: 1.0,
            z_v: 0.0,
        }
    }

    fn input(&mut self, state: ElementState, code: VirtualKeyCode) {
        match state {
            ElementState::Pressed => match code {
                VirtualKeyCode::Z => {
                    if self.z_v > -0.02 {
                        self.z_v -= 0.02
                    }
                }
                VirtualKeyCode::X => {
                    if self.z_v < 0.02 {
                        self.z_v += 0.02
                    }
                }
                _ => {}
            },
            ElementState::Released => match code {
                VirtualKeyCode::Z => {
                    if self.z_v <= -0.02 {
                        self.z_v += 0.02
                    }
                }
                VirtualKeyCode::X => {
                    if self.z_v >= 0.02 {
                        self.z_v -= 0.02
                    }
                }
                _ => {}
            },
        }
    }

    fn camera_update(&mut self, camera: &mut Camera3D) {
        if self.zoom < 0.1 && self.z_v < 0.0 {
            self.z_v = 0.0;
        }
        if self.zoom > 1.5 && self.z_v > 0.0 {
            self.z_v = 0.0;
        }

        self.zoom += self.z_v;
        self.t += 1;

        let angle = self.t as f32 / 200.0;

        // camera.rotation.x = 30.0f32.to_radians();

        camera.position.y = self.zoom * 2000.0 + 500.0;
        camera.position.x = angle.cos() * self.zoom * 2500.0 + 2500.0;
        camera.position.z = angle.sin() * self.zoom * 3000.0 + 3000.0;

        let d = (Vector3::new(512.0 * 5.0, 200.0, 512.0 * 6.0) - camera.position).normalize();
        // println!("{:?}", d);
        // let a = Vector2::new(0.0, 1.0).angle(&d);
        // println!("{}, {}", a, angle);
        // camera.rotation.y = angle - std::f32::consts::FRAC_PI_2;
        camera.o_mat = Some(Rotation3::look_at_rh(&d, &Vector3::y()).to_homogeneous());
    }
}

pub struct SimpleCamera3DController {
    wasd: [bool; 4],
    space: bool,
    lshift: bool,
    arrows: [bool; 4],
}

impl CameraController for SimpleCamera3DController {
    fn create() -> Self {
        Self {
            wasd: [false; 4],
            space: false,
            lshift: false,
            arrows: [false; 4],
        }
    }

    fn input(&mut self, state: ElementState, code: VirtualKeyCode) {
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
    fn camera_update(&mut self, camera: &mut Camera3D) {
        const SPEED: f32 = 5.0;
        if self.wasd[0] {
            camera.position.z -= camera.rotation.y.cos() * SPEED;
            camera.position.x += camera.rotation.y.sin() * SPEED;
        }
        if self.wasd[1] {
            camera.position.z -= camera.rotation.y.sin() * SPEED;
            camera.position.x -= camera.rotation.y.cos() * SPEED;
        }
        if self.wasd[2] {
            camera.position.z += camera.rotation.y.cos() * SPEED;
            camera.position.x -= camera.rotation.y.sin() * SPEED;
        }
        if self.wasd[3] {
            camera.position.z += camera.rotation.y.sin() * SPEED;
            camera.position.x += camera.rotation.y.cos() * SPEED;
        }

        camera.position.y += (self.space as u8 as f32) * SPEED;
        camera.position.y -= (self.lshift as u8 as f32) * SPEED;

        camera.rotation.y -= (self.arrows[2] as u8 as f32) * 0.023;
        camera.rotation.y += (self.arrows[3] as u8 as f32) * 0.023;
    }
}

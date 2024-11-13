use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use winit::{
    event::{Event, KeyboardInput, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod camera;
mod model;
mod render;

fn main() {
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Warn)
        .init()
        .unwrap();

    let mut args = std::env::args();
    let _ = args.next().unwrap();
    let modelpath = args.next().unwrap();
    let modelskin: bool = args
        .next()
        .map(|a| a.parse().unwrap_or(true))
        .unwrap_or(true);

    let skel = PathBuf::new().join(modelpath).with_extension("skel");
    let model = modelskin.then(|| {
        (
            skel.with_extension("obj"),
            std::fs::read_to_string(skel.with_extension("attach")).unwrap(),
        )
    });
    let skel = std::fs::read_to_string(&skel).unwrap();
    let skel = model::skel_model(
        &skel,
        model.as_ref().map(|(p, s)| (p.as_path(), s.as_str())),
    );

    // test transform
    let mut sign = -1.0;
    let (mut test_animation_1, mut test_animation_2) = {
        let j = skel.root.children[0].lock().unwrap();
        let ja = j.children[0].lock().unwrap();
        let jaa = ja.children[0].lock().unwrap();
        let jaaa = jaa.children[0].lock().unwrap();
        let s = model::Animation {
            start_position: nalgebra::Vector3::new(jaaa.abs_pos.0, jaaa.abs_pos.1, jaaa.abs_pos.2),
            end_position: nalgebra::Vector3::new(jaaa.abs_pos.0, jaaa.abs_pos.1, jaaa.abs_pos.2),
            start_rotation: Default::default(),
            end_rotation: nalgebra::UnitQuaternion::from_euler_angles(
                35.0f32.to_radians(),
                0.0,
                0.0,
            ),
            progress: 0.0,
        };
        let jaa = ja.children[1].lock().unwrap();
        let jaaa = jaa.children[0].lock().unwrap();
        let t = model::Animation {
            start_position: nalgebra::Vector3::new(jaaa.abs_pos.0, jaaa.abs_pos.1, jaaa.abs_pos.2),
            end_position: nalgebra::Vector3::new(jaaa.abs_pos.0, jaaa.abs_pos.1, jaaa.abs_pos.2),
            start_rotation: Default::default(),
            end_rotation: nalgebra::UnitQuaternion::from_euler_angles(
                -35.0f32.to_radians(),
                0.0,
                0.0,
            ),
            progress: 0.0,
        };
        (s, t)
    };

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Hierarchical Modeling")
        .with_resizable(true)
        .build(&event_loop)
        .unwrap();
    let mut cam = camera::Camera3D {
        position: nalgebra::Vector3::new(0.0, 0.0, 1.0),
        rotation: nalgebra::Vector3::new(0.0, 0.0, 0.0),
    };
    let mut controller = camera::SimpleCamera3DController::create();
    let mut render_state = render::RenderState::create(&window);

    let main_jts = render_state.new_jts(&skel);

    // 60 fps
    const DUR: Duration = Duration::from_micros(16667);
    event_loop.run(move |event, _eloop, control_flow| match event {
        Event::NewEvents(StartCause::Init) => {
            *control_flow = ControlFlow::WaitUntil(Instant::now() + DUR)
        }
        Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
            *control_flow = ControlFlow::WaitUntil(Instant::now() + DUR);

            // animation progress
            {
                test_animation_1.progress += 0.01 * sign;
                test_animation_2.progress += 0.01 * sign;
                if test_animation_1.progress.abs() >= 1.0 {
                    sign *= -1.0;
                }
                let j = skel.root.children[0].lock().unwrap();
                let ja = j.children[0].lock().unwrap();
                let jaa = ja.children[0].lock().unwrap();
                let mut jaaa = jaa.children[0].lock().unwrap();
                jaaa.apply_animation(&test_animation_1);
                let jaa = ja.children[1].lock().unwrap();
                let mut jaaa = jaa.children[0].lock().unwrap();
                jaaa.apply_animation(&test_animation_2);
            }

            let (mats, _): (Vec<_>, Vec<_>) =
                model::generate_matrices(&skel.root).into_iter().unzip();
            render_state.update_jts_staging(&main_jts, &mats);

            controller.camera_update(&mut cam);
            render_state.projview = render_state.proj * cam.matrix();
            render_state.render(&[&main_jts]);
        }
        Event::WindowEvent { event, .. } => match event {
            WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        state,
                        virtual_keycode,
                        ..
                    },
                ..
            } => {
                if let Some(keycode) = virtual_keycode {
                    controller.input(state, keycode);
                }
            }
            WindowEvent::Resized(size) => {
                log::debug!("Resizing to {:?}", size);
                render_state.resize(size.into());
            }
            _ => {}
        },
        Event::LoopDestroyed => {
            log::info!("Closing time!");
        }
        _ => {}
    })
}

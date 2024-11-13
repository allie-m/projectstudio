use std::io::Read;
use std::time::{Duration, Instant};
use winit::event::{Event, KeyboardInput, StartCause, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

mod camera;
mod render;
mod swpparse;

fn main() {
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Warn)
        .init()
        .unwrap();

    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    let (curves, surfaces) = swpparse::parse(&s);
    // println!("{:?}", curves[0].points.iter().map(|a| (a.position.x, a.position.y)).collect::<Vec<_>>());

    // look at the first surface for now
    let models = surfaces
        .iter()
        .map(|s| swpparse::triangulate_surface(s, &curves))
        .collect::<Vec<_>>();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Curves & Surfaces")
        .with_resizable(true)
        .build(&event_loop)
        .unwrap();
    let mut cam = camera::Camera3D {
        position: nalgebra::Vector3::new(0.0, 0.0, 10.0),
        rotation: nalgebra::Vector3::new(0.0, 0.0, 0.0),
    };
    let mut controller = camera::SimpleCamera3DController::create();
    let mut render_state = render::RenderState::create(&window, &models);

    // 60 fps
    const DUR: Duration = Duration::from_micros(16667);
    event_loop.run(move |event, _eloop, control_flow| match event {
        Event::NewEvents(StartCause::Init) => {
            *control_flow = ControlFlow::WaitUntil(Instant::now() + DUR)
        }
        Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
            *control_flow = ControlFlow::WaitUntil(Instant::now() + DUR);
            controller.camera_update(&mut cam);
            render_state.projview = render_state.proj * cam.matrix();
            render_state.render();
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

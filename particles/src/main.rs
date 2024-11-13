use log::LevelFilter;
use std::time::{Duration, Instant};
use winit::event::{Event, KeyboardInput, StartCause, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

mod camera;
mod render;

// ok so
// the assignment's specifications don't make sense for me to follow
// like really at all
// so i'm going to freestyle this a bit
// instead of doing multiple cpuside implementations
// i think i'll do the cloth physics in a shader instead

fn main() {
    simple_logger::SimpleLogger::new()
        .with_level(LevelFilter::Warn)
        .init()
        .unwrap();

    let mut args = std::env::args();
    let _ = args.next().unwrap();
    let times = args.next().map(|a| a.parse().unwrap()).unwrap();

    let flag = args
        .next()
        .map(|a| match a.as_str() {
            "rainbow" => 0u32,
            "trans" => 1u32,
            _ => panic!(),
        })
        .unwrap();

    let frames: u32 = args.next().map(|a| a.parse().unwrap()).unwrap_or(2000);

    // nope we'll just use rk4 with a manual stepsize
    //
    // let solver = args.next().expect("Provide a solver");
    // let stepsize = args.next().expect("Provide a stepsize");

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Cloth Physics Simulation")
        .with_resizable(true)
        .build(&event_loop)
        .unwrap();
    let cam = camera::Camera3D {
        position: nalgebra::Vector3::new(64.0, 64.0, 150.0),
        rotation: nalgebra::Vector3::new(0.0, 0.0, 0.0),
    };
    let mut controller = camera::SimpleCamera3DController::create();
    let mut render_state = render::RenderState::create(&window);

    let mut t = 0;

    // 60 fps
    const DUR: Duration = Duration::from_micros(16667);
    // println!("{:?}", DUR);
    event_loop.run(move |event, _eloop, control_flow| match event {
        Event::NewEvents(StartCause::Init) => {
            *control_flow = ControlFlow::WaitUntil(Instant::now() + DUR)
        }
        Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
            *control_flow = ControlFlow::WaitUntil(Instant::now() + DUR);
            // controller.camera_update(&mut cam);
            render_state.projview = render_state.proj * cam.matrix();
            render_state.render(times, flag);
            t += 1;
            if t > frames {
                *control_flow = ControlFlow::Exit;
            }
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

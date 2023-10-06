use std::time::{Duration, Instant};

use winit::{
    event::{ElementState, Event, KeyboardInput, StartCause, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use crate::input::CameraController;

mod chunk;
mod heightmap;
mod input;
mod render;
mod water;

fn main() {
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .init()
        .unwrap();

    let mut args = std::env::args();
    let _ = args.next().unwrap();
    let heightmap_path = args
        .next()
        .unwrap_or_else(|| panic!("Random terrain generation is not implemented yet!"));

    let triangulated = heightmap::get_heightmap(heightmap_path.into(), 10, 20, 4, 16);
    let triangulated = triangulated
        .into_iter()
        .enumerate()
        .map(|(index, mut hm)| {
            hm.index = index;
            hm
        })
        .collect::<Vec<_>>();
    // let triangulated = hm.triangulate(&[70.0, 70.0], heightmap::CHUNK_SIZE); //&[0.01, 1.0], CHUNK_SIZE);

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Realtime Renderer")
        .with_resizable(true)
        .build(&event_loop)
        .unwrap();

    let mut camera = input::Camera3D {
        position: nalgebra::Vector3::new(0.0, 10.0, 0.0),
        rotation: nalgebra::Vector3::new(0.0, 0.0, 0.0),
        o_mat: None,
    };
    let mut controller = input::AutoCamera3DController::create();

    let mut render_state = render::RenderState::create(&window);
    let chunks = triangulated
        .iter()
        .map(|t| render_state.upload_heightmap(t))
        .collect::<Vec<_>>();
    let normal_map = render_state.normal_map(&triangulated, heightmap::CHUNK_SIZE);

    let mut lod = 2;

    // 60 fps
    const DUR: Duration = Duration::from_micros(16667);

    event_loop.run(move |event, _eloop, control_flow| match event {
        Event::NewEvents(StartCause::Init) => {
            *control_flow = ControlFlow::WaitUntil(Instant::now() + DUR)
        }
        // per-frame actions
        Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
            *control_flow = ControlFlow::WaitUntil(Instant::now() + DUR);
            controller.camera_update(&mut camera);
            render_state.update_view(camera.matrix());
            render_state.render(&chunks, &normal_map, heightmap::CHUNK_SIZE, lod);
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
                if let Some(code) = virtual_keycode {
                    controller.input(state, code);
                    if let ElementState::Pressed = state {
                        match code {
                            VirtualKeyCode::K => lod = (lod + 1).min(2),
                            VirtualKeyCode::L => lod = lod.checked_sub(1).unwrap_or(0),
                            _ => {}
                        }
                    }
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

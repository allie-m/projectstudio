use std::{path::PathBuf, str::FromStr};

mod gpu;
mod perspective;
mod scene;

fn main() {
    // simple_logger::SimpleLogger::new()
    //     .with_level(log::LevelFilter::Info)
    //     .init()
    //     .unwrap();

    let mut args = std::env::args();
    let _ = args.next().unwrap();

    let width: u32 = args.next().unwrap().parse().unwrap();
    let height: u32 = args.next().unwrap().parse().unwrap();
    let output: bool = true; //args.next().unwrap().parse().unwrap();
    let max_bounces: u32 = args.next().unwrap().parse().unwrap();

    // image copy alignment is 256
    // so gotta be in multiples of 256
    assert_eq!(width % 256, 0);
    assert_eq!(height % 256, 0);

    let scene_path = args.next().unwrap();

    let sc = scene::load_scene(PathBuf::from_str(&scene_path).unwrap());

    let gpu_state = gpu::GpuState::create(width, height, &sc, max_bounces);
    gpu_state.render();
    if output {
        gpu_state.export_image(std::io::stdout());
    }
}

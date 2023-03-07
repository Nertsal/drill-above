use geng::prelude::*;
use geng::Camera2d;

mod assets;
mod editor;
mod game;
mod intro;
mod menu;
mod model;
mod render;
mod ui;
mod util;

use assets::*;
use model::*;
use render::*;

const FPS: f64 = 60.0;

const PIXELS_PER_UNIT: usize = 16;
const SCREEN_RESOLUTION: vec2<usize> = vec2(480, 270);

#[derive(clap::Parser)]
struct Opt {
    #[clap(long)]
    editor: bool,
    #[clap(long)]
    room: Option<String>,
    /// Hot reload assets on change detection.
    #[clap(long)]
    hot_reload: bool,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(clap::Subcommand)]
enum Command {
    #[cfg(not(target_arch = "wasm32"))]
    TileSet(TileSetOpt),
    #[cfg(not(target_arch = "wasm32"))]
    ChangeSize(ChangeSizeOpt),
    #[cfg(not(target_arch = "wasm32"))]
    RenameTile(RenameTileOpt),
    #[cfg(not(target_arch = "wasm32"))]
    Format,
}

#[derive(clap::Args)]
struct TileSetOpt {
    tileset: String,
    size: String,
}

#[derive(clap::Args)]
struct ChangeSizeOpt {
    size: String,
}

#[derive(clap::Args)]
struct RenameTileOpt {
    from: String,
    to: String,
}

fn main() {
    logger::init().unwrap();
    geng::setup_panic_handler();
    let opt: Opt = program_args::parse();

    let geng = Geng::new_with(geng::ContextOptions {
        title: "Drill above".to_string(),
        fixed_delta_time: 1.0 / FPS,
        ..Default::default()
    });

    if let Some(command) = opt.command {
        match command {
            #[cfg(not(target_arch = "wasm32"))]
            Command::TileSet(opt) => {
                let size = parse_size(&opt.size).expect("Failed to parse size");
                let state = {
                    let future = {
                        async move {
                            let path = run_dir().join(opt.tileset);
                            let image = image::open(&path)
                                .unwrap_or_else(|_| panic!("Failed to load {path:?}"));
                            let texture = match image {
                                image::DynamicImage::ImageRgba8(image) => image,
                                _ => image.to_rgba8(),
                            };
                            let config = TileSetConfig::generate_from(&texture, size);
                            let file = std::fs::File::create(run_dir().join("tileset_config.json"))
                                .unwrap();
                            let writer = std::io::BufWriter::new(file);
                            serde_json::to_writer_pretty(writer, &config).unwrap();
                            std::process::exit(0);
                            #[allow(unreachable_code)]
                            geng::EmptyLoadingScreen
                        }
                    };
                    geng::LoadingScreen::new(&geng, geng::EmptyLoadingScreen, future)
                };
                geng::run(&geng, state)
            }
            #[cfg(not(target_arch = "wasm32"))]
            Command::ChangeSize(config) => {
                let state = {
                    let future = {
                        let geng = geng.clone();
                        async move {
                            let room_path = opt
                                .room
                                .as_ref()
                                .expect("change size requires a --room argument");
                            let size = parse_size(&config.size).expect("Failed to parse size");
                            let mut room = Room::load(room_path).expect("Failed to load the room");

                            let assets: Assets =
                                geng::LoadAsset::load(&geng, &run_dir().join("assets"))
                                    .await
                                    .expect("Failed to load assets");

                            room.change_size(size, &assets);

                            let room_path = "new_room.json";
                            room.save(room_path).expect("Failed to save the room");
                            info!("Saved the changed room at {}", room_path);

                            std::process::exit(0);
                            #[allow(unreachable_code)]
                            geng::EmptyLoadingScreen
                        }
                    };
                    geng::LoadingScreen::new(&geng, geng::EmptyLoadingScreen, future)
                };

                geng::run(&geng, state)
            }
            #[cfg(not(target_arch = "wasm32"))]
            Command::RenameTile(config) => {
                let room_path = opt
                    .room
                    .as_ref()
                    .expect("format requires a --room argument");
                let mut room = Room::load(room_path).expect("Failed to load the room");

                for layer in room.layers.iter_mut() {
                    for tile in &mut layer.tiles.tiles {
                        if *tile == config.from {
                            *tile = config.to.to_owned();
                        }
                    }
                }

                room.save(room_path).expect("Failed to save the room");
                info!("Saved the changed room at {}", room_path);
            }
            #[cfg(not(target_arch = "wasm32"))]
            Command::Format => {
                let room_path = opt
                    .room
                    .as_ref()
                    .expect("format requires a --room argument");

                fn format_room(room_path: impl AsRef<std::path::Path>) {
                    let room_path = room_path.as_ref();
                    info!("Formatting {room_path:?}");
                    let Ok(room) = util::report_err(Room::load(room_path), "Failed to load the room") else {
                        return;
                    };
                    room.save(room_path).expect("Failed to save the room");
                }

                if room_path == "*" {
                    fn format_dir(path: impl AsRef<std::path::Path>) {
                        let dir =
                            std::fs::read_dir(path).expect("Failed to open assets/rooms directory");
                        for file in dir {
                            let file = file.expect("Failed to access a directory entry");
                            let meta = file.metadata().expect("Failed to access metadata");
                            if meta.is_dir() {
                                format_dir(file.path());
                            } else {
                                format_room(file.path());
                            }
                        }
                    }

                    format_dir(run_dir().join("assets").join("rooms"))
                } else {
                    format_room(room_path)
                }
            }
        }
        return;
    }

    if opt.editor {
        geng::run(&geng, editor::run(&geng, opt.room, opt.hot_reload))
    } else if let Some(room) = &opt.room {
        geng::run(&geng, game::run(&geng, None, room))
    } else {
        geng::run(&geng, intro::run(&geng))
    }
}

fn parse_size(input: &str) -> Option<vec2<usize>> {
    let mut xs = input.split('x');
    let pos = vec2(xs.next()?.parse().ok()?, xs.next()?.parse().ok()?);
    if xs.next().is_some() {
        return None;
    }
    Some(pos)
}

fn time_ms(mut time: Time) -> (u32, u32, Time) {
    let minutes = (time / Time::new(60.0)).floor();
    time -= minutes * Time::new(60.0);
    let seconds = time.floor();
    (
        minutes.as_f32() as _,
        seconds.as_f32() as _,
        (time - seconds) * Time::new(1e3),
    )
}

fn room_path(name: impl AsRef<std::path::Path>) -> std::path::PathBuf {
    run_dir().join("assets").join("rooms").join(name)
}

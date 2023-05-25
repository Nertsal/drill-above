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
    level: Option<String>,
    #[clap(long)]
    room: Option<String>,
    /// Hot reload assets on change detection.
    #[clap(long)]
    hot_reload: bool,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(clap::Subcommand, Clone)]
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

#[derive(clap::Args, Clone)]
struct TileSetOpt {
    tileset: String,
    size: String,
}

#[derive(clap::Args, Clone)]
struct ChangeSizeOpt {
    size: String,
}

#[derive(clap::Args, Clone)]
struct RenameTileOpt {
    from: String,
    to: String,
}

macro_rules! exit_state {
    () => {{
        std::process::exit(0);
        #[allow(unreachable_code)]
        geng::EmptyState
    }};
}

fn main() {
    logger::init();
    geng::setup_panic_handler();
    let opt: Opt = program_args::parse();

    let geng = Geng::new_with(geng::ContextOptions {
        title: "Drill above".to_string(),
        fixed_delta_time: 1.0 / FPS,
        ..Default::default()
    });

    if let Some(command) = opt.command.clone() {
        match command {
            #[cfg(not(target_arch = "wasm32"))]
            Command::TileSet(opt) => {
                let size = parse_size(&opt.size).expect("Failed to parse size");
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
                        let path = path.with_extension("json");
                        let file = std::fs::File::create(&path).unwrap();
                        let writer = std::io::BufWriter::new(file);
                        serde_json::to_writer_pretty(writer, &config).unwrap();
                        info!("Saved the config at {:?}", path);

                        exit_state!()
                    }
                };
                geng.run_loading(future)
            }
            #[cfg(not(target_arch = "wasm32"))]
            Command::ChangeSize(config) => {
                let future = {
                    let geng = geng.clone();
                    async move {
                        let id = opt.room_id().expect("expected full room id");
                        let path = id.full_path();
                        let size = parse_size(&config.size).expect("Failed to parse size");
                        let mut room = Room::load(&path).expect("Failed to load the room");

                        let assets: Assets =
                            geng::LoadAsset::load(&geng, &run_dir().join("assets"))
                                .await
                                .expect("Failed to load assets");

                        room.change_size(size, &assets);

                        room.save(&path).expect("Failed to save the room");
                        info!("Saved the changed room at {:?}", path);

                        exit_state!()
                    }
                };
                geng.run_loading(future)
            }
            #[cfg(not(target_arch = "wasm32"))]
            Command::RenameTile(config) => {
                let id = opt.room_id().expect("expected full room id");
                let path = id.full_path();
                let mut room = Room::load(&path).expect("Failed to load the room");

                for layer in room.layers.iter_mut() {
                    for tile in &mut layer.tiles.tiles {
                        if *tile == config.from {
                            *tile = config.to.to_owned();
                        }
                    }
                }

                room.save(&path).expect("Failed to save the room");
                info!("Saved the changed room at {:?}", path);
            }
            #[cfg(not(target_arch = "wasm32"))]
            Command::Format => {
                fn format_room(room_path: impl AsRef<std::path::Path>) {
                    let room_path = room_path.as_ref();
                    info!("Formatting {room_path:?}");
                    let Ok(room) = util::report_err(Room::load(room_path), "Failed to load the room") else {
                        return;
                    };
                    room.save(room_path).expect("Failed to save the room");
                }

                if opt.room == Some("*".to_string()) {
                    fn format_dir(path: impl AsRef<std::path::Path>) {
                        let dir = std::fs::read_dir(path)
                            .expect("Failed to open assets/levels directory");
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
                    let path = opt.room_id().expect("expected full room id").full_path();
                    format_room(path)
                }
            }
        }
        return;
    }

    if opt.editor {
        let future = editor::run(
            &geng,
            opt.level
                .clone()
                .expect("Editor requires the --level argument"),
            opt.room.clone(),
            opt.hot_reload,
        );
        geng.run_loading(future);
    } else if let Some(id) = opt.room_id() {
        let future = game::run(&geng, None, id);
        geng.run_loading(future);
    } else {
        let future = intro::run(&geng);
        geng.run_loading(future);
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

impl Opt {
    fn room_id(&self) -> Option<RoomId> {
        Some(RoomId {
            level: self.level.as_ref()?.clone(),
            name: self.room.as_ref()?.clone(),
        })
    }
}

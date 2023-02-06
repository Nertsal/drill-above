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
    /// Hot reload assets on change detection.
    #[clap(long)]
    hot_reload: bool,
    #[clap(long)]
    #[cfg(not(target_arch = "wasm32"))]
    format: bool,
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
                            // let texture: ugli::Texture =
                            //     geng::LoadAsset::load(&geng, &run_dir().join(opt.tileset))
                            //         .await
                            //         .expect("Failed to load texture");
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
                    geng::LoadingScreen::new(&geng, geng::EmptyLoadingScreen, future, |state| state)
                };
                geng::run(&geng, state)
            }
            #[cfg(not(target_arch = "wasm32"))]
            Command::ChangeSize(config) => {
                let state = {
                    let future = {
                        let geng = geng.clone();
                        async move {
                            let level_path = opt
                                .level
                                .as_ref()
                                .expect("change size requires a --level argument");
                            let size = parse_size(&config.size).expect("Failed to parse size");
                            let mut level =
                                Level::load(level_path).expect("Failed to load the level");

                            let assets: Assets =
                                geng::LoadAsset::load(&geng, &run_dir().join("assets"))
                                    .await
                                    .expect("Failed to load assets");

                            level.change_size(size, &assets);

                            let level_path = "new_level.json";
                            level.save(level_path).expect("Failed to save the level");
                            info!("Saved the changed level at {}", level_path);

                            std::process::exit(0);
                            #[allow(unreachable_code)]
                            geng::EmptyLoadingScreen
                        }
                    };
                    geng::LoadingScreen::new(&geng, geng::EmptyLoadingScreen, future, |state| state)
                };

                geng::run(&geng, state)
            }
            #[cfg(not(target_arch = "wasm32"))]
            Command::Format => {
                let level_path = opt
                    .level
                    .as_ref()
                    .expect("format requires a --level argument");
                let level = Level::load(level_path).expect("Failed to load the level");
                level.save(level_path).expect("Failed to save the level");
                info!("Saved the changed level at {}", level_path);
            }
        }
        return;
    }

    if opt.editor {
        geng::run(&geng, editor::run(&geng, opt.level, opt.hot_reload))
    } else if let Some(level) = &opt.level {
        geng::run(&geng, game::run(&geng, None, level))
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

use geng::prelude::*;
use geng::Camera2d;

mod assets;
mod editor;
mod game;
mod intro;
mod model;
mod render;
mod ui;
mod util;

use assets::*;
use model::*;
use render::*;

const FPS: f64 = 60.0;

const PIXELS_PER_UNIT: f32 = 8.0;

#[derive(clap::Parser)]
struct Opt {
    #[clap(long)]
    editor: bool,
    #[clap(long)]
    level: Option<String>,
    #[clap(long)]
    #[cfg(not(target_arch = "wasm32"))]
    change_size: Option<String>,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(clap::Subcommand)]
enum Command {
    #[cfg(not(target_arch = "wasm32"))]
    TileSet(TileSetOpt),
}

#[derive(clap::Args)]
struct TileSetOpt {
    tileset: String,
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
                                .unwrap_or_else(|_| panic!("Failed to load {:?}", path));
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
        }
        return;
    }

    #[cfg(not(target_arch = "wasm32"))]
    if let Some(size) = &opt.change_size {
        let level_path = opt
            .level
            .as_ref()
            .expect("expand requires a --level argument");
        let size = parse_size(size).expect("Failed to parse size");
        let mut level = Level::load(level_path).expect("Failed to load the level");
        level.change_size(size);

        let level_path = "new_level.json";
        level.save(level_path).expect("Failed to save the level");
        info!("Saved the changed level at {}", level_path);
        return;
    }

    if opt.editor {
        geng::run(&geng, editor::run(&geng, opt.level))
    } else if let Some(level) = &opt.level {
        geng::run(&geng, game::run(&geng, None, level))
    } else {
        geng::run(&geng, intro::run(&geng))
    }
}

fn parse_size(input: &str) -> Option<Vec2<usize>> {
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
        time - seconds * Time::new(1e3),
    )
}

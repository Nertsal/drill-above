use geng::prelude::*;
use geng::Camera2d;

mod assets;
mod editor;
mod game;
mod model;
mod render;
mod ui;
mod util;

use assets::*;
use model::*;
use render::*;

const FPS: f64 = 30.0;

#[derive(clap::Parser)]
struct Opt {
    #[clap(long)]
    editor: bool,
    #[clap(long)]
    level: Option<String>,
    #[clap(long)]
    change_size: Option<String>,
}

fn main() {
    logger::init().unwrap();
    geng::setup_panic_handler();
    let opt: Opt = program_args::parse();

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

    let geng = Geng::new_with(geng::ContextOptions {
        title: "Love, Money + Gameplay".to_string(),
        fixed_delta_time: 1.0 / FPS,
        ..Default::default()
    });

    if opt.editor {
        geng::run(&geng, editor::run(&geng, opt.level))
    } else {
        let level = opt.level.unwrap_or_else(|| "intro_01.json".to_string());
        geng::run(&geng, game::run(&geng, level))
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

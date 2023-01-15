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
}

fn main() {
    logger::init().unwrap();
    geng::setup_panic_handler();
    let opt: Opt = program_args::parse();

    let geng = Geng::new_with(geng::ContextOptions {
        title: "Love, Money + Gameplay".to_string(),
        fixed_delta_time: 1.0 / FPS,
        ..Default::default()
    });

    if opt.editor {
        geng::run(&geng, editor::run(&geng, opt.level))
    } else {
        let level = opt.level.unwrap_or_else(|| "a.json".to_string());
        geng::run(&geng, game::run(&geng, level))
    }
}

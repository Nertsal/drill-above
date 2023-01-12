use geng::prelude::*;

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

#[derive(clap::Parser)]
struct Opt {
    #[clap(long)]
    editor: bool,
}

fn main() {
    logger::init().unwrap();
    geng::setup_panic_handler();
    let opt: Opt = program_args::parse();

    let geng = Geng::new_with(geng::ContextOptions {
        title: "Love, Money + Gameplay".to_string(),
        ..Default::default()
    });

    if opt.editor {
        geng::run(&geng, editor::run(&geng))
    } else {
        geng::run(&geng, game::run(&geng))
    }
}

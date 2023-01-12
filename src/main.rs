use geng::prelude::*;

#[derive(geng::Assets)]
pub struct Assets {}

#[derive(clap::Parser)]
struct Opt {}

pub struct Game {}

impl Game {
    pub fn new(geng: &Geng, assets: &Rc<Assets>) -> Self {
        Self {}
    }
}

impl geng::State for Game {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {}
}

fn main() {
    logger::init().unwrap();
    geng::setup_panic_handler();
    let _opt: Opt = program_args::parse();

    let geng = Geng::new_with(geng::ContextOptions {
        title: "Love, Money + Gameplay".to_string(),
        ..Default::default()
    });

    geng::run(
        &geng,
        geng::LoadingScreen::new(
            &geng,
            geng::EmptyLoadingScreen,
            <Assets as geng::LoadAsset>::load(&geng, &run_dir().join("assets")),
            {
                let geng = geng.clone();
                move |assets| {
                    let assets = assets.unwrap();
                    let assets = Rc::new(assets);
                    Game::new(&geng, &assets)
                }
            },
        ),
    )
}

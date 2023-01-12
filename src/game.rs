use super::*;

pub struct Game {}

impl Game {
    pub fn new(_geng: &Geng, _assets: &Rc<Assets>) -> Self {
        Self {}
    }
}

impl geng::State for Game {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        ugli::clear(framebuffer, Some(Rgba::BLACK), None, None);
    }
}

pub fn run(geng: &Geng) -> impl geng::State {
    let future = {
        let geng = geng.clone();
        async move {
            let assets: Rc<Assets> = geng::LoadAsset::load(&geng, &run_dir().join("assets"))
                .await
                .expect("Failed to load assets");
            Game::new(&geng, &assets)
        }
    };
    geng::LoadingScreen::new(geng, geng::EmptyLoadingScreen, future, |state| state)
}

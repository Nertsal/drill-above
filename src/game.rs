use super::*;

pub struct Game {
    geng: Geng,
    render: Render,
    world: World,
    controls: Controls,
    control: PlayerControl,
}

struct Controls {
    left: Vec<geng::Key>,
    right: Vec<geng::Key>,
    jump: Vec<geng::Key>,
}

impl Game {
    pub fn new(geng: &Geng, assets: &Rc<Assets>, level: Level) -> Self {
        Self {
            geng: geng.clone(),
            render: Render::new(geng, assets),
            world: World::new(assets.rules.clone(), level),
            control: PlayerControl::default(),
            controls: Controls {
                left: vec![geng::Key::Left],
                right: vec![geng::Key::Right],
                jump: vec![geng::Key::Z],
            },
        }
    }

    fn update_control(&mut self) {
        macro_rules! pressed {
            ($keys:expr) => {{
                let window = self.geng.window();
                $keys.iter().any(|&key| window.is_key_pressed(key))
            }};
        }

        if pressed!(self.controls.jump) {
            self.control.hold_jump = true;
        }

        let mut dir = Vec2::ZERO;
        if pressed!(self.controls.left) {
            dir.x -= Coord::ONE;
        }
        if pressed!(self.controls.right) {
            dir.x += Coord::ONE;
        }
        self.control.move_dir = dir;
    }
}

impl geng::State for Game {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        ugli::clear(framebuffer, Some(Rgba::BLACK), None, None);

        self.render.draw_world(&self.world, framebuffer);
    }

    fn update(&mut self, delta_time: f64) {
        let delta_time = Time::new(delta_time as f32);
        self.update_control();
        let control = self.control.take();
        self.world.update(control, delta_time);
    }

    fn handle_event(&mut self, event: geng::Event) {
        if let geng::Event::KeyDown { key } = event {
            if self.controls.jump.contains(&key) {
                self.control.jump = true;
            }
        }
    }
}

pub fn run(geng: &Geng, level: impl AsRef<std::path::Path>) -> impl geng::State {
    let future = {
        let geng = geng.clone();
        let level = level.as_ref().to_owned();
        async move {
            let assets: Rc<Assets> = geng::LoadAsset::load(&geng, &run_dir().join("assets"))
                .await
                .expect("Failed to load assets");
            let level: Level =
                geng::LoadAsset::load(&geng, &run_dir().join("assets").join("levels").join(level))
                    .await
                    .expect("Failed to load level");
            Game::new(&geng, &assets, level)
        }
    };
    geng::LoadingScreen::new(geng, geng::EmptyLoadingScreen, future, |state| state)
}

use super::*;

pub struct Game {
    geng: Geng,
    assets: Rc<Assets>,
    render: Render,
    framebuffer_size: Vec2<usize>,
    pixel_texture: ugli::Texture,
    world: World,
    draw_hitboxes: bool,
    controls: Controls,
    control: PlayerControl,
    fade: Time,
    accumulated_time: Time,
    show_time: bool,
}

struct Controls {
    left: Vec<geng::Key>,
    right: Vec<geng::Key>,
    down: Vec<geng::Key>,
    up: Vec<geng::Key>,
    jump: Vec<geng::Key>,
    drill: Vec<geng::Key>,
}

impl Game {
    pub fn new(
        geng: &Geng,
        assets: &Rc<Assets>,
        level: Level,
        coins: usize,
        time: Time,
        show_time: bool,
    ) -> Self {
        let mut world = World::new(assets, assets.rules.clone(), level);
        world.coins_collected = coins;
        Self {
            geng: geng.clone(),
            assets: assets.clone(),
            render: Render::new(geng, assets),
            framebuffer_size: vec2(1, 1),
            pixel_texture: {
                let mut texture =
                    ugli::Texture::new_with(geng.ugli(), vec2(320, 180), |_| Rgba::BLACK);
                texture.set_filter(ugli::Filter::Nearest);
                texture
            },
            draw_hitboxes: false,
            fade: Time::ONE,
            control: PlayerControl::default(),
            controls: Controls {
                left: vec![geng::Key::Left],
                right: vec![geng::Key::Right],
                down: vec![geng::Key::Down],
                up: vec![geng::Key::Up],
                jump: vec![geng::Key::Z],
                drill: vec![geng::Key::C],
            },
            accumulated_time: time,
            show_time,
            world,
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

        if pressed!(self.controls.drill) {
            self.control.drill = true;
        }

        let mut dir = Vec2::ZERO;
        if pressed!(self.controls.left) {
            dir.x -= Coord::ONE;
        }
        if pressed!(self.controls.right) {
            dir.x += Coord::ONE;
        }
        if pressed!(self.controls.down) {
            dir.y -= Coord::ONE;
        }
        if pressed!(self.controls.up) {
            dir.y += Coord::ONE;
        }
        self.control.move_dir = dir;
    }
}

impl geng::State for Game {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.framebuffer_size = framebuffer.size();
        ugli::clear(framebuffer, Some(Rgba::BLACK), None, None);

        // Render the game onto the texture
        let mut pixel_framebuffer = ugli::Framebuffer::new_color(
            self.geng.ugli(),
            ugli::ColorAttachment::Texture(&mut self.pixel_texture),
        );
        ugli::clear(&mut pixel_framebuffer, Some(Rgba::BLACK), None, None);
        self.render
            .draw_world(&self.world, self.draw_hitboxes, &mut pixel_framebuffer);

        // Render background
        let reference_size = vec2(16.0, 9.0);
        let ratio = framebuffer.size().map(|x| x as f32) / reference_size;
        let ratio = ratio.x.min(ratio.y);
        let target_size = reference_size * ratio;
        let screen = AABB::point(framebuffer.size().map(|x| x as f32) / 2.0)
            .extend_symmetric(target_size / 2.0);
        self.geng.draw_2d(
            framebuffer,
            &geng::PixelPerfectCamera,
            &draw_2d::TexturedQuad::new(screen, &self.assets.sprites.room),
        );

        // Render the texture onto the screen
        // let target = AABB::from_corners(
        //     screen.size() * vec2(50.0, 180.0 - 111.0) / vec2(320.0, 180.0),
        //     screen.size() * vec2(163.0, 180.0 - 47.0) / vec2(320.0, 180.0),
        // )
        // .translate(screen.bottom_left());
        let target = screen;
        self.geng.draw_2d(
            framebuffer,
            &geng::PixelPerfectCamera,
            &draw_2d::TexturedQuad::new(target, &self.pixel_texture),
        );

        self.render.draw_ui(
            self.show_time
                .then_some(self.accumulated_time + self.world.time),
            &self.world,
            framebuffer,
        );

        // Fade
        if self.fade > Time::ZERO {
            self.geng.draw_2d(
                framebuffer,
                &geng::PixelPerfectCamera,
                &draw_2d::Quad::new(
                    screen,
                    Rgba::new(0.196, 0.196, 0.196, self.fade.as_f32().min(1.0)),
                ),
            );
        }
    }

    fn update(&mut self, delta_time: f64) {
        let delta_time = Time::new(delta_time as f32);
        if self.fade > Time::ZERO {
            self.fade -= delta_time;
        }
    }

    fn fixed_update(&mut self, delta_time: f64) {
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
            if self.controls.drill.contains(&key) {
                self.control.drill = true;
            }
            match key {
                geng::Key::F1 => {
                    self.draw_hitboxes = !self.draw_hitboxes;
                }
                geng::Key::F2 => {
                    self.show_time = !self.show_time;
                }
                _ => (),
            }
        }
    }

    fn transition(&mut self) -> Option<geng::Transition> {
        self.world.level_transition.take().map(|level| {
            geng::Transition::Switch(Box::new(game::level_change(
                &self.geng,
                Some(&self.assets),
                level,
                self.world.coins_collected,
                self.accumulated_time + self.world.time,
                self.show_time,
            )))
        })
    }
}

pub fn run(
    geng: &Geng,
    assets: Option<&Rc<Assets>>,
    level: impl AsRef<std::path::Path>,
) -> impl geng::State {
    level_change(geng, assets, level, 0, Time::ZERO, false)
}

fn level_change(
    geng: &Geng,
    assets: Option<&Rc<Assets>>,
    level: impl AsRef<std::path::Path>,
    coins: usize,
    time: Time,
    show_time: bool,
) -> impl geng::State {
    let future = {
        let geng = geng.clone();
        let assets = assets.cloned();
        let level = level.as_ref().to_owned();
        async move {
            let assets = match assets {
                Some(assets) => assets,
                None => geng::LoadAsset::load(&geng, &run_dir().join("assets"))
                    .await
                    .expect("Failed to load assets"),
            };
            let level: Level =
                geng::LoadAsset::load(&geng, &run_dir().join("assets").join("levels").join(level))
                    .await
                    .expect("Failed to load level");
            Game::new(&geng, &assets, level, coins, time, show_time)
        }
    };
    geng::LoadingScreen::new(geng, geng::EmptyLoadingScreen, future, |state| state)
}

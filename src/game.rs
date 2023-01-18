use super::*;

pub struct Game {
    geng: Geng,
    assets: Rc<Assets>,
    render: Render,
    framebuffer_size: Vec2<usize>,
    pixel_texture: ugli::Texture,
    level_name: String,
    world: World,
    draw_hitboxes: bool,
    controls: Controls,
    control: PlayerControl,
    fade: Time,
    accumulated_time: Time,
    show_time: bool,
    music: Option<geng::SoundEffect>,
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
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        geng: &Geng,
        assets: &Rc<Assets>,
        level_name: String,
        level: Level,
        coins: usize,
        time: Time,
        show_time: bool,
        music: Option<geng::SoundEffect>,
    ) -> Self {
        let mut world = World::new(assets, assets.rules.clone(), level);
        world.coins_collected = coins;
        let mut music = music.unwrap_or_else(|| assets.music.play());
        music.set_volume(world.volume);
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
            music: Some(music),
            level_name,
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

        let is_credits = self.level_name == "credits.json";
        if !is_credits {
            let show_time = self
                .show_time
                .then_some(self.accumulated_time + self.world.time);
            self.render.draw_ui(show_time, &self.world, framebuffer);
        }

        if is_credits {
            let framebuffer_size = framebuffer.size().map(|x| x as f32);
            let center = framebuffer_size * vec2(0.5, 0.7);

            // Coins
            let texture = &self.assets.sprites.coin;
            let pos = center + vec2(0.0, framebuffer_size.y * 0.03);
            let size = framebuffer_size.y * 0.07;
            let size = texture
                .size()
                .map(|x| x as f32 / texture.size().x as f32 * size);
            self.geng.draw_2d(
                framebuffer,
                &geng::PixelPerfectCamera,
                &draw_2d::TexturedQuad::new(
                    AABB::point(pos).extend_left(size.x).extend_up(size.y),
                    texture,
                ),
            );
            self.geng.draw_2d(
                framebuffer,
                &geng::PixelPerfectCamera,
                &draw_2d::Text::unit(
                    &*self.assets.font,
                    format!("{}", self.world.coins_collected),
                    Rgba::try_from("#e3a912").unwrap(),
                )
                .scale_uniform(size.y * 0.3)
                .align_bounding_box(vec2(0.0, 0.5))
                .translate(pos + vec2(size.x / 2.0, size.y / 2.0)),
            );

            // Time
            let size = framebuffer_size.y * 0.02;
            let (m, s, ms) = time_ms(self.accumulated_time);
            self.geng.draw_2d(
                framebuffer,
                &geng::PixelPerfectCamera,
                &draw_2d::Text::unit(
                    &*self.assets.font,
                    format!("{:02}:{:02}.{:.3}", m, s, ms),
                    Rgba::BLACK,
                )
                .scale_uniform(size)
                .align_bounding_box(vec2(0.5, 1.0))
                .translate(center),
            );
        }

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
        if let Some(level) = self.world.level_transition.take() {
            if level == self.level_name {
                self.world = World::new(
                    &self.assets,
                    self.assets.rules.clone(),
                    self.world.level.clone(),
                );
                return None;
            }

            return Some(geng::Transition::Switch(Box::new(game::level_change(
                &self.geng,
                Some(&self.assets),
                level,
                self.world.coins_collected,
                self.accumulated_time + self.world.time,
                self.show_time,
                self.music.take(),
            ))));
        }
        None
    }
}

pub fn run(
    geng: &Geng,
    assets: Option<&Rc<Assets>>,
    level: impl AsRef<std::path::Path>,
) -> impl geng::State {
    level_change(geng, assets, level, 0, Time::ZERO, false, None)
}

fn level_change(
    geng: &Geng,
    assets: Option<&Rc<Assets>>,
    level: impl AsRef<std::path::Path>,
    coins: usize,
    time: Time,
    show_time: bool,
    music: Option<geng::SoundEffect>,
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
            let level_name = level.to_string_lossy().to_string();
            let level: Level =
                geng::LoadAsset::load(&geng, &run_dir().join("assets").join("levels").join(level))
                    .await
                    .expect("Failed to load level");
            Game::new(
                &geng, &assets, level_name, level, coins, time, show_time, music,
            )
        }
    };
    geng::LoadingScreen::new(geng, geng::EmptyLoadingScreen, future, |state| state)
}

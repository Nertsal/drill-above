use super::*;

pub struct Game {
    geng: Geng,
    assets: Rc<Assets>,
    render: GameRender,
    framebuffer_size: vec2<usize>,
    pixel_texture: ugli::Texture,
    level_name: String,
    world: World,
    draw_hitboxes: bool,
    controls: Controls,
    control: PlayerControl,
    fade: Time,
    accumulated_time: Time,
    deaths: usize,
    show_time: bool,
    music: Option<geng::SoundEffect>,
    show_debug: bool,
}

struct Controls {
    left: Vec<geng::Key>,
    right: Vec<geng::Key>,
    down: Vec<geng::Key>,
    up: Vec<geng::Key>,
    jump: Vec<geng::Key>,
    drill: Vec<geng::Key>,
    retry: Vec<geng::Key>,
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
        deaths: usize,
        show_time: bool,
        music: Option<geng::SoundEffect>,
    ) -> Self {
        geng.window().set_cursor_type(geng::CursorType::None);

        let mut world = World::new(geng, assets, assets.rules.clone(), level);
        world.coins_collected = coins;
        let mut music = music.unwrap_or_else(|| assets.music.play());
        music.set_volume((world.volume - 0.3).max(0.0));
        Self {
            geng: geng.clone(),
            assets: assets.clone(),
            render: GameRender::new(geng, assets),
            framebuffer_size: vec2(1, 1),
            pixel_texture: {
                let mut texture =
                    ugli::Texture::new_with(geng.ugli(), SCREEN_RESOLUTION, |_| Rgba::BLACK);
                texture.set_filter(ugli::Filter::Nearest);
                texture
            },
            draw_hitboxes: false,
            show_debug: false,
            fade: Time::ONE,
            control: PlayerControl::default(),
            controls: Controls {
                left: vec![geng::Key::Left],
                right: vec![geng::Key::Right],
                down: vec![geng::Key::Down],
                up: vec![geng::Key::Up],
                jump: vec![geng::Key::Z, geng::Key::Space],
                drill: vec![geng::Key::C],
                retry: vec![geng::Key::R],
            },
            accumulated_time: time,
            music: Some(music),
            deaths,
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
            self.control.hold_drill = true;
        }

        let mut dir = vec2::ZERO;
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
        let framebuffer_size = framebuffer.size().map(|x| x as f32);
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
        let screen = Aabb2::point(framebuffer.size().map(|x| x as f32) / 2.0)
            .extend_symmetric(target_size / 2.0);
        self.geng.draw_2d(
            framebuffer,
            &geng::PixelPerfectCamera,
            &draw_2d::TexturedQuad::new(screen, &self.assets.sprites.room),
        );

        // Render the texture onto the screen
        // let target = Aabb2::from_corners(
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
            let center = framebuffer_size * vec2(0.5, 0.7);

            // Coins
            let texture = &self.assets.sprites.coin;
            let pos = center + vec2(0.0, framebuffer_size.y * 0.03 * 4.0);
            let size = framebuffer_size.y * 0.07;
            let size = texture
                .size()
                .map(|x| x as f32 / texture.size().x as f32 * size);
            self.geng.draw_2d(
                framebuffer,
                &geng::PixelPerfectCamera,
                &draw_2d::TexturedQuad::new(
                    Aabb2::point(pos).extend_left(size.x).extend_up(size.y),
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

            // Deaths
            let texture = &self.assets.sprites.skull;
            let pos = center + vec2(0.0, framebuffer_size.y * 0.03);
            let size = framebuffer_size.y * 0.07;
            let size = texture
                .size()
                .map(|x| x as f32 / texture.size().x as f32 * size);
            self.geng.draw_2d(
                framebuffer,
                &geng::PixelPerfectCamera,
                &draw_2d::TexturedQuad::new(
                    Aabb2::point(pos).extend_left(size.x).extend_up(size.y),
                    texture,
                ),
            );
            self.geng.draw_2d(
                framebuffer,
                &geng::PixelPerfectCamera,
                &draw_2d::Text::unit(&*self.assets.font, format!("{}", self.deaths), Rgba::BLACK)
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
                    format!("{:02}:{:02}.{:03}", m, s, ms.floor()),
                    Rgba::BLACK,
                )
                .scale_uniform(size)
                .align_bounding_box(vec2(0.5, 1.0))
                .translate(center),
            );
        }

        if self.show_debug {
            let size = framebuffer_size.y * 0.02;
            self.geng.draw_2d(
                framebuffer,
                &geng::PixelPerfectCamera,
                &draw_2d::Text::unit(
                    &*self.assets.font,
                    format!("Speed: {:.2}", self.world.player.velocity.len()),
                    Rgba::BLACK,
                )
                .scale_uniform(size)
                .align_bounding_box(vec2(0.0, 0.0))
                .translate(vec2(framebuffer_size.x - size * 20.0, size * 5.0)),
            );
            self.geng.draw_2d(
                framebuffer,
                &geng::PixelPerfectCamera,
                &draw_2d::Text::unit(
                    &*self.assets.font,
                    format!("x: {:6.2}", self.world.player.velocity.x),
                    Rgba::BLACK,
                )
                .scale_uniform(size * 0.8)
                .align_bounding_box(vec2(0.0, 0.0))
                .translate(vec2(framebuffer_size.x - size * 20.0, size * 3.0)),
            );
            self.geng.draw_2d(
                framebuffer,
                &geng::PixelPerfectCamera,
                &draw_2d::Text::unit(
                    &*self.assets.font,
                    format!("y: {:6.2}", self.world.player.velocity.y),
                    Rgba::BLACK,
                )
                .scale_uniform(size)
                .align_bounding_box(vec2(0.0, 0.0))
                .translate(vec2(framebuffer_size.x - size * 20.0, size)),
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
            if self.controls.retry.contains(&key) {
                self.world.kill_player();
            }
            match key {
                geng::Key::F1 => {
                    self.draw_hitboxes = !self.draw_hitboxes;
                }
                geng::Key::F2 => {
                    self.show_time = !self.show_time;
                }
                geng::Key::F4 => {
                    self.show_debug = !self.show_debug;
                }
                _ => (),
            }
        }
    }

    fn transition(&mut self) -> Option<geng::Transition> {
        if let Some(level) = self.world.level_transition.take() {
            if level == self.level_name {
                let coins = self.world.coins_collected;
                self.world = World::new(
                    &self.geng,
                    &self.assets,
                    self.assets.rules.clone(),
                    self.world.level.clone(),
                );
                self.world.coins_collected = coins;
                return None;
            }

            return Some(geng::Transition::Switch(Box::new(game::level_change(
                &self.geng,
                Some(&self.assets),
                level,
                self.world.coins_collected,
                self.accumulated_time + self.world.time,
                self.deaths + self.world.deaths,
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
    level_change(geng, assets, level, 0, Time::ZERO, 0, false, None)
}

#[allow(clippy::too_many_arguments)]
fn level_change(
    geng: &Geng,
    assets: Option<&Rc<Assets>>,
    level: impl AsRef<std::path::Path>,
    coins: usize,
    time: Time,
    deaths: usize,
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
                &geng, &assets, level_name, level, coins, time, deaths, show_time, music,
            )
        }
    };
    geng::LoadingScreen::new(geng, geng::EmptyLoadingScreen, future, |state| state)
}

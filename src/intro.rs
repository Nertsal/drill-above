use super::*;

pub struct Intro {
    geng: Geng,
    assets: Rc<Assets>,
    intro: Animation,
    time: Time,
    zoom: R32,
    transition: Option<geng::Transition>,
    play_button: Option<AABB<f32>>,
    hit_play: bool,
    cursor_pos: Vec2<f32>,
    animation_frame: usize,
    next_frame: Time,
}

impl Intro {
    pub fn new(geng: &Geng, assets: &Rc<Assets>, intro: Animation) -> Self {
        geng.window().set_cursor_type(geng::CursorType::None);
        let volume = 1.0;
        let mut effect = assets.sounds.cutscene.play();
        effect.set_volume(volume);

        Self {
            geng: geng.clone(),
            assets: assets.clone(),
            time: Time::ZERO,
            zoom: R32::ONE,
            transition: None,
            play_button: None,
            hit_play: false,
            cursor_pos: Vec2::ZERO,
            animation_frame: 0,
            next_frame: Time::new(intro.first().unwrap().1),
            intro,
        }
    }
}

impl geng::State for Intro {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        let framebuffer_size = framebuffer.size().map(|x| x as f32);

        ugli::clear(framebuffer, Some(Rgba::BLACK), None, None);
        if self.time > Time::new(1.5) && self.hit_play {
            self.transition = Some(geng::Transition::Switch(Box::new(game::run(
                &self.geng,
                Some(&self.assets),
                "intro_01.json",
            ))));
            return;
        }

        let reference_size = vec2(16.0, 9.0);
        let ratio = framebuffer.size().map(|x| x as f32) / reference_size;
        let ratio = ratio.x.min(ratio.y);
        let target_size = reference_size * ratio;

        self.zoom = self.time.max(Time::ONE);
        let zoom = (self.zoom.as_f32() - 1.0).min(1.0);
        let zoom = 3.0 * zoom * zoom - 2.0 * zoom * zoom * zoom; // Smoothstep
        let screen = AABB::from_corners(
            vec2(112.0, 180.0 - 97.0) / vec2(320.0, 180.0) * target_size,
            vec2(207.0, 180.0 - 41.0) / vec2(320.0, 180.0) * target_size,
        );
        let scale = 1.0 + (target_size.y - screen.height()) * zoom / screen.height();
        let offset = (screen.center() - target_size / 2.0) * zoom;

        let aabb = AABB::point(framebuffer_size / 2.0 - offset * scale)
            .extend_symmetric(target_size / 2.0 * scale);
        let screen = AABB::from_corners(
            screen.bottom_left() / target_size * aabb.size(),
            screen.top_right() / target_size * aabb.size(),
        )
        .translate(aabb.bottom_left());

        self.play_button = None;
        let texture = if self.animation_frame < self.intro.len() {
            self.intro
                .get(self.animation_frame)
                .map(|(texture, _)| texture)
        } else {
            self.play_button = Some(
                AABB::from_corners(
                    aabb.size() * vec2(161.0, 180.0 - 85.0) / vec2(320.0, 180.0),
                    aabb.size() * vec2(169.0, 180.0 - 71.0) / vec2(320.0, 180.0),
                )
                .translate(aabb.bottom_left()),
            );
            self.play_button
                .and_then(|button| {
                    button
                        .contains(self.cursor_pos)
                        .then_some(&self.assets.sprites.drill_hover)
                })
                .or_else(|| self.intro.last().map(|(texture, _)| texture))
        };

        if let Some(texture) = texture {
            self.geng.draw_2d(
                framebuffer,
                &geng::PixelPerfectCamera,
                &draw_2d::TexturedQuad::new(aabb, texture),
            );
        }

        if self.time > Time::ZERO {
            let texture = &self.assets.sprites.cursor;
            let size = texture.size().map(|x| x as f32) * aabb.height() / 180.0;
            let pos = self.cursor_pos.clamp_aabb(screen);
            self.geng.draw_2d(
                framebuffer,
                &geng::PixelPerfectCamera,
                &draw_2d::TexturedQuad::new(
                    AABB::point(pos).extend_right(size.x).extend_down(size.y),
                    texture,
                ),
            );
        }
    }

    fn handle_event(&mut self, event: geng::Event) {
        match event {
            geng::Event::MouseMove { position, .. } => {
                self.cursor_pos = position.map(|x| x as f32);
            }
            geng::Event::MouseDown {
                position,
                button: geng::MouseButton::Left,
            } => {
                if let Some(button) = self.play_button {
                    if button.contains(position.map(|x| x as f32)) {
                        self.hit_play = true;
                    }
                }
            }
            _ => {}
        }
    }

    fn update(&mut self, delta_time: f64) {
        let delta_time = Time::new(delta_time as f32);
        if self.animation_frame >= self.intro.len() {
            self.time += delta_time;
        }
        self.next_frame -= delta_time;
        if self.next_frame < Time::ZERO {
            self.animation_frame += 1;
            self.next_frame = self
                .intro
                .get(self.animation_frame)
                .map(|(_, delay)| Time::new(*delay))
                .unwrap_or(Time::ZERO);
        }
    }

    fn transition(&mut self) -> Option<geng::Transition> {
        self.transition.take()
    }
}

pub fn run(geng: &Geng) -> impl geng::State {
    let future = {
        let geng = geng.clone();
        async move {
            let assets: Rc<Assets> = geng::LoadAsset::load(&geng, &run_dir().join("assets"))
                .await
                .expect("Failed to load assets");
            let intro: Animation =
                geng::LoadAsset::load(&geng, &run_dir().join("assets").join("intro.gif"))
                    .await
                    .expect("Failed to load intro animation");
            Intro::new(&geng, &assets, intro)
        }
    };
    geng::LoadingScreen::new(geng, geng::EmptyLoadingScreen, future, |state| state)
}

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
}

impl Intro {
    pub fn new(geng: &Geng, assets: &Rc<Assets>, intro: Animation) -> Self {
        Self {
            geng: geng.clone(),
            assets: assets.clone(),
            intro,
            time: Time::ZERO,
            zoom: R32::ONE,
            transition: None,
            play_button: None,
            hit_play: false,
            cursor_pos: Vec2::ZERO,
        }
    }
}

impl geng::State for Intro {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        let framebuffer_size = framebuffer.size().map(|x| x as f32);

        let animation_time = Time::new(10.0);

        ugli::clear(framebuffer, Some(Rgba::BLACK), None, None);
        if self.time > animation_time + Time::new(1.5) && self.hit_play {
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

        self.zoom = (self.time - animation_time).max(Time::ONE);
        let zoom = (self.zoom.as_f32() - 1.0).min(1.0);
        let zoom = 3.0 * zoom * zoom - 2.0 * zoom * zoom * zoom; // Smoothstep
        let screen = AABB::from_corners(
            vec2(760.0, 1632.0 - 935.0) / vec2(2448.0, 1632.0) * target_size,
            vec2(1692.0, 1632.0 - 384.0) / vec2(2448.0, 1632.0) * target_size,
        );
        let scale = 1.0 + (target_size.y - screen.height()) * zoom / screen.height();
        let offset = (screen.center() - target_size / 2.0) * zoom;

        let aabb = AABB::point(framebuffer_size / 2.0 - offset * scale)
            .extend_symmetric(target_size / 2.0 * scale);

        self.play_button = None;
        let frame = if self.time < animation_time {
            let t = self.time / animation_time;
            (t.as_f32() * (self.intro.frames.len() as f32 - 2.0)).floor() as usize
        } else if !self.hit_play {
            self.play_button = Some(
                AABB::from_corners(
                    aabb.size() * vec2(1180.0, 1632.0 - 747.0) / vec2(2448.0, 1632.0),
                    aabb.size() * vec2(1277.0, 1632.0 - 604.0) / vec2(2448.0, 1632.0),
                )
                .translate(aabb.bottom_left()),
            );
            self.intro.len() - 3
        } else {
            self.intro.len() - 1
        };

        if let Some(texture) = self.intro.get(frame) {
            self.geng.draw_2d(
                framebuffer,
                &geng::PixelPerfectCamera,
                &draw_2d::TexturedQuad::new(aabb, texture),
            );
        }
        if let Some(button) = self.play_button {
            if button.contains(self.cursor_pos) {
                self.geng.draw_2d(
                    framebuffer,
                    &geng::PixelPerfectCamera,
                    &draw_2d::Quad::new(button, Rgba::new(0.0, 0.0, 0.0, 0.5)),
                );
            }
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
        self.time += delta_time;
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

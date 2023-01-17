use super::*;

pub struct Intro {
    geng: Geng,
    assets: Rc<Assets>,
    intro: Animation,
    time: Time,
    zoom: R32,
    transition: Option<geng::Transition>,
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
        }
    }
}

impl geng::State for Intro {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        let framebuffer_size = framebuffer.size().map(|x| x as f32);

        ugli::clear(framebuffer, Some(Rgba::BLACK), None, None);
        if self.time > Time::new(2.5) {
            self.transition = Some(geng::Transition::Switch(Box::new(game::run(
                &self.geng,
                Some(&self.assets),
                "intro_01.json",
            ))));
            return;
        }
        self.zoom = self.time.max(Time::ONE);

        if let Some(texture) = self.intro.get_frame(self.time.min(Time::new(0.999999))) {
            let reference_size = vec2(16.0, 9.0);
            let ratio = framebuffer.size().map(|x| x as f32) / reference_size;
            let ratio = ratio.x.min(ratio.y);
            let target_size = reference_size * ratio;

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
            self.geng.draw_2d(
                framebuffer,
                &geng::PixelPerfectCamera,
                &draw_2d::TexturedQuad::new(aabb, texture),
            );
        }
    }

    fn update(&mut self, delta_time: f64) {
        let delta_time = Time::new(delta_time as f32);
        let scale = Time::new(if self.time < Time::ONE {
            1.0 / 10.0
        } else {
            1.0
        });
        self.time += delta_time * scale;
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

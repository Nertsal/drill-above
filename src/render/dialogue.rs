use super::*;

const DIALOGUE_BOX_COLOR: Rgba<f32> = Rgba {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 0.8,
};

impl GameRender {
    pub fn draw_dialogue(&self, dialogue: &Dialogue, framebuffer: &mut ugli::Framebuffer) {
        let framebuffer_size = framebuffer.size().map(|x| x as f32);
        let camera = geng::PixelPerfectCamera;
        let view = Aabb2::ZERO.extend_positive(framebuffer_size);

        let view_point = |pos| view.bottom_left() + view.size() * pos;
        let view_box = |a, b| Aabb2::from_corners(view_point(a), view_point(b));

        let dialogue_box = view_box(vec2(0.2, 0.6), vec2(0.8, 0.8));
        self.geng.draw_2d(
            framebuffer,
            &camera,
            &draw_2d::Quad::new(dialogue_box, DIALOGUE_BOX_COLOR),
        );

        self.geng.draw_2d(
            framebuffer,
            &camera,
            &draw_2d::Text::unit(self.assets.font.clone(), &dialogue.text, Rgba::WHITE)
                .fit_into(dialogue_box),
        );
    }
}

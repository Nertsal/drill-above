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
        let text_size = view.height() * 0.05;

        let dialogue_box = view_box(vec2(0.2, 0.6), vec2(0.8, 0.8));
        self.geng.draw_2d(
            framebuffer,
            &camera,
            &draw_2d::Quad::new(dialogue_box, DIALOGUE_BOX_COLOR),
        );

        let text_box = dialogue_box.extend_uniform(-10.0);
        for (i, line) in crate::util::split_text_lines(
            &dialogue.text,
            &self.assets.fonts.dialogue,
            text_size,
            text_box.width(),
        )
        .into_iter()
        .enumerate()
        {
            let pos = text_box.top_left() - vec2::UNIT_Y * text_size * 1.5 * (i as f32 + 0.5);
            self.assets.fonts.dialogue.draw(
                framebuffer,
                &camera,
                &line,
                pos,
                geng::TextAlign::LEFT,
                text_size,
                Rgba::WHITE,
            );
        }
    }
}

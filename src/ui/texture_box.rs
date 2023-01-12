use super::*;

use geng::ui::*;

pub struct TextureBox<T: std::borrow::Borrow<ugli::Texture>> {
    pub texture: T,
    pub size: Vec2<f32>,
}

impl<T: std::borrow::Borrow<ugli::Texture>> TextureBox<T> {
    pub fn new(texture: T) -> Self {
        Self {
            texture,
            size: Vec2::ZERO,
        }
    }
}

impl<T: std::borrow::Borrow<ugli::Texture>> Widget for TextureBox<T> {
    fn draw(&mut self, cx: &mut DrawContext) {
        cx.geng.draw_2d(
            cx.framebuffer,
            &geng::PixelPerfectCamera,
            &draw_2d::TexturedQuad::new(cx.position.map(|x| x as f32), self.texture.borrow()),
        );
    }

    fn calc_constraints(&mut self, _children: &ConstraintsContext) -> Constraints {
        Constraints {
            min_size: self.size.map(|x| x as f64),
            flex: vec2(0.0, 0.0),
        }
    }
}

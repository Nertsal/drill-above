use super::*;

use geng::ui::*;

#[derive(ugli::Vertex)]
struct Vertex {
    a_pos: Vec2<f32>,
    a_uv: Vec2<f32>,
}

pub struct TextureBox<T: std::borrow::Borrow<ugli::Texture>> {
    pub texture: T,
    assets: Rc<Assets>,
    pub size: Vec2<f32>,
    geometry: ugli::VertexBuffer<Vertex>,
}

impl<T: std::borrow::Borrow<ugli::Texture>> TextureBox<T> {
    pub fn new(geng: &Geng, assets: &Rc<Assets>, texture: T, geometry: [Vec2<f32>; 4]) -> Self {
        let quad = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)].map(|(x, y)| vec2(x, y));
        Self {
            assets: assets.clone(),
            size: Vec2::ZERO,
            geometry: ugli::VertexBuffer::new_dynamic(
                geng.ugli(),
                geometry
                    .into_iter()
                    .zip(quad)
                    .map(|(a_uv, a_pos)| Vertex { a_pos, a_uv })
                    .collect(),
            ),
            texture,
        }
    }
}

impl<T: std::borrow::Borrow<ugli::Texture>> Widget for TextureBox<T> {
    fn draw(&mut self, cx: &mut DrawContext) {
        let matrix = (Mat3::translate(cx.position.bottom_left()) * Mat3::scale(cx.position.size()))
            .map(|x| x as f32);
        ugli::draw(
            cx.framebuffer,
            &self.assets.shaders.texture,
            ugli::DrawMode::TriangleFan,
            &self.geometry,
            (
                ugli::uniforms! {
                    u_model_matrix: matrix,
                    u_texture: self.texture.borrow(),
                },
                geng::camera2d_uniforms(
                    &geng::PixelPerfectCamera,
                    cx.framebuffer.size().map(|x| x as f32),
                ),
            ),
            ugli::DrawParameters::default(),
        );
    }

    fn calc_constraints(&mut self, _children: &ConstraintsContext) -> Constraints {
        Constraints {
            min_size: self.size.map(|x| x as f64),
            flex: vec2(0.0, 0.0),
        }
    }
}

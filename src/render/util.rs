use super::*;

pub fn new_texture(geng: &Geng) -> ugli::Texture {
    ugli::Texture::new_with(geng.ugli(), vec2(1, 1), |_| Rgba::BLACK)
}

pub fn attach_texture<'a>(texture: &'a mut ugli::Texture, geng: &Geng) -> ugli::Framebuffer<'a> {
    ugli::Framebuffer::new_color(geng.ugli(), ugli::ColorAttachment::Texture(texture))
}

pub fn pixel_perfect_pos(pos: vec2<Coord>) -> vec2<f32> {
    let pos = pos.map(Coord::as_f32);
    let pixel = pos.map(|x| (x * PIXELS_PER_UNIT as f32).round());
    pixel / PIXELS_PER_UNIT as f32
}

pub fn update_texture_size(texture: &mut ugli::Texture, size: vec2<usize>, geng: &Geng) {
    if texture.size() != size {
        *texture = ugli::Texture::new_with(geng.ugli(), size, |_| Rgba::BLACK);
        texture.set_filter(ugli::Filter::Nearest);
    }
}

pub struct UtilRender {
    geng: Geng,
    assets: Rc<Assets>,
    quad_geometry: ugli::VertexBuffer<draw_2d::Vertex>,
}

impl UtilRender {
    pub fn new(geng: &Geng, assets: &Rc<Assets>) -> Self {
        Self {
            geng: geng.clone(),
            assets: assets.clone(),
            quad_geometry: ugli::VertexBuffer::new_static(
                geng.ugli(),
                vec![
                    draw_2d::Vertex {
                        a_pos: vec2(-1.0, -1.0),
                    },
                    draw_2d::Vertex {
                        a_pos: vec2(1.0, -1.0),
                    },
                    draw_2d::Vertex {
                        a_pos: vec2(1.0, 1.0),
                    },
                    draw_2d::Vertex {
                        a_pos: vec2(-1.0, 1.0),
                    },
                ],
            ),
        }
    }

    pub fn draw_collider(
        &self,
        collider: &Collider,
        color: Rgba<f32>,
        camera: &impl geng::AbstractCamera2d,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        self.geng.draw_2d(
            framebuffer,
            camera,
            &draw_2d::Quad::new(collider.raw().map(Coord::as_f32), color),
        );
    }

    pub fn draw_grid(
        &self,
        grid: &Grid,
        size: vec2<usize>,
        camera: &impl geng::AbstractCamera2d,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        let matrix = grid.matrix().map(Coord::as_f32);
        ugli::draw(
            framebuffer,
            &self.assets.shaders.grid,
            ugli::DrawMode::TriangleFan,
            &self.quad_geometry,
            (
                ugli::uniforms! {
                    u_grid_matrix: matrix,
                    u_grid_size: size,
                    u_grid_color: Rgba::GRAY,
                    u_grid_width: vec2(0.01, 0.01),
                },
                geng::camera2d_uniforms(camera, framebuffer.size().map(|x| x as f32)),
            ),
            ugli::DrawParameters::default(),
        )
    }
}

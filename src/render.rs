use super::*;

pub struct Render {
    geng: Geng,
    assets: Rc<Assets>,
    quad_geometry: ugli::VertexBuffer<draw_2d::Vertex>,
}

impl Render {
    pub fn new(geng: &Geng, assets: &Rc<Assets>) -> Self {
        Self {
            geng: geng.clone(),
            assets: assets.clone(),
            quad_geometry: ugli::VertexBuffer::new_dynamic(
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

    pub fn draw_grid(
        &self,
        grid: &Grid,
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
                    u_grid_color: Rgba::GRAY,
                    u_grid_width: vec2(0.01, 0.01),
                },
                geng::camera2d_uniforms(camera, framebuffer.size().map(|x| x as f32)),
            ),
            ugli::DrawParameters::default(),
        )
    }
}

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

    pub fn draw_world(
        &self,
        world: &World,
        draw_hitboxes: bool,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        self.draw_level(&world.level, draw_hitboxes, &world.camera, framebuffer);

        // Player
        if let PlayerState::Respawning { time } = world.player.state {
        } else {
            self.geng.draw_2d(
                framebuffer,
                &world.camera,
                &draw_2d::Quad::new(world.player.collider.raw().map(Coord::as_f32), Rgba::GREEN),
            );
        }
    }

    pub fn draw_level(
        &self,
        level: &Level,
        draw_hitboxes: bool,
        camera: &impl geng::AbstractCamera2d,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        self.draw_tiles(level, &level.tiles, camera, framebuffer);
        self.draw_hazards(&level.hazards, draw_hitboxes, camera, framebuffer);
    }

    pub fn draw_level_editor(
        &self,
        level: &Level,
        draw_hitboxes: bool,
        camera: &impl geng::AbstractCamera2d,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        self.draw_level(level, draw_hitboxes, camera, framebuffer);

        // Spawnpoint
        self.geng.draw_2d(
            framebuffer,
            camera,
            &draw_2d::Quad::new(
                Player::new(level.spawn_point)
                    .collider
                    .raw()
                    .map(Coord::as_f32),
                Rgba::new(0.0, 1.0, 0.0, 0.5),
            ),
        );
    }

    pub fn draw_tiles(
        &self,
        level: &Level,
        tiles: &TileMap,
        camera: &impl geng::AbstractCamera2d,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        for (i, tile) in tiles.tiles().iter().enumerate() {
            let pos = index_to_pos(i, level.size.x);
            let pos = level.grid.grid_to_world(pos.map(|x| x as isize));
            let pos = AABB::point(pos)
                .extend_positive(level.grid.cell_size)
                .map(Coord::as_f32);
            let texture = self.assets.sprites.tiles.get_texture(tile);
            self.geng.draw_2d(
                framebuffer,
                camera,
                &draw_2d::TexturedQuad::new(pos, texture),
            );
        }
    }

    pub fn draw_hazards(
        &self,
        hazards: &[Hazard],
        draw_hitboxes: bool,
        camera: &impl geng::AbstractCamera2d,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        for hazard in hazards {
            let texture = self.assets.sprites.hazards.get_texture(&hazard.hazard_type);
            self.geng.draw_2d(
                framebuffer,
                camera,
                &draw_2d::TexturedQuad::new(hazard.sprite.map(Coord::as_f32), texture),
            );
            if draw_hitboxes {
                self.geng.draw_2d(
                    framebuffer,
                    camera,
                    &draw_2d::Quad::new(
                        hazard.collider.raw().map(Coord::as_f32),
                        Rgba::new(1.0, 0.0, 0.0, 0.5),
                    ),
                );
            }
        }
    }
}

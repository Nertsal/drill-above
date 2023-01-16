use super::*;

#[derive(ugli::Vertex, Debug, Clone, Copy)]
struct Vertex {
    a_pos: Vec2<f32>,
    a_uv: Vec2<f32>,
}

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

    fn draw_pixel_perfect(
        &self,
        bottom_left: Vec2<Coord>,
        texture: &ugli::Texture,
        flip: bool,
        transform: Mat3<f32>,
        camera: &impl geng::AbstractCamera2d,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        // TODO: remove hardcoded pixels per unit
        let size = texture.size().map(|x| x as f32) / 8.0;
        let pos = bottom_left.map(Coord::as_f32);
        let pixel = pos.map(|x| (x * 8.0).floor());
        let pos = pixel / 8.0 + size / 2.0;
        let transform = Mat3::translate(pos) * transform;
        self.geng.draw_2d_transformed(
            framebuffer,
            camera,
            &draw_2d::TexturedQuad::new(
                AABB::ZERO.extend_symmetric(size / 2.0 * vec2(if flip { -1.0 } else { 1.0 }, 1.0)),
                texture,
            ),
            transform,
        )
    }

    pub fn draw_grid(
        &self,
        grid: &Grid,
        size: Vec2<usize>,
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

    pub fn draw_world(
        &self,
        world: &World,
        draw_hitboxes: bool,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        self.draw_level(&world.level, draw_hitboxes, &world.camera, framebuffer);

        // Player
        if let PlayerState::Respawning { .. } = world.player.state {
        } else {
            let sprites = &self.assets.sprites.player;
            let mut flip = world.player.facing_left;
            let (texture, transform) = match world.player.state {
                PlayerState::Drilling => {
                    flip = false;
                    let mut angle = world.player.velocity.arg().as_f32() / f32::PI * 4.0 + 2.0;
                    let drill = if angle.floor() as i32 % 2 == 0 {
                        // Vertical/horizontal
                        &sprites.drill.drill_v0
                    } else {
                        // Diagonal
                        angle -= 1.0;
                        &sprites.drill.drill_d0
                    };
                    (drill, Mat3::rotate(angle.floor() * f32::PI / 4.0))
                }
                PlayerState::WallSliding { wall_normal } => {
                    flip = wall_normal.x < Coord::ZERO;
                    (&sprites.slide0, Mat3::identity())
                }
                _ => (&sprites.idle0, Mat3::identity()),
            };
            self.draw_pixel_perfect(
                world.player.collider.raw().bottom_left(),
                texture,
                flip,
                transform,
                &world.camera,
                framebuffer,
            );
        }

        self.draw_particles(&world.particles, &world.camera, framebuffer);
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
        self.draw_coins(&level.coins, draw_hitboxes, camera, framebuffer);

        // Finish
        self.geng.draw_2d(
            framebuffer,
            camera,
            &draw_2d::Quad::new(
                Player::new(level.finish).collider.raw().map(Coord::as_f32),
                Rgba::new(0.0, 0.0, 1.0, 0.9),
            ),
        );
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
        let mut tiles_geometry = HashMap::<Tile, Vec<Vertex>>::new();
        for (i, tile) in tiles.tiles().iter().enumerate() {
            let connections = tiles.get_tile_connections(i);
            let pos = index_to_pos(i, level.size.x);
            let pos = level.grid.grid_to_world(pos.map(|x| x as isize));
            let pos = AABB::point(pos)
                .extend_positive(level.grid.cell_size)
                .map(Coord::as_f32);
            let set = self.assets.sprites.tiles.get_tile_set(tile);
            let geometry = set.get_tile_connected(connections);
            let vertices = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
            let vertices = [0, 1, 2, 3].map(|i| Vertex {
                a_pos: vec2(vertices[i].0, vertices[i].1),
                a_uv: geometry[i],
            });
            let geometry = [
                vertices[0],
                vertices[1],
                vertices[2],
                vertices[0],
                vertices[2],
                vertices[3],
            ];
            let matrix = Mat3::translate(pos.bottom_left()) * Mat3::scale(pos.size());
            tiles_geometry
                .entry(*tile)
                .or_default()
                .extend(geometry.into_iter().map(|vertex| {
                    let pos = matrix * vertex.a_pos.extend(1.0);
                    Vertex {
                        a_pos: pos.xy() / pos.z,
                        ..vertex
                    }
                }));
        }
        for (tile, geometry) in tiles_geometry {
            let set = self.assets.sprites.tiles.get_tile_set(&tile);
            let texture = set.texture();
            let geometry = ugli::VertexBuffer::new_dynamic(self.geng.ugli(), geometry);
            ugli::draw(
                framebuffer,
                &self.assets.shaders.texture,
                ugli::DrawMode::Triangles,
                &geometry,
                (
                    ugli::uniforms! {
                        u_model_matrix: Mat3::identity(),
                        u_texture: texture,
                    },
                    geng::camera2d_uniforms(camera, framebuffer.size().map(|x| x as f32)),
                ),
                ugli::DrawParameters {
                    blend_mode: Some(ugli::BlendMode::default()),
                    ..Default::default()
                },
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
            let transform = (Mat3::translate(hazard.sprite.center())
                * Mat3::rotate(
                    hazard
                        .direction
                        .map_or(Coord::ZERO, |dir| dir.arg() - Coord::PI / Coord::new(2.0)),
                ))
            .map(Coord::as_f32);
            self.geng.draw_2d_transformed(
                framebuffer,
                camera,
                &draw_2d::TexturedQuad::new(
                    AABB::ZERO.extend_symmetric(hazard.sprite.size().map(Coord::as_f32) / 2.0),
                    texture,
                ),
                transform,
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

    pub fn draw_coins(
        &self,
        coins: &[Coin],
        draw_hitboxes: bool,
        camera: &impl geng::AbstractCamera2d,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        for coin in coins {
            let texture = &self.assets.sprites.coin;
            self.geng.draw_2d(
                framebuffer,
                camera,
                &draw_2d::TexturedQuad::new(coin.collider.raw().map(Coord::as_f32), texture),
            );
            if draw_hitboxes {
                self.geng.draw_2d(
                    framebuffer,
                    camera,
                    &draw_2d::Quad::new(
                        coin.collider.raw().map(Coord::as_f32),
                        Rgba::new(1.0, 1.0, 0.0, 0.5),
                    ),
                );
            }
        }
    }

    pub fn draw_particles(
        &self,
        particles: &[Particle],
        camera: &impl geng::AbstractCamera2d,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        for particle in particles {
            let texture = match particle.particle_type {
                ParticleType::Heart4 => &self.assets.sprites.heart4,
                ParticleType::Heart8 => &self.assets.sprites.heart8,
            };
            let size = texture.size().map(|x| x as f32 / 8.0); // TODO: remove hardcode
            self.geng.draw_2d(
                framebuffer,
                camera,
                &draw_2d::TexturedQuad::new(
                    AABB::point(particle.position.map(Coord::as_f32)).extend_symmetric(size / 2.0),
                    texture,
                ),
            );
        }
    }

    pub fn draw_ui(&self, world: &World, framebuffer: &mut ugli::Framebuffer) {
        let framebuffer_size = framebuffer.size().map(|x| x as f32);
        let text_size = framebuffer_size.y * 0.05;

        // Coins collected
        self.geng.draw_2d(
            framebuffer,
            &geng::PixelPerfectCamera,
            &draw_2d::Text::unit(
                &**self.geng.default_font(),
                format!("Coins: {}", world.coins_collected),
                Rgba::WHITE,
            )
            .scale_uniform(text_size)
            .align_bounding_box(vec2(1.0, 1.0))
            .translate(framebuffer_size * 0.95),
        );
    }
}

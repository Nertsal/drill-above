use super::*;

const PIXELS_PER_UNIT: f32 = 8.0;

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
        let size = texture.size().map(|x| x as f32) / PIXELS_PER_UNIT;
        let pos = pixel_perfect_pos(bottom_left) + size / 2.0;
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
        self.draw_background(world, framebuffer);
        self.draw_level(&world.level, draw_hitboxes, &world.camera, framebuffer);
        self.draw_player(&world.player, &world.camera, framebuffer);
        self.draw_particles(&world.particles, &world.camera, framebuffer);
    }

    pub fn draw_background(&self, world: &World, framebuffer: &mut ugli::Framebuffer) {
        let texture = &self.assets.sprites.background;
        let size = texture.texture().size().map(|x| x as f32 / PIXELS_PER_UNIT) / vec2(1.0, 4.0);
        let bounds = world.level.bounds().map(Coord::as_f32);
        let texture_bounds = bounds.extend_positive(vec2(0.5, 0.0) - size);
        let camera_bounds = world.camera_bounds().map(Coord::as_f32);

        // Parallax background
        for i in (0..4).rev() {
            let geometry = texture.get_tile_uv(i);
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

            let move_speed = 1.0 + (i as f32 / 3.0) * 0.1;
            let mut pos =
                (world.camera.center - camera_bounds.bottom_left()) / camera_bounds.size();
            if camera_bounds.width().approx_eq(&0.0) {
                pos.x = 0.0;
            }
            if camera_bounds.height().approx_eq(&0.0) {
                pos.y = 0.0;
            }
            let pos = (texture_bounds.size() * pos - vec2(0.5, 0.5)) * move_speed;
            let mut pos = texture_bounds.bottom_left() + pos;
            let move_speed = (1.0 - i as f32 / 3.0) * 0.1;
            pos.x = world.camera.center.x * move_speed;
            pos.x = world.camera.center.x + pos.x - (pos.x / size.x + 0.5).floor() * size.x;
            let pos = pixel_perfect_pos(pos.map(Coord::new));

            let geometry = itertools::chain![
                geometry.iter().map(|&(mut v)| {
                    v.a_pos -= vec2(1.0, 0.0);
                    v
                }),
                geometry.iter().map(|&(mut v)| {
                    v.a_pos += vec2(1.0, 0.0);
                    v
                }),
                geometry,
            ]
            .collect();

            let matrix = Mat3::translate(pos) * Mat3::scale(size);
            let geometry = ugli::VertexBuffer::new_dynamic(self.geng.ugli(), geometry);
            ugli::draw(
                framebuffer,
                &self.assets.shaders.texture,
                ugli::DrawMode::Triangles,
                &geometry,
                (
                    ugli::uniforms! {
                        u_model_matrix: matrix,
                        u_texture: texture.texture(),
                    },
                    geng::camera2d_uniforms(&world.camera, framebuffer.size().map(|x| x as f32)),
                ),
                ugli::DrawParameters {
                    blend_mode: Some(ugli::BlendMode::default()),
                    ..Default::default()
                },
            );
        }

        let texture = &self.assets.sprites.sun;
        let size = texture.size().map(|x| x as f32 / PIXELS_PER_UNIT);
        let move_speed = 0.9;
        let mut pos = bounds.bottom_left();
        pos += (world.camera.center - camera_bounds.bottom_left()) * move_speed;
        let matrix = Mat3::translate(pos) * Mat3::scale(size);
        let vertices = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        let vertices = vertices.map(|(x, y)| Vertex {
            a_pos: vec2(x, y),
            a_uv: vec2(x, y),
        });
        let geometry = ugli::VertexBuffer::new_dynamic(self.geng.ugli(), vertices.to_vec());
        ugli::draw(
            framebuffer,
            &self.assets.shaders.texture,
            ugli::DrawMode::TriangleFan,
            &geometry,
            (
                ugli::uniforms! {
                    u_model_matrix: matrix,
                    u_texture: texture,
                },
                geng::camera2d_uniforms(&world.camera, framebuffer.size().map(|x| x as f32)),
            ),
            ugli::DrawParameters {
                blend_mode: Some(ugli::BlendMode::default()),
                ..Default::default()
            },
        );
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
            if let Tile::Air = tile {
                continue;
            }

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

    pub fn draw_player(
        &self,
        player: &Player,
        camera: &impl geng::AbstractCamera2d,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        if let PlayerState::Respawning { .. } = player.state {
        } else {
            let sprites = &self.assets.sprites.player;
            let mut flip = player.facing_left;
            let (texture, transform) = match player.state {
                PlayerState::Drilling => {
                    flip = false;
                    let mut angle = player.velocity.arg().as_f32() / f32::PI * 4.0 + 2.0;
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
                PlayerState::WallSliding { wall_normal, .. } => {
                    flip = wall_normal.x < Coord::ZERO;
                    (&sprites.slide0, Mat3::identity())
                }
                _ => (&sprites.idle0, Mat3::identity()),
            };
            self.draw_pixel_perfect(
                player.collider.raw().bottom_left(),
                texture,
                flip,
                transform,
                camera,
                framebuffer,
            );
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
                ParticleType::Circle { radius, color } => {
                    let radius = (radius * particle.lifetime.min(Time::ONE)).as_f32();
                    self.geng.draw_2d(
                        framebuffer,
                        camera,
                        &draw_2d::Ellipse::circle(
                            particle.position.map(Coord::as_f32),
                            radius,
                            color,
                        ),
                    );
                    continue;
                }
            };
            let size = texture.size().map(|x| x as f32 / PIXELS_PER_UNIT);
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

        // Coins collected
        let texture = &self.assets.sprites.coin;
        let size = texture.size().map(|x| x as f32 * 2.0);
        let pos = vec2(0.05, 0.95) * framebuffer_size;
        self.geng.draw_2d(
            framebuffer,
            &geng::PixelPerfectCamera,
            &draw_2d::TexturedQuad::new(
                AABB::point(pos).extend_right(size.x).extend_down(size.y),
                texture,
            ),
        );
        self.geng.draw_2d(
            framebuffer,
            &geng::PixelPerfectCamera,
            &draw_2d::Text::unit(
                &**self.geng.default_font(),
                format!("{}", world.coins_collected),
                Rgba::YELLOW,
            )
            .scale_uniform(size.y * 0.3)
            .align_bounding_box(vec2(0.0, 0.5))
            .translate(pos + vec2(size.x, -size.y / 2.0)),
        );
    }
}

fn pixel_perfect_pos(pos: Vec2<Coord>) -> Vec2<f32> {
    let pos = pos.map(Coord::as_f32);
    let pixel = pos.map(|x| (x * PIXELS_PER_UNIT).round());
    pixel / PIXELS_PER_UNIT
}

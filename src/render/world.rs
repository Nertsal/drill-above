use super::*;

pub struct WorldRender {
    geng: Geng,
    assets: Rc<Assets>,
}

impl WorldRender {
    pub fn new(geng: &Geng, assets: &Rc<Assets>) -> Self {
        Self {
            geng: geng.clone(),
            assets: assets.clone(),
        }
    }

    pub fn draw_world(
        &self,
        world: &World,
        draw_hitboxes: bool,
        framebuffer: &mut ugli::Framebuffer,
        mut normal_framebuffer: Option<&mut ugli::Framebuffer>,
    ) {
        self.draw_background(world, framebuffer);

        macro_rules! draw_layers {
            ($layers:expr) => {{
                for layer in $layers {
                    self.draw_room_layer(
                        &world.room,
                        &world.cache,
                        layer,
                        1.0,
                        draw_hitboxes,
                        &world.camera,
                        framebuffer,
                        normal_framebuffer.as_deref_mut(),
                    );
                }
            }};
        }

        draw_layers!([ActiveLayer::Background, ActiveLayer::Main]);

        self.draw_player(world, draw_hitboxes, &world.camera, framebuffer);
        self.draw_particles(&world.particles, &world.camera, framebuffer);

        draw_layers!([ActiveLayer::Foreground]);
    }

    pub fn draw_background(&self, world: &World, framebuffer: &mut ugli::Framebuffer) {
        let texture = &self.assets.sprites.background;
        let size = texture.size().map(|x| x as f32 / PIXELS_PER_UNIT as f32) / vec2(1.0, 4.0);
        let bounds = world.room.bounds().map(Coord::as_f32);
        let camera_bounds = world.camera_bounds().map(Coord::as_f32);

        // Parallax background
        for i in (0..4).rev() {
            let geometry = get_tile_uv(i, vec2(1, 4));
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

            let camera_view = Aabb2::point(world.camera.center).extend_symmetric(
                vec2(
                    world.camera.fov * framebuffer.size().x as f32 / framebuffer.size().y as f32,
                    world.camera.fov,
                ) / 2.0,
            );
            let mut pos = camera_view.bottom_left();
            let move_speed = 0.2 - (i as f32 / 3.0) * 0.1;
            pos -= (world.camera.center - camera_bounds.bottom_left()) * move_speed;

            // Horizontal correction
            pos.x -= ((pos.x - world.camera.center.x) / size.x + 1.0).floor() * size.x;

            let pos = pixel_perfect_pos(pos.map(Coord::new));

            // Horizontal tiling
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

            let matrix = mat3::translate(pos) * mat3::scale(size);
            let geometry = ugli::VertexBuffer::new_dynamic(self.geng.ugli(), geometry);
            ugli::draw(
                framebuffer,
                &self.assets.shaders.texture,
                ugli::DrawMode::Triangles,
                &geometry,
                (
                    ugli::uniforms! {
                        u_model_matrix: matrix,
                        u_texture: texture,
                        u_color: Rgba::WHITE,
                    },
                    geng::camera2d_uniforms(&world.camera, framebuffer.size().map(|x| x as f32)),
                ),
                ugli::DrawParameters {
                    blend_mode: Some(ugli::BlendMode::straight_alpha()),
                    ..Default::default()
                },
            );
        }

        let texture = &self.assets.sprites.sun;
        let size = texture.size().map(|x| x as f32 / PIXELS_PER_UNIT as f32);
        let move_speed = 0.9;
        let mut pos = bounds.bottom_left();
        pos += (world.camera.center - camera_bounds.bottom_left()) * move_speed;
        let matrix = mat3::translate(pos) * mat3::scale(size);
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
                    u_color: Rgba::WHITE,
                },
                geng::camera2d_uniforms(&world.camera, framebuffer.size().map(|x| x as f32)),
            ),
            ugli::DrawParameters {
                blend_mode: Some(ugli::BlendMode::straight_alpha()),
                ..Default::default()
            },
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_room_layer(
        &self,
        room: &Room,
        cache: &RenderCache,
        layer: ActiveLayer,
        alpha: f32,
        draw_hitboxes: bool,
        camera: &impl geng::AbstractCamera2d,
        framebuffer: &mut ugli::Framebuffer,
        normal_framebuffer: Option<&mut ugli::Framebuffer>,
    ) {
        let (room_layer, tile_geometry) = match layer {
            ActiveLayer::Background => (&room.layers.background, &cache.background_geometry),
            ActiveLayer::Main => (&room.layers.main, &cache.main_geometry),
            ActiveLayer::Foreground => (&room.layers.foreground, &cache.foreground_geometry),
        };
        self.draw_props(
            &room_layer.props,
            alpha,
            camera,
            framebuffer,
            normal_framebuffer,
        );
        self.draw_tiles(tile_geometry, alpha, camera, framebuffer);

        if let ActiveLayer::Main = layer {
            self.draw_hazards(&room.hazards, alpha, draw_hitboxes, camera, framebuffer);
            self.draw_coins(&room.coins, alpha, draw_hitboxes, camera, framebuffer);
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_room_editor(
        &self,
        room: &Room,
        cache: &RenderCache,
        active_layer: ActiveLayer,
        draw_hitboxes: bool,
        camera: &impl geng::AbstractCamera2d,
        framebuffer: &mut ugli::Framebuffer,
        mut normal_framebuffer: Option<&mut ugli::Framebuffer>,
    ) {
        for layer in [
            ActiveLayer::Background,
            ActiveLayer::Main,
            ActiveLayer::Foreground,
        ] {
            let alpha = if layer == active_layer { 1.0 } else { 0.2 };
            self.draw_room_layer(
                room,
                cache,
                layer,
                alpha,
                draw_hitboxes,
                camera,
                framebuffer,
                normal_framebuffer.as_deref_mut(),
            );
        }

        // Spawnpoint
        self.geng.draw_2d(
            framebuffer,
            camera,
            &draw_2d::Quad::new(
                Aabb2::point(room.spawn_point.map(Coord::as_f32))
                    .extend_symmetric(vec2(0.5, 0.0))
                    .extend_up(1.0),
                Rgba::new(0.0, 1.0, 0.0, 0.5),
            ),
        );

        // Spotlights
        for spotlight in &room.spotlights {
            let pos = pixel_perfect_pos(spotlight.position);
            let size = vec2(1.0, 1.0);
            let aabb = Aabb2::point(pos).extend_symmetric(size / 2.0);
            self.geng.draw_2d(
                framebuffer,
                camera,
                &draw_2d::TexturedQuad::new(aabb, &self.assets.sprites.spotlight),
            );
        }

        self.draw_transitions(&room.transitions, camera, framebuffer);
    }

    pub fn draw_tiles(
        &self,
        (tiles_geometry, masked_geometry): &TilesGeometry,
        alpha: f32,
        camera: &impl geng::AbstractCamera2d,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        let mask = self.assets.sprites.tiles.mask.texture.texture();
        for (tile, geometry) in masked_geometry {
            let set = self.assets.sprites.tiles.get_tile_set(tile);
            let texture = set.texture.texture();
            ugli::draw(
                framebuffer,
                &self.assets.shaders.texture_mask,
                ugli::DrawMode::Triangles,
                geometry,
                (
                    ugli::uniforms! {
                        u_model_matrix: mat3::identity(),
                        u_texture: texture,
                        u_mask: mask,
                        u_color: Rgba::new(1.0, 1.0, 1.0, alpha),
                    },
                    geng::camera2d_uniforms(camera, framebuffer.size().map(|x| x as f32)),
                ),
                ugli::DrawParameters {
                    blend_mode: Some(ugli::BlendMode::straight_alpha()),
                    ..Default::default()
                },
            );
        }
        for (tile, geometry) in tiles_geometry {
            let set = self.assets.sprites.tiles.get_tile_set(tile);
            let texture = set.texture.texture();
            ugli::draw(
                framebuffer,
                &self.assets.shaders.texture,
                ugli::DrawMode::Triangles,
                geometry,
                (
                    ugli::uniforms! {
                        u_model_matrix: mat3::identity(),
                        u_texture: texture,
                        u_color: Rgba::new(1.0, 1.0, 1.0, alpha),
                    },
                    geng::camera2d_uniforms(camera, framebuffer.size().map(|x| x as f32)),
                ),
                ugli::DrawParameters {
                    blend_mode: Some(ugli::BlendMode::straight_alpha()),
                    ..Default::default()
                },
            );
        }
    }

    pub fn draw_props(
        &self,
        props: &[Prop],
        alpha: f32,
        camera: &impl geng::AbstractCamera2d,
        framebuffer: &mut ugli::Framebuffer,
        mut normal_framebuffer: Option<&mut ugli::Framebuffer>,
    ) {
        for prop in props {
            let texture = self.assets.sprites.props.get_texture(&prop.prop_type);
            self.geng.draw_2d(
                framebuffer,
                camera,
                &draw_2d::TexturedQuad::colored(
                    prop.sprite.render_aabb(),
                    texture.texture(),
                    Rgba::new(1.0, 1.0, 1.0, alpha),
                ),
            );

            if let Some(framebuffer) = &mut normal_framebuffer {
                if let Some(texture) = texture.normal() {
                    // TODO: manage transparency
                    self.geng.draw_2d(
                        framebuffer,
                        camera,
                        &draw_2d::TexturedQuad::new(prop.sprite.render_aabb(), texture),
                    );
                }
            }
        }
    }

    pub fn draw_hazards(
        &self,
        hazards: &[Hazard],
        alpha: f32,
        draw_hitboxes: bool,
        camera: &impl geng::AbstractCamera2d,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        for hazard in hazards {
            let texture = self.assets.sprites.hazards.get_texture(&hazard.hazard_type);
            let transform = (mat3::translate(hazard.sprite.pos.center())
                * mat3::rotate(
                    hazard
                        .direction
                        .map_or(Coord::ZERO, |dir| dir.arg() - Coord::PI / Coord::new(2.0)),
                ))
            .map(Coord::as_f32);
            self.geng.draw_2d_transformed(
                framebuffer,
                camera,
                &draw_2d::TexturedQuad::colored(
                    Aabb2::ZERO.extend_symmetric(hazard.sprite.pos.size().map(Coord::as_f32) / 2.0),
                    texture.texture(),
                    Rgba::new(1.0, 1.0, 1.0, alpha),
                ),
                transform,
            );
            if draw_hitboxes {
                self.geng.draw_2d(
                    framebuffer,
                    camera,
                    &draw_2d::Quad::new(
                        hazard.collider.raw().map(Coord::as_f32),
                        Rgba::new(1.0, 0.0, 0.0, 0.5 * alpha),
                    ),
                );
            }
        }
    }

    pub fn draw_coins(
        &self,
        coins: &[Coin],
        alpha: f32,
        draw_hitboxes: bool,
        camera: &impl geng::AbstractCamera2d,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        for coin in coins {
            let texture = &self.assets.sprites.coin;
            self.geng.draw_2d(
                framebuffer,
                camera,
                &draw_2d::TexturedQuad::colored(
                    coin.collider.raw().map(Coord::as_f32),
                    texture,
                    Rgba::new(1.0, 1.0, 1.0, alpha),
                ),
            );
            if draw_hitboxes {
                self.geng.draw_2d(
                    framebuffer,
                    camera,
                    &draw_2d::Quad::new(
                        coin.collider.raw().map(Coord::as_f32),
                        Rgba::new(1.0, 1.0, 0.0, 0.5 * alpha),
                    ),
                );
            }
        }
    }

    pub fn draw_player(
        &self,
        world: &World,
        draw_hitboxes: bool,
        camera: &impl geng::AbstractCamera2d,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        let player = &world.player;
        if let PlayerState::Respawning { .. } = player.state {
        } else {
            let sprites = &self.assets.sprites.player;
            let mut flip = player.facing_left;
            let (texture, transform) = match player.state {
                PlayerState::Drilling | PlayerState::AirDrill { .. } => {
                    let mut velocity = player.velocity.map(|x| {
                        if x.as_f32().abs() < 1.0 {
                            0.00
                        } else {
                            x.as_f32()
                        }
                    });
                    if velocity == vec2::ZERO {
                        velocity.y = 1.0;
                    }
                    flip = false;
                    let mut angle = (velocity.arg() / f32::PI * 4.0 + 2.0).round();
                    let drill = if angle as i32 % 2 == 0 {
                        // Vertical/horizontal
                        &sprites.drill.drill_v0
                    } else {
                        // Diagonal
                        angle -= 1.0;
                        &sprites.drill.drill_d0
                    };
                    (drill, mat3::rotate(angle * f32::PI / 4.0))
                }
                PlayerState::WallSliding { wall_normal, .. } if player.velocity.y < Coord::ZERO => {
                    flip = wall_normal.x < Coord::ZERO;
                    (&sprites.slide0, mat3::identity())
                }
                _ => (&sprites.player, mat3::identity()),
            };

            let actor = world.actors.get(&player.id).unwrap();
            let pos = actor.collider.feet();
            let size = texture.size().map(|x| x as f32) / PIXELS_PER_UNIT as f32;
            let pos = pixel_perfect_pos(pos) + vec2(0.0, size.y / 2.0);
            let transform = mat3::translate(pos) * transform;
            self.geng.draw_2d_transformed(
                framebuffer,
                camera,
                &draw_2d::TexturedQuad::new(
                    Aabb2::ZERO
                        .extend_symmetric(size / 2.0 * vec2(if flip { -1.0 } else { 1.0 }, 1.0)),
                    texture,
                ),
                transform,
            );
            if draw_hitboxes {
                self.geng.draw_2d(
                    framebuffer,
                    camera,
                    &draw_2d::Quad::new(
                        actor.collider.raw().map(Coord::as_f32),
                        Rgba::new(0.0, 1.0, 0.0, 0.7),
                    ),
                );
                self.geng.draw_2d(
                    framebuffer,
                    camera,
                    &draw_2d::Quad::new(
                        actor.feet_collider().raw().map(Coord::as_f32),
                        Rgba::new(1.0, 0.0, 0.0, 0.7),
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
            // let texture =
            match particle.particle_type {
                ParticleType::Circle { radius, color } => {
                    let t =
                        particle.lifetime.min(Time::ONE) / particle.initial_lifetime.min(Time::ONE);
                    let radius = (radius * t).as_f32();
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
            // let size = texture.size().map(|x| x as f32 / PIXELS_PER_UNIT as f32);
            // self.geng.draw_2d(
            //     framebuffer,
            //     camera,
            //     &draw_2d::TexturedQuad::new(
            //         Aabb2::point(particle.position.map(Coord::as_f32)).extend_symmetric(size / 2.0),
            //         texture,
            //     ),
            // );
        }
    }

    fn draw_transitions(
        &self,
        transitions: &[RoomTransition],
        camera: &impl geng::AbstractCamera2d,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        for transition in transitions {
            let collider = transition.collider.raw().map(Coord::as_f32);
            self.geng.draw_2d(
                framebuffer,
                camera,
                &draw_2d::Quad::new(collider, Rgba::new(0.0, 0.0, 1.0, 0.5)),
            );
            self.geng.draw_2d(
                framebuffer,
                camera,
                &draw_2d::Text::unit(
                    &**self.geng.default_font(),
                    &transition.to_room,
                    Rgba::WHITE,
                )
                .fit_into(collider),
            );
        }
    }
}

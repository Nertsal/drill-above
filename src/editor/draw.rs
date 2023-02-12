use super::*;

impl Editor {
    pub fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        // Render the game onto the texture
        let mut pixel_framebuffer = ugli::Framebuffer::new_color(
            self.geng.ugli(),
            ugli::ColorAttachment::Texture(&mut self.pixel_texture),
        );
        ugli::clear(&mut pixel_framebuffer, Some(Rgba::BLACK), None, None);

        if self.preview {
            // Render as in game
            self.world.camera = self.camera.clone();
            if let Some(actor) = self.world.actors.get_mut(&self.world.player.id) {
                actor.collider.teleport(self.world.level.spawn_point);
            }
            self.preview_render
                .draw_world(&self.world, false, &mut pixel_framebuffer);
        } else {
            // Draw the world and normals ignoring lighting
            let (mut world_framebuffer, mut normal_framebuffer) =
                self.render.lights.start_render(&mut pixel_framebuffer);

            // Render level
            self.render.world.draw_level_editor(
                &self.world.level,
                &self.world.cache.geometry.0,
                &self.world.cache.geometry.1,
                true,
                &self.camera,
                &mut world_framebuffer,
                Some(&mut normal_framebuffer),
            );

            self.render.lights.finish_render(
                &self.world.level,
                &self.world.cache,
                &self.camera,
                &mut pixel_framebuffer,
            );
        }

        // Render the texture onto the screen
        let reference_size = vec2(16.0, 9.0);
        let ratio = framebuffer.size().map(|x| x as f32) / reference_size;
        let ratio = ratio.y; // ratio.x.min(ratio.y); // TODO: fix scaling for non 16/9 resolutions
        let target_size = reference_size * ratio;
        let target = Aabb2::point(framebuffer.size().map(|x| x as f32) / 2.0)
            .extend_symmetric(target_size / 2.0);
        self.geng.draw_2d(
            framebuffer,
            &geng::PixelPerfectCamera,
            &draw_2d::TexturedQuad::new(target, &self.pixel_texture),
        );

        // Draw moving
        if let Some(dragging) = &self.dragging {
            if let Some(DragAction::MoveBlocks {
                blocks,
                initial_pos,
            }) = &dragging.action
            {
                for block in blocks {
                    let collider = block
                        .sprite(&self.world.level.grid)
                        .translate(self.cursor_world_pos - *initial_pos);
                    let unit =
                        [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)].map(|(x, y)| vec2(x, y));
                    let (texture, uv) = match block {
                        Placeable::Tile((tile, _)) => {
                            let set = self.assets.sprites.tiles.get_tile_set(tile);
                            (
                                set.texture.texture(),
                                set.get_tile_geometry(
                                    set.get_tile_connected([Connection::None; 8])
                                        .first()
                                        .copied()
                                        .unwrap_or(0),
                                ),
                            )
                        }
                        Placeable::Hazard(hazard) => (
                            self.assets.sprites.hazards.get_texture(&hazard.hazard_type),
                            unit,
                        ),
                        Placeable::Coin(_) => (&self.assets.sprites.coin, unit),
                        Placeable::Prop(prop) => (
                            self.assets
                                .sprites
                                .props
                                .get_texture(&prop.prop_type)
                                .texture(),
                            unit,
                        ),
                        Placeable::Spotlight(..) => (&self.assets.sprites.spotlight, unit),
                    };

                    let quad =
                        [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)].map(|(x, y)| vec2(x, y));
                    let geometry = ugli::VertexBuffer::new_dynamic(
                        self.geng.ugli(),
                        uv.into_iter()
                            .zip(quad)
                            .map(|(a_uv, a_pos)| Vertex { a_pos, a_uv })
                            .collect(),
                    );

                    let matrix = (mat3::translate(collider.bottom_left())
                        * mat3::scale(collider.size()))
                    .map(|x| x.as_f32());
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
                            geng::camera2d_uniforms(
                                &self.camera,
                                framebuffer.size().map(|x| x as f32),
                            ),
                        ),
                        ugli::DrawParameters {
                            blend_mode: Some(ugli::BlendMode::straight_alpha()),
                            ..Default::default()
                        },
                    );
                }
            }
        }

        // Draw hovered/selected
        let mut colliders = Vec::new();
        for &block in itertools::chain![&self.hovered, &self.selection] {
            let Some(block) = self.world.level.get_block(block) else {
                continue
            };
            let collider = block.sprite(&self.world.level.grid);
            let color = match block {
                Placeable::Tile(_) => Rgba::new(0.7, 0.7, 0.7, 0.5),
                Placeable::Hazard(_) => Rgba::new(1.0, 0.0, 0.0, 0.5),
                Placeable::Prop(_) => Rgba::new(1.0, 1.0, 1.0, 0.5),
                Placeable::Coin(_) => Rgba::new(1.0, 1.0, 0.0, 0.5),
                Placeable::Spotlight(light) => {
                    let mut color = light.color;
                    color.a = 0.5;
                    color
                }
            };
            colliders.push((Collider::new(collider), color));
        }
        for (collider, color) in colliders {
            self.render
                .util
                .draw_collider(&collider, color, &self.camera, framebuffer);
        }

        if !self.preview {
            if self.draw_grid {
                self.render.util.draw_grid(
                    &self.world.level.grid,
                    self.world.level.size,
                    &self.camera,
                    framebuffer,
                );
            }

            if let Some(dragging) = &self.dragging {
                if let Some(DragAction::RectSelection) = &dragging.action {
                    // Draw the rectangular selection
                    self.geng.draw_2d(
                        framebuffer,
                        &self.camera,
                        &draw_2d::Quad::new(
                            Aabb2::from_corners(dragging.initial_world_pos, self.cursor_world_pos)
                                .map(Coord::as_f32),
                            Rgba::new(0.5, 0.5, 0.5, 0.5),
                        ),
                    );
                }
            }
        }
    }
}

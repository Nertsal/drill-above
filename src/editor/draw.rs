use super::*;

impl LevelEditor {
    pub fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.framebuffer_size = framebuffer.size();

        if let Some(room) = active_room_mut!(self) {
            // Room editor
            room.draw(framebuffer);
        } else {
            // Level editor
            self.draw_level_editor(framebuffer);
        }
    }

    fn draw_level_editor(&mut self, framebuffer: &mut ugli::Framebuffer) {
        let font = self.geng.default_font();
        for (id, room) in &self.rooms {
            let aabb = room.aabb();
            let hovered = aabb.contains(self.cursor_world_pos);
            let aabb = aabb.map(Coord::as_f32);
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &draw_2d::TexturedQuad::new(aabb, &room.preview_texture),
            );
            let color = if hovered { Rgba::RED } else { Rgba::GRAY };
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &draw_2d::Chain::new(util::aabb_outline(aabb), 0.5, color, 1),
            );

            let room_size_text = format!(
                "{}x{}",
                room.editor.world.room.size.x, room.editor.world.room.size.y
            );

            let max_size = {
                let mut text = id.name.clone();
                if hovered {
                    text += " ";
                    text += &room_size_text;
                }
                util::fit_text(text, font, aabb)
            };
            let text_size = max_size.min(5.0);
            font.draw(
                framebuffer,
                &self.camera,
                &id.name,
                aabb.bottom_left(),
                geng::TextAlign::LEFT,
                text_size,
                Rgba::WHITE,
            );

            if hovered {
                font.draw(
                    framebuffer,
                    &self.camera,
                    &room_size_text,
                    aabb.bottom_right(),
                    geng::TextAlign::RIGHT,
                    text_size,
                    Rgba::WHITE,
                );
            }
        }

        if let Some(dragging) = &self.dragging {
            if let Some(LevelDragAction::CreateRoom { initial_pos }) = &dragging.action {
                let pos = self.grid.world_to_grid(self.cursor_world_pos).0;
                let aabb = Aabb2::from_corners(*initial_pos, pos);
                self.geng.draw_2d(
                    framebuffer,
                    &self.camera,
                    &draw_2d::Chain::new(
                        util::aabb_outline(aabb.map(|x| x as f32)),
                        0.5,
                        Rgba::GREEN,
                        1,
                    ),
                );

                let text = format!("{}x{}", aabb.width(), aabb.height(),);
                let max_size = util::fit_text(&text, font, aabb.map(|x| x as f32));
                let text_size = max_size.min(5.0);
                font.draw(
                    framebuffer,
                    &self.camera,
                    &text,
                    aabb.bottom_right().map(|x| x as f32),
                    geng::TextAlign::RIGHT,
                    text_size,
                    Rgba::WHITE,
                );
            }
        }
    }
}

impl RoomEditor {
    pub fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.framebuffer_size = framebuffer.size();

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
                actor.collider.teleport(self.world.room.spawn_point);
            }
            self.preview_render
                .draw_world(&self.world, false, &mut pixel_framebuffer);
        } else {
            // Draw the world and normals ignoring lighting
            let (mut world_framebuffer, mut normal_framebuffer) =
                self.render.lights.start_render(&mut pixel_framebuffer);

            // Render the room
            self.render.world.draw_room_editor(
                &self.world.room,
                &self.world.cache,
                self.active_layer,
                true,
                &self.camera,
                &mut world_framebuffer,
                Some(&mut normal_framebuffer),
            );

            self.render.lights.finish_render(
                &self.world.room,
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

        self.render
            .world
            .draw_transitions(&self.world.room.transitions, &self.camera, framebuffer);

        // Draw moving
        if let Some(dragging) = &self.dragging {
            if let Some(RoomDragAction::MoveBlocks {
                blocks,
                initial_pos,
            }) = &dragging.action
            {
                for block in blocks {
                    let collider = block
                        .sprite(&self.world.room.grid)
                        .translate(self.cursor_world_pos - *initial_pos);

                    let sprite = match block {
                        Placeable::Tile((tile, _)) => {
                            let set = self.assets.sprites.tiles.get_tile_set(tile);
                            let texture = set.texture.texture();
                            let uv = set.get_tile_geometry(
                                set.get_tile_connected([Connection::None; 8])
                                    .first()
                                    .copied()
                                    .unwrap_or(0),
                            );

                            let quad = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]
                                .map(|(x, y)| vec2(x, y));
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
                                        u_color: Rgba::WHITE,
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

                            None
                        }
                        Placeable::Hazard(hazard) => Some((
                            hazard.sprite,
                            self.assets
                                .sprites
                                .hazards
                                .get_texture(&hazard.hazard_type)
                                .texture(),
                        )),
                        Placeable::Coin(coin) => {
                            Some((Sprite::new(coin.collider.raw()), &self.assets.sprites.coin))
                        }
                        Placeable::Prop(prop) => Some((
                            prop.sprite,
                            self.assets
                                .sprites
                                .props
                                .get_texture(&prop.prop_type)
                                .texture(),
                        )),
                        Placeable::Npc(npc) => Some((
                            npc.sprite,
                            self.assets.sprites.npc.get_texture(&npc.npc_type).texture(),
                        )),
                        Placeable::Spotlight(..) => Some((
                            Sprite::new(block.sprite(&self.world.room.grid)),
                            &self.assets.sprites.spotlight,
                        )),
                    };
                    if let Some((mut sprite, texture)) = sprite {
                        sprite.translate(self.cursor_world_pos - *initial_pos);
                        self.geng.draw_2d(
                            framebuffer,
                            &self.camera,
                            &draw_2d::TexturedQuad::new(sprite.render_aabb(), texture),
                        );
                    }
                }
            }
        }

        // Draw hovered/selected
        let mut colliders = Vec::new();
        for &block in itertools::chain![&self.hovered, &self.selection] {
            let Some(block) = self.world.room.get_block(block, self.active_layer) else {
                continue
            };
            let collider = block.sprite(&self.world.room.grid);
            let color = match block {
                Placeable::Tile(_) => Rgba::new(0.7, 0.7, 0.7, 0.5),
                Placeable::Hazard(_) => Rgba::new(1.0, 0.0, 0.0, 0.5),
                Placeable::Prop(_) => Rgba::new(1.0, 1.0, 1.0, 0.5),
                Placeable::Coin(_) => Rgba::new(1.0, 1.0, 0.0, 0.5),
                Placeable::Npc(_) => Rgba::new(0.0, 0.0, 1.0, 0.5),
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
                    &self.world.room.grid,
                    self.world.room.size,
                    &self.camera,
                    framebuffer,
                );
            }

            if let Some(dragging) = &self.dragging {
                if let Some(RoomDragAction::RectSelection) = &dragging.action {
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

    pub fn create_preview(&self) -> ugli::Texture {
        let size = self.world.room.size;
        if size.x == 0 || size.y == 0 {
            return ugli::Texture::new_with(self.geng.ugli(), vec2(1, 1), |_| Rgba::BLACK);
        }
        let mut texture = ugli::Texture::new_with(self.geng.ugli(), size, |pos| {
            let tile = &self
                .world
                .room
                .layers
                .main
                .tiles
                .get_tile_isize(pos.map(|x| x as isize))
                .unwrap();
            match tile.as_str() {
                "air" => Rgba::TRANSPARENT_WHITE,
                _ => {
                    if self
                        .assets
                        .rules
                        .tiles
                        .get(*tile)
                        .map(|tile| tile.drillable)
                        .unwrap_or(false)
                    {
                        // Drillable tile
                        Rgba::opaque(0.5, 0.7, 0.5)
                    } else {
                        Rgba::GRAY
                    }
                }
            }
        });
        texture.set_filter(ugli::Filter::Nearest);
        texture
    }
}

use super::*;

impl LevelEditor {
    pub fn ui<'a>(&'a mut self, cx: &'a geng::ui::Controller) -> Box<dyn geng::ui::Widget + 'a> {
        use geng::ui::*;

        let mut stack = geng::ui::stack![geng::ui::Void];

        if let Some(room) = active_room_mut!(self) {
            stack.push(room.ui(cx));
        }

        stack.boxed()
    }
}

impl RoomEditor {
    pub fn ui<'a>(&'a mut self, cx: &'a geng::ui::Controller) -> Box<dyn geng::ui::Widget + 'a> {
        use geng::ui::*;

        let framebuffer_size = self.framebuffer_size.map(|x| x as f32);

        let text_size = framebuffer_size.y * 0.025;
        // let font = &self.assets.font;
        let font = self.geng.default_font().clone();
        let slider = move |name, range, value: &mut f32| {
            ui::slider(cx, name, value, range, font.clone(), text_size)
        };
        let font = self.geng.default_font().clone();
        let text = move |text| geng::ui::Text::new(text, font.clone(), text_size, Rgba::WHITE);

        // let (cell_pos, cell_offset) = self.world.room.grid.world_to_grid(self.cursor_world_pos);
        // let cell_pos = Text::new(
        //     format!(
        //         "({}, {}) + ({:.1}, {:.1})",
        //         cell_pos.x, cell_pos.y, cell_offset.x, cell_offset.y
        //     ),
        //     self.geng.default_font(),
        //     framebuffer_size.y * 0.05,
        //     Rgba::WHITE,
        // );

        let mut input = ui::TextInput::new(
            cx,
            &self.geng,
            self.room_name.clone(),
            self.geng.default_font().clone(),
            text_size,
            Rgba::WHITE,
        );
        if let Some(events) = self.input_events.take() {
            // Forward events to the widget
            for event in &events {
                input.handle_input(event);
            }
        }
        if input.is_focused() {
            self.input_events = Some(Vec::new());
        }
        if self.room_name != *input.get_text() {
            // Update room name
            self.room_name = input.get_text().to_owned();
        }

        let mut room_info = geng::ui::column![
            input,
            {
                let button = Button::new(cx, "Save");
                if button.was_clicked() {
                    let _ = util::report_err(self.save_room(), "Failed to save the room");
                }
                button
            },
            {
                let button = Button::new(cx, "Playtest");
                if button.was_clicked() {
                    self.playtest = true;
                }
                button
            },
            {
                let text = if self.draw_grid {
                    "Hide grid"
                } else {
                    "Show grid"
                };
                let button = Button::new(cx, text);
                if button.was_clicked() {
                    self.draw_grid = !self.draw_grid;
                }
                button
            },
            {
                let button = Button::new(cx, "Preview");
                if button.was_clicked() {
                    self.preview = !self.preview;
                }
                button
            },
        ];

        let mut update_geometry = false;
        if let Some(tab) = self.tabs.get(self.active_tab) {
            if let RoomEditorMode::Room = tab.mode {
                room_info.extend([
                    geng::ui::row![
                        text(format!("width: {}", self.world.room.size.x))
                            .padding_right(text_size.into()),
                        {
                            let inc = Button::new(cx, "+");
                            if inc.was_clicked() {
                                self.keep_state();
                                self.world
                                    .room
                                    .change_size(self.world.room.size + vec2(1, 0), &self.assets);
                                update_geometry = true;
                            }
                            inc.padding_right(text_size.into())
                        },
                        {
                            let dec = Button::new(cx, "-");
                            if dec.was_clicked() {
                                self.keep_state();
                                self.world
                                    .room
                                    .change_size(self.world.room.size - vec2(1, 0), &self.assets);
                                update_geometry = true;
                            }
                            dec.padding_right(text_size.into())
                        },
                    ]
                    .padding_top(text_size.into())
                    .boxed(),
                    geng::ui::row![
                        text(format!("height: {}", self.world.room.size.y))
                            .padding_right(text_size.into()),
                        {
                            let inc = Button::new(cx, "+");
                            if inc.was_clicked() {
                                self.keep_state();
                                self.world
                                    .room
                                    .change_size(self.world.room.size + vec2(0, 1), &self.assets);
                                update_geometry = true;
                            }
                            inc.padding_right(text_size.into())
                        },
                        {
                            let dec = Button::new(cx, "-");
                            if dec.was_clicked() {
                                self.keep_state();
                                self.world
                                    .room
                                    .change_size(self.world.room.size - vec2(0, 1), &self.assets);
                                update_geometry = true;
                            }
                            dec.padding_right(text_size.into())
                        },
                    ]
                    .boxed(),
                    text("Translate".to_string()).boxed(),
                    geng::ui::row![
                        text("Horizontal".to_string()).padding_right(text_size.into()),
                        {
                            let left = Button::new(cx, "left");
                            if left.was_clicked() {
                                self.keep_state();
                                self.world.room.translate(vec2(-1, 0), &self.assets);
                                update_geometry = true;
                            }
                            left.padding_right(text_size.into())
                        },
                        {
                            let right = Button::new(cx, "right");
                            if right.was_clicked() {
                                self.keep_state();
                                self.world.room.translate(vec2(1, 0), &self.assets);
                                update_geometry = true;
                            }
                            right.padding_right(text_size.into())
                        },
                    ]
                    .boxed(),
                    geng::ui::row![
                        text("Vertical".to_string()).padding_right(text_size.into()),
                        {
                            let down = Button::new(cx, "down");
                            if down.was_clicked() {
                                self.keep_state();
                                self.world.room.translate(vec2(0, -1), &self.assets);
                                update_geometry = true;
                            }
                            down.padding_right(text_size.into())
                        },
                        {
                            let up = Button::new(cx, "up");
                            if up.was_clicked() {
                                self.keep_state();
                                self.world.room.translate(vec2(0, 1), &self.assets);
                                update_geometry = true;
                            }
                            up.padding_right(text_size.into())
                        },
                    ]
                    .boxed(),
                    {
                        let flip_h = Button::new(cx, "Flip horizontal");
                        if flip_h.was_clicked() {
                            self.keep_state();
                            self.world.room.flip_h(&self.assets);
                            update_geometry = true;
                        }
                        flip_h
                    }
                    .boxed(),
                    {
                        let flip_v = Button::new(cx, "Flip vertical");
                        if flip_v.was_clicked() {
                            self.keep_state();
                            self.world.room.flip_v(&self.assets);
                            update_geometry = true;
                        }
                        flip_v
                    }
                    .boxed(),
                ]);
            }
        }

        if update_geometry {
            self.update_geometry();
        }
        let font = self.geng.default_font();

        let tabs = self
            .tabs
            .iter()
            .enumerate()
            .map(|(i, tab)| {
                let color = if i == self.active_tab {
                    Rgba::opaque(0.1, 0.1, 0.3)
                } else {
                    Rgba::GRAY
                };
                let button = geng::ui::Button::new(cx, &tab.name);
                if button.was_clicked() {
                    self.active_tab = i;
                }
                Box::new(
                    geng::ui::stack![
                        geng::ui::ColorBox::new(color),
                        // geng::ui::Text::new(
                        //     &tab.name,
                        //     self.assets.font.clone(),
                        //     framebuffer_size.y * 0.05,
                        //     Rgba::WHITE
                        // ),
                        button,
                    ]
                    .padding_right(framebuffer_size.x as f64 * 0.02),
                ) as Box<dyn geng::ui::Widget>
            })
            .collect();

        let mut duplicate = false;
        let mut remove = false;
        let mut block_info_ui = |name: String, dupe: bool, ui: Box<dyn geng::ui::Widget>| {
            let mut column = geng::ui::column![geng::ui::Text::new(
                name,
                font.clone(),
                text_size * 1.5,
                Rgba::WHITE
            )];

            if dupe {
                let button = geng::ui::Button::new(cx, "Duplicate");
                if button.was_clicked() {
                    duplicate = true;
                }
                column.push(button.boxed());

                let button = geng::ui::Button::new(cx, "Remove");
                if button.was_clicked() {
                    remove = true;
                }
                column.push(button.boxed());
            }
            column.push(ui.padding_top(text_size.into()).boxed());

            geng::ui::stack![
                geng::ui::ColorBox::new(Rgba::new(0.0, 0.0, 0.0, 0.5)),
                column,
            ]
            .fixed_size(framebuffer_size.map(|x| x as f64) * vec2(0.2, 0.7))
            .align(vec2(1.0, 0.5))
            .boxed()
        };

        let mut block_info: Option<Box<dyn geng::ui::Widget>> = if self.selection.len() == 1 {
            self.selection
                .iter()
                .next()
                .and_then(|&id| self.world.room.get_block_mut(id, self.active_layer))
                .map(|block| match block {
                    PlaceableMut::Tile((name, _)) => {
                        let config = &self.assets.rules.tiles[&name];
                        let tile = geng::ui::column![
                            text(format!("Layer: {}", config.layer)),
                            text(format!("Drillable: {}", config.drillable)),
                            text(format!("Acceleration: {:.2}", config.acceleration)),
                            text(format!("Deceleration: {:.2}", config.deceleration)),
                            text(format!("Drill bounciness: {:.2}", config.drill_bounciness)),
                        ];
                        block_info_ui(format!("Tile {name}"), false, tile.boxed())
                    }
                    PlaceableMut::Hazard(_) => {
                        block_info_ui("Hazard".to_string(), true, geng::ui::Void.boxed())
                    }
                    PlaceableMut::Prop(_) => {
                        block_info_ui("Prop".to_string(), true, geng::ui::Void.boxed())
                    }
                    PlaceableMut::Coin(_) => {
                        block_info_ui("Coin".to_string(), true, geng::ui::Void.boxed())
                    }
                    PlaceableMut::Spotlight(config) => {
                        // Spotlight
                        let angle = slider("Direction", 0.0..=f64::PI * 2.0, &mut config.angle);
                        let angle_range =
                            slider("Angle", 0.0..=f64::PI * 2.0, &mut config.angle_range);
                        let color = ui::color_selector(
                            cx,
                            &mut config.color,
                            &mut self.light_float_scale,
                            &mut self.color_mode,
                            font.clone(),
                            text_size,
                        );
                        let intensity = slider("Intensity", 0.0..=1.0, &mut config.intensity);
                        let max_distance = {
                            let mut d = config.max_distance.as_f32();
                            let slider = slider("Distance", 0.0..=50.0, &mut d);
                            config.max_distance = Coord::new(d);
                            slider
                        };
                        let volume = slider("Volume", 0.0..=1.0, &mut config.volume);
                        let angle_gradient =
                            slider("Angle Gradient", 0.5..=5.0, &mut config.angle_gradient);
                        let distance_gradient = slider(
                            "Distance Gradient",
                            0.5..=5.0,
                            &mut config.distance_gradient,
                        );

                        let light = geng::ui::column![
                            angle,
                            angle_range,
                            angle_gradient,
                            color,
                            intensity,
                            max_distance,
                            distance_gradient,
                            volume,
                        ];
                        block_info_ui("Spotlight".to_string(), true, light.boxed())
                    }
                })
        } else if !self.selection.is_empty() {
            let mut text = String::new();
            for item in self.selection.iter().take(3) {
                text += match item {
                    PlaceableId::Tile(_) => "Tile",
                    PlaceableId::Hazard(_) => "Hazard",
                    PlaceableId::Prop(_) => "Prop",
                    PlaceableId::Coin(_) => "Coin",
                    PlaceableId::Spotlight(_) => "Spotlight",
                };
                text += ", ";
            }
            text.pop();
            text.pop();
            if self.selection.len() > 3 {
                text += &format!(" + {} more", self.selection.len() - 3);
            }
            Some(block_info_ui(text, true, geng::ui::Void.boxed()))
        } else {
            None
        };

        if block_info.is_none() {
            if let Some(tab) = &mut self.tabs.get_mut(self.active_tab) {
                if let RoomEditorMode::Lights { .. } = &mut tab.mode {
                    // Global light
                    let config = &mut self.world.room.global_light;
                    let color = ui::color_selector(
                        cx,
                        &mut config.color,
                        &mut self.light_float_scale,
                        &mut self.color_mode,
                        font.clone(),
                        text_size,
                    );
                    let intensity = slider("Intensity", 0.0..=1.0, &mut config.intensity);
                    let light = geng::ui::column![color, intensity];
                    block_info = Some(block_info_ui(
                        "Global light".to_string(),
                        false,
                        light.boxed(),
                    ));
                }
            }
        }

        if duplicate {
            self.duplicate_selected();
        }
        if remove {
            self.remove_selected();
        }

        let block_ui = |block: &PlaceableType| {
            let unit = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)].map(|(x, y)| vec2(x, y));
            let (texture, uv) = match block {
                PlaceableType::Tile(tile) => {
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
                PlaceableType::Hazard(hazard) => (
                    self.assets.sprites.hazards.get_texture(hazard).texture(),
                    unit,
                ),
                PlaceableType::Coin => (&self.assets.sprites.coin, unit),
                PlaceableType::Prop(prop) => {
                    (self.assets.sprites.props.get_texture(prop).texture(), unit)
                }
                PlaceableType::Spotlight(..) => (&self.assets.sprites.spotlight, unit),
            };
            let texture_size = (uv[2] - uv[0]) * texture.size().map(|x| x as f32);
            let scale = framebuffer_size.y / 90.0;
            let max_size = framebuffer_size * 0.15;
            let mut size = texture_size * scale;
            if size.x > max_size.x {
                size *= max_size.x / size.x;
            }
            if size.y > max_size.y {
                size *= max_size.y / size.y;
            }
            ui::TextureBox::new(&self.geng, &self.assets, texture, uv)
                .fixed_size(size.map(|x| x as f64))
        };

        let selected_block: Box<dyn geng::ui::Widget> = self
            .selected_block()
            .map_or(Box::new(geng::ui::Void), |block| Box::new(block_ui(&block)));

        let mut stack = geng::ui::stack![
            room_info
                .fixed_size(framebuffer_size.map(|x| x as f64) * vec2(0.2, 0.2))
                .align(vec2(1.0, 1.0)),
            geng::ui::row(tabs)
                .align(vec2(0.0, 1.0))
                .padding_left(framebuffer_size.x as f64 * 0.02),
            selected_block
                .align(vec2(1.0, 0.0))
                .uniform_padding(framebuffer_size.y as f64 * 0.05),
        ];

        if let Some(ui) = block_info {
            stack.push(ui);
        }

        Box::new(stack)
    }
}

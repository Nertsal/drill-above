use super::*;

impl Editor {
    pub fn ui<'a>(&'a mut self, cx: &'a geng::ui::Controller) -> Box<dyn geng::ui::Widget + 'a> {
        use geng::ui::*;

        let framebuffer_size = self.framebuffer_size.map(|x| x as f32);

        // let (cell_pos, cell_offset) = self.level.grid.world_to_grid(self.cursor_world_pos);
        // let cell_pos = Text::new(
        //     format!(
        //         "({}, {}) + ({:.1}, {:.1})",
        //         cell_pos.x, cell_pos.y, cell_offset.x, cell_offset.y
        //     ),
        //     self.geng.default_font(),
        //     framebuffer_size.y * 0.05,
        //     Rgba::WHITE,
        // );

        let text_size = framebuffer_size.y * 0.05;
        let mut level_info = geng::ui::column![
            Text::new(
                self.level_name.clone(),
                self.geng.default_font().clone(),
                text_size,
                Rgba::WHITE
            ),
            {
                let button = Button::new(cx, "Save");
                if button.was_clicked() {
                    self.save_level();
                }
                button.padding_bottom(text_size.into())
            },
            {
                let button = Button::new(cx, "Playtest");
                if button.was_clicked() {
                    self.playtest = true;
                }
                button.padding_bottom(text_size.into())
            },
        ];

        let mut update_geometry = false;
        if let Some(tab) = self.tabs.get(self.active_tab) {
            if let EditorMode::Level = tab.mode {
                level_info.extend([
                    Box::new(geng::ui::row![
                        Text::new(
                            format!("width: {}", self.level.size.x),
                            self.geng.default_font().clone(),
                            text_size,
                            Rgba::WHITE
                        )
                        .padding_right(text_size.into()),
                        {
                            let inc = Button::new(cx, "+");
                            if inc.was_clicked() {
                                self.level
                                    .change_size(self.level.size + vec2(1, 0), &self.assets);
                                update_geometry = true;
                            }
                            inc.padding_right(text_size.into())
                        },
                        {
                            let dec = Button::new(cx, "-");
                            if dec.was_clicked() {
                                self.level
                                    .change_size(self.level.size - vec2(1, 0), &self.assets);
                                update_geometry = true;
                            }
                            dec.padding_right(text_size.into())
                        },
                    ]) as Box<dyn Widget>,
                    Box::new(geng::ui::row![
                        Text::new(
                            format!("height: {}", self.level.size.y),
                            self.geng.default_font().clone(),
                            text_size,
                            Rgba::WHITE
                        )
                        .padding_right(text_size.into()),
                        {
                            let inc = Button::new(cx, "+");
                            if inc.was_clicked() {
                                self.level
                                    .change_size(self.level.size + vec2(0, 1), &self.assets);
                                update_geometry = true;
                            }
                            inc.padding_right(text_size.into())
                        },
                        {
                            let dec = Button::new(cx, "-");
                            if dec.was_clicked() {
                                self.level
                                    .change_size(self.level.size - vec2(0, 1), &self.assets);
                                update_geometry = true;
                            }
                            dec.padding_right(text_size.into())
                        },
                    ]),
                    Box::new(Text::new(
                        "Translate",
                        self.geng.default_font().clone(),
                        text_size,
                        Rgba::WHITE,
                    )),
                    Box::new(geng::ui::row![
                        Text::new(
                            "Horizontal",
                            self.geng.default_font().clone(),
                            text_size,
                            Rgba::WHITE,
                        )
                        .padding_right(text_size.into()),
                        {
                            let left = Button::new(cx, "left");
                            if left.was_clicked() {
                                self.level.translate(vec2(-1, 0), &self.assets);
                                update_geometry = true;
                            }
                            left.padding_right(text_size.into())
                        },
                        {
                            let right = Button::new(cx, "right");
                            if right.was_clicked() {
                                self.level.translate(vec2(1, 0), &self.assets);
                                update_geometry = true;
                            }
                            right.padding_right(text_size.into())
                        },
                    ]),
                    Box::new(geng::ui::row![
                        Text::new(
                            "Vertical",
                            self.geng.default_font().clone(),
                            text_size,
                            Rgba::WHITE,
                        )
                        .padding_right(text_size.into()),
                        {
                            let down = Button::new(cx, "down");
                            if down.was_clicked() {
                                self.level.translate(vec2(0, -1), &self.assets);
                                update_geometry = true;
                            }
                            down.padding_right(text_size.into())
                        },
                        {
                            let up = Button::new(cx, "up");
                            if up.was_clicked() {
                                self.level.translate(vec2(0, 1), &self.assets);
                                update_geometry = true;
                            }
                            up.padding_right(text_size.into())
                        },
                    ]),
                ]);
            }
        }

        if update_geometry {
            self.update_geometry();
        }

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
                PlaceableType::Hazard(hazard) => {
                    (self.assets.sprites.hazards.get_texture(hazard), unit)
                }
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
            level_info.align(vec2(1.0, 1.0)),
            geng::ui::row(tabs)
                .align(vec2(0.0, 1.0))
                .padding_left(framebuffer_size.x as f64 * 0.02),
            selected_block
                .align(vec2(1.0, 0.0))
                .uniform_padding(framebuffer_size.y as f64 * 0.05),
        ];

        let text_size = framebuffer_size.y * 0.03;
        // let font = &self.assets.font;
        let font = self.geng.default_font();
        let slider =
            |name, range, value: &mut f32| ui::slider(cx, name, value, range, font, text_size);

        if let Some(tab) = &mut self.tabs.get_mut(self.active_tab) {
            if let EditorMode::Lights = &mut tab.mode {
                if let Some(config) = self.selected_block.and_then(|id| {
                    if let PlaceableId::Spotlight(id) = id {
                        self.level.spotlights.get_mut(id)
                    } else {
                        None
                    }
                }) {
                    // Spotlight
                    let angle = slider("Direction", 0.0..=f64::PI * 2.0, &mut config.angle);
                    let angle_range = slider("Angle", 0.0..=f64::PI * 2.0, &mut config.angle_range);
                    let color = ui::color_selector(
                        cx,
                        &mut config.color,
                        &mut self.light_float_scale,
                        &mut self.light_hsv,
                        font,
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

                    let light = geng::ui::stack![
                        geng::ui::ColorBox::new(Rgba::new(0.0, 0.0, 0.0, 0.5)),
                        geng::ui::column![
                            angle,
                            angle_range,
                            angle_gradient,
                            color,
                            intensity,
                            max_distance,
                            distance_gradient,
                            volume,
                        ]
                    ]
                    .fixed_size(framebuffer_size.map(|x| x as f64) * vec2(0.2, 0.7))
                    .align(vec2(1.0, 0.5))
                    .uniform_padding(framebuffer_size.x as f64 * 0.05);
                    stack.push(Box::new(light));
                } else {
                    // Global light
                    let config = &mut self.level.global_light;
                    let color = ui::color_selector(
                        cx,
                        &mut config.color,
                        &mut self.light_float_scale,
                        &mut self.light_hsv,
                        font,
                        text_size,
                    );
                    let intensity = slider("Intensity", 0.0..=1.0, &mut config.intensity);

                    let light = geng::ui::stack![
                        geng::ui::ColorBox::new(Rgba::new(0.0, 0.0, 0.0, 0.5)),
                        geng::ui::column![color, intensity],
                    ]
                    .fixed_size(framebuffer_size.map(|x| x as f64) * vec2(0.2, 0.5))
                    .align(vec2(1.0, 0.5))
                    .uniform_padding(framebuffer_size.x as f64 * 0.05);
                    stack.push(Box::new(light));
                }
            }
        }

        Box::new(stack)
    }
}

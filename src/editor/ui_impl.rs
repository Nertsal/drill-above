use super::*;

impl Editor {
    pub fn ui<'a>(&'a mut self, cx: &'a geng::ui::Controller) -> Box<dyn geng::ui::Widget + 'a> {
        use geng::ui::*;

        let framebuffer_size = self.framebuffer_size.map(|x| x as f32);

        let (cell_pos, cell_offset) = self.level.grid.world_to_grid(self.cursor_world_pos);
        let cell_pos = Text::new(
            format!(
                "({}, {}) + ({:.1}, {:.1})",
                cell_pos.x, cell_pos.y, cell_offset.x, cell_offset.y
            ),
            self.geng.default_font(),
            framebuffer_size.y * 0.05,
            Rgba::WHITE,
        );

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

        let block_ui = |block: &BlockType| {
            let unit = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)].map(|(x, y)| vec2(x, y));
            let (texture, uv) = match block {
                BlockType::Tile(tile) => {
                    let set = self.assets.sprites.tiles.get_tile_set(tile);
                    (set.texture(), set.get_tile_connected([Connection::None; 8]))
                }
                BlockType::Hazard(hazard) => {
                    (self.assets.sprites.hazards.get_texture(hazard), unit)
                }
                BlockType::Coin => (&self.assets.sprites.coin, unit),
                BlockType::Prop(prop) => (self.assets.sprites.props.get_texture(prop), unit),
                BlockType::Spotlight(..) => (&self.assets.sprites.spotlight, unit),
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
            cell_pos.align(vec2(1.0, 1.0)),
            geng::ui::row(tabs)
                .align(vec2(0.0, 1.0))
                .padding_left(framebuffer_size.x as f64 * 0.02),
            selected_block
                .align(vec2(1.0, 0.0))
                .uniform_padding(framebuffer_size.y as f64 * 0.05),
        ];

        if let Some(tab) = &mut self.tabs.get_mut(self.active_tab) {
            if let EditorMode::Spotlight { config } = &mut tab.mode {
                let text_size = framebuffer_size.y * 0.03;
                let font = &self.assets.font;

                let slider = |name, range, value: &mut f32| {
                    let slider = ui::Slider::new(cx, (*value).into(), range);
                    if let Some(change) = slider.get_change() {
                        *value = change as f32;
                    }
                    geng::ui::row![
                        geng::ui::Text::new(name, font, text_size, Rgba::WHITE),
                        slider
                    ]
                };

                // pub position: vec2<Coord>,
                // pub angle: f32,
                // pub angle_range: f32,
                // pub color: Rgba<f32>,
                // pub intensity: f32,
                // pub max_distance: Coord,
                // pub volume: f32,
                let angle = slider("Direction", 0.0..=f64::PI * 2.0, &mut config.angle);
                let angle_range = slider("Angle", 0.0..=f64::PI * 2.0, &mut config.angle_range);
                let color = geng::ui::Void; // TODO
                let intensity = slider("Intensity", 0.0..=1.0, &mut config.intensity);
                let max_distance = {
                    let mut d = config.max_distance.as_f32();
                    let slider = slider("Distance", 0.0..=50.0, &mut d);
                    config.max_distance = Coord::new(d);
                    slider
                };
                let volume = slider("Volume", 0.0..=1.0, &mut config.volume);

                let light = geng::ui::stack![
                    geng::ui::ColorBox::new(Rgba::new(0.0, 0.0, 0.0, 0.5)),
                    geng::ui::column![angle, angle_range, color, intensity, max_distance, volume]
                ]
                .fixed_size(framebuffer_size.map(|x| x as f64) * vec2(0.2, 0.5))
                .align(vec2(1.0, 0.5))
                .uniform_padding(framebuffer_size.x as f64 * 0.05);
                stack.push(Box::new(light));
            }
        }

        Box::new(stack)
    }
}

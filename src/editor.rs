use super::*;

const CAMERA_MOVE_SPEED: f32 = 20.0;

pub struct Editor {
    geng: Geng,
    assets: Rc<Assets>,
    render: Render,
    camera: Camera2d,
    framebuffer_size: Vec2<usize>,
    level_name: String,
    level: Level,
    draw_grid: bool,
    cursor_pos: Vec2<f64>,
    cursor_world_pos: Vec2<Coord>,
    dragging: Option<geng::MouseButton>,
    tabs: Vec<EditorTab>,
    active_tab: usize,
}

#[derive(Debug, Clone)]
struct EditorTab {
    pub name: String,
    pub blocks: Vec<Block>,
    pub selected: usize,
}

#[derive(Debug, Clone, Copy)]
enum Block {
    Tile(Tile),
    Hazard(HazardType),
    Prop(PropType),
    Coin,
}

impl Editor {
    pub fn new(geng: &Geng, assets: &Rc<Assets>, level_name: Option<String>) -> Self {
        let level_name = level_name.unwrap_or_else(|| "new_level.json".to_string());
        Self {
            geng: geng.clone(),
            assets: assets.clone(),
            render: Render::new(geng, assets),
            camera: Camera2d {
                center: vec2(0.0, 0.25),
                rotation: 0.0,
                fov: 22.5,
            },
            framebuffer_size: vec2(1, 1),
            level: util::report_err(Level::load(&level_name), "Failed to load level")
                .unwrap_or_default(),
            level_name,
            draw_grid: true,
            cursor_pos: Vec2::ZERO,
            cursor_world_pos: Vec2::ZERO,
            dragging: None,
            tabs: vec![
                EditorTab::new(
                    "Tiles",
                    Tile::all()
                        .into_iter()
                        .filter(|tile| !matches!(tile, Tile::Air))
                        .map(Block::Tile)
                        .collect(),
                ),
                EditorTab::new("Collectables", vec![Block::Coin]),
                EditorTab::new(
                    "Hazards",
                    HazardType::all().into_iter().map(Block::Hazard).collect(),
                ),
                EditorTab::new(
                    "Props",
                    PropType::all().into_iter().map(Block::Prop).collect(),
                ),
            ],
            active_tab: 0,
        }
    }

    fn scroll_selected(&mut self, delta: isize) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
            let current = tab.selected as isize;
            let target = current + delta;
            tab.selected = target.rem_euclid(tab.blocks.len() as isize) as usize;
        }
    }

    fn selected_block(&self) -> Option<Block> {
        self.tabs
            .get(self.active_tab)
            .and_then(|tab| tab.blocks.get(tab.selected))
            .copied()
    }

    fn place_block(&mut self) {
        let pos = self.level.grid.world_to_grid(self.cursor_world_pos).0;
        if let Some(block) = self.selected_block() {
            match block {
                Block::Tile(tile) => {
                    self.level.tiles.set_tile_isize(pos, tile);
                }
                Block::Hazard(hazard) => {
                    self.level.place_hazard(pos, hazard);
                }
                Block::Coin => {
                    self.level.place_coin(pos);
                }
                Block::Prop(prop) => {
                    let size = self
                        .assets
                        .sprites
                        .props
                        .get_texture(&prop)
                        .size()
                        .map(|x| x as f32 / PIXELS_PER_UNIT)
                        .map(Coord::new);
                    self.level.place_prop(pos, size, prop);
                }
            }
        }
    }

    fn remove_block(&mut self) {
        self.level.remove_all_at(self.cursor_world_pos);
    }

    fn update_cursor(&mut self, cursor_pos: Vec2<f64>) {
        self.cursor_pos = cursor_pos;
        self.cursor_world_pos = self
            .camera
            .screen_to_world(
                self.framebuffer_size.map(|x| x as f32),
                cursor_pos.map(|x| x as f32),
            )
            .map(Coord::new);

        if let Some(button) = self.dragging {
            match button {
                geng::MouseButton::Left => {
                    self.place_block();
                }
                geng::MouseButton::Right => {
                    self.remove_block();
                }
                geng::MouseButton::Middle => {}
            }
        }
    }

    fn click(&mut self, position: Vec2<f64>, button: geng::MouseButton) {
        self.update_cursor(position);
        self.dragging = Some(button);

        match button {
            geng::MouseButton::Left => {
                self.place_block();
            }
            geng::MouseButton::Right => {
                self.remove_block();
            }
            _ => (),
        }
    }

    fn release(&mut self, _button: geng::MouseButton) {
        self.dragging = None;
    }

    fn save_level(&self) -> anyhow::Result<()> {
        self.level.save(&self.level_name)
    }
}

impl geng::State for Editor {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.framebuffer_size = framebuffer.size();
        let color = Rgba::try_from("#341a22").unwrap();
        ugli::clear(framebuffer, Some(color), None, None);

        self.render
            .draw_level_editor(&self.level, true, &self.camera, framebuffer);

        if self.draw_grid {
            self.render
                .draw_grid(&self.level.grid, self.level.size, &self.camera, framebuffer);
        }
    }

    fn update(&mut self, delta_time: f64) {
        let delta_time = delta_time as f32;
        let window = self.geng.window();
        let mut dir = Vec2::ZERO;
        if window.is_key_pressed(geng::Key::A) {
            dir.x -= 1.0;
        }
        if window.is_key_pressed(geng::Key::D) {
            dir.x += 1.0;
        }
        if window.is_key_pressed(geng::Key::S) {
            dir.y -= 1.0;
        }
        if window.is_key_pressed(geng::Key::W) {
            dir.y += 1.0;
        }
        self.camera.center += dir * CAMERA_MOVE_SPEED * delta_time;
    }

    fn handle_event(&mut self, event: geng::Event) {
        match event {
            geng::Event::MouseDown { position, button } => {
                self.click(position, button);
            }
            geng::Event::MouseMove { position, .. } => {
                self.update_cursor(position);
            }
            geng::Event::MouseUp { button, .. } => {
                // self.update_cursor(position);
                self.release(button);
            }
            geng::Event::Wheel { delta } => {
                self.scroll_selected(delta.signum() as isize);
            }
            geng::Event::KeyDown { key } => match key {
                geng::Key::S if self.geng.window().is_key_pressed(geng::Key::LCtrl) => {
                    if let Ok(()) = util::report_err(self.save_level(), "Failed to save level") {
                        info!("Saved the level");
                    }
                }
                geng::Key::R => {
                    self.level.spawn_point = self.cursor_world_pos;
                }
                geng::Key::F => {
                    self.level.finish = self.cursor_world_pos;
                }
                geng::Key::Left => {
                    self.scroll_selected(-1);
                }
                geng::Key::Right => {
                    self.scroll_selected(1);
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn ui<'a>(&'a mut self, cx: &'a geng::ui::Controller) -> Box<dyn geng::ui::Widget + 'a> {
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
                    Rgba::from_rgb(0.1, 0.1, 0.3)
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

        let block_ui = |block: &Block| {
            let unit = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)].map(|(x, y)| vec2(x, y));
            let (texture, uv) = match block {
                Block::Tile(tile) => {
                    let set = self.assets.sprites.tiles.get_tile_set(tile);
                    (set.texture(), set.get_tile_connected([false; 8]))
                }
                Block::Hazard(hazard) => (self.assets.sprites.hazards.get_texture(hazard), unit),
                Block::Coin => (&self.assets.sprites.coin, unit),
                Block::Prop(prop) => (self.assets.sprites.props.get_texture(prop), unit),
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

        let ui = geng::ui::stack![
            cell_pos.align(vec2(1.0, 1.0)),
            geng::ui::row(tabs)
                .align(vec2(0.0, 1.0))
                .padding_left(framebuffer_size.x as f64 * 0.02),
            selected_block
                .align(vec2(1.0, 0.0))
                .uniform_padding(framebuffer_size.y as f64 * 0.05),
        ];

        Box::new(ui)
    }
}

impl EditorTab {
    pub fn new(name: impl Into<String>, blocks: Vec<Block>) -> Self {
        Self {
            selected: 0,
            name: name.into(),
            blocks,
        }
    }
}

pub fn run(geng: &Geng, level: Option<String>) -> impl geng::State {
    let future = {
        let geng = geng.clone();
        async move {
            let assets: Rc<Assets> = geng::LoadAsset::load(&geng, &run_dir().join("assets"))
                .await
                .expect("Failed to load assets");
            Editor::new(&geng, &assets, level)
        }
    };
    geng::LoadingScreen::new(geng, geng::EmptyLoadingScreen, future, |state| state)
}

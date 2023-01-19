use super::*;

mod action;
mod ui_impl;

use action::*;

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
    undo_actions: Vec<Action>,
    redo_actions: Vec<Action>,
}

#[derive(Debug, Clone)]
struct EditorTab {
    pub name: String,
    pub blocks: Vec<BlockType>,
    pub selected: usize,
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
                        .map(BlockType::Tile)
                        .collect(),
                ),
                EditorTab::new("Collectables", vec![BlockType::Coin]),
                EditorTab::new(
                    "Hazards",
                    HazardType::all()
                        .into_iter()
                        .map(BlockType::Hazard)
                        .collect(),
                ),
                EditorTab::new(
                    "Props",
                    PropType::all().into_iter().map(BlockType::Prop).collect(),
                ),
            ],
            active_tab: 0,
            undo_actions: default(),
            redo_actions: default(),
        }
    }

    fn scroll_selected(&mut self, delta: isize) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
            let current = tab.selected as isize;
            let target = current + delta;
            tab.selected = target.rem_euclid(tab.blocks.len() as isize) as usize;
        }
    }

    fn selected_block(&self) -> Option<BlockType> {
        self.tabs
            .get(self.active_tab)
            .and_then(|tab| tab.blocks.get(tab.selected))
            .copied()
    }

    fn place_block(&mut self) {
        if let Some(block) = self.selected_block() {
            self.action(Action::Place {
                block,
                pos: self.cursor_world_pos,
            });
        }
    }

    fn remove_block(&mut self) {
        self.action(Action::Remove {
            pos: self.cursor_world_pos,
        });
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
                geng::Key::Z if self.geng.window().is_key_pressed(geng::Key::LCtrl) => {
                    if self.geng.window().is_key_pressed(geng::Key::LShift) {
                        self.redo();
                    } else {
                        self.undo();
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
        self.ui(cx)
    }
}

impl EditorTab {
    pub fn new(name: impl Into<String>, blocks: Vec<BlockType>) -> Self {
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

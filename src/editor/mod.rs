use super::*;

mod action;
mod ui_impl;

use action::*;

const CAMERA_MOVE_SPEED: f32 = 20.0;

struct Render {
    world: WorldRender,
    lights: LightsRender,
    util: UtilRender,
}

pub struct Editor {
    geng: Geng,
    assets: Rc<Assets>,
    render: Render,
    camera: Camera2d,
    framebuffer_size: vec2<usize>,
    level_name: String,
    level: Level,
    draw_grid: bool,
    cursor_pos: vec2<f64>,
    cursor_world_pos: vec2<Coord>,
    dragging: Option<Dragging>,
    selected_block: Option<BlockId>,
    tabs: Vec<EditorTab>,
    active_tab: usize,
    undo_actions: Vec<Action>,
    redo_actions: Vec<Action>,
    hovered: Vec<BlockId>,
}

#[derive(Debug)]
struct Dragging {
    pub initial_cursor_pos: vec2<f64>,
    pub initial_world_pos: vec2<Coord>,
    pub action: Option<DragAction>,
}

#[derive(Debug)]
enum DragAction {
    PlaceTile,
    RemoveTile,
    MoveBlock {
        id: BlockId,
        initial_pos: vec2<Coord>,
    },
}

#[derive(Debug, Clone)]
struct EditorTab {
    pub name: String,
    pub hoverable: Vec<BlockType>,
    pub mode: EditorMode,
}

#[derive(Debug, Clone)]
enum EditorMode {
    Block {
        blocks: Vec<BlockType>,
        selected: usize,
    },
    Spotlight {
        config: SpotlightSource,
    },
}

impl Editor {
    pub fn new(geng: &Geng, assets: &Rc<Assets>, level_name: Option<String>) -> Self {
        let level_name = level_name.unwrap_or_else(|| "new_level.json".to_string());
        Self {
            geng: geng.clone(),
            assets: assets.clone(),
            render: Render {
                world: WorldRender::new(geng, assets),
                lights: LightsRender::new(geng, assets),
                util: UtilRender::new(geng, assets),
            },
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
            cursor_pos: vec2::ZERO,
            cursor_world_pos: vec2::ZERO,
            dragging: None,
            selected_block: None,
            tabs: vec![
                EditorTab::block(
                    "Tiles",
                    Tile::all()
                        .into_iter()
                        .filter(|tile| !matches!(tile, Tile::Air))
                        .map(BlockType::Tile)
                        .collect(),
                ),
                EditorTab::block("Collectables", vec![BlockType::Coin]),
                EditorTab::block(
                    "Hazards",
                    HazardType::all()
                        .into_iter()
                        .map(BlockType::Hazard)
                        .collect(),
                ),
                EditorTab::block(
                    "Props",
                    PropType::all().into_iter().map(BlockType::Prop).collect(),
                ),
                EditorTab {
                    name: "Lights".into(),
                    hoverable: vec![BlockType::Spotlight(default())],
                    mode: EditorMode::Spotlight { config: default() },
                },
            ],
            active_tab: 0,
            undo_actions: default(),
            redo_actions: default(),
            hovered: Vec::new(),
        }
    }

    fn scroll_selected(&mut self, delta: isize) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
            if let EditorMode::Block { selected, blocks } = &mut tab.mode {
                let current = *selected as isize;
                let target = current + delta;
                *selected = target.rem_euclid(blocks.len() as isize) as usize;
            }
        }
    }

    fn selected_block(&self) -> Option<BlockType> {
        self.tabs
            .get(self.active_tab)
            .and_then(|tab| match &tab.mode {
                EditorMode::Block { blocks, selected } => blocks.get(*selected).copied(),
                EditorMode::Spotlight { config } => Some(BlockType::Spotlight(*config)),
            })
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

    fn move_block(&mut self, id: BlockId, pos: vec2<Coord>) {
        match id {
            BlockId::Tile(_) => unimplemented!(),
            BlockId::Hazard(id) => {
                if let Some(hazard) = self.level.hazards.get_mut(id) {
                    hazard.teleport(pos);
                }
            }
            BlockId::Prop(id) => {
                if let Some(prop) = self.level.props.get_mut(id) {
                    prop.teleport(pos);
                }
            }
            BlockId::Coin(id) => {
                if let Some(coin) = self.level.coins.get_mut(id) {
                    coin.teleport(pos);
                }
            }
            BlockId::Spotlight(id) => {
                if let Some(light) = self.level.spotlights.get_mut(id) {
                    light.position = pos;
                }
            }
        }
    }

    fn select_block(&mut self, id: BlockId) {
        self.selected_block = Some(id);
        let Some(block) = self.level.get_block(id) else {
            return;
        };
        if let Block::Spotlight(light) = block {
            if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                if let EditorMode::Spotlight { config } = &mut tab.mode {
                    *config = light;
                }
            }
        }
    }

    fn update_selected_block(&mut self) {
        let Some(id) = self.selected_block else {
            return;
        };
        if let BlockId::Spotlight(id) = id {
            if let Some(light) = self.level.spotlights.get_mut(id) {
                if let Some(tab) = self.tabs.get(self.active_tab) {
                    if let &EditorMode::Spotlight { config } = &tab.mode {
                        *light = config;
                    }
                }
            }
        }
    }

    fn update_cursor(&mut self, cursor_pos: vec2<f64>) {
        self.cursor_pos = cursor_pos;
        self.cursor_world_pos = self
            .camera
            .screen_to_world(
                self.framebuffer_size.map(|x| x as f32),
                cursor_pos.map(|x| x as f32),
            )
            .map(Coord::new);

        self.hovered = self.level.get_hovered(self.cursor_world_pos);
        if let Some(tab) = &self.tabs.get(self.active_tab) {
            self.hovered
                .retain(|id| tab.hoverable.iter().any(|&ty| id.fits_type(ty)))
        }

        if let Some(dragging) = &self.dragging {
            if let Some(action) = &dragging.action {
                match action {
                    DragAction::PlaceTile => self.place_block(),
                    DragAction::RemoveTile => self.remove_block(),
                    &DragAction::MoveBlock { id, initial_pos } => self.move_block(
                        id,
                        initial_pos + self.cursor_world_pos - dragging.initial_world_pos,
                    ),
                }
            }
        }
    }

    fn click(&mut self, position: vec2<f64>, button: geng::MouseButton) {
        self.release(button);
        self.update_cursor(position);

        let action = match button {
            geng::MouseButton::Left => {
                if let Some(BlockType::Tile(_)) = self.selected_block() {
                    Some(DragAction::PlaceTile)
                } else if let Some(&id) = self.hovered.first() {
                    self.level.get_block(id).map(|block| DragAction::MoveBlock {
                        id,
                        initial_pos: block.position(),
                    })
                } else {
                    None
                }
            }
            geng::MouseButton::Right => {
                if let Some(BlockType::Tile(_)) = self.selected_block() {
                    Some(DragAction::RemoveTile)
                } else {
                    None
                }
            }
            geng::MouseButton::Middle => None,
        };

        self.selected_block = None;
        self.dragging = Some(Dragging {
            initial_cursor_pos: position,
            initial_world_pos: self.cursor_world_pos,
            action,
        });

        self.update_cursor(position);
    }

    fn release(&mut self, button: geng::MouseButton) {
        if let Some(dragging) = self.dragging.take() {
            if dragging.initial_cursor_pos == self.cursor_pos {
                // Click
                match button {
                    geng::MouseButton::Left => {
                        if let Some(&id) = self.hovered.first() {
                            self.select_block(id);
                        } else {
                            self.place_block()
                        }
                    }
                    geng::MouseButton::Right => self.remove_block(),
                    geng::MouseButton::Middle => {}
                }
            }
        }
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

        // Draw the world and normals ignoring lighting
        let (mut world_framebuffer, _normal_framebuffer) =
            self.render.lights.start_render(framebuffer);

        // Render level
        self.render.world.draw_level_editor(
            &self.level,
            true,
            &self.camera,
            &mut world_framebuffer,
        );

        self.render
            .lights
            .finish_render(&self.level, &self.camera, framebuffer);

        // Draw hovered
        let mut colliders = Vec::new();
        for &block in itertools::chain![&self.hovered, &self.selected_block] {
            match block {
                BlockId::Tile(_) => {}
                BlockId::Hazard(id) => {
                    let hazard = &self.level.hazards[id];
                    colliders.push((hazard.collider, Rgba::new(1.0, 0.0, 0.0, 0.5)));
                }
                BlockId::Prop(id) => {
                    let prop = &self.level.props[id];
                    colliders.push((Collider::new(prop.sprite), Rgba::new(1.0, 1.0, 1.0, 0.5)));
                }
                BlockId::Coin(id) => {
                    let coin = &self.level.coins[id];
                    colliders.push((coin.collider, Rgba::new(1.0, 1.0, 0.0, 0.5)));
                }
                BlockId::Spotlight(id) => {
                    let light = &self.level.spotlights[id];
                    let collider =
                        Collider::new(Aabb2::point(light.position).extend_uniform(Coord::new(0.5)));
                    let mut color = light.color;
                    color.a = 0.5;
                    colliders.push((collider, color));
                }
            }
        }
        for (collider, color) in colliders {
            self.render
                .util
                .draw_collider(&collider, color, &self.camera, framebuffer);
        }

        if self.draw_grid {
            self.render.util.draw_grid(
                &self.level.grid,
                self.level.size,
                &self.camera,
                framebuffer,
            );
        }
    }

    fn update(&mut self, delta_time: f64) {
        let delta_time = delta_time as f32;
        let window = self.geng.window();
        let mut dir = vec2::ZERO;
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

        self.update_selected_block();
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
    pub fn block(name: impl Into<String>, blocks: Vec<BlockType>) -> Self {
        Self {
            name: name.into(),
            hoverable: blocks.clone(),
            mode: EditorMode::Block {
                selected: 0,
                blocks,
            },
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

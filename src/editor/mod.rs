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
    pixel_texture: ugli::Texture,
    render: Render,
    camera: Camera2d,
    framebuffer_size: vec2<usize>,
    level_name: String,
    level: Level,
    geometry: (
        HashMap<Tile, ugli::VertexBuffer<Vertex>>,
        HashMap<Tile, ugli::VertexBuffer<MaskedVertex>>,
    ),
    light_geometry: ugli::VertexBuffer<NormalVertex>,
    normal_geometry: Vec<ugli::VertexBuffer<NormalVertex>>,
    draw_grid: bool,
    cursor_pos: vec2<f64>,
    cursor_world_pos: vec2<Coord>,
    dragging: Option<Dragging>,
    selected_block: Option<PlaceableId>,
    tabs: Vec<EditorTab>,
    active_tab: usize,
    undo_actions: Vec<Action>,
    redo_actions: Vec<Action>,
    hovered: Vec<PlaceableId>,
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
        id: PlaceableId,
        initial_pos: vec2<Coord>,
    },
}

#[derive(Debug, Clone)]
struct EditorTab {
    pub name: String,
    pub hoverable: Vec<PlaceableType>,
    pub mode: EditorMode,
}

#[derive(Debug, Clone)]
enum EditorMode {
    Level,
    Block {
        blocks: Vec<PlaceableType>,
        selected: usize,
    },
    Lights,
}

impl Editor {
    pub fn new(geng: &Geng, assets: &Rc<Assets>, level_name: Option<String>) -> Self {
        let level_name = level_name.unwrap_or_else(|| "new_level.json".to_string());
        let level =
            util::report_err(Level::load(&level_name), "Failed to load level").unwrap_or_default();
        Self {
            geng: geng.clone(),
            assets: assets.clone(),
            pixel_texture: {
                let mut texture =
                    ugli::Texture::new_with(geng.ugli(), SCREEN_RESOLUTION, |_| Rgba::BLACK);
                texture.set_filter(ugli::Filter::Nearest);
                texture
            },
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
            geometry: level.calculate_geometry(geng, assets),
            light_geometry: level.calculate_light_geometry(geng),
            normal_geometry: level.calculate_normal_geometry(geng),
            draw_grid: true,
            cursor_pos: vec2::ZERO,
            cursor_world_pos: vec2::ZERO,
            dragging: None,
            selected_block: None,
            tabs: vec![
                EditorTab {
                    name: "Level".into(),
                    hoverable: vec![],
                    mode: EditorMode::Level,
                },
                EditorTab::block(
                    "Tiles",
                    Tile::all()
                        .into_iter()
                        .filter(|tile| !matches!(tile, Tile::Air))
                        .map(PlaceableType::Tile)
                        .collect(),
                ),
                EditorTab::block("Collectables", vec![PlaceableType::Coin]),
                EditorTab::block(
                    "Hazards",
                    HazardType::all()
                        .into_iter()
                        .map(PlaceableType::Hazard)
                        .collect(),
                ),
                EditorTab::block(
                    "Props",
                    PropType::all()
                        .into_iter()
                        .map(PlaceableType::Prop)
                        .collect(),
                ),
                EditorTab {
                    name: "Lights".into(),
                    hoverable: vec![PlaceableType::Spotlight(default())],
                    mode: EditorMode::Lights,
                },
            ],
            active_tab: 0,
            undo_actions: default(),
            redo_actions: default(),
            hovered: Vec::new(),
            level,
            level_name,
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

    fn selected_block(&self) -> Option<PlaceableType> {
        self.tabs
            .get(self.active_tab)
            .and_then(|tab| match &tab.mode {
                EditorMode::Level => None,
                EditorMode::Block { blocks, selected } => blocks.get(*selected).copied(),
                EditorMode::Lights => Some(PlaceableType::Spotlight(default())),
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

    fn move_block(&mut self, id: PlaceableId, pos: vec2<Coord>) {
        match id {
            PlaceableId::Tile(_) => unimplemented!(),
            PlaceableId::Hazard(id) => {
                if let Some(hazard) = self.level.hazards.get_mut(id) {
                    hazard.teleport(pos);
                }
            }
            PlaceableId::Prop(id) => {
                if let Some(prop) = self.level.props.get_mut(id) {
                    prop.teleport(pos);
                }
            }
            PlaceableId::Coin(id) => {
                if let Some(coin) = self.level.coins.get_mut(id) {
                    coin.teleport(pos);
                }
            }
            PlaceableId::Spotlight(id) => {
                if let Some(light) = self.level.spotlights.get_mut(id) {
                    light.position = pos;
                }
            }
        }
    }

    fn select_block(&mut self, id: PlaceableId) {
        self.selected_block = Some(id);
        let Some(_block) = self.level.get_block(id) else {
            return;
        };
    }

    fn update_selected_block(&mut self) {
        let Some(_id) = self.selected_block else {
            return;
        };
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
                if let Some(PlaceableType::Tile(_)) = self.selected_block() {
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
                if let Some(PlaceableType::Tile(_)) = self.selected_block() {
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

    fn save_level(&self) {
        if let Ok(()) = util::report_err(self.level.save(&self.level_name), "Failed to save level")
        {
            info!("Saved the level");
        }
    }
}

impl geng::State for Editor {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.framebuffer_size = framebuffer.size();
        let color = Rgba::try_from("#341a22").unwrap();
        ugli::clear(framebuffer, Some(color), None, None);

        // Render the game onto the texture
        let mut pixel_framebuffer = ugli::Framebuffer::new_color(
            self.geng.ugli(),
            ugli::ColorAttachment::Texture(&mut self.pixel_texture),
        );
        ugli::clear(&mut pixel_framebuffer, Some(Rgba::BLACK), None, None);

        // Draw the world and normals ignoring lighting
        let (mut world_framebuffer, _normal_framebuffer) =
            self.render.lights.start_render(&mut pixel_framebuffer);

        // Render level
        self.render.world.draw_level_editor(
            &self.level,
            &self.geometry.0,
            &self.geometry.1,
            true,
            &self.camera,
            &mut world_framebuffer,
        );

        self.render.lights.finish_render(
            &self.level,
            &self.light_geometry,
            &self.normal_geometry,
            &self.camera,
            &mut pixel_framebuffer,
        );

        // Render the texture onto the screen
        let reference_size = vec2(16.0, 9.0);
        let ratio = framebuffer.size().map(|x| x as f32) / reference_size;
        let ratio = ratio.x.min(ratio.y);
        let target_size = reference_size * ratio;
        let target = Aabb2::point(framebuffer.size().map(|x| x as f32) / 2.0)
            .extend_symmetric(target_size / 2.0);
        self.geng.draw_2d(
            framebuffer,
            &geng::PixelPerfectCamera,
            &draw_2d::TexturedQuad::new(target, &self.pixel_texture),
        );

        // Draw hovered
        let mut colliders = Vec::new();
        for &block in itertools::chain![&self.hovered, &self.selected_block] {
            match block {
                PlaceableId::Tile(_) => {}
                PlaceableId::Hazard(id) => {
                    let hazard = &self.level.hazards[id];
                    colliders.push((hazard.collider, Rgba::new(1.0, 0.0, 0.0, 0.5)));
                }
                PlaceableId::Prop(id) => {
                    let prop = &self.level.props[id];
                    colliders.push((Collider::new(prop.sprite), Rgba::new(1.0, 1.0, 1.0, 0.5)));
                }
                PlaceableId::Coin(id) => {
                    let coin = &self.level.coins[id];
                    colliders.push((coin.collider, Rgba::new(1.0, 1.0, 0.0, 0.5)));
                }
                PlaceableId::Spotlight(id) => {
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
                    self.save_level();
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
    pub fn block(name: impl Into<String>, blocks: Vec<PlaceableType>) -> Self {
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

use super::*;

const ROOM_CAMERA_MOVE_SPEED: f32 = 20.0;
const ROOM_FOV_MIN: usize = 10 * PIXELS_PER_UNIT;
const ROOM_FOV_MAX: usize = 70 * PIXELS_PER_UNIT;

/// A combination of all renderers used by the editor.
pub struct RoomRender {
    pub world: WorldRender,
    pub lights: LightsRender,
    pub util: UtilRender,
}

impl RoomRender {
    pub fn new(geng: &Geng, assets: &Rc<Assets>) -> Self {
        Self {
            world: WorldRender::new(geng, assets),
            lights: LightsRender::new(geng, assets),
            util: UtilRender::new(geng, assets),
        }
    }
}

pub struct RoomEditor {
    pub geng: Geng,
    pub assets: Rc<Assets>,
    /// The downscaled texture used for pixel-perfect rendering of the world.
    pub pixel_texture: ugli::Texture,
    /// The renderer used by the editor.
    pub render: RoomRender,
    /// The renderer used for the preview.
    pub preview_render: GameRender,

    pub camera: Camera2d,
    /// Size of the actual screen size of the application.
    pub framebuffer_size: vec2<usize>,
    /// Size of the downscaled version of the screen.
    pub screen_resolution: vec2<usize>,

    /// The name of the currently loaded room.
    pub room_name: String,
    /// The world that contains the room.
    pub world: World,
    /// Currently active layer of the room.
    pub active_layer: ActiveLayer,
    /// Whether we should transition into the playtest state.
    pub playtest: bool,

    /// Current position of the cursor in screen coordinates.
    pub cursor_pos: vec2<f64>,
    /// Current position of the cursor in world coordinates.
    pub cursor_world_pos: vec2<Coord>,
    /// Dragging state (e.g. rectangular selection).
    pub dragging: Option<RoomDragging>,
    /// Currently selected blocks.
    pub selection: HashSet<PlaceableId>,
    /// Currently hovered blocks.
    pub hovered: Vec<PlaceableId>,
    /// Stack of undo actions.
    pub undo_stack: Vec<Room>,
    /// Stack of redo actions.
    pub redo_stack: Vec<Room>,

    /// All available editor tabs.
    pub tabs: Vec<RoomEditorTab>,
    /// The currently active editor tab.
    pub active_tab: usize,

    /// Whether the grid should be rendered.
    pub draw_grid: bool,
    /// Whether the world should be rendered in preview mode or editor mode.
    pub preview: bool,
    /// Whether the color values should be presented in float scale or integer scale.
    pub light_float_scale: bool,
    /// The color mode that is used in color selection: RGB, HSV, or HSL.
    pub color_mode: Option<ColorMode>,
    /// If the text input widget is active, the inputs are saved and forwarded to the widget.
    pub input_events: Option<Vec<geng::Event>>,
}

/// An editor tab.
#[derive(Debug, Clone)]
pub struct RoomEditorTab {
    /// The name of the tab.
    pub name: String,
    /// Possible blocks that can be hovered (and selected),
    /// when this tab is active.
    pub hoverable: Vec<PlaceableType>,
    /// The mode of the tab.
    pub mode: RoomEditorMode,
}

/// Mode of the editor tab.
#[derive(Debug, Clone)]
pub enum RoomEditorMode {
    /// Modify room information.
    /// Also allows to select all blocks in the room,
    /// regardless of the `hoverable` field in the tab.
    Room,
    /// Place blocks.
    Block {
        /// All placeable blocks in that mode.
        blocks: Vec<PlaceableType>,
        /// Currently selected placeable block.
        selected: usize,
    },
    /// Modify global light and other lights in the room.
    Lights { spotlight: SpotlightSource },
}

/// The dragging state in the room editor.
#[derive(Debug)]
pub struct RoomDragging {
    /// Initial cursor positiion in screen coordinates.
    pub initial_cursor_pos: vec2<f64>,
    /// Initial cursor positiion in world coordinates.
    pub initial_world_pos: vec2<Coord>,
    /// The action of the drag (e.g. rectangular selection).
    pub action: Option<RoomDragAction>,
}

/// The action of the drag (e.g. rectangular selection) in the room editor.
#[derive(Debug)]
pub enum RoomDragAction {
    /// Place tile under cursor.
    PlaceTile,
    /// Remove tile under cursor.
    RemoveTile,
    /// Move the specified blocks, respecting their offsets.
    MoveBlocks {
        blocks: Vec<Placeable>,
        /// The initial position used for reference.
        initial_pos: vec2<Coord>,
    },
    /// Select blocks in a rectangle.
    RectSelection,
    MoveCamera {
        initial_camera_pos: vec2<Coord>,
    },
}

impl RoomEditor {
    pub fn new(geng: &Geng, assets: &Rc<Assets>, room_name: String) -> Self {
        // Load the room
        let room =
            util::report_err(Room::load(&room_name), "Failed to load room").unwrap_or_default();
        Self::new_room(geng, assets, room_name, room)
    }

    pub fn new_room(geng: &Geng, assets: &Rc<Assets>, room_name: String, mut room: Room) -> Self {
        // Update geometry in case it was not specified in the json file.
        for layer in room.layers.iter_mut() {
            layer.tiles.update_geometry(assets);
        }
        Self {
            geng: geng.clone(),
            assets: assets.clone(),
            pixel_texture: {
                let mut texture =
                    ugli::Texture::new_with(geng.ugli(), SCREEN_RESOLUTION, |_| Rgba::BLACK);
                texture.set_filter(ugli::Filter::Nearest);
                texture
            },
            render: RoomRender::new(geng, assets),
            preview_render: GameRender::new(geng, assets),
            screen_resolution: SCREEN_RESOLUTION,
            camera: Camera2d {
                center: vec2(0.0, 0.25),
                rotation: 0.0,
                fov: (SCREEN_RESOLUTION.x / PIXELS_PER_UNIT) as f32 * 9.0 / 16.0,
            },
            framebuffer_size: vec2(1, 1),
            world: World::new(geng, assets, assets.rules.clone(), room, None),
            active_layer: ActiveLayer::Main,
            draw_grid: true,
            cursor_pos: vec2::ZERO,
            cursor_world_pos: vec2::ZERO,
            dragging: None,
            selection: default(),
            tabs: vec![
                RoomEditorTab {
                    name: "Room".into(),
                    hoverable: vec![],
                    mode: RoomEditorMode::Room,
                },
                RoomEditorTab::block(
                    "Tiles",
                    assets
                        .rules
                        .tiles
                        .keys()
                        .cloned()
                        .filter(|tile| tile != "air")
                        .map(PlaceableType::Tile)
                        .collect(),
                ),
                RoomEditorTab::block("Collectables", vec![PlaceableType::Coin]),
                RoomEditorTab::block(
                    "Hazards",
                    assets
                        .sprites
                        .hazards
                        .0
                        .keys()
                        .cloned()
                        .map(PlaceableType::Hazard)
                        .collect(),
                ),
                RoomEditorTab::block(
                    "Props",
                    assets
                        .sprites
                        .props
                        .0
                        .keys()
                        .cloned()
                        .map(PlaceableType::Prop)
                        .collect(),
                ),
                RoomEditorTab {
                    name: "Lights".into(),
                    hoverable: vec![PlaceableType::Spotlight(default())],
                    mode: RoomEditorMode::Lights {
                        spotlight: default(),
                    },
                },
                RoomEditorTab::block(
                    "Npc",
                    assets
                        .sprites
                        .npc
                        .0
                        .keys()
                        .cloned()
                        .map(PlaceableType::Npc)
                        .collect(),
                ),
            ],
            active_tab: 0,
            undo_stack: default(),
            redo_stack: default(),
            hovered: Vec::new(),
            light_float_scale: true,
            color_mode: None,
            playtest: false,
            preview: false,
            room_name,
            input_events: None,
        }
    }

    /// Change the currently selected placeable block in the currently active tab.
    fn scroll_selected(&mut self, delta: isize) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
            if let RoomEditorMode::Block { selected, blocks } = &mut tab.mode {
                let current = *selected as isize;
                let target = current + delta;
                *selected = target.rem_euclid(blocks.len() as isize) as usize;
            }
        }
    }

    pub fn change_layer(&mut self, layer: ActiveLayer) {
        if layer == self.active_layer {
            return;
        }

        self.clear_selection();
        self.active_layer = layer;
    }

    /// Get the currently selected placeable block (if any).
    pub fn selected_block(&self) -> Option<PlaceableType> {
        self.tabs
            .get(self.active_tab)
            .and_then(|tab| match &tab.mode {
                RoomEditorMode::Room => None,
                RoomEditorMode::Block { blocks, selected } => blocks.get(*selected).cloned(),
                RoomEditorMode::Lights { spotlight } => Some(PlaceableType::Spotlight(*spotlight)),
            })
    }

    /// Place the currently selected placeable block.
    fn place_block(&mut self) {
        if let Some(block) = self.selected_block() {
            self.action_place(block, self.cursor_world_pos);
        }
    }

    /// Delete all hovered blocks.
    fn remove_hovered(&mut self) {
        self.keep_state();
        self.action_remove(&self.hovered.clone());
    }

    /// Delete all selected blocks.
    pub fn remove_selected(&mut self) {
        self.keep_state();
        self.action_remove(&self.selection.clone());
    }

    /// Clear the selection and reset the color mode.
    fn clear_selection(&mut self) {
        self.selection.clear();
        self.color_mode = None;
    }

    fn update_selected_block(&mut self) {
        for _id in &self.selection {}
    }

    /// Get all blocks inside the `aabb`. The blocks are filtered
    /// by the currently active tab's hovered list (unless it's in `Room` mode).
    pub fn get_hovered(&self, aabb: Aabb2<Coord>) -> Vec<PlaceableId> {
        let mut hovered = self.world.room.get_hovered(aabb, self.active_layer);
        if let Some(tab) = &self.tabs.get(self.active_tab) {
            if let RoomEditorMode::Room = tab.mode {
            } else {
                hovered.retain(|id| tab.hoverable.iter().any(|ty| id.fits_type(ty)))
            }
        }
        hovered
    }

    /// Update the cursor position.
    fn update_cursor(&mut self, cursor_pos: vec2<f64>) {
        self.cursor_pos = cursor_pos;
        // Calculate world position
        self.cursor_world_pos = self
            .camera
            .screen_to_world(
                self.framebuffer_size.map(|x| x as f32),
                cursor_pos.map(|x| x as f32),
            )
            .map(Coord::new);

        // Snap to grid if needed
        let snap_cursor = self.geng.window().is_key_pressed(geng::Key::LCtrl);
        if snap_cursor {
            let snap_size = self.world.room.grid.cell_size / Coord::new(2.0);
            self.cursor_world_pos =
                (self.cursor_world_pos / snap_size).map(|x| x.round()) * snap_size;
        }

        // Update hovered blocks
        self.hovered = self.get_hovered(Aabb2::point(self.cursor_world_pos));

        // Update the dragging state
        if let Some(mut dragging) = self.dragging.take() {
            if let Some(action) = &mut dragging.action {
                match action {
                    RoomDragAction::PlaceTile => self.place_block(),
                    RoomDragAction::RemoveTile => self.remove_hovered(),
                    RoomDragAction::MoveBlocks { .. } => {}
                    RoomDragAction::RectSelection => {}
                    &mut RoomDragAction::MoveCamera { initial_camera_pos } => {
                        let from = self
                            .camera
                            .screen_to_world(
                                self.framebuffer_size.map(|x| x as f32),
                                dragging.initial_cursor_pos.map(|x| x as f32),
                            )
                            .map(Coord::new);
                        self.camera.center =
                            (initial_camera_pos + from - self.cursor_world_pos).map(Coord::as_f32);
                    }
                }
            }
            self.dragging = Some(dragging);
        }
    }

    /// Handle the click event.
    fn click(&mut self, position: vec2<f64>, button: geng::MouseButton) {
        // Release in case the old click was not reset
        self.release(button);
        // Update cursor position
        self.update_cursor(position);

        // Check what action should be performed
        let action = match button {
            geng::MouseButton::Left => (!self.geng.window().is_key_pressed(geng::Key::LShift))
                .then(|| {
                    if matches!(self.selected_block(), Some(PlaceableType::Tile(_)))
                        && (self.selection.is_empty() || self.hovered.is_empty())
                    {
                        self.keep_state();
                        Some(RoomDragAction::PlaceTile)
                    } else if let Some(&id) = self.hovered.first() {
                        // If the block was already selected,
                        // then we move all the selected blocks.
                        // Otherwise, we create a new selection
                        // with the block we just clicked.
                        let ids: Vec<_> = if self.selection.contains(&id) {
                            // Move all selection
                            self.selection.iter().copied().collect()
                        } else {
                            // Create a new selection
                            self.clear_selection();
                            self.selection.insert(id);
                            vec![id]
                        };

                        self.keep_state();
                        let blocks =
                            self.world
                                .room
                                .remove_blocks(&ids, self.active_layer, &self.assets);
                        Some(RoomDragAction::MoveBlocks {
                            blocks,
                            initial_pos: self.cursor_world_pos,
                        })
                    } else {
                        None
                    }
                })
                .flatten()
                .or(Some(RoomDragAction::RectSelection)),
            geng::MouseButton::Right => {
                self.clear_selection();
                if let Some(PlaceableType::Tile(_)) = self.selected_block() {
                    self.keep_state();
                    Some(RoomDragAction::RemoveTile)
                } else {
                    None
                }
            }
            geng::MouseButton::Middle => Some(RoomDragAction::MoveCamera {
                initial_camera_pos: self.camera.center.map(Coord::new),
            }),
        };

        // Start the dragging state
        self.dragging = Some(RoomDragging {
            initial_cursor_pos: position,
            initial_world_pos: self.cursor_world_pos,
            action,
        });

        // Update cursor again to act on the dragging state
        self.update_cursor(position);
    }

    /// Handle release event.
    fn release(&mut self, button: geng::MouseButton) {
        if let Some(dragging) = self.dragging.take() {
            if let Some(action) = dragging.action {
                match action {
                    RoomDragAction::MoveBlocks {
                        blocks,
                        initial_pos,
                    } => {
                        // Move blocks and update selection
                        self.keep_state();
                        self.selection.clear();
                        let delta = self.cursor_world_pos - initial_pos;
                        for mut block in blocks {
                            block.translate(delta, &self.world.room.grid);
                            let id =
                                self.world
                                    .room
                                    .place_block(block, self.active_layer, &self.assets);
                            self.selection.insert(id);
                        }
                        self.update_geometry();

                        // Update hovered blocks
                        self.hovered = self.get_hovered(Aabb2::point(self.cursor_world_pos));
                    }
                    RoomDragAction::RectSelection => {
                        // Select blocks in a rectangle
                        if !self.geng.window().is_key_pressed(geng::Key::LShift) {
                            self.clear_selection();
                        }
                        let aabb =
                            Aabb2::from_corners(dragging.initial_world_pos, self.cursor_world_pos);
                        let hovered = self.get_hovered(aabb);
                        self.selection.extend(hovered);
                    }
                    _ => (),
                }
            }

            if dragging.initial_cursor_pos == self.cursor_pos {
                // Click
                match button {
                    geng::MouseButton::Left => {
                        if let Some(&id) = self.hovered.first() {
                            // Change selection
                            if !self.geng.window().is_key_pressed(geng::Key::LShift) {
                                self.clear_selection();
                            }
                            if !self.selection.insert(id) {
                                self.selection.remove(&id);
                            }
                        } else {
                            self.clear_selection();
                            self.place_block()
                        }
                    }
                    geng::MouseButton::Right => self.remove_hovered(),
                    geng::MouseButton::Middle => {
                        self.goto_hovered();
                    }
                }
            }
        }
    }

    /// Zoom in/out.
    fn zoom(&mut self, delta: isize) {
        let current = self.screen_resolution.x;
        let delta = delta * PIXELS_PER_UNIT as isize;
        let target_width = (delta.saturating_add_unsigned(current).max(0) as usize)
            .clamp(ROOM_FOV_MIN, ROOM_FOV_MAX);
        let ratio = self.screen_resolution.y as f32 / self.screen_resolution.x as f32;
        self.screen_resolution = vec2(target_width, (target_width as f32 * ratio).round() as usize);
        self.camera.fov = (self.screen_resolution.x / PIXELS_PER_UNIT) as f32 * ratio;

        render::update_texture_size(&mut self.pixel_texture, self.screen_resolution, &self.geng);
    }

    /// Update cached geometry.
    pub fn update_geometry(&mut self) {
        self.world.cache = RenderCache::calculate(&self.world.room, &self.geng, &self.assets);
    }

    /// Duplicate all selected blocks.
    pub fn duplicate_selected(&mut self) {
        if let Some(initial_pos) = self
            .selection
            .iter()
            .next()
            .and_then(|id| self.world.room.get_block(*id, self.active_layer))
            .map(|block| block.position(&self.world.room.grid))
        {
            self.keep_state();
            let blocks: Vec<_> = self
                .selection
                .iter()
                .flat_map(|&id| self.world.room.get_block(id, self.active_layer))
                .collect();
            self.dragging = Some(RoomDragging {
                initial_cursor_pos: self.cursor_pos,
                initial_world_pos: self.cursor_world_pos,
                action: Some(RoomDragAction::MoveBlocks {
                    blocks,
                    initial_pos,
                }),
            });
        }
    }

    /// Swithes the tab and selects the hovered block.
    fn goto_hovered(&mut self) {
        let Some(&hovered_id) = self.hovered.first() else { return };
        let Some(hovered) = self.world.room.get_block(hovered_id, self.active_layer) else {
            return
        };
        let hovered = hovered.get_type();

        for (tab_i, tab) in self.tabs.iter_mut().enumerate() {
            match &mut tab.mode {
                RoomEditorMode::Block { blocks, selected } => {
                    if let Some(i) = blocks.iter().position(|block| *block == hovered) {
                        *selected = i;
                        self.active_tab = tab_i;
                        break;
                    }
                }
                RoomEditorMode::Lights { spotlight } => {
                    if let PlaceableType::Spotlight(selected) = hovered {
                        *spotlight = selected;
                        self.active_tab = tab_i;
                        break;
                    }
                }
                _ => (),
            }
        }
    }

    /// Cancels the current action, clears the selection.
    fn cancel(&mut self) {
        self.clear_selection();
        if let Some(dragging) = &mut self.dragging {
            if let Some(RoomDragAction::MoveBlocks { .. }) = dragging.action {
                self.dragging = None;
            }
        }
    }

    /// Save the room to file.
    pub fn save_room(&self) -> anyhow::Result<()> {
        self.world.room.save(&self.room_name)
    }
}

impl RoomEditor {
    pub fn update(&mut self, delta_time: f64) {
        let delta_time = delta_time as f32;
        let window = self.geng.window();

        let ctrl = window.is_key_pressed(geng::Key::LCtrl);
        if self.input_events.is_none() && !ctrl {
            // Move camera
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
            self.camera.center += dir * ROOM_CAMERA_MOVE_SPEED * delta_time;
        }

        self.update_selected_block();
    }

    pub fn handle_event(&mut self, event: geng::Event) {
        if let Some(events) = &mut self.input_events {
            events.push(event);
            return;
        }

        let ctrl = self.geng.window().is_key_pressed(geng::Key::LCtrl);
        let shift = self.geng.window().is_key_pressed(geng::Key::LShift);
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
                self.zoom(-delta.signum() as isize);
            }
            geng::Event::KeyDown { key } => match key {
                geng::Key::Escape => self.cancel(),
                geng::Key::S if ctrl => {
                    let _ = util::report_err(self.save_room(), "Failed to save the room");
                }
                geng::Key::Z if ctrl => {
                    if shift {
                        self.redo()
                    } else {
                        self.undo()
                    }
                }
                geng::Key::D if ctrl => self.duplicate_selected(),
                geng::Key::R => {
                    self.world.room.spawn_point = self.cursor_world_pos;
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

    pub fn transition(&mut self) -> Option<geng::Transition> {
        std::mem::take(&mut self.playtest).then(|| {
            // Start the playtest state
            let state = game::Game::new(
                &self.geng,
                &self.assets,
                self.room_name.clone(),
                self.world.room.clone(),
                None,
                0,
                Time::ZERO,
                0,
                false,
                None,
            );

            // The state is pushed on top of the editor state,
            // so that when we exit the playtest, we return to the editor.
            geng::Transition::Push(Box::new(state))
        })
    }
}

impl RoomEditorTab {
    /// Constructor for the editor tab in `Block` mode.
    pub fn block(name: impl Into<String>, blocks: Vec<PlaceableType>) -> Self {
        Self {
            name: name.into(),
            hoverable: blocks.clone(),
            mode: RoomEditorMode::Block {
                selected: 0,
                blocks,
            },
        }
    }
}

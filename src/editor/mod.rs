use super::*;
use ui::ColorMode;

mod action;
mod ui_impl;

use action::*;

const CAMERA_MOVE_SPEED: f32 = 20.0;
const EDITOR_FOV_MIN: usize = 10 * PIXELS_PER_UNIT;
const EDITOR_FOV_MAX: usize = 70 * PIXELS_PER_UNIT;

/// A combination of all renderers used by the editor.
struct Render {
    world: WorldRender,
    lights: LightsRender,
    util: UtilRender,
}

impl Render {
    pub fn new(geng: &Geng, assets: &Rc<Assets>) -> Self {
        Self {
            world: WorldRender::new(geng, assets),
            lights: LightsRender::new(geng, assets),
            util: UtilRender::new(geng, assets),
        }
    }
}

pub struct Editor {
    geng: Geng,
    assets: Rc<Assets>,
    /// The downscaled texture used for pixel-perfect rendering of the world.
    pixel_texture: ugli::Texture,
    /// The renderer used by the editor.
    render: Render,
    /// The renderer used for the preview.
    preview_render: GameRender,

    camera: Camera2d,
    /// Size of the actual screen size of the application.
    framebuffer_size: vec2<usize>,
    /// Size of the downscaled version of the screen.
    screen_resolution: vec2<usize>,

    /// The name of the currently loaded level.
    level_name: String,
    /// The world that contains the level.
    world: World,
    /// Whether we should transition into the playtest state.
    playtest: bool,

    #[cfg(not(target_arch = "wasm32"))]
    /// State for hot reloading assets.
    hot_reload: Option<HotReload>,

    /// Current position of the cursor in screen coordinates.
    cursor_pos: vec2<f64>,
    /// Current position of the cursor in world coordinates.
    cursor_world_pos: vec2<Coord>,
    /// Dragging state (e.g. rectangular selection).
    dragging: Option<Dragging>,
    /// Currently selected blocks.
    selection: HashSet<PlaceableId>,
    /// Currently hovered blocks.
    hovered: Vec<PlaceableId>,
    /// Stack of undo actions.
    undo_actions: Vec<Action>,
    /// Stack of redo actions.
    redo_actions: Vec<Action>,

    /// All available editor tabs.
    tabs: Vec<EditorTab>,
    /// The currently active editor tab.
    active_tab: usize,

    /// Whether the grid should be rendered.
    draw_grid: bool,
    /// Whether the world should be rendered in preview mode or editor mode.
    preview: bool,
    /// Whether the color values should be presented in float scale or integer scale.
    light_float_scale: bool,
    /// The color mode that is used in color selection: RGB, HSV, or HSL.
    color_mode: Option<ColorMode>,
}

/// The hot reload state.
#[cfg(not(target_arch = "wasm32"))]
struct HotReload {
    /// The receiver of the events sent by the watcher.
    receiver: std::sync::mpsc::Receiver<notify::Result<notify::Event>>,
    /// The watcher that sends events on change detection.
    _watcher: notify::RecommendedWatcher,
}

/// The dragging state.
#[derive(Debug)]
struct Dragging {
    /// Initial cursor positiion in screen coordinates.
    pub initial_cursor_pos: vec2<f64>,
    /// Initial cursor positiion in world coordinates.
    pub initial_world_pos: vec2<Coord>,
    /// The action of the drag (e.g. rectangular selection).
    pub action: Option<DragAction>,
}

/// The action of the drag (e.g. rectangular selection).
#[derive(Debug)]
enum DragAction {
    /// Place tile under cursor.
    PlaceTile,
    /// Remove tile under cursor.
    RemoveTile,
    /// Move the specified blocks, respecting their offsets.
    MoveBlocks {
        /// IDs of the blocks with their offsets, relative to the `initial_pos`.
        ids: Vec<(PlaceableId, vec2<Coord>)>,
        /// The initial position used for reference.
        initial_pos: vec2<Coord>,
    },
    /// Select blocks in a rectangle.
    RectSelection,
    MoveCamera {
        initial_camera_pos: vec2<Coord>,
    },
}

/// An editor tab.
#[derive(Debug, Clone)]
struct EditorTab {
    /// The name of the tab.
    pub name: String,
    /// Possible blocks that can be hovered (and selected),
    /// when this tab is active.
    pub hoverable: Vec<PlaceableType>,
    /// The mode of the tab.
    pub mode: EditorMode,
}

/// Mode of the editor tab.
#[derive(Debug, Clone)]
enum EditorMode {
    /// Modify level information.
    /// Also allows to select all blocks in the level,
    /// regardless of the `hoverable` field in the tab.
    Level,
    /// Place blocks.
    Block {
        /// All placeable blocks in that mode.
        blocks: Vec<PlaceableType>,
        /// Currently selected placeable block.
        selected: usize,
    },
    /// Modify global light and other lights in the level.
    Lights { spotlight: SpotlightSource },
}

impl Editor {
    pub fn new(
        geng: &Geng,
        assets: &Rc<Assets>,
        level_name: Option<String>,
        hot_reload: bool,
    ) -> Self {
        // Load the level and update its geometry
        let level_name = level_name.unwrap_or_else(|| "new_level.json".to_string());
        let mut level =
            util::report_err(Level::load(&level_name), "Failed to load level").unwrap_or_default();
        // Update geometry in case it was not specified in the json file.
        level.tiles.update_geometry(assets);

        #[cfg(target_arch = "wasm32")]
        if hot_reload {
            warn!("Hot reloading assets does nothing on the web");
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
            render: Render::new(geng, assets),
            preview_render: GameRender::new(geng, assets),
            screen_resolution: SCREEN_RESOLUTION,
            camera: Camera2d {
                center: vec2(0.0, 0.25),
                rotation: 0.0,
                fov: (SCREEN_RESOLUTION.x / PIXELS_PER_UNIT) as f32 * 9.0 / 16.0,
            },
            framebuffer_size: vec2(1, 1),
            world: World::new(geng, assets, assets.rules.clone(), level),
            draw_grid: true,
            cursor_pos: vec2::ZERO,
            cursor_world_pos: vec2::ZERO,
            dragging: None,
            selection: default(),
            tabs: vec![
                EditorTab {
                    name: "Level".into(),
                    hoverable: vec![],
                    mode: EditorMode::Level,
                },
                EditorTab::block(
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
                    mode: EditorMode::Lights {
                        spotlight: default(),
                    },
                },
            ],
            active_tab: 0,
            undo_actions: default(),
            redo_actions: default(),
            hovered: Vec::new(),
            light_float_scale: true,
            color_mode: None,
            playtest: false,
            preview: false,
            level_name,

            #[cfg(not(target_arch = "wasm32"))]
            hot_reload: hot_reload.then(|| {
                use notify::Watcher;

                let (tx, rx) = std::sync::mpsc::channel();
                let mut watcher: notify::RecommendedWatcher = notify::Watcher::new(
                    tx,
                    notify::Config::default().with_poll_interval(std::time::Duration::from_secs(1)),
                )
                .expect("Failed to initialize the watcher");

                // Watch `assets` folder recursively
                watcher
                    .watch(&run_dir().join("assets"), notify::RecursiveMode::Recursive)
                    .expect("Failed to start watching assets directory");

                info!("Initialized the watcher for assets");

                HotReload {
                    receiver: rx,
                    _watcher: watcher,
                }
            }),
        }
    }

    /// Handles events from the hot reload watcher.
    #[cfg(not(target_arch = "wasm32"))]
    fn handle_notify(&mut self, event: notify::Result<notify::Event>) {
        debug!("Received event from hot reload: {event:?}");
        let event = match event {
            Ok(event) => event,
            Err(err) => {
                error!("Received error from hot reload channel: {err}");
                return;
            }
        };

        if let notify::EventKind::Modify(_) = event.kind {
            self.reload_assets();
        }
    }

    /// Reload all assets.
    #[cfg(not(target_arch = "wasm32"))]
    fn reload_assets(&mut self) {
        let assets = futures::executor::block_on({
            let geng = self.geng.clone();
            async move { <Assets as geng::LoadAsset>::load(&geng, &run_dir().join("assets")).await }
        });

        let assets = match assets {
            Ok(assets) => assets,
            Err(err) => {
                error!("Failed to reload assets: {err}");
                return;
            }
        };

        let assets = Rc::new(assets);
        self.world.assets = assets.clone();
        self.assets = assets;

        self.render = Render::new(&self.geng, &self.assets);
        self.preview_render = GameRender::new(&self.geng, &self.assets);

        self.update_geometry();

        info!("Successfully reloaded assets");
    }

    /// Change the currently selected placeable block in the currently active tab.
    fn scroll_selected(&mut self, delta: isize) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
            if let EditorMode::Block { selected, blocks } = &mut tab.mode {
                let current = *selected as isize;
                let target = current + delta;
                *selected = target.rem_euclid(blocks.len() as isize) as usize;
            }
        }
    }

    /// Get the currently selected placeable block (if any).
    fn selected_block(&self) -> Option<PlaceableType> {
        self.tabs
            .get(self.active_tab)
            .and_then(|tab| match &tab.mode {
                EditorMode::Level => None,
                EditorMode::Block { blocks, selected } => blocks.get(*selected).cloned(),
                EditorMode::Lights { spotlight } => Some(PlaceableType::Spotlight(*spotlight)),
            })
    }

    /// Place the currently selected placeable block.
    fn place_block(&mut self) {
        if let Some(block) = self.selected_block() {
            self.action(Action::Place {
                block,
                pos: self.cursor_world_pos,
            });
        }
    }

    /// Delete all hovered blocks.
    fn remove_hovered(&mut self) {
        self.action(Action::Remove {
            ids: self.hovered.clone(),
        });
    }

    /// Delete all selected blocks.
    fn remove_selected(&mut self) {
        self.action(Action::Remove {
            ids: self.selection.iter().copied().collect(),
        });
    }

    /// Move blocks, respecting their offsets.
    fn move_blocks(&mut self, ids: &mut [(PlaceableId, vec2<Coord>)], pos: vec2<Coord>) {
        // Remember all tile actions, since they need to be
        // done in a specific order.
        let mut tile_clears = Vec::new();
        let mut tile_moves = Vec::new();
        let mut tile_selections = Vec::new();

        for (id, offset) in ids {
            let pos = pos + *offset;
            let grid_pos = self.world.level.grid.world_to_grid(pos).0;
            match *id {
                PlaceableId::Tile(pos) => {
                    if let Some(tile) = self.world.level.tiles.get_tile_isize(pos) {
                        if self.selection.remove(id) {
                            tile_selections.push(PlaceableId::Tile(grid_pos));
                        }
                        *id = PlaceableId::Tile(grid_pos);
                        tile_clears.push(pos);
                        tile_moves.push((tile.to_owned(), grid_pos));
                    }
                }
                PlaceableId::Hazard(id) => {
                    if let Some(hazard) = self.world.level.hazards.get_mut(id) {
                        hazard.teleport(pos);
                    }
                }
                PlaceableId::Prop(id) => {
                    if let Some(prop) = self.world.level.props.get_mut(id) {
                        prop.teleport(pos);
                    }
                }
                PlaceableId::Coin(id) => {
                    if let Some(coin) = self.world.level.coins.get_mut(id) {
                        coin.teleport(pos);
                    }
                }
                PlaceableId::Spotlight(id) => {
                    if let Some(light) = self.world.level.spotlights.get_mut(id) {
                        light.position = pos;
                    }
                }
            }
        }

        // Manage the tiles
        for pos in tile_clears {
            self.world
                .level
                .tiles
                .set_tile_isize(pos, "air".to_owned(), &self.assets);
        }
        for (tile, pos) in tile_moves {
            self.world
                .level
                .tiles
                .set_tile_isize(pos, tile, &self.assets);
        }
        self.selection.extend(tile_selections);

        // Update geometry
        self.update_geometry();
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
    /// by the currently active tab's hovered list (unless it's in `Level` mode).
    fn get_hovered(&self, aabb: Aabb2<Coord>) -> Vec<PlaceableId> {
        let mut hovered = self.world.level.get_hovered(aabb);
        if let Some(tab) = &self.tabs.get(self.active_tab) {
            if let EditorMode::Level = tab.mode {
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
            let snap_size = self.world.level.grid.cell_size / Coord::new(2.0);
            self.cursor_world_pos =
                (self.cursor_world_pos / snap_size).map(|x| x.round()) * snap_size;
        }

        // Update hovered blocks
        self.hovered = self.get_hovered(Aabb2::point(self.cursor_world_pos));

        // Update the dragging state
        if let Some(mut dragging) = self.dragging.take() {
            if let Some(action) = &mut dragging.action {
                match action {
                    DragAction::PlaceTile => self.place_block(),
                    DragAction::RemoveTile => self.remove_hovered(),
                    DragAction::MoveBlocks { ids, initial_pos } => self.move_blocks(
                        ids,
                        *initial_pos + self.cursor_world_pos - dragging.initial_world_pos,
                    ),
                    DragAction::RectSelection => {}
                    &mut DragAction::MoveCamera { initial_camera_pos } => {
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
                        Some(DragAction::PlaceTile)
                    } else if let Some(&id) = self.hovered.first() {
                        self.world.level.get_block(id).map(|block| {
                            let pos = block.position(&self.world.level.grid);

                            // If the block was already selected,
                            // then we move all the selected blocks.
                            // Otherwise, we create a new selection
                            // with the block we just clicked.
                            let ids: Vec<_> = if self.selection.contains(&id) {
                                // Move all selection, and remember the offsets.
                                self.selection
                                    .iter()
                                    .filter_map(|&id| {
                                        self.world.level.get_block(id).map(|block| {
                                            (id, block.position(&self.world.level.grid) - pos)
                                        })
                                    })
                                    .collect()
                            } else {
                                // Create a new selection
                                self.clear_selection();
                                self.selection.insert(id);
                                vec![(id, vec2::ZERO)]
                            };

                            DragAction::MoveBlocks {
                                ids,
                                initial_pos: pos,
                            }
                        })
                    } else {
                        None
                    }
                })
                .flatten()
                .or(Some(DragAction::RectSelection)),
            geng::MouseButton::Right => {
                self.clear_selection();
                if let Some(PlaceableType::Tile(_)) = self.selected_block() {
                    Some(DragAction::RemoveTile)
                } else {
                    None
                }
            }
            geng::MouseButton::Middle => Some(DragAction::MoveCamera {
                initial_camera_pos: self.camera.center.map(Coord::new),
            }),
        };

        // Start the dragging state
        self.dragging = Some(Dragging {
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
                            self.place_block()
                        }
                    }
                    geng::MouseButton::Right => self.remove_hovered(),
                    geng::MouseButton::Middle => {
                        self.goto_hovered();
                    }
                }
            } else if let Some(DragAction::RectSelection) = dragging.action {
                // Select blocks in a rectangle
                if !self.geng.window().is_key_pressed(geng::Key::LShift) {
                    self.clear_selection();
                }
                let aabb = Aabb2::from_corners(dragging.initial_world_pos, self.cursor_world_pos);
                let hovered = self.get_hovered(aabb);
                self.selection.extend(hovered);
            }
        }
    }

    /// Zoom in/out.
    fn zoom(&mut self, delta: isize) {
        let current = self.screen_resolution.x;
        let delta = delta * PIXELS_PER_UNIT as isize;
        let target_width = (delta.saturating_add_unsigned(current).max(0) as usize)
            .clamp(EDITOR_FOV_MIN, EDITOR_FOV_MAX);
        let ratio = self.screen_resolution.y as f32 / self.screen_resolution.x as f32;
        self.screen_resolution = vec2(target_width, (target_width as f32 * ratio).round() as usize);
        self.camera.fov = (self.screen_resolution.x / PIXELS_PER_UNIT) as f32 * ratio;

        render::update_texture_size(&mut self.pixel_texture, self.screen_resolution, &self.geng);
    }

    /// Update cached geometry.
    fn update_geometry(&mut self) {
        self.world.cache = RenderCache::calculate(&self.world.level, &self.geng, &self.assets);
    }

    /// Duplicate all selected blocks.
    fn duplicate_selected(&mut self) {
        for &id in &self.selection {
            let Some(mut block) = self.world.level.get_block(id) else {
                continue;
            };
            // Translate the block a bit, so it is visibly distinct
            block.translate(self.world.level.grid.cell_size);
            self.world.level.place_block(block, &self.assets);
        }
    }

    /// Swithes the tab and selects the hovered block.
    fn goto_hovered(&mut self) {
        let Some(&hovered_id) = self.hovered.first() else { return };
        let Some(hovered) = self.world.level.get_block(hovered_id) else {
            return
        };
        let hovered = hovered.get_type();

        for (tab_i, tab) in self.tabs.iter_mut().enumerate() {
            match &mut tab.mode {
                EditorMode::Block { blocks, selected } => {
                    if let Some(i) = blocks.iter().position(|block| *block == hovered) {
                        *selected = i;
                        self.active_tab = tab_i;
                        break;
                    }
                }
                EditorMode::Lights { spotlight } => {
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

    /// Save the level to file.
    fn save_level(&self) {
        if let Ok(()) = util::report_err(
            self.world.level.save(&self.level_name),
            "Failed to save level",
        ) {
            info!("Saved the level");
        }
    }
}

impl geng::State for Editor {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.framebuffer_size = framebuffer.size();
        ugli::clear(framebuffer, Some(Rgba::BLACK), None, None);

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

        // Draw hovered/selected
        let mut colliders = Vec::new();
        for &block in itertools::chain![&self.hovered, &self.selection] {
            let Some(block) = self.world.level.get_block(block) else {
                continue
            };
            match block {
                Placeable::Tile((_, pos)) => {
                    let collider = self.world.level.grid.cell_collider(pos);
                    colliders.push((collider, Rgba::new(0.7, 0.7, 0.7, 0.5)));
                }
                Placeable::Hazard(hazard) => {
                    colliders.push((hazard.collider, Rgba::new(1.0, 0.0, 0.0, 0.5)));
                }
                Placeable::Prop(prop) => {
                    colliders.push((Collider::new(prop.sprite), Rgba::new(1.0, 1.0, 1.0, 0.5)));
                }
                Placeable::Coin(coin) => {
                    colliders.push((coin.collider, Rgba::new(1.0, 1.0, 0.0, 0.5)));
                }
                Placeable::Spotlight(light) => {
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

    fn update(&mut self, delta_time: f64) {
        let delta_time = delta_time as f32;
        let window = self.geng.window();

        let ctrl = window.is_key_pressed(geng::Key::LCtrl);
        if !ctrl {
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
            self.camera.center += dir * CAMERA_MOVE_SPEED * delta_time;
        }

        self.update_selected_block();

        #[cfg(not(target_arch = "wasm32"))]
        if let Some(hot) = &self.hot_reload {
            // Check hot reload events
            use std::sync::mpsc::TryRecvError;
            match hot.receiver.try_recv() {
                Ok(event) => self.handle_notify(event),
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    error!("Disconnected from the hot reload channel");
                }
            }
        }
    }

    fn handle_event(&mut self, event: geng::Event) {
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
                geng::Key::Escape => {
                    self.clear_selection();
                }
                geng::Key::S if ctrl => self.save_level(),
                geng::Key::Z if ctrl => {
                    if shift {
                        self.redo()
                    } else {
                        self.undo()
                    }
                }
                geng::Key::D if ctrl => self.duplicate_selected(),
                geng::Key::R => {
                    self.world.level.spawn_point = self.cursor_world_pos;
                }
                geng::Key::F => {
                    self.world.level.finish = self.cursor_world_pos;
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

    fn transition(&mut self) -> Option<geng::Transition> {
        std::mem::take(&mut self.playtest).then(|| {
            // Start the playtest state
            let state = game::Game::new(
                &self.geng,
                &self.assets,
                self.level_name.clone(),
                self.world.level.clone(),
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

    fn ui<'a>(&'a mut self, cx: &'a geng::ui::Controller) -> Box<dyn geng::ui::Widget + 'a> {
        self.ui(cx)
    }
}

impl EditorTab {
    /// Constructor for the editor tab in `Block` mode.
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

/// Run the editor.
pub fn run(geng: &Geng, level: Option<String>, hot_reload: bool) -> impl geng::State {
    let future = {
        let geng = geng.clone();
        async move {
            let assets: Rc<Assets> = geng::LoadAsset::load(&geng, &run_dir().join("assets"))
                .await
                .expect("Failed to load assets");
            Editor::new(&geng, &assets, level, hot_reload)
        }
    };
    geng::LoadingScreen::new(geng, geng::EmptyLoadingScreen, future, |state| state)
}

use super::*;

const LEVEL_CAMERA_MOVE_SPEED: f32 = 100.0;
const LEVEL_FOV_MIN: f32 = 50.0;
const LEVEL_FOV_MAX: f32 = 300.0;

#[macro_export]
macro_rules! active_room_mut {
    ($editor:ident) => {{
        $editor
            .active_room
            .as_ref()
            .and_then(|room| $editor.rooms.get_mut(room).map(|room| &mut room.editor))
    }};
}

pub struct LevelEditor {
    pub geng: Geng,
    pub assets: Rc<Assets>,

    pub camera: Camera2d,
    /// Size of the actual screen size of the application.
    pub framebuffer_size: vec2<usize>,

    /// Currently active room.
    pub active_room: Option<String>,
    /// All opened rooms.
    pub rooms: HashMap<String, RoomState>,

    /// Current position of the cursor in screen coordinates.
    pub cursor_pos: vec2<f64>,
    /// Current position of the cursor in world coordinates.
    pub cursor_world_pos: vec2<Coord>,
    /// Dragging state (e.g. rectangular selection).
    pub dragging: Option<LevelDragging>,

    #[cfg(not(target_arch = "wasm32"))]
    /// State for hot reloading assets.
    pub hot_reload: Option<HotReload>,
}

pub struct RoomState {
    pub pos: vec2<Coord>,
    pub editor: RoomEditor,
}

/// The hot reload state.
#[cfg(not(target_arch = "wasm32"))]
pub struct HotReload {
    /// The receiver of the events sent by the watcher.
    pub receiver: std::sync::mpsc::Receiver<notify::Result<notify::Event>>,
    /// The watcher that sends events on change detection.
    pub _watcher: notify::RecommendedWatcher,
}

/// The dragging state in the level editor.
#[derive(Debug)]
pub struct LevelDragging {
    /// Initial cursor positiion in screen coordinates.
    pub initial_cursor_pos: vec2<f64>,
    /// Initial cursor positiion in world coordinates.
    pub initial_world_pos: vec2<Coord>,
    /// The action of the drag (e.g. rectangular selection).
    pub action: Option<LevelDragAction>,
}

/// The action of the drag (e.g. rectangular selection) in the level editor.
#[derive(Debug)]
pub enum LevelDragAction {
    MoveCamera { initial_camera_pos: vec2<Coord> },
}

impl LevelEditor {
    pub fn new(geng: &Geng, assets: &Rc<Assets>, room: Option<String>, hot_reload: bool) -> Self {
        #[cfg(target_arch = "wasm32")]
        if hot_reload {
            warn!("Hot reloading assets does nothing on the web");
        }

        let rooms = Self::load_rooms(
            geng,
            assets,
            room.unwrap_or_else(|| "new_room.json".to_string()),
        );

        Self {
            geng: geng.clone(),
            assets: assets.clone(),

            camera: Camera2d {
                center: vec2::ZERO,
                rotation: 0.0,
                fov: 100.0,
            },
            framebuffer_size: vec2(1, 1),

            active_room: None,
            rooms,

            cursor_pos: vec2::ZERO,
            cursor_world_pos: vec2::ZERO,
            dragging: None,

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

    fn load_rooms(
        geng: &Geng,
        assets: &Rc<Assets>,
        room_name: String,
    ) -> HashMap<String, RoomState> {
        let mut rooms = HashMap::new();

        // Load rooms
        let mut to_load = Vec::new();
        to_load.push(room_name);
        while let Some(room_name) = to_load.pop() {
            if rooms.contains_key(&room_name) {
                continue;
            }

            // Load
            let room = RoomState {
                pos: vec2::ZERO,
                editor: RoomEditor::new(geng, assets, room_name.clone()),
            };

            // Queue adjacent rooms to load
            to_load.extend(
                room.editor
                    .world
                    .room
                    .transitions
                    .iter()
                    .map(|trans| trans.to_room.clone()),
            );

            // Insert
            rooms.insert(room_name, room);
        }

        // Layout
        fn layout_room(
            room_name: &str,
            pos: Option<vec2<Coord>>,
            rooms: &mut HashMap<String, RoomState>,
            layout: &mut HashSet<String>,
            blocked: &mut HashSet<String>,
        ) {
            if let Some(pos) = pos {
                rooms.get_mut(room_name).unwrap().pos = pos;
            }

            blocked.remove(room_name);
            if !layout.insert(room_name.to_owned()) {
                return;
            }

            'pos: {
                if pos.is_none() {
                    // Check if some connected room is laid out already
                    let room = rooms.get(room_name).unwrap();
                    for transition in room.editor.world.room.transitions.clone() {
                        let connected = transition.to_room;
                        let laid = if layout.contains(&connected) {
                            true
                        } else if !blocked.contains(&connected) {
                            blocked.insert(room_name.to_owned());
                            layout_room(&connected, None, rooms, layout, blocked);
                            blocked.remove(room_name);
                            true
                        } else {
                            false
                        };
                        if laid {
                            let connected = rooms.get(&connected).unwrap();
                            // TODO: proper connection
                            let pos = connected.pos;
                            let room = rooms.get_mut(room_name).unwrap();
                            room.pos = pos - vec2::UNIT_X * room.editor.world.room.bounds().width();
                            break 'pos;
                        }
                    }

                    // Calculate occupied space
                    let mut occupied = Aabb2::ZERO;
                    let mut extend_to = |room: &RoomState| {
                        let bounds = Aabb2::point(room.pos)
                            .extend_positive(room.editor.world.room.bounds().size());
                        occupied = Aabb2 {
                            min: occupied.min.min(**bounds.min).into(),
                            max: occupied.max.max(**bounds.max).into(),
                        };
                    };
                    for room in layout.iter() {
                        extend_to(rooms.get(room).unwrap());
                    }

                    // Pick some empty space
                    let pos = occupied.bottom_right() + vec2::UNIT_X * Coord::new(5.0);
                    let room = rooms.get_mut(room_name).unwrap();
                    room.pos = pos;
                }
            }

            // Layout connected rooms
            let room = rooms.get(room_name).unwrap();
            for transition in room.editor.world.room.transitions.clone() {
                let connected = transition.to_room;
                if layout.contains(&connected) {
                    continue;
                }

                // TODO: proper connections
                let room = rooms.get(room_name).unwrap();
                let pos = room.pos + vec2::UNIT_X * room.editor.world.room.bounds().width();
                layout_room(&connected, Some(pos), rooms, layout, blocked);
            }
        }
        let mut layout = HashSet::new();
        let mut names = rooms.keys().cloned().collect::<Vec<_>>();
        if let Some(name) = names.pop() {
            layout_room(
                &name,
                Some(vec2::ZERO),
                &mut rooms,
                &mut layout,
                &mut HashSet::new(),
            );
        }
        for name in names {
            layout_room(&name, None, &mut rooms, &mut layout, &mut HashSet::new());
        }

        rooms
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

        // Update the dragging state
        if let Some(mut dragging) = self.dragging.take() {
            if let Some(action) = &mut dragging.action {
                match action {
                    &mut LevelDragAction::MoveCamera { initial_camera_pos } => {
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
            geng::MouseButton::Left => None,
            geng::MouseButton::Right => None,
            geng::MouseButton::Middle => Some(LevelDragAction::MoveCamera {
                initial_camera_pos: self.camera.center.map(Coord::new),
            }),
        };

        // Start the dragging state
        self.dragging = Some(LevelDragging {
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
            // if let Some(action) = dragging.action {
            //     match action {
            //         _ => {}
            //     }
            // }

            if dragging.initial_cursor_pos == self.cursor_pos {
                // Click
                if let geng::MouseButton::Left = button {
                    self.edit_room_hovered();
                }
            }
        }
    }

    fn edit_room_hovered(&mut self) {
        let hovered = self.rooms.iter().find(|(_, room)| {
            Aabb2::point(room.pos)
                .extend_positive(room.editor.world.room.bounds().size())
                .contains(self.cursor_world_pos)
        });
        if let Some((room, _)) = hovered {
            self.active_room = Some(room.to_owned());
        }
    }

    /// Zoom in/out.
    fn zoom(&mut self, delta: isize) {
        self.camera.fov =
            (self.camera.fov * 1.1.powi(delta as i32)).clamp(LEVEL_FOV_MIN, LEVEL_FOV_MAX);
    }

    /// Cancels the current action.
    fn cancel(&mut self) {
        if let Some(_dragging) = &mut self.dragging {}
    }

    /// Save the level to file.
    fn save_level(&self) {
        for room in self.rooms.values() {
            room.editor.save_room();
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
        for room in self.rooms.values_mut() {
            room.editor.world.assets = assets.clone();
            room.editor.render = RoomRender::new(&self.geng, &self.assets);
            room.editor.preview_render = GameRender::new(&self.geng, &self.assets);
            room.editor.update_geometry();
        }
        self.assets = assets;

        info!("Successfully reloaded assets");
    }
}

impl geng::State for LevelEditor {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.framebuffer_size = framebuffer.size();
        ugli::clear(framebuffer, Some(Rgba::BLACK), None, None);

        self.draw(framebuffer)
    }

    fn update(&mut self, delta_time: f64) {
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

        if let Some(room) = active_room_mut!(self) {
            room.update(delta_time);
        } else {
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
                self.camera.center += dir * LEVEL_CAMERA_MOVE_SPEED * delta_time;
            }
        }
    }

    fn handle_event(&mut self, event: geng::Event) {
        let ctrl = self.geng.window().is_key_pressed(geng::Key::LCtrl);
        // let shift = self.geng.window().is_key_pressed(geng::Key::LShift);
        if let Some(room) = active_room_mut!(self) {
            room.handle_event(event);
        } else {
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
                    geng::Key::S if ctrl => self.save_level(),
                    _ => {}
                },
                _ => {}
            }
        }
    }

    fn transition(&mut self) -> Option<geng::Transition> {
        active_room_mut!(self).and_then(|room| room.transition())
    }

    fn ui<'a>(&'a mut self, cx: &'a geng::ui::Controller) -> Box<dyn geng::ui::Widget + 'a> {
        self.ui(cx)
    }
}

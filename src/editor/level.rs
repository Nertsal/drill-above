use super::*;

const LEVEL_CAMERA_MOVE_SPEED: f32 = 100.0;
const LEVEL_FOV_MIN: f32 = 50.0;
const LEVEL_FOV_MAX: f32 = 300.0;

const SNAP_DISTANCE: f32 = 0.5;

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

    pub level_name: String,
    /// Currently active room.
    pub active_room: Option<RoomId>,
    /// All opened rooms.
    pub rooms: HashMap<RoomId, RoomState>,
    /// Snap grid.
    pub grid: Grid,

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
    pub pos: vec2<isize>,
    pub editor: RoomEditor,
    pub preview_texture: ugli::Texture,
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
    CreateRoom {
        initial_pos: vec2<isize>,
    },
    MoveRoom {
        room: RoomId,
        initial_pos: vec2<isize>,
    },
    MoveCamera {
        initial_camera_pos: vec2<Coord>,
    },
}

impl LevelEditor {
    pub fn new(
        geng: &Geng,
        assets: &Rc<Assets>,
        level: String,
        room: Option<String>,
        hot_reload: bool,
    ) -> Self {
        #[cfg(target_arch = "wasm32")]
        if hot_reload {
            warn!("Hot reloading assets does nothing on the web");
        }

        let level_name = level.clone();
        let active_room = room.map(|name| RoomId { level, name });

        let rooms = Self::load_level_rooms(geng, assets, level_name.clone());

        Self {
            geng: geng.clone(),
            assets: assets.clone(),

            camera: Camera2d {
                center: vec2::ZERO,
                rotation: 0.0,
                fov: 100.0,
            },
            framebuffer_size: vec2(1, 1),

            level_name,
            active_room,
            rooms,
            grid: Grid::new(vec2(Coord::ONE, Coord::ONE)),

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

    fn load_level_rooms(
        geng: &Geng,
        assets: &Rc<Assets>,
        level: String,
    ) -> HashMap<RoomId, RoomState> {
        let path = run_dir().join("assets").join("levels").join(&level);
        let mut rooms = HashMap::new();

        // Load rooms
        let mut to_load = Vec::<RoomId>::new();

        #[cfg(not(target_arch = "wasm32"))]
        {
            let dir = std::fs::read_dir(path).expect("Failed to open assets/levels directory");
            for file in dir.flatten() {
                if let Ok(meta) = file.metadata() {
                    if meta.is_file() {
                        if let Some(ext) = file.path().extension() {
                            if ext.to_str() == Some("json") {
                                if let Ok(name) = file.file_name().into_string() {
                                    let id = RoomId {
                                        level: level.clone(),
                                        name,
                                    };
                                    to_load.push(id);
                                }
                            }
                        }
                    }
                }
            }
        }

        while let Some(room_id) = to_load.pop() {
            if rooms.contains_key(&room_id) {
                continue;
            }

            // Load
            let room = RoomState::new(geng, assets, vec2::ZERO, room_id.clone(), None);

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
            rooms.insert(room_id, room);
        }

        // Layout
        fn layout_room(
            room_id: RoomId,
            pos: Option<vec2<isize>>,
            rooms: &mut HashMap<RoomId, RoomState>,
            layout: &mut HashSet<RoomId>,
            blocked: &mut HashSet<RoomId>,
        ) {
            if let Some(pos) = pos {
                rooms.get_mut(&room_id).unwrap().pos = pos;
            }

            blocked.remove(&room_id);
            if !layout.insert(room_id.clone()) {
                return;
            }

            'pos: {
                if pos.is_none() {
                    // Check if some connected room is laid out already
                    let room = rooms.get(&room_id).unwrap();
                    for transition in room.editor.world.room.transitions.clone() {
                        let connected = transition.to_room;
                        let laid = if layout.contains(&connected) {
                            true
                        } else if !blocked.contains(&connected) {
                            blocked.insert(room_id.to_owned());
                            layout_room(connected.clone(), None, rooms, layout, blocked);
                            blocked.remove(&room_id);
                            true
                        } else {
                            false
                        };
                        if laid {
                            let connected = rooms.get(&connected).unwrap();
                            let pos = connected.pos;
                            let room = rooms.get_mut(&room_id).unwrap();
                            room.pos = pos + transition.offset;
                            break 'pos;
                        }
                    }

                    // Calculate occupied space
                    let mut occupied = Aabb2::ZERO;
                    let mut extend_to = |room: &RoomState| {
                        let bounds = room.aabb_i();
                        occupied = Aabb2 {
                            min: occupied.min.min(**bounds.min).into(),
                            max: occupied.max.max(**bounds.max).into(),
                        };
                    };
                    for room in layout.iter() {
                        extend_to(rooms.get(room).unwrap());
                    }

                    // Pick some empty space
                    let pos = occupied.bottom_right() + vec2::UNIT_X * 5;
                    let room = rooms.get_mut(&room_id).unwrap();
                    room.pos = pos;
                }
            }

            // Layout connected rooms
            let room = rooms.get(&room_id).unwrap();
            for transition in room.editor.world.room.transitions.clone() {
                let connected = transition.to_room;
                if layout.contains(&connected) {
                    continue;
                }

                let room = rooms.get(&room_id).unwrap();
                let pos = room.pos - transition.offset;
                layout_room(connected, Some(pos), rooms, layout, blocked);
            }
        }
        let mut layout = HashSet::new();
        let mut ids = rooms.keys().cloned().collect::<Vec<_>>();
        if let Some(id) = ids.pop() {
            layout_room(
                id,
                Some(vec2::ZERO),
                &mut rooms,
                &mut layout,
                &mut HashSet::new(),
            );
        }
        for id in ids {
            layout_room(id, None, &mut rooms, &mut layout, &mut HashSet::new());
        }

        rooms
    }

    fn snap_room_pos(&self, room: &RoomId, pos: vec2<Coord>) -> vec2<isize> {
        let room_size = self
            .rooms
            .get(room)
            .expect("Snapping an unknown room")
            .aabb_i()
            .size();
        // Snap each coordinate separately
        let mut snap = vec2(None, None);

        let snap_target = |pos: Coord, target: isize, snap: &mut Option<(isize, f32)>| {
            let dist = (pos.as_f32() - target as f32).abs();
            if dist > SNAP_DISTANCE {
                return;
            }
            let v = (target, dist);
            match snap {
                Some(best) => {
                    if best.1 < v.1 {
                        *best = v;
                    }
                }
                None => *snap = Some(v),
            }
        };

        for target in self
            .rooms
            .iter()
            .filter(|(name, _)| *name != room)
            .map(|(_, room)| room.aabb_i())
            .flat_map(|room| {
                [
                    // Right edge
                    room.bottom_right(),
                    room.top_right() - vec2::UNIT_Y * room_size.y,
                    // Top edge
                    room.top_left(),
                    room.top_right() - vec2::UNIT_X * room_size.x,
                    // Left edge
                    room.bottom_left() - vec2::UNIT_X * room_size.x,
                    room.top_left() - room_size,
                    // Bottom edge
                    room.bottom_left() - vec2::UNIT_Y * room_size.y,
                    room.bottom_right() - room_size,
                ]
            })
        {
            snap_target(pos.x, target.x, &mut snap.x);
            snap_target(pos.y, target.y, &mut snap.y);
        }

        let resolve = |snap: Option<_>, pos| snap.map_or(pos, |(target, _)| target);
        let pos = self.grid.world_to_grid(pos).0;
        vec2(resolve(snap.x, pos.x), resolve(snap.y, pos.y))
    }

    fn move_room(&mut self, room: RoomId, pos: vec2<isize>) {
        // Set position
        let room = self.rooms.get_mut(&room).expect("Dragging a deleted room");
        room.pos = pos;
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
            if let Some(action) = dragging
                .action
                .as_mut()
                .filter(|_| dragging.initial_cursor_pos != cursor_pos)
            {
                match action {
                    LevelDragAction::MoveRoom { room, initial_pos } => {
                        let pos = self.grid.grid_to_world(*initial_pos) + self.cursor_world_pos
                            - dragging.initial_world_pos;
                        let pos = self.snap_room_pos(room, pos);
                        self.move_room(room.to_owned(), pos);
                    }
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
                    _ => {}
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
            geng::MouseButton::Left => self
                .rooms
                .iter()
                .find(|(_, room)| room.aabb().contains(self.cursor_world_pos))
                .map(|(name, room)| LevelDragAction::MoveRoom {
                    room: name.to_owned(),
                    initial_pos: room.pos,
                })
                .or_else(|| {
                    Some(LevelDragAction::CreateRoom {
                        initial_pos: self.grid.world_to_grid(self.cursor_world_pos).0,
                    })
                }),
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
            if let Some(action) = dragging.action {
                match action {
                    LevelDragAction::CreateRoom { initial_pos } => {
                        let pos = self.grid.world_to_grid(self.cursor_world_pos).0;
                        let aabb = Aabb2::from_corners(initial_pos, pos);
                        let room = Room::new(aabb.size().map(|x| x as usize));
                        let mut rng = thread_rng();
                        let name: String = (0..5).map(|_| rng.gen_range('A'..='Z')).collect();
                        let id = RoomId {
                            level: self.level_name.clone(),
                            name,
                        };
                        let room = RoomState::new(
                            &self.geng,
                            &self.assets,
                            aabb.bottom_left(),
                            id.clone(),
                            Some(room),
                        );
                        self.rooms.insert(id.clone(), room);
                        self.update_room_transitions(id);
                    }
                    LevelDragAction::MoveRoom { room, .. } => {
                        self.update_room_transitions(room);
                    }
                    _ => {}
                }
            }

            if dragging.initial_cursor_pos == self.cursor_pos {
                // Click
                match button {
                    geng::MouseButton::Left => {
                        self.edit_room_hovered();
                    }
                    geng::MouseButton::Right => {
                        self.remove_room_hovered();
                    }
                    _ => (),
                }
            }
        }
    }

    fn edit_room_hovered(&mut self) {
        let hovered = self
            .rooms
            .iter()
            .find(|(_, room)| room.aabb().contains(self.cursor_world_pos));
        if let Some((room, _)) = hovered {
            self.active_room = Some(room.to_owned());
        }
    }

    fn remove_room_hovered(&mut self) {
        let hovered = self
            .rooms
            .iter()
            .find(|(_, room)| room.aabb().contains(self.cursor_world_pos));
        if let Some((name, _)) = hovered {
            let name = name.to_owned();
            let _ = self.rooms.remove(&name).unwrap();
            if Some(name.clone()) == self.active_room {
                self.active_room = None;
            }
            self.update_room_transitions(name);
        }
    }

    /// Zoom in/out.
    fn zoom(&mut self, delta: isize) {
        self.camera.fov =
            (self.camera.fov * 1.1.powi(delta as i32)).clamp(LEVEL_FOV_MIN, LEVEL_FOV_MAX);
    }

    /// Cancels the current action.
    fn cancel(&mut self) {
        if let Some(dragging) = &mut self.dragging {
            if let Some(action) = &mut dragging.action {
                match action {
                    LevelDragAction::CreateRoom { .. } => {
                        self.dragging = None;
                    }
                    LevelDragAction::MoveRoom { room, initial_pos } => {
                        let room = self.rooms.get_mut(room).expect("Dragging a deleted room");
                        room.pos = *initial_pos;
                        self.dragging = None;
                    }
                    _ => (),
                }
            }
        }
    }

    fn update_room_name(&mut self, old_id: RoomId, new_id: RoomId) {
        if old_id == new_id {
            return;
        }

        // Check if renaming is valid, i.e. that
        // no other room has the same `new_name`
        // TODO: check outside the loaded scope, there might be unloaded maps with that name
        if self.rooms.iter().any(|(name, _)| *name == new_id) {
            error!("Cannot rename the room to {new_id:?} as there already exists a room with that name");
            return;
        }

        // Update room name
        let room = self.rooms.remove(&old_id).unwrap();
        self.rooms.insert(new_id.clone(), room);

        // Update references to the old room
        for room in self.rooms.values_mut() {
            for trans in &mut room.editor.world.room.transitions {
                if trans.to_room == old_id {
                    trans.to_room = new_id.clone();
                }
            }
        }

        if let Ok(()) = self.save_level() {
            #[cfg(not(target_arch = "wasm32"))]
            {
                // Remove the old room
                let _ = util::report_err(
                    std::fs::remove_file(old_id.full_path()),
                    "Failed to remove old room file",
                );
            }
            info!("Successfully renamed the room {old_id:?} to {new_id:?}");
        }
    }

    fn update_room_transitions(&mut self, room_id: RoomId) {
        // Remove old transitions to/from the room
        for room in self.rooms.values_mut() {
            room.editor
                .world
                .room
                .transitions
                .retain(|trans| trans.to_room != room_id);
        }
        let Some(room) = self.rooms.get_mut(&room_id) else {
            return
        };
        room.editor.world.room.transitions.clear();

        // Calculate new transitions
        let mut new_transitions = HashMap::<RoomId, Vec<RoomTransition>>::new();
        let room = self.rooms.get(&room_id).unwrap();
        let room_aabb = room.aabb_i();
        for (other_id, other) in &self.rooms {
            if *other_id == room_id {
                continue;
            }

            let other_aabb = other.aabb_i();
            if room_aabb.intersects(&other_aabb) {
                error!("The rooms {room_id:?} and {other_id:?} intersect each other");
                continue;
            }

            let room_transition = |room: &RoomState, pos: Aabb2<isize>| -> Aabb2<Coord> {
                let pos = pos.translate(-room.pos);
                let grid = &room.editor.world.room.grid;
                Aabb2 {
                    min: grid.grid_to_world(pos.min),
                    max: grid.grid_to_world(pos.max),
                }
            };

            // Horizontal transition (goes to the side)
            let x = if room_aabb.max.x == other_aabb.min.x {
                Some((other_aabb.min.x, room_aabb.max.x - 1))
            } else if other_aabb.max.x == room_aabb.min.x {
                Some((other_aabb.max.x - 1, room_aabb.min.x))
            } else {
                None
            };
            if let Some((room_x, other_x)) = x {
                let y_min = room_aabb.min.y.max(other_aabb.min.y);
                let y_max = room_aabb.max.y.min(other_aabb.max.y);
                if y_max >= y_min {
                    // Rooms actually have a common vertical edge
                    for (x, room, room_id, other, other_name) in [
                        (room_x, room, &room_id, other, other_id),
                        (other_x, other, other_id, room, &room_id),
                    ] {
                        let aabb = Aabb2::point(vec2(x, y_min))
                            .extend_up(y_max - y_min)
                            .extend_right(1);
                        let transition = room_transition(room, aabb);
                        let transition = RoomTransition {
                            collider: Collider::new(transition),
                            to_room: other_name.to_owned(),
                            offset: room.pos - other.pos,
                        };
                        new_transitions
                            .entry(room_id.to_owned())
                            .or_default()
                            .push(transition);
                    }
                }
            }

            // Vertical transition (goes up or down)
            let y = if room_aabb.max.y == other_aabb.min.y {
                Some((other_aabb.min.y, room_aabb.max.y - 1))
            } else if other_aabb.max.y == room_aabb.min.y {
                Some((other_aabb.max.y - 1, room_aabb.min.y))
            } else {
                None
            };
            if let Some((room_y, other_y)) = y {
                let x_min = room_aabb.min.x.max(other_aabb.min.x);
                let x_max = room_aabb.max.x.min(other_aabb.max.x);
                if x_max >= x_min {
                    // Rooms actually have a common horizontal edge
                    for (y, room, room_name, other, other_name) in [
                        (room_y, room, &room_id, other, other_id),
                        (other_y, other, other_id, room, &room_id),
                    ] {
                        let aabb = Aabb2::point(vec2(x_min, y))
                            .extend_right(x_max - x_min)
                            .extend_up(1);
                        let transition = room_transition(room, aabb);
                        let transition = RoomTransition {
                            collider: Collider::new(transition),
                            to_room: other_name.to_owned(),
                            offset: room.pos - other.pos,
                        };
                        new_transitions
                            .entry(room_name.to_owned())
                            .or_default()
                            .push(transition);
                    }
                }
            }
        }

        // Add new transitions
        for (room_name, transitions) in new_transitions {
            self.rooms
                .get_mut(&room_name)
                .unwrap()
                .editor
                .world
                .room
                .transitions
                .extend(transitions);
        }
    }

    /// Save the level to file.
    fn save_level(&self) -> anyhow::Result<()> {
        self.rooms
            .values()
            .try_for_each(|room| room.editor.save_room())
    }

    /// Handles events from the hot reload watcher.
    #[cfg(not(target_arch = "wasm32"))]
    fn handle_notify(&mut self, events: Vec<notify::Result<notify::Event>>) {
        let mut reload = false;

        for event in events {
            debug!("Received event from hot reload: {event:?}");
            let event = match event {
                Ok(event) => event,
                Err(err) => {
                    error!("Received error from hot reload channel: {err}");
                    break;
                }
            };

            if let notify::EventKind::Modify(_) = event.kind {
                reload = true;
            }
        }

        if reload {
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
            let mut events = Vec::new();
            loop {
                match hot.receiver.try_recv() {
                    Ok(event) => {
                        events.push(event);
                    }
                    Err(TryRecvError::Empty) => {
                        break;
                    }
                    Err(TryRecvError::Disconnected) => {
                        error!("Disconnected from the hot reload channel");
                    }
                }
            }

            self.handle_notify(events);
        }

        if let Some(room) = active_room_mut!(self) {
            room.update(delta_time);
            let old_id = self.active_room.as_ref().unwrap();
            if room.room_id != *old_id && room.input_events.is_none() {
                let new_name = room.room_id.clone();
                self.update_room_name(old_id.to_owned(), new_name.clone());
                self.active_room = Some(new_name);
            }
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
        let shift = self.geng.window().is_key_pressed(geng::Key::LShift);
        if let Some(room) = active_room_mut!(self) {
            match event {
                geng::Event::KeyDown {
                    key: geng::Key::Escape,
                } if shift => {
                    self.rooms
                        .get_mut(self.active_room.as_ref().unwrap())
                        .unwrap()
                        .update_preview();
                    self.active_room = None;
                    return;
                }
                _ => {}
            }
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
                    geng::Key::S if ctrl => {
                        let _ = util::report_err(self.save_level(), "Failed to save the level");
                    }
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

impl RoomState {
    pub fn new(
        geng: &Geng,
        assets: &Rc<Assets>,
        pos: vec2<isize>,
        room_id: RoomId,
        room: Option<Room>,
    ) -> Self {
        let editor = if let Some(room) = room {
            RoomEditor::new_room(geng, assets, room_id, room)
        } else {
            RoomEditor::new(geng, assets, room_id)
        };
        Self {
            pos,
            preview_texture: editor.create_preview(),
            editor,
        }
    }

    pub fn aabb(&self) -> Aabb2<Coord> {
        self.aabb_i().map(|x| Coord::new(x as f32))
    }

    pub fn aabb_i(&self) -> Aabb2<isize> {
        Aabb2::point(self.pos).extend_positive(self.editor.world.room.size.map(|x| x as isize))
    }

    pub fn update_preview(&mut self) {
        self.preview_texture = self.editor.create_preview();
    }
}

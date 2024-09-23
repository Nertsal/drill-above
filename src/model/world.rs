use super::*;

pub struct World {
    pub id_gen: IdGen,
    pub assets: Rc<Assets>,
    pub rules: Rules,
    pub volume: f64,
    pub screen_resolution: vec2<usize>,
    pub camera: Camera2d,
    pub cache: RenderCache,
    pub room: Room,
    pub room_transition: Option<RoomTransition>,
    pub coins_collected: usize,
    pub drill_sound: Option<geng::SoundEffect>,
    pub time: Time,
    pub deaths: usize,
    pub dialogue: Option<Dialogue>,

    pub player: Player,
    pub actors: Collection<Actor>,
    pub blocks: Collection<Block>,
    pub particles: Vec<Particle>,
}

pub type CollisionCallback = Rc<dyn Fn(&mut Logic<'_>, Id, MoveCollision)>;

#[derive(HasId)]
pub struct Actor {
    pub id: Id,
    pub collider: Collider,
    pub riding: Option<Id>,
    pub move_remainder: vec2<Coord>,
    pub on_squish: CollisionCallback,
}

#[derive(Debug, Clone, Serialize, Deserialize, HasId)]
pub struct Block {
    pub id: Id,
    pub tile: Tile,
    pub move_remainder: vec2<Coord>,
    pub collider: Collider,
}

impl World {
    pub fn new(
        geng: &Geng,
        assets: &Rc<Assets>,
        rules: Rules,
        mut room: Room,
        player: Option<(Player, Actor)>,
    ) -> Self {
        for layer in room.layers.iter_mut() {
            layer.tiles.update_geometry(assets);
        }
        let cache = RenderCache::calculate(&room, geng, assets);
        Self::with_cache(assets, rules, room, player, cache)
    }

    pub fn with_cache(
        assets: &Rc<Assets>,
        rules: Rules,
        room: Room,
        player: Option<(Player, Actor)>,
        cache: RenderCache,
    ) -> Self {
        let mut id_gen = IdGen::new();
        let mut actors = Collection::new();

        let player_id = id_gen.gen();
        let (player, player_actor) = match player {
            Some((mut player, mut actor)) => {
                player.id = player_id;
                actor.id = player_id;
                player.inside_transition = true;
                (player, actor)
            }
            None => {
                let mut collider = {
                    let height = Coord::new(0.9);
                    let width = Coord::new(0.9);
                    Collider::new(
                        Aabb2::ZERO.extend_symmetric(vec2(width, height) / Coord::new(2.0)),
                    )
                };
                collider.teleport(room.spawn_point);
                (Player::new(player_id), Actor::new(player_id, collider))
            }
        };

        actors.insert(player_actor);

        Self {
            assets: assets.clone(),
            volume: 0.5,
            screen_resolution: SCREEN_RESOLUTION,
            camera: {
                let fov = (SCREEN_RESOLUTION.x / PIXELS_PER_UNIT) as f32 * 9.0 / 16.0;
                Camera2d {
                    center: vec2(
                        0.0,
                        ((SCREEN_RESOLUTION.y / PIXELS_PER_UNIT) as f32 - fov) / 2.0,
                    ),
                    rotation: Angle::ZERO,
                    fov,
                }
            },
            room_transition: None,
            dialogue: None,
            coins_collected: 0,
            time: Time::ZERO,
            drill_sound: None,
            deaths: 0,
            player,
            blocks: default(),
            particles: default(),
            cache,
            id_gen,
            actors,
            rules,
            room,
        }
    }

    pub fn update_screen_size(&mut self, target_width: usize) {
        let ratio = self.screen_resolution.y as f32 / self.screen_resolution.x as f32;
        self.screen_resolution = vec2(target_width, (target_width as f32 * ratio).round() as usize);
        self.camera.fov = (self.screen_resolution.x / PIXELS_PER_UNIT) as f32 * ratio;
    }

    pub fn play_sound(&self, sound: &geng::Sound) {
        let mut sound = sound.play();
        sound.set_volume(self.volume);
    }

    pub fn kill_player(&mut self) {
        self.player.velocity = vec2::ZERO;
        self.player.state = PlayerState::Respawning { time: Time::ONE };
        self.deaths += 1;
        self.play_sound(&self.assets.sounds.death);
    }

    pub fn camera_bounds(&self) -> Aabb2<Coord> {
        let room_bounds = self.room.bounds();
        // room_bounds.min.y += self.room.grid.cell_size.y * Coord::new(0.5);
        let camera_view =
            (vec2(self.camera.fov * (16.0 / 9.0), self.camera.fov) / 2.0).map(Coord::new); // TODO: remove hardcode
        Aabb2::from_corners(
            room_bounds.bottom_left() + camera_view,
            room_bounds.top_right() - camera_view,
        )
    }
}

impl Actor {
    pub fn new(id: Id, collider: Collider) -> Self {
        Self {
            id,
            collider,
            riding: None,
            move_remainder: vec2::ZERO,
            on_squish: Rc::new(|_, _, _| {}),
        }
    }

    pub fn wall_collider(&self) -> Collider {
        let mut collider = Collider::new(
            Aabb2::ZERO
                .extend_symmetric(self.collider.raw().size() * vec2(0.55, 0.45).map(Coord::new)),
        );
        collider.translate(self.collider.pos());
        collider
    }

    pub fn feet_collider(&self) -> Collider {
        let mut collider = Collider::new(
            Aabb2::ZERO
                .extend_symmetric(vec2::UNIT_X * self.collider.raw().width() * Coord::new(0.45))
                .extend_down(Coord::new(0.1)),
        );
        collider.translate(self.collider.feet());
        collider
    }
}

impl Block {
    // pub fn new(id: Id, tile: Tile, collider: Collider) -> Self {
    //     Self {
    //         id,
    //         tile,
    //         move_remainder: vec2::ZERO,
    //         collider,
    //     }
    // }
}

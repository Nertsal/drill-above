use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, geng::Assets)]
#[asset(json)]
pub struct Rules {
    pub gravity: vec2<Coord>,
    pub move_speed: Coord,
    pub full_control_acc: Coord,
    pub low_control_acc: Coord,
    pub jump_buffer_time: Time,
    pub coyote_time: Time,
    pub normal_jump_strength: Coord,
    pub wall_jump_strength: Coord,
    pub wall_jump_angle: R32,
    pub wall_jump_timeout: Time,
    pub fall_multiplier: Coord,
    pub low_jump_multiplier: Coord,
    pub free_fall_speed: Coord,
    pub wall_slide_speed: Coord,
    pub can_drill_dash: bool,
    pub drill_release_time: Time,
    pub drill_speed_min: Coord,
    pub drill_mistimed_inc: Coord,
    pub drill_speed_inc: Coord,
    pub drill_dash_time: Time,
    pub drill_dash_speed_min: Coord,
    pub drill_dash_speed_inc: Coord,
    pub drill_jump_speed_min: Coord,
    pub drill_jump_speed_inc: Coord,
}

pub struct World {
    pub id_gen: IdGen,
    pub assets: Rc<Assets>,
    pub rules: Rules,
    pub volume: f64,
    pub camera: Camera2d,
    pub geometry: (
        HashMap<Tile, ugli::VertexBuffer<Vertex>>,
        HashMap<Tile, ugli::VertexBuffer<MaskedVertex>>,
    ),
    pub light_geometry: Vec<StaticPolygon>,
    pub level: Level,
    pub level_transition: Option<String>,
    pub coins_collected: usize,
    pub drill_sound: Option<geng::SoundEffect>,
    pub time: Time,
    pub deaths: usize,

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
    pub fn new(geng: &Geng, assets: &Rc<Assets>, rules: Rules, level: Level) -> Self {
        let mut id_gen = IdGen::new();
        let mut actors = Collection::new();

        let player_id = id_gen.gen();
        let height = Coord::new(0.9);
        let half_width = Coord::new(0.9 / 2.0);
        let player_actor = Actor::new(
            player_id,
            Collider::new(Aabb2::from_corners(
                level.spawn_point - vec2(half_width, Coord::ZERO),
                level.spawn_point + vec2(half_width, height),
            )),
        );

        actors.insert(player_actor);

        Self {
            assets: assets.clone(),
            volume: 0.5,
            camera: Camera2d {
                center: vec2(0.0, 0.25),
                rotation: 0.0,
                fov: 22.5,
            },
            geometry: level.calculate_geometry(geng, assets),
            light_geometry: level.calculate_light_geometry(geng),
            level_transition: None,
            coins_collected: 0,
            time: Time::ZERO,
            drill_sound: None,
            deaths: 0,
            player: Player::new(player_id),
            blocks: default(),
            particles: default(),
            id_gen,
            actors,
            rules,
            level,
        }
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
        let mut level_bounds = self.level.bounds();
        level_bounds.min.y += self.level.grid.cell_size.y * Coord::new(0.5);
        let camera_view =
            (vec2(self.camera.fov * (16.0 / 9.0), self.camera.fov) / 2.0).map(Coord::new); // TODO: remove hardcode
        Aabb2::from_corners(
            level_bounds.bottom_left() + camera_view,
            level_bounds.top_right() - camera_view,
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

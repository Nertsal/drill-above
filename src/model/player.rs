use super::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PlayerControl {
    pub jump: bool,
    pub hold_jump: bool,
    pub move_dir: vec2<Coord>,
    pub drill: bool,
    pub hold_drill: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub id: Id,
    pub velocity: vec2<Coord>,
    pub state: PlayerState,
    pub touching_wall: Option<(Tile, vec2<Coord>)>,
    pub control_timeout: Option<Time>,
    pub facing_left: bool,
    pub can_hold_jump: bool,
    pub can_drill_dash: bool,
    pub coyote_time: Option<(Coyote, Time)>,
    pub jump_buffer: Option<Time>,
    pub drill_release: Option<Time>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlayerState {
    Grounded(Tile),
    WallSliding {
        tile: Tile,
        wall_normal: vec2<Coord>,
    },
    Airborn,
    Respawning {
        time: Time,
    },
    AirDrill {
        dash: Option<Time>,
    },
    Drilling,
    Finished {
        time: Time,
        next_heart: Time,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Coyote {
    Ground,
    Wall { wall_normal: vec2<Coord> },
    DrillJump { direction: vec2<Coord> },
    DrillDirection { initial: vec2<Coord> },
}

impl Player {
    pub fn new(id: Id) -> Self {
        Self {
            id,
            velocity: vec2::ZERO,
            state: PlayerState::Airborn,
            touching_wall: None,
            control_timeout: None,
            facing_left: false,
            can_hold_jump: false,
            can_drill_dash: false,
            coyote_time: None,
            jump_buffer: None,
            drill_release: None,
        }
    }

    pub fn hurtbox(&self, head: vec2<Coord>) -> Collider {
        let height = Coord::new(0.8);
        let width = Coord::new(0.9);
        Collider::new(
            Aabb2::point(head)
                .extend_symmetric(vec2::UNIT_X * width / Coord::new(2.0))
                .extend_down(height),
        )
    }
}

impl PlayerState {
    pub fn is_airborn(&self) -> bool {
        matches!(self, Self::Airborn)
    }

    pub fn is_grounded(&self) -> bool {
        matches!(self, Self::Grounded(..))
    }

    pub fn is_wall_sliding(&self) -> bool {
        matches!(self, Self::WallSliding { .. })
    }

    pub fn is_drilling(&self) -> bool {
        matches!(self, Self::Drilling)
    }

    pub fn is_air_drilling(&self) -> bool {
        matches!(self, Self::AirDrill { .. })
    }

    pub fn using_drill(&self) -> bool {
        self.is_drilling() || self.is_air_drilling()
    }

    pub fn finished_state(&self) -> Option<Self> {
        self.has_finished().then(|| self.clone())
    }

    pub fn has_finished(&self) -> bool {
        matches!(self, Self::Finished { .. })
    }
}

impl PlayerControl {
    pub fn take(&mut self) -> Self {
        std::mem::take(self)
    }
}

impl Default for PlayerControl {
    fn default() -> Self {
        Self {
            jump: false,
            hold_jump: false,
            move_dir: vec2::ZERO,
            drill: false,
            hold_drill: false,
        }
    }
}

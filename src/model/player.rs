use super::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PlayerControl {
    pub jump: bool,
    pub hold_jump: bool,
    pub move_dir: Vec2<Coord>,
    pub drill: bool,
    pub hold_drill: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub collider: Collider,
    pub velocity: Vec2<Coord>,
    pub state: PlayerState,
    pub touching_wall: Option<(Tile, Vec2<Coord>)>,
    pub control_timeout: Option<Time>,
    pub facing_left: bool,
    pub can_hold_jump: bool,
    pub can_drill_dash: bool,
    pub coyote_time: Option<(Coyote, Time)>,
    pub jump_buffer: Option<Time>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PlayerState {
    Grounded(Tile),
    WallSliding {
        tile: Tile,
        wall_normal: Vec2<Coord>,
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
    Wall { wall_normal: Vec2<Coord> },
    DrillJump { direction: Vec2<Coord> },
}

impl Player {
    pub fn new(feet_pos: Vec2<Coord>) -> Self {
        let height = Coord::new(0.9);
        let half_width = Coord::new(0.9 / 2.0);
        Self {
            collider: Collider::new(AABB::from_corners(
                feet_pos - vec2(half_width, Coord::ZERO),
                feet_pos + vec2(half_width, height),
            )),
            velocity: Vec2::ZERO,
            state: PlayerState::Airborn,
            touching_wall: None,
            control_timeout: None,
            facing_left: false,
            can_hold_jump: false,
            can_drill_dash: false,
            coyote_time: None,
            jump_buffer: None,
        }
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
            move_dir: Vec2::ZERO,
            drill: false,
            hold_drill: false,
        }
    }
}

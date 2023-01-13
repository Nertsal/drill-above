use super::*;

const NORMAL_JUMP: f32 = 5.0;
const WALL_JUMP: f32 = 5.0;
const WALL_JUMP_ANGLE: f32 = f32::PI / 4.0;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PlayerControl {
    pub jump: bool,
    pub hold_jump: bool,
    pub move_dir: Vec2<Coord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub collider: Collider,
    pub velocity: Vec2<Coord>,
    pub state: PlayerState,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PlayerState {
    Grounded,
    WallSliding { wall_normal: Vec2<Coord> },
    Airborn,
}

impl Player {
    pub fn new(feet_pos: Vec2<Coord>) -> Self {
        let height = Coord::new(2.0);
        let half_width = Coord::new(1.0 / 2.0);
        Self {
            collider: Collider::new(AABB::from_corners(
                feet_pos - vec2(half_width, Coord::ZERO),
                feet_pos + vec2(half_width, height),
            )),
            velocity: Vec2::ZERO,
            state: PlayerState::Airborn,
        }
    }
}

impl PlayerState {
    pub fn jump_velocity(&self) -> Option<Vec2<Coord>> {
        match self {
            PlayerState::Grounded => Some(vec2(0.0, NORMAL_JUMP).map(Coord::new)),
            PlayerState::WallSliding { wall_normal } => {
                let angle = Coord::new(WALL_JUMP_ANGLE) * wall_normal.x.signum();
                Some(wall_normal.rotate(angle) * Coord::new(WALL_JUMP))
            }
            PlayerState::Airborn => None,
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
        }
    }
}

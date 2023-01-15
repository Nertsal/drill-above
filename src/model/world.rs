use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, geng::Assets)]
#[asset(json)]
pub struct Rules {
    pub gravity: Vec2<Coord>,
    pub move_speed: Coord,
    pub full_control_acc: Coord,
    pub low_control_acc: Coord,
    pub normal_jump_strength: Coord,
    pub wall_jump_strength: Coord,
    pub wall_jump_angle: R32,
    pub wall_jump_timeout: Time,
    pub fall_multiplier: Coord,
    pub low_jump_multiplier: Coord,
    pub free_fall_speed: Coord,
    pub wall_slide_speed: Coord,
    pub drill_speed: Coord,
}

pub struct World {
    pub rules: Rules,
    pub camera: Camera2d,
    pub level: Level,
    pub level_transition: Option<String>,
    pub player: Player,
    pub particles: Vec<Particle>,
}

impl World {
    pub fn new(rules: Rules, level: Level) -> Self {
        Self {
            camera: Camera2d {
                center: vec2(0.0, 0.25),
                rotation: 0.0,
                fov: 22.5,
            },
            player: Player::new(level.spawn_point),
            particles: default(),
            level_transition: None,
            rules,
            level,
        }
    }
}

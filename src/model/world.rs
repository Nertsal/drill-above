use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, geng::Assets)]
#[asset(json)]
pub struct Rules {
    pub gravity: Vec2<Coord>,
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
    pub drill_speed: Coord,
    pub drill_jump_speed: Coord,
}

pub struct World {
    pub assets: Rc<Assets>,
    pub rules: Rules,
    pub volume: f64,
    pub camera: Camera2d,
    pub level: Level,
    pub level_transition: Option<String>,
    pub player: Player,
    pub particles: Vec<Particle>,
    pub coins_collected: usize,
    pub drill_sound: Option<geng::SoundEffect>,
}

impl World {
    pub fn new(assets: &Rc<Assets>, rules: Rules, level: Level) -> Self {
        Self {
            assets: assets.clone(),
            volume: 0.5,
            camera: Camera2d {
                center: vec2(0.0, 0.25),
                rotation: 0.0,
                fov: 22.5,
            },
            player: Player::new(level.spawn_point),
            particles: default(),
            level_transition: None,
            coins_collected: 0,
            drill_sound: None,
            rules,
            level,
        }
    }
}

use super::*;

pub struct World {
    pub camera: Camera2d,
    pub level: Level,
    pub player: Player,
}

impl World {
    pub fn load_level(level: Level) -> Self {
        Self {
            camera: Camera2d {
                center: vec2(0.0, 0.25),
                rotation: 0.0,
                fov: 22.5,
            },
            player: Player::new(level.spawn_point),
            level,
        }
    }
}

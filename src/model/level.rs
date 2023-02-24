use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, geng::Assets)]
#[asset(json)]
pub struct Level {
    pub room: Room,
}

impl Level {
    pub fn new(room: Room) -> Self {
        Self { room }
    }
}

impl Default for Level {
    fn default() -> Self {
        Self::new(default())
    }
}

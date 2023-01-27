use super::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GlobalLightSource {
    pub color: Rgba<f32>,
    pub intensity: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SpotlightSource {
    pub position: vec2<Coord>,
    pub angle: f32,
    pub angle_range: f32,
    pub color: Rgba<f32>,
    pub intensity: f32,
    pub max_distance: Coord,
    pub volume: f32,
}

impl Default for GlobalLightSource {
    fn default() -> Self {
        Self {
            color: Rgba::WHITE,
            intensity: 0.5,
        }
    }
}

impl Default for SpotlightSource {
    fn default() -> Self {
        Self {
            position: vec2::ZERO,
            angle: 0.0,
            angle_range: 1.0,
            color: Rgba::WHITE,
            intensity: 0.5,
            max_distance: Coord::new(5.0),
            volume: 0.5,
        }
    }
}

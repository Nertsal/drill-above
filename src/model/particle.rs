use super::*;

#[derive(Debug, Clone)]
pub struct Particle {
    pub initial_lifetime: Time,
    pub lifetime: Time,
    pub position: vec2<Coord>,
    pub velocity: vec2<Coord>,
    pub particle_type: ParticleType,
}

#[derive(Debug, Clone, Copy)]
pub enum ParticleType {
    Circle { radius: Coord, color: Rgba<f32> },
}

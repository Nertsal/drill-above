use super::*;

#[derive(Debug, Clone)]
pub struct Particle {
    pub lifetime: Time,
    pub position: Vec2<Coord>,
    pub velocity: Vec2<Coord>,
    pub particle_type: ParticleType,
}

#[derive(Debug, Clone, Copy)]
pub enum ParticleType {
    Heart4,
    Heart8,
}

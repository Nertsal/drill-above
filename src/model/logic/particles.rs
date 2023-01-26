use super::*;

#[derive(Debug, Clone)]
pub struct ParticleSpawn {
    pub lifetime: Time,
    pub position: Vec2<Coord>,
    pub velocity: Vec2<Coord>,
    pub amount: usize,
    pub color: Rgba<f32>,
    pub radius: Coord,
    pub radius_range: RangeInclusive<Coord>,
    pub angle_range: RangeInclusive<Coord>,
    pub color_range: RangeInclusive<f32>,
}

impl Default for ParticleSpawn {
    fn default() -> Self {
        Self {
            lifetime: Time::ONE,
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
            amount: 1,
            color: Rgba::WHITE,
            radius: Coord::ONE,
            radius_range: Coord::new(0.9)..=Coord::new(1.1),
            angle_range: Coord::new(-0.5)..=Coord::new(0.5),
            color_range: -0.05..=0.05,
        }
    }
}

impl Logic<'_> {
    pub fn spawn_particles(&mut self, config: ParticleSpawn) {
        let mut rng = thread_rng();
        for _ in 0..config.amount {
            let radius = config.radius * rng.gen_range(config.radius_range.clone());
            let color_delta = Rgba::new(
                rng.gen_range(config.color_range.clone()),
                rng.gen_range(config.color_range.clone()),
                rng.gen_range(config.color_range.clone()),
                0.0,
            );
            let color = config
                .color
                .zip_map(color_delta, |s, t| (s + t).clamp(0.0, 1.0));
            let angle = rng.gen_range(config.angle_range.clone());
            let velocity = config.velocity.rotate(angle);
            self.world.particles.push(Particle {
                initial_lifetime: config.lifetime,
                lifetime: config.lifetime,
                position: config.position,
                velocity,
                particle_type: ParticleType::Circle { radius, color },
            });
        }
    }

    pub fn process_particles(&mut self) {
        for particle in &mut self.world.particles {
            particle.lifetime -= self.delta_time;
            particle.position += particle.velocity * self.delta_time;
        }
        self.world
            .particles
            .retain(|particle| particle.lifetime > Time::ZERO);
    }
}

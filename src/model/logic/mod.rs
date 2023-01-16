use super::*;

struct Logic<'a> {
    world: &'a mut World,
    player_control: PlayerControl,
    delta_time: Time,
}

impl World {
    pub fn update(&mut self, player_control: PlayerControl, delta_time: Time) {
        let mut logic = Logic {
            world: self,
            player_control,
            delta_time,
        };
        logic.process();
    }
}

impl Logic<'_> {
    fn process(&mut self) {
        self.process_player();
        self.process_collisions();
        self.process_particles();
        self.process_camera();
    }

    fn kill_player(&mut self) {
        self.world.player.velocity = Vec2::ZERO;
        self.world.player.state = PlayerState::Respawning { time: Time::ONE };
    }

    fn next_level(&mut self) {
        if let Some(level) = self.world.level.next_level.clone() {
            self.world.level_transition = Some(level);
        } else {
            // TODO: exit game or smth
        }
    }

    fn process_player(&mut self) {
        match &mut self.world.player.state {
            PlayerState::Respawning { time } => {
                *time -= self.delta_time;
                if *time <= Time::ZERO {
                    self.world.player.state = PlayerState::Airborn;
                    self.world
                        .player
                        .collider
                        .teleport(self.world.level.spawn_point);
                }
                return;
            }
            PlayerState::Finished { time, next_heart } => {
                *time -= self.delta_time;
                if *time <= Time::ZERO {
                    self.next_level();
                    return;
                }
                *next_heart -= self.delta_time;
                if *next_heart <= Time::ZERO {
                    *next_heart += Time::new(0.5);
                    self.world.particles.push(Particle {
                        lifetime: Time::new(2.0),
                        position: self.world.level.finish
                            + vec2(Coord::ZERO, self.world.player.collider.raw().height()),
                        velocity: vec2(0.0, 1.5)
                            .rotate(thread_rng().gen_range(-0.5..=0.5))
                            .map(Coord::new),
                        particle_type: ParticleType::Heart4,
                    });
                }
                return;
            }
            _ => (),
        }

        let player = &mut self.world.player;
        if player.facing_left && player.velocity.x > Coord::ZERO
            || !player.facing_left && player.velocity.x < Coord::ZERO
        {
            player.facing_left = !player.facing_left;
        }

        if self.player_control.drill {
            if let Some(drill_dir) = match self.world.player.state {
                PlayerState::Grounded => Some(vec2(0.0, -1.0).map(Coord::new)),
                PlayerState::WallSliding { wall_normal } => Some(-wall_normal),
                _ => None,
            } {
                if Vec2::dot(self.player_control.move_dir, drill_dir) > Coord::ZERO {
                    self.world.player.velocity = self.player_control.move_dir.normalize_or_zero()
                        * self.world.rules.drill_speed;
                    self.world.player.state = PlayerState::Drilling;
                }
            }
        }

        if let PlayerState::Drilling = self.world.player.state {
            self.world
                .player
                .collider
                .translate(self.world.player.velocity * self.delta_time);
            return;
        }

        self.world.player.velocity += self.world.rules.gravity * self.delta_time;

        if self.world.player.velocity.y < Coord::ZERO {
            self.world.player.velocity.y += self.world.rules.gravity.y
                * (self.world.rules.fall_multiplier - Coord::ONE)
                * self.delta_time;
            let cap = match self.world.player.state {
                PlayerState::WallSliding { .. } => self.world.rules.wall_slide_speed,
                _ => self.world.rules.free_fall_speed,
            };
            self.world.player.velocity.y = self.world.player.velocity.y.clamp_abs(cap);
        } else if self.world.player.velocity.y > Coord::ZERO && !self.player_control.hold_jump {
            self.world.player.velocity.y += self.world.rules.gravity.y
                * (self.world.rules.low_jump_multiplier - Coord::ONE)
                * self.delta_time;
        }

        if let Some(time) = &mut self.world.player.control_timeout {
            *time -= self.delta_time;
            if *time <= Time::ZERO {
                self.world.player.control_timeout = None;
            }
        } else {
            let target = self.player_control.move_dir.x * self.world.rules.move_speed;
            let acc = if self.world.player.velocity.x.abs() > self.world.rules.move_speed {
                self.world.rules.low_control_acc
            } else {
                self.world.rules.full_control_acc
            };
            let current = self.world.player.velocity.x;
            // If target is aligned with velocity, then do not slow down
            if target == Coord::ZERO
                || target.signum() != current.signum()
                || target.abs() > current.abs()
            {
                self.world.player.velocity.x += (target - current).clamp_abs(acc * self.delta_time);
            }
        }

        if self.player_control.jump {
            let rules = &self.world.rules;
            match self.world.player.state {
                PlayerState::Grounded => {
                    let jump_vel = rules.normal_jump_strength;
                    self.world.player.velocity.y = jump_vel;
                    self.world.player.state = PlayerState::Airborn;
                }
                PlayerState::WallSliding { wall_normal } => {
                    let angle = rules.wall_jump_angle * wall_normal.x.signum();
                    let jump_vel = wall_normal.rotate(angle) * rules.wall_jump_strength;
                    self.world.player.velocity = jump_vel;
                    self.world.player.control_timeout = Some(self.world.rules.wall_jump_timeout);
                    self.world.player.state = PlayerState::Airborn;
                }
                _ => {}
            }
        }

        self.world
            .player
            .collider
            .translate(self.world.player.velocity * self.delta_time);
    }

    fn process_collisions(&mut self) {
        if let PlayerState::Respawning { .. } = self.world.player.state {
            return;
        }

        // Level bounds
        let level = &self.world.level;
        let level_bounds = level.bounds();
        let player = &mut self.world.player;
        if player.collider.feet().y > level_bounds.y_max {
            player.collider.translate(vec2(
                Coord::ZERO,
                level_bounds.y_max - player.collider.feet().y,
            ));
        }
        let offset = player.collider.feet().x - level_bounds.center().x;
        if offset.abs() > level_bounds.width() / Coord::new(2.0) {
            player.collider.translate(vec2(
                offset.signum() * (level_bounds.width() / Coord::new(2.0) - offset.abs()),
                Coord::ZERO,
            ));
        }

        let finished = matches!(self.world.player.state, PlayerState::Finished { .. });
        if finished {
            return;
        }
        let drilling = matches!(self.world.player.state, PlayerState::Drilling);
        if !drilling {
            self.world.player.state = PlayerState::Airborn;
        }
        let mut still_drilling = false;
        for _ in 0..2 {
            // Player-tiles
            let player_aabb = self.world.player.collider.grid_aabb(&self.world.level.grid);
            let collisions = (player_aabb.x_min..=player_aabb.x_max)
                .flat_map(move |x| (player_aabb.y_min..=player_aabb.y_max).map(move |y| vec2(x, y)))
                .filter(|&pos| {
                    self.world
                        .level
                        .tiles
                        .get_tile_isize(pos)
                        .filter(|tile| {
                            let air = matches!(tile, Tile::Air);
                            let drill = drilling && tile.is_drillable();
                            if !air && drill {
                                still_drilling = true;
                            }
                            !air && !drill
                        })
                        .is_some()
                })
                .filter_map(|pos| {
                    let collider = Collider::new(
                        AABB::point(self.world.level.grid.grid_to_world(pos))
                            .extend_positive(self.world.level.grid.cell_size),
                    );
                    self.world.player.collider.check(&collider)
                })
                .filter(|collision| {
                    Vec2::dot(collision.normal, self.world.player.velocity) >= Coord::ZERO
                });
            if let Some(collision) = collisions.max_by_key(|collision| collision.penetration) {
                self.world
                    .player
                    .collider
                    .translate(-collision.normal * collision.penetration);
                let bounciness = Coord::new(if drilling { 1.0 } else { 0.0 });
                self.world.player.velocity -= collision.normal
                    * Vec2::dot(self.world.player.velocity, collision.normal)
                    * (Coord::ONE + bounciness);
                if !drilling {
                    if collision.normal.x.approx_eq(&Coord::ZERO) {
                        self.world.player.state = PlayerState::Grounded;
                    } else if collision.normal.y.approx_eq(&Coord::ZERO)
                        && !matches!(self.world.player.state, PlayerState::Grounded)
                    {
                        self.world.player.state = PlayerState::WallSliding {
                            wall_normal: -collision.normal,
                        };
                    }
                }
            }
        }

        if drilling && !still_drilling {
            self.world.player.state = PlayerState::Airborn;
        }

        // Finish
        if !drilling && !finished && self.world.player.collider.contains(self.world.level.finish) {
            self.world.player.state = PlayerState::Finished {
                time: Time::new(2.0),
                next_heart: Time::new(0.5),
            };
            self.world.particles.push(Particle {
                lifetime: Time::new(2.0),
                position: self.world.player.collider.head()
                    + vec2(Coord::ZERO, self.world.player.collider.raw().height()),
                velocity: vec2(0.0, 1.5).map(Coord::new),
                particle_type: ParticleType::Heart8,
            });
            return;
        }

        // Player-coins
        for coin in &mut self.world.level.coins {
            if !coin.collected && self.world.player.collider.check(&coin.collider).is_some() {
                self.world.coins_collected += 1;
                coin.collected = true;
            }
        }
        self.world.level.coins.retain(|coin| !coin.collected);

        // Screen edge
        let player = &mut self.world.player;
        if player.collider.feet().y < level_bounds.y_min {
            self.kill_player();
            return;
        }

        // Player-hazards
        for hazard in &self.world.level.hazards {
            if self.world.player.collider.check(&hazard.collider).is_some()
                && hazard.direction.map_or(true, |dir| {
                    Vec2::dot(self.world.player.velocity, dir) <= Coord::ZERO
                })
            {
                self.kill_player();
                break;
            }
        }
    }

    fn process_particles(&mut self) {
        for particle in &mut self.world.particles {
            particle.lifetime -= self.delta_time;
            particle.position += particle.velocity * self.delta_time;
        }
        self.world
            .particles
            .retain(|particle| particle.lifetime > Time::ZERO);
    }

    fn process_camera(&mut self) {
        let level_bounds = self.world.level.bounds();
        let camera_view =
            vec2(self.world.camera.fov * (16.0 / 9.0), self.world.camera.fov).map(Coord::new); // TODO: remove hardcode
        let camera_bounds = AABB::from_corners(
            level_bounds.bottom_left() + camera_view,
            level_bounds.top_right() - camera_view,
        );
        let target = self.world.player.collider.pos();
        let target = target.clamp_aabb(camera_bounds);
        self.world.camera.center = target.map(Coord::as_f32);
    }
}

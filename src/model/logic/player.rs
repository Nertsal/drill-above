use super::*;

impl Player {
    fn update_timers(&mut self, delta_time: Time) {
        // Coyote Time
        if let Some((_, time)) = &mut self.coyote_time {
            *time -= delta_time;
            if *time <= Time::ZERO {
                self.coyote_time = None;
            }
        }

        // Jump Buffer
        if let Some(time) = &mut self.jump_buffer {
            *time -= delta_time;
            if *time <= Time::ZERO {
                self.jump_buffer = None;
            }
        }

        // Drill Dash
        if let PlayerState::AirDrill { dash } = &mut self.state {
            if let Some(time) = dash {
                *time -= delta_time;
                if *time <= Time::ZERO {
                    *dash = None;
                }
            }
        }

        // Controll timeout
        if let Some(time) = &mut self.control_timeout {
            // No horizontal control
            *time -= delta_time;
            if *time <= Time::ZERO {
                self.control_timeout = None;
            }
        }
    }
}

impl Logic<'_> {
    pub fn process_player(&mut self) {
        if !matches!(self.world.player.state, PlayerState::Drilling) {
            if let Some(mut sound) = self.world.drill_sound.take() {
                sound.stop();
            }
        }

        self.world.player.update_timers(self.delta_time);

        // Drill Dash Cancel
        self.drill_dash_cancel();

        // Update Jump Buffer
        if self.player_control.jump {
            self.world.player.jump_buffer = Some(self.world.rules.jump_buffer_time);
        }

        // Update Jump Hold
        if self.world.player.can_hold_jump && !self.player_control.hold_jump {
            self.world.player.can_hold_jump = false;
        }

        // Pause states
        if self.pause_state() {
            return;
        }

        self.restore_drill_dash();
        self.drill_dash();

        // Update look direction
        let player = &mut self.world.player;
        if player.facing_left && player.velocity.x > Coord::ZERO
            || !player.facing_left && player.velocity.x < Coord::ZERO
        {
            player.facing_left = !player.facing_left;
        }

        // Drill or Drill Dash - no control or gravity
        if !matches!(
            self.world.player.state,
            PlayerState::Drilling | PlayerState::AirDrill { dash: Some(_) }
        ) {
            // Apply gravity
            self.world.player.velocity += self.world.rules.gravity * self.delta_time;

            self.variable_jump();
            self.horizontal_control();
            self.jump();
        }

        self.world
            .player
            .collider
            .translate(self.world.player.velocity * self.delta_time);
    }

    fn pause_state(&mut self) -> bool {
        match &mut self.world.player.state {
            PlayerState::Respawning { time } => {
                *time -= self.delta_time;
                if *time <= Time::ZERO {
                    // Respawn
                    self.world.player.state = PlayerState::Airborn;
                    self.world.player.velocity = Vec2::ZERO;
                    self.world
                        .player
                        .collider
                        .teleport(self.world.level.spawn_point);
                }
                true
            }
            PlayerState::Finished { time, next_heart } => {
                *time -= self.delta_time;
                if *time <= Time::ZERO {
                    // Level transition
                    self.next_level();
                    return true;
                }
                *next_heart -= self.delta_time;
                if *next_heart <= Time::ZERO {
                    *next_heart += Time::new(0.5);
                    self.world.particles.push(Particle {
                        initial_lifetime: Time::new(2.0),
                        lifetime: Time::new(2.0),
                        position: self.world.level.finish
                            + vec2(Coord::ZERO, self.world.player.collider.raw().height()),
                        velocity: vec2(0.0, 1.5)
                            .rotate(thread_rng().gen_range(-0.5..=0.5))
                            .map(Coord::new),
                        particle_type: ParticleType::Heart4,
                    });
                }
                self.world.player.velocity += self.world.rules.gravity * self.delta_time;
                self.world.player.velocity.x = Coord::ZERO;
                self.world
                    .player
                    .collider
                    .translate(self.world.player.velocity * self.delta_time);
                true
            }
            _ => false,
        }
    }

    fn drill_dash(&mut self) {
        // Drill Dash
        if let PlayerState::Drilling = self.world.player.state {
            self.world.player.can_drill_dash = false;
            return;
        }

        if !self.player_control.drill
            || matches!(self.world.player.state, PlayerState::AirDrill { .. })
        {
            return;
        }

        let mut dash = None;
        let dir = self.player_control.move_dir;
        if self.world.player.can_drill_dash && dir != Vec2::ZERO {
            // Dash
            let dir = dir.normalize_or_zero();
            let vel_dir = self.world.player.velocity.normalize_or_zero();
            let rules = &self.world.rules;
            let acceleration = rules.drill_dash_speed_inc;
            let speed = self.world.player.velocity.len();
            let angle = Coord::new(Vec2::dot(vel_dir, dir).as_f32().acos() / 2.0);
            let current = speed * angle.cos();
            let speed = (current + acceleration).max(rules.drill_dash_speed_min);
            self.world.player.velocity = dir * speed;
            self.world.player.can_drill_dash = false;
            dash = Some(self.world.rules.drill_dash_time);

            self.spawn_particles(ParticleSpawn {
                lifetime: Time::ONE,
                position: self.world.player.collider.pos(),
                velocity: -vel_dir * Coord::new(0.5),
                amount: 5,
                color: Rgba::from_rgb(0.8, 0.25, 0.2),
                radius: Coord::new(0.2),
                ..Default::default()
            });
        }

        // Turn into a drill
        self.world.player.state = PlayerState::AirDrill { dash };
    }

    fn drill_dash_cancel(&mut self) {
        let PlayerState::AirDrill { dash: None } = &mut self.world.player.state else {
            // Cannot cancel yet
            return;
        };

        if self.player_control.hold_drill {
            // Input holds dash
            return;
        }

        // Turn back from drill
        let mut player = &mut self.world.player;
        player.state = PlayerState::Airborn;

        if player.drill_release.take().is_some() {
            // No slow-down
            return;
        }

        // Slow-down punishment
        let spawn = ParticleSpawn {
            lifetime: Time::new(0.3),
            position: player.collider.pos(),
            velocity: player.velocity,
            amount: 5,
            color: Rgba::from_rgb(0.6, 0.6, 0.6),
            radius: Coord::new(0.4),
            angle_range: Coord::new(-0.1)..=Coord::new(0.1),
            ..Default::default()
        };
        player.velocity.x = player.velocity.x.clamp_abs(self.world.rules.move_speed);
        self.spawn_particles(spawn);
    }

    fn restore_drill_dash(&mut self) {
        // Restore Drill Dash
        // Spawn particles on walk/wallslide
        match self.world.player.state {
            PlayerState::Grounded(..) => {
                self.world.player.can_drill_dash = true;
                if self.world.player.velocity.x.abs() > Coord::new(0.1)
                    && thread_rng().gen_bool(0.1)
                {
                    self.spawn_particles(ParticleSpawn {
                        lifetime: Time::ONE,
                        position: self.world.player.collider.feet(),
                        velocity: vec2(self.world.player.velocity.x.signum(), Coord::ONE)
                            * Coord::new(0.5),
                        amount: 2,
                        color: Rgba::from_rgb(0.8, 0.8, 0.8),
                        radius: Coord::new(0.1),
                        ..Default::default()
                    });
                }
            }
            PlayerState::WallSliding { wall_normal, .. } => {
                self.world.player.can_drill_dash = true;
                if self.world.player.velocity.y < Coord::new(-0.1) && thread_rng().gen_bool(0.1) {
                    self.spawn_particles(ParticleSpawn {
                        lifetime: Time::ONE,
                        position: self.world.player.collider.pos()
                            - wall_normal
                                * self.world.player.collider.raw().width()
                                * Coord::new(0.5),
                        velocity: vec2(wall_normal.x * Coord::new(0.2), Coord::ONE)
                            * Coord::new(0.5),
                        amount: 2,
                        color: Rgba::from_rgb(0.8, 0.8, 0.8),
                        radius: Coord::new(0.1),
                        ..Default::default()
                    });
                }
            }
            _ => (),
        }
    }

    fn variable_jump(&mut self) {
        if matches!(self.world.player.state, PlayerState::AirDrill { .. }) {
            return;
        }

        // Variable jump height
        if self.world.player.velocity.y < Coord::ZERO {
            // Faster drop
            self.world.player.velocity.y += self.world.rules.gravity.y
                * (self.world.rules.fall_multiplier - Coord::ONE)
                * self.delta_time;
            let cap = match self.world.player.state {
                PlayerState::WallSliding { .. } => self.world.rules.wall_slide_speed,
                _ => self.world.rules.free_fall_speed,
            };
            self.world.player.velocity.y = self.world.player.velocity.y.clamp_abs(cap);
        } else if self.world.player.velocity.y > Coord::ZERO
            && !(self.player_control.hold_jump && self.world.player.can_hold_jump)
        {
            // Low jump
            self.world.player.velocity.y += self.world.rules.gravity.y
                * (self.world.rules.low_jump_multiplier - Coord::ONE)
                * self.delta_time;
        }
    }

    fn horizontal_control(&mut self) {
        if self.world.player.control_timeout.is_some()
            || matches!(self.world.player.state, PlayerState::AirDrill { .. })
        {
            return;
        }

        // Horizontal speed control
        let target = self.player_control.move_dir.x * self.world.rules.move_speed;
        let acc = if self.world.player.velocity.x.abs() > self.world.rules.move_speed {
            self.world.rules.low_control_acc
        } else {
            self.world.rules.full_control_acc
        };
        let current = self.world.player.velocity.x;

        // If target speed is aligned with velocity, then do not slow down
        if target == Coord::ZERO
            || target.signum() != current.signum()
            || target.abs() > current.abs()
        {
            self.world.player.velocity.x += (target - current).clamp_abs(acc * self.delta_time);
        }
    }

    fn jump(&mut self) {
        if self.world.player.jump_buffer.is_none() {
            return;
        }

        // Try jump
        let rules = &self.world.rules;
        let jump = match self.world.player.state {
            PlayerState::Grounded { .. } => Some(Coyote::Ground),
            PlayerState::WallSliding { wall_normal, .. } => Some(Coyote::Wall { wall_normal }),
            PlayerState::Airborn | PlayerState::AirDrill { .. } => {
                self.world.player.coyote_time.map(|(coyote, _)| coyote)
            }
            _ => None,
        };
        let Some(jump) = jump else { return };

        // Use jump
        self.world.player.coyote_time = None;
        self.world.player.jump_buffer = None;
        self.world.player.can_hold_jump = true;
        match jump {
            Coyote::Ground => {
                let jump_vel = rules.normal_jump_strength;
                self.world.player.velocity.y = jump_vel;
                self.world.player.state = PlayerState::Airborn;
                self.play_sound(&self.world.assets.sounds.jump);
                self.spawn_particles(ParticleSpawn {
                    lifetime: Time::ONE,
                    position: self.world.player.collider.feet(),
                    velocity: vec2(Coord::ZERO, Coord::ONE),
                    amount: 3,
                    color: Rgba::WHITE,
                    radius: Coord::new(0.1),
                    ..Default::default()
                });
            }
            Coyote::Wall { wall_normal } => {
                let angle = rules.wall_jump_angle * wall_normal.x.signum();
                let jump_vel = wall_normal.rotate(angle) * rules.wall_jump_strength;
                self.world.player.velocity = jump_vel;
                self.world.player.control_timeout = Some(self.world.rules.wall_jump_timeout);
                self.world.player.state = PlayerState::Airborn;
                self.play_sound(&self.world.assets.sounds.jump);
                self.spawn_particles(ParticleSpawn {
                    lifetime: Time::ONE,
                    position: self.world.player.collider.feet()
                        - wall_normal * self.world.player.collider.raw().width() * Coord::new(0.5),
                    velocity: jump_vel.normalize_or_zero(),
                    amount: 3,
                    color: Rgba::WHITE,
                    radius: Coord::new(0.1),
                    ..Default::default()
                });
            }
            Coyote::DrillJump { direction } => {
                let rules = &self.world.rules;
                let acceleration = rules.drill_jump_speed_inc;
                let current = Vec2::dot(self.world.player.velocity, direction);
                self.world.player.velocity =
                    direction * (current + acceleration).max(rules.drill_jump_speed_min);
                self.play_sound(&self.world.assets.sounds.drill_jump);
                self.spawn_particles(ParticleSpawn {
                    lifetime: Time::ONE,
                    position: self.world.player.collider.pos(),
                    velocity: direction,
                    amount: 5,
                    color: Rgba::from_rgb(0.8, 0.25, 0.2),
                    radius: Coord::new(0.3),
                    ..Default::default()
                });
            }
        }
    }
}

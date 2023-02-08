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
        // Update collider
        let actor = self.world.actors.get_mut(&self.world.player.id).unwrap();
        let pos = actor.collider.pos();
        actor.collider = if self.world.player.state.using_drill() {
            Player::drill_collider()
        } else {
            Player::collider()
        };
        actor.collider.translate(pos);

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
        } else if self.world.player.state.is_drilling()
            && self.player_control.move_dir != vec2::ZERO
        {
            if let Some((Coyote::DrillDirection { initial }, _)) = self.world.player.coyote_time {
                // Change drill direction
                if vec2::dot(self.player_control.move_dir, initial) >= Coord::ZERO {
                    self.world.player.velocity = self.player_control.move_dir.normalize_or_zero()
                        * self.world.player.velocity.len();
                    self.world.player.coyote_time = None;
                }
            }
        }
    }

    pub fn player_movement(&mut self) {
        let delta = self.world.player.velocity * self.delta_time;
        let callback: CollisionCallback = Rc::new(|logic, _id, col| {
            let player = &mut logic.world.player;
            if player.state.is_wall_sliding() {
                player.state = PlayerState::Airborn;
            }

            let bounciness = Coord::new(if player.state.using_drill() { 1.0 } else { 0.0 } + 1.0);
            if let Some((_, col)) = col.y {
                player.velocity -= col.normal * vec2::dot(player.velocity, col.normal) * bounciness;
            }

            let player = &mut logic.world.player;
            if let Some((_, col)) = col.x {
                player.velocity -= col.normal * vec2::dot(player.velocity, col.normal) * bounciness;
            }
        });
        self.move_actor(self.world.player.id, delta, Some(callback));

        if let PlayerState::Respawning { .. } = self.world.player.state {
            return;
        }

        // Level bounds
        if self.level_bounds() {
            return;
        }

        // Stay in finish state
        if let Some(state) = self.world.player.state.finished_state() {
            self.world.player.state = state;
            return;
        }

        self.update_state();

        self.player_coins();

        // Finish
        if self.check_finish() {
            return;
        }

        self.player_hazards();
    }

    fn pause_state(&mut self) -> bool {
        let actor = self.world.actors.get_mut(&self.world.player.id).unwrap();
        match &mut self.world.player.state {
            PlayerState::Respawning { time } => {
                *time -= self.delta_time;
                if *time <= Time::ZERO {
                    // Respawn
                    self.world.player.state = PlayerState::Airborn;
                    self.world.player.velocity = vec2::ZERO;
                    actor.collider.teleport(self.world.level.spawn_point);
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
                            + vec2(Coord::ZERO, actor.collider.raw().height()),
                        velocity: vec2(0.0, 1.5)
                            .rotate(thread_rng().gen_range(-0.5..=0.5))
                            .map(Coord::new),
                        particle_type: ParticleType::Heart4,
                    });
                }
                self.world.player.velocity += self.world.rules.gravity * self.delta_time;
                self.world.player.velocity.x = Coord::ZERO;
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

        if !self.world.level.drill_allowed
            || !self.player_control.drill
            || matches!(self.world.player.state, PlayerState::AirDrill { .. })
        {
            return;
        }

        let mut dash = None;
        let dir = self.player_control.move_dir;
        if self.world.rules.drill.can_dash && self.world.player.can_drill_dash && dir != vec2::ZERO
        {
            // Dash
            let dir = dir.normalize_or_zero();
            let vel_dir = self.world.player.velocity.normalize_or_zero();
            let rules = &self.world.rules;
            // let acceleration = rules.drill_dash_speed_inc;
            // let speed = self.world.player.velocity.len();
            // let angle = Coord::new(vec2::dot(vel_dir, dir).as_f32().acos() / 2.0);
            // let current = speed * angle.cos();
            // let speed = (current + acceleration).max(rules.drill_dash_speed_min);
            let speed = rules.drill.dash_speed_min;
            let mut target = dir * speed;

            let real = self.world.player.velocity;
            if target.x != Coord::ZERO
                && target.x.signum() == real.x.signum()
                && real.x.abs() > target.x.abs()
            {
                target.x = real.x;
            }
            if target.y != Coord::ZERO
                && target.y.signum() == real.y.signum()
                && real.y.abs() > target.y.abs()
            {
                target.y = real.y;
            }

            self.world.player.velocity = target;
            self.world.player.can_drill_dash = false;
            dash = Some(self.world.rules.drill.dash_time);

            let actor = self.world.actors.get(&self.world.player.id).unwrap();
            self.spawn_particles(ParticleSpawn {
                lifetime: Time::ONE,
                position: actor.collider.pos(),
                velocity: -vel_dir * Coord::new(0.5),
                amount: 5,
                color: Rgba::opaque(0.8, 0.25, 0.2),
                radius: Coord::new(0.2),
                ..Default::default()
            });
        } else if !matches!(self.world.player.state, PlayerState::Drilling)
            && self.player_control.drill
            && self.world.level.drill_allowed
        {
            let dirs = itertools::chain![
                match self.world.player.state {
                    PlayerState::Grounded(tile) if self.world.rules.tiles[&tile].drillable =>
                        Some(vec2(0.0, -1.0).map(Coord::new)),
                    PlayerState::WallSliding { tile, wall_normal }
                        if self.world.rules.tiles[&tile].drillable =>
                        Some(-wall_normal),
                    _ => None,
                },
                self.world
                    .player
                    .touching_wall
                    .and_then(|(tile, normal)| self.world.rules.tiles[&tile]
                        .drillable
                        .then_some(-normal))
            ];
            for drill_dir in dirs {
                if vec2::dot(self.player_control.move_dir, drill_dir) > Coord::ZERO {
                    self.world.player.velocity = self.player_control.move_dir.normalize_or_zero()
                        * self.world.rules.drill.speed_min;
                }
            }
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
        player.velocity.x = player.velocity.x.clamp_abs(self.world.rules.move_speed);
        let actor = self.world.actors.get(&player.id).unwrap();
        let spawn = ParticleSpawn {
            lifetime: Time::new(0.3),
            position: actor.collider.pos(),
            velocity: player.velocity,
            amount: 5,
            color: Rgba::opaque(0.6, 0.6, 0.6),
            radius: Coord::new(0.4),
            angle_range: Coord::new(-0.1)..=Coord::new(0.1),
            ..Default::default()
        };
        self.spawn_particles(spawn);
    }

    fn restore_drill_dash(&mut self) {
        // Restore Drill Dash
        // Spawn particles on walk/wallslide
        let actor = self.world.actors.get(&self.world.player.id).unwrap();
        match self.world.player.state {
            PlayerState::Grounded(..) => {
                self.world.player.can_drill_dash = true;
                if self.world.player.velocity.x.abs() > Coord::new(0.1)
                    && thread_rng().gen_bool(0.1)
                {
                    self.spawn_particles(ParticleSpawn {
                        lifetime: Time::ONE,
                        position: actor.collider.feet(),
                        velocity: vec2(self.world.player.velocity.x.signum(), Coord::ONE)
                            * Coord::new(0.5),
                        amount: 2,
                        color: Rgba::opaque(0.8, 0.8, 0.8),
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
                        position: actor.collider.pos()
                            - wall_normal * actor.collider.raw().width() * Coord::new(0.5),
                        velocity: vec2(wall_normal.x * Coord::new(0.2), Coord::ONE)
                            * Coord::new(0.5),
                        amount: 2,
                        color: Rgba::opaque(0.8, 0.8, 0.8),
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
                * (self.world.rules.jump.fall_multiplier - Coord::ONE)
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
                * (self.world.rules.jump.low_multiplier - Coord::ONE)
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
        let current = self.world.player.velocity.x;
        let target = self.player_control.move_dir.x * self.world.rules.move_speed;

        let mut acc = if self.world.player.velocity.x.abs() > self.world.rules.move_speed {
            self.world.rules.low_control_acc
        } else {
            self.world.rules.full_control_acc
        };
        if let PlayerState::Grounded(tile) = &self.world.player.state {
            acc += self.world.rules.tiles[tile].friction * self.world.rules.gravity.len();
        }

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
        let actor = self.world.actors.get(&self.world.player.id).unwrap();
        match jump {
            Coyote::Ground => {
                let jump_vel = rules.jump.normal_strength;
                self.world.player.velocity.y = jump_vel;
                self.world.player.state = PlayerState::Airborn;
                self.world.play_sound(&self.world.assets.sounds.jump);
                self.spawn_particles(ParticleSpawn {
                    lifetime: Time::ONE,
                    position: actor.collider.feet(),
                    velocity: vec2(Coord::ZERO, Coord::ONE),
                    amount: 3,
                    color: Rgba::WHITE,
                    radius: Coord::new(0.1),
                    ..Default::default()
                });
            }
            Coyote::Wall { wall_normal } => {
                let angle = rules.jump.wall_angle * wall_normal.x.signum();
                let mut jump_vel = wall_normal.rotate(angle) * rules.jump.wall_strength;
                let player = &mut self.world.player;
                jump_vel.y = jump_vel.y.max(player.velocity.y);
                player.velocity = jump_vel;
                player.control_timeout = Some(self.world.rules.jump.wall_timeout);
                player.state = PlayerState::Airborn;
                self.world.play_sound(&self.world.assets.sounds.jump);
                self.spawn_particles(ParticleSpawn {
                    lifetime: Time::ONE,
                    position: actor.collider.feet()
                        - wall_normal * actor.collider.raw().width() * Coord::new(0.5),
                    velocity: jump_vel.normalize_or_zero(),
                    amount: 3,
                    color: Rgba::WHITE,
                    radius: Coord::new(0.1),
                    ..Default::default()
                });
            }
            Coyote::DrillJump { direction } => {
                let rules = &self.world.rules;
                let acceleration = rules.drill.jump_speed_inc;
                let current = vec2::dot(self.world.player.velocity, direction);
                self.world.player.velocity =
                    direction * (current + acceleration).max(rules.drill.jump_speed_min);
                self.world.play_sound(&self.world.assets.sounds.drill_jump);
                self.spawn_particles(ParticleSpawn {
                    lifetime: Time::ONE,
                    position: actor.collider.pos(),
                    velocity: direction,
                    amount: 5,
                    color: Rgba::opaque(0.8, 0.25, 0.2),
                    radius: Coord::new(0.3),
                    ..Default::default()
                });
            }
            Coyote::DrillDirection { .. } => {}
        }
    }

    fn check_wall(&mut self) {
        let player = &mut self.world.player;

        player.touching_wall = None;

        let update_state = player.state.is_airborn() || player.state.is_wall_sliding();
        if update_state {
            player.state = PlayerState::Airborn;
        }

        let actor = self.world.actors.get(&self.world.player.id).unwrap();
        let collider = actor.wall_collider();

        if let Some((id, col)) = self.check_collision(&collider) {
            let player = &mut self.world.player;
            let tile = match id {
                ColliderId::Tile(pos) => self.world.level.tiles.get_tile_isize(pos).unwrap(),
                ColliderId::Entity(id) => self.world.blocks.get(&id).unwrap().tile,
            };
            let wall_normal = col.normal;
            player.touching_wall = Some((tile, wall_normal));

            if let PlayerState::Airborn = player.state {
                player.state = PlayerState::WallSliding { tile, wall_normal };
                player.coyote_time =
                    Some((Coyote::Wall { wall_normal }, self.world.rules.coyote_time));
            }
        }
    }

    fn check_ground(&mut self) {
        let player = &mut self.world.player;
        let was_grounded = player.state.is_grounded();
        if was_grounded {
            player.state = PlayerState::Airborn;
        }
        let update_state =
            (player.state.is_airborn() || was_grounded || player.state.is_wall_sliding())
                && player.velocity.y <= Coord::ZERO;

        if update_state {
            let actor = self.world.actors.get(&self.world.player.id).unwrap();
            let collider = actor.feet_collider();

            if let Some((id, _)) = self.check_collision(&collider) {
                let player = &mut self.world.player;
                let tile = match id {
                    ColliderId::Tile(pos) => self.world.level.tiles.get_tile_isize(pos).unwrap(),
                    ColliderId::Entity(id) => self.world.blocks.get(&id).unwrap().tile,
                };
                player.state = PlayerState::Grounded(tile);
                player.coyote_time = Some((Coyote::Ground, self.world.rules.coyote_time));

                if !was_grounded {
                    // Just landed
                    let spawn = ParticleSpawn {
                        lifetime: Time::ONE,
                        position: actor.collider.feet(),
                        velocity: vec2(Coord::ZERO, Coord::ONE) * Coord::new(0.5),
                        amount: 3,
                        color: Rgba::WHITE,
                        radius: Coord::new(0.1),
                        ..Default::default()
                    };
                    self.spawn_particles(spawn);
                }
            }
        }
    }

    fn check_tiles(&self) -> bool {
        let player = self.world.actors.get(&self.world.player.id).unwrap();
        self.world
            .level
            .grid
            .tile_collisions(&player.collider)
            .any(|pos| {
                self.world
                    .level
                    .tiles
                    .get_tile_isize(pos)
                    .filter(|tile| {
                        let air = matches!(tile, Tile::Air);
                        let drill = self.world.rules.tiles[tile].drillable;
                        !air && drill
                    })
                    .filter(|_| {
                        let collider = Collider::new(
                            Aabb2::point(self.world.level.grid.grid_to_world(pos))
                                .extend_positive(self.world.level.grid.cell_size),
                        );
                        player.collider.check(&collider)
                    })
                    .is_some()
            })
    }

    fn update_state(&mut self) {
        self.check_wall();
        self.check_ground();
        let can_drill = self.check_tiles();
        let actor = self.world.actors.get(&self.world.player.id).unwrap();

        // Update drill state
        if self.world.player.state.is_drilling() {
            if !can_drill {
                // Exited the ground in drill mode
                self.world.player.can_drill_dash = true;
                self.world.player.state = if self.player_control.hold_drill {
                    self.world.player.drill_release = Some(self.world.rules.drill.release_time);
                    PlayerState::AirDrill { dash: None }
                } else {
                    PlayerState::Airborn
                };

                let direction = self.world.player.velocity.normalize_or_zero();
                self.world.player.coyote_time = Some((
                    Coyote::DrillJump { direction },
                    self.world.rules.coyote_time,
                ));
                self.spawn_particles(ParticleSpawn {
                    lifetime: Time::ONE,
                    position: actor.collider.pos(),
                    velocity: direction * Coord::new(0.3),
                    amount: 8,
                    color: Rgba::opaque(0.7, 0.7, 0.7),
                    radius: Coord::new(0.2),
                    ..Default::default()
                });
            } else if thread_rng().gen_bool(0.2) {
                // Drilling through the ground
                self.spawn_particles(ParticleSpawn {
                    lifetime: Time::ONE,
                    position: actor.collider.pos(),
                    velocity: -self.world.player.velocity.normalize_or_zero() * Coord::new(0.5),
                    amount: 2,
                    color: Rgba::opaque(0.8, 0.8, 0.8),
                    radius: Coord::new(0.1),
                    ..Default::default()
                });
            }
        } else if self.world.player.state.is_air_drilling() && can_drill {
            // Entered the ground in drill mode
            let speed = self.world.player.velocity.len();
            let dir = self.world.player.velocity.normalize_or_zero();

            self.world.player.coyote_time = Some((
                Coyote::DrillDirection { initial: dir },
                self.world.rules.coyote_time,
            ));
            self.world.player.velocity = dir * speed.max(self.world.rules.drill.speed_min);
            self.world.player.state = PlayerState::Drilling;

            self.spawn_particles(ParticleSpawn {
                lifetime: Time::ONE,
                position: actor.collider.pos(),
                velocity: -dir * Coord::new(0.3),
                amount: 5,
                color: Rgba::opaque(0.7, 0.7, 0.7),
                radius: Coord::new(0.2),
                ..Default::default()
            });

            let sound = self
                .world
                .drill_sound
                .get_or_insert_with(|| self.world.assets.sounds.drill.play());
            sound.set_volume(self.world.volume);
        }
    }

    fn check_finish(&mut self) -> bool {
        let actor = self.world.actors.get(&self.world.player.id).unwrap();
        if self.world.player.state.is_drilling()
            || self.world.player.state.has_finished()
            || !actor.collider.check(&self.world.level.finish())
        {
            return false;
        }

        self.world.player.state = PlayerState::Finished {
            time: Time::new(2.0),
            next_heart: Time::new(0.5),
        };
        self.world.particles.push(Particle {
            initial_lifetime: Time::new(2.0),
            lifetime: Time::new(2.0),
            position: actor.collider.head() + vec2(Coord::ZERO, actor.collider.raw().height()),
            velocity: vec2(0.0, 1.5).map(Coord::new),
            particle_type: ParticleType::Heart8,
        });
        self.world.play_sound(&self.world.assets.sounds.charm);

        true
    }

    fn player_coins(&mut self) {
        // Collect coins
        let mut collected = None;
        let actor = self.world.actors.get(&self.world.player.id).unwrap();
        for coin in &mut self.world.level.coins {
            if !coin.collected && actor.collider.check(&coin.collider) {
                self.world.coins_collected += 1;
                coin.collected = true;
                collected = Some(coin.collider.pos());
            }
        }
        self.world.level.coins.retain(|coin| !coin.collected);
        if let Some(position) = collected {
            self.world.play_sound(&self.world.assets.sounds.coin);
            self.spawn_particles(ParticleSpawn {
                lifetime: Time::ONE,
                position,
                velocity: vec2(Coord::ZERO, Coord::ONE) * Coord::new(0.5),
                amount: 5,
                color: Rgba::try_from("#e3a912").unwrap(),
                radius: Coord::new(0.2),
                ..Default::default()
            });
        }
    }

    fn player_hazards(&mut self) {
        // Die from hazards
        let actor = self.world.actors.get(&self.world.player.id).unwrap();
        for hazard in &self.world.level.hazards {
            if actor.collider.check(&hazard.collider)
                && hazard.direction.map_or(true, |dir| {
                    vec2::dot(self.world.player.velocity, dir) <= Coord::ZERO
                })
            {
                self.world.kill_player();
                break;
            }
        }
    }

    fn level_bounds(&mut self) -> bool {
        let level = &self.world.level;
        let level_bounds = level.bounds();
        let player = &mut self.world.player;
        let actor = self.world.actors.get_mut(&player.id).unwrap();

        // Top
        if actor.collider.head().y > level_bounds.max.y {
            actor.collider.translate(vec2(
                Coord::ZERO,
                level_bounds.max.y - actor.collider.head().y,
            ));
            player.velocity.y = if player.state.is_drilling() {
                -player.velocity.y
            } else {
                Coord::ZERO
            };
        }

        // Horizontal
        let offset = actor.collider.feet().x - level_bounds.center().x;
        if offset.abs() > level_bounds.width() / Coord::new(2.0) {
            actor.collider.translate(vec2(
                offset.signum() * (level_bounds.width() / Coord::new(2.0) - offset.abs()),
                Coord::ZERO,
            ));
            player.velocity.x = Coord::ZERO;
        }

        // Bottom
        if actor.collider.feet().y < level_bounds.min.y {
            self.world.kill_player();
            return true;
        }

        false
    }
}

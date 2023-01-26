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
        if !matches!(self.world.player.state, PlayerState::Finished { .. }) {
            self.world.time += self.delta_time;
        }

        self.process_player();
        self.process_collisions();
        self.process_particles();
        self.process_camera();
    }

    fn play_sound(&self, sound: &geng::Sound) {
        let mut sound = sound.play();
        sound.set_volume(self.world.volume);
    }

    fn kill_player(&mut self) {
        self.world.player.velocity = Vec2::ZERO;
        self.world.player.state = PlayerState::Respawning { time: Time::ONE };
        self.world.deaths += 1;
        self.play_sound(&self.world.assets.sounds.death);
    }

    fn next_level(&mut self) {
        if let Some(level) = self.world.level.next_level.clone() {
            self.world.level_transition = Some(level);
        } else {
            self.world.level_transition = Some("credits.json".to_string());
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn spawn_particles(
        &mut self,
        lifetime: Time,
        position: Vec2<Coord>,
        direction: Vec2<Coord>,
        speed: Coord,
        amount: usize,
        base_color: Rgba<f32>,
        base_radius: Coord,
    ) {
        let mut rng = thread_rng();
        for _ in 0..amount {
            let radius = base_radius * Coord::new(rng.gen_range(0.9..=1.1));
            let color_delta = Rgba::new(
                rng.gen_range(-0.05..=0.05),
                rng.gen_range(-0.05..=0.05),
                rng.gen_range(-0.05..=0.05),
                0.0,
            );
            let color = base_color.zip_map(color_delta, |s, t| (s + t).clamp(0.0, 1.0));
            let angle = Coord::new(rng.gen_range(-0.5..=0.5));
            let direction = direction.rotate(angle);
            self.world.particles.push(Particle {
                lifetime,
                position,
                velocity: direction * speed,
                particle_type: ParticleType::Circle { radius, color },
            });
        }
    }

    fn process_player(&mut self) {
        if let Some((_, time)) = &mut self.world.player.coyote_time {
            *time -= self.delta_time;
            if *time <= Time::ZERO {
                self.world.player.coyote_time = None;
            }
        }

        if !matches!(self.world.player.state, PlayerState::Drilling) {
            if let Some(mut sound) = self.world.drill_sound.take() {
                sound.stop();
            }
        }

        if self.world.player.can_hold_jump && !self.player_control.hold_jump {
            self.world.player.can_hold_jump = false;
        }

        if let Some(time) = &mut self.world.player.jump_buffer {
            *time -= self.delta_time;
            if *time <= Time::ZERO {
                self.world.player.jump_buffer = None;
            }
        }
        if let PlayerState::AirDrill { dash } = &mut self.world.player.state {
            if let Some(time) = dash {
                *time -= self.delta_time;
                if *time <= Time::ZERO {
                    *dash = None;
                }
            } else if !self.player_control.hold_drill {
                let mut player = &mut self.world.player;
                player.state = PlayerState::Airborn;
                player.velocity.x = player.velocity.x.clamp_abs(self.world.rules.move_speed);
            }
        }

        if self.player_control.jump {
            self.world.player.jump_buffer = Some(self.world.rules.jump_buffer_time);
        }

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
                self.world.player.velocity += self.world.rules.gravity * self.delta_time;
                self.world.player.velocity.x = Coord::ZERO;
                self.world
                    .player
                    .collider
                    .translate(self.world.player.velocity * self.delta_time);
                return;
            }
            _ => (),
        }

        if let PlayerState::Drilling = self.world.player.state {
            self.world.player.can_drill_dash = false;
        } else if self.player_control.drill
            && !matches!(self.world.player.state, PlayerState::AirDrill { .. })
        {
            let mut dash = None;
            if self.world.player.can_drill_dash {
                let dir = self.player_control.move_dir.normalize_or_zero();
                if dir != Vec2::ZERO {
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

                    self.spawn_particles(
                        Time::ONE,
                        self.world.player.collider.pos(),
                        -vel_dir,
                        Coord::new(0.5),
                        5,
                        Rgba::from_rgb(0.8, 0.25, 0.2),
                        Coord::new(0.2),
                    );
                }
            }
            self.world.player.state = PlayerState::AirDrill { dash };
        }

        match self.world.player.state {
            PlayerState::Grounded(..) => {
                self.world.player.can_drill_dash = true;
                if self.world.player.velocity.x.abs() > Coord::new(0.1)
                    && thread_rng().gen_bool(0.1)
                {
                    self.spawn_particles(
                        Time::ONE,
                        self.world.player.collider.feet(),
                        vec2(self.world.player.velocity.x.signum(), Coord::ONE),
                        Coord::new(0.5),
                        2,
                        Rgba::from_rgb(0.8, 0.8, 0.8),
                        Coord::new(0.1),
                    );
                }
            }
            PlayerState::WallSliding { wall_normal, .. } => {
                self.world.player.can_drill_dash = true;
                if self.world.player.velocity.y < Coord::new(-0.1) && thread_rng().gen_bool(0.1) {
                    self.spawn_particles(
                        Time::ONE,
                        self.world.player.collider.pos()
                            - wall_normal
                                * self.world.player.collider.raw().width()
                                * Coord::new(0.5),
                        vec2(wall_normal.x * Coord::new(0.2), Coord::ONE),
                        Coord::new(0.5),
                        2,
                        Rgba::from_rgb(0.8, 0.8, 0.8),
                        Coord::new(0.1),
                    );
                }
            }
            _ => (),
        }

        let player = &mut self.world.player;
        if player.facing_left && player.velocity.x > Coord::ZERO
            || !player.facing_left && player.velocity.x < Coord::ZERO
        {
            player.facing_left = !player.facing_left;
        }

        if let PlayerState::Drilling | PlayerState::AirDrill { dash: Some(_) } =
            self.world.player.state
        {
            self.world
                .player
                .collider
                .translate(self.world.player.velocity * self.delta_time);
            return;
        }

        // Apply gravity
        self.world.player.velocity += self.world.rules.gravity * self.delta_time;

        if !matches!(self.world.player.state, PlayerState::AirDrill { .. }) {
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
                // Higher jump
                self.world.player.velocity.y += self.world.rules.gravity.y
                    * (self.world.rules.low_jump_multiplier - Coord::ONE)
                    * self.delta_time;
            }
        }

        if let Some(time) = &mut self.world.player.control_timeout {
            // No horizontal control
            *time -= self.delta_time;
            if *time <= Time::ZERO {
                self.world.player.control_timeout = None;
            }
        } else if let PlayerState::AirDrill { .. } = self.world.player.state {
            // You cannot control your drill LUL
        } else {
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

        if self.world.player.jump_buffer.is_some() {
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
            if let Some(jump) = jump {
                self.world.player.coyote_time = None;
                self.world.player.jump_buffer = None;
                self.world.player.can_hold_jump = true;
                match jump {
                    Coyote::Ground => {
                        let jump_vel = rules.normal_jump_strength;
                        self.world.player.velocity.y = jump_vel;
                        self.world.player.state = PlayerState::Airborn;
                        self.play_sound(&self.world.assets.sounds.jump);
                        self.spawn_particles(
                            Time::ONE,
                            self.world.player.collider.feet(),
                            vec2(Coord::ZERO, Coord::ONE),
                            Coord::new(1.0),
                            3,
                            Rgba::WHITE,
                            Coord::new(0.1),
                        );
                    }
                    Coyote::Wall { wall_normal } => {
                        let angle = rules.wall_jump_angle * wall_normal.x.signum();
                        let jump_vel = wall_normal.rotate(angle) * rules.wall_jump_strength;
                        self.world.player.velocity = jump_vel;
                        self.world.player.control_timeout =
                            Some(self.world.rules.wall_jump_timeout);
                        self.world.player.state = PlayerState::Airborn;
                        self.play_sound(&self.world.assets.sounds.jump);
                        self.spawn_particles(
                            Time::ONE,
                            self.world.player.collider.feet()
                                - wall_normal
                                    * self.world.player.collider.raw().width()
                                    * Coord::new(0.5),
                            jump_vel.normalize_or_zero(),
                            Coord::new(1.0),
                            3,
                            Rgba::WHITE,
                            Coord::new(0.1),
                        );
                    }
                    Coyote::DrillJump { direction } => {
                        let rules = &self.world.rules;
                        let acceleration = rules.drill_jump_speed_inc;
                        let current = Vec2::dot(self.world.player.velocity, direction);
                        self.world.player.velocity =
                            direction * (current + acceleration).max(rules.drill_jump_speed_min);
                        self.play_sound(&self.world.assets.sounds.drill_jump);
                        self.spawn_particles(
                            Time::ONE,
                            self.world.player.collider.pos(),
                            direction,
                            Coord::new(1.0),
                            5,
                            Rgba::from_rgb(0.8, 0.25, 0.2),
                            Coord::new(0.3),
                        );
                    }
                }
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
        if player.collider.head().y > level_bounds.y_max {
            player.collider.translate(vec2(
                Coord::ZERO,
                level_bounds.y_max - player.collider.head().y,
            ));
        }
        let offset = player.collider.feet().x - level_bounds.center().x;
        if offset.abs() > level_bounds.width() / Coord::new(2.0) {
            player.collider.translate(vec2(
                offset.signum() * (level_bounds.width() / Coord::new(2.0) - offset.abs()),
                Coord::ZERO,
            ));
        }

        let finished = matches!(self.world.player.state, PlayerState::Finished { .. })
            .then_some(self.world.player.state);
        let air_drill = matches!(self.world.player.state, PlayerState::AirDrill { .. });
        let drilling = matches!(self.world.player.state, PlayerState::Drilling);
        let was_grounded = matches!(self.world.player.state, PlayerState::Grounded(..));
        let update_state = !drilling && !air_drill;
        if update_state {
            self.world.player.state = PlayerState::Airborn;
        }
        let mut can_drill = false;
        self.world.player.touching_wall = None;
        for _ in 0..2 {
            // Player-tiles
            let player_aabb = self.world.player.collider.grid_aabb(&self.world.level.grid);
            let collisions = (player_aabb.x_min..=player_aabb.x_max)
                .flat_map(move |x| (player_aabb.y_min..=player_aabb.y_max).map(move |y| vec2(x, y)))
                .filter_map(|pos| {
                    self.world
                        .level
                        .tiles
                        .get_tile_isize(pos)
                        .filter(|tile| {
                            let air = matches!(tile, Tile::Air);
                            let drill = (drilling || air_drill) && tile.is_drillable();
                            if !air && drill {
                                can_drill = true;
                            }
                            !air && !drill
                        })
                        .and_then(|tile| {
                            let collider = Collider::new(
                                AABB::point(self.world.level.grid.grid_to_world(pos))
                                    .extend_positive(self.world.level.grid.cell_size),
                            );
                            self.world
                                .player
                                .collider
                                .check(&collider)
                                .and_then(|collision| {
                                    (Vec2::dot(collision.normal, self.world.player.velocity)
                                        >= Coord::ZERO)
                                        .then_some((tile, collision))
                                })
                        })
                });
            if let Some((tile, collision)) =
                collisions.max_by_key(|(_, collision)| collision.penetration)
            {
                self.world
                    .player
                    .collider
                    .translate(-collision.normal * collision.penetration);
                let bounciness = Coord::new(if drilling || air_drill { 1.0 } else { 0.0 });
                self.world.player.velocity -= collision.normal
                    * Vec2::dot(self.world.player.velocity, collision.normal)
                    * (Coord::ONE + bounciness);
                if !drilling && !air_drill {
                    if collision.normal.x.approx_eq(&Coord::ZERO)
                        && collision.normal.y < Coord::ZERO
                    {
                        if !was_grounded && finished.is_none() {
                            self.spawn_particles(
                                Time::ONE,
                                self.world.player.collider.feet(),
                                vec2(Coord::ZERO, Coord::ONE),
                                Coord::new(0.5),
                                3,
                                Rgba::WHITE,
                                Coord::new(0.1),
                            );
                        }
                        if update_state {
                            self.world.player.state = PlayerState::Grounded(tile);
                            self.world.player.coyote_time =
                                Some((Coyote::Ground, self.world.rules.coyote_time));
                        }
                    } else if collision.normal.y.approx_eq(&Coord::ZERO) {
                        let wall_normal = -collision.normal;
                        self.world.player.touching_wall = Some((tile, wall_normal));
                        if update_state {
                            self.world.player.state =
                                PlayerState::WallSliding { tile, wall_normal };
                            self.world.player.coyote_time =
                                Some((Coyote::Wall { wall_normal }, self.world.rules.coyote_time));
                        }
                    }
                }
            }
        }

        if let Some(state) = finished {
            self.world.player.state = state;
            return;
        }

        if drilling {
            if !can_drill {
                self.world.player.can_drill_dash = true;
                self.world.player.state = if self.player_control.hold_drill {
                    PlayerState::AirDrill { dash: None }
                } else {
                    PlayerState::Airborn
                };

                let direction = self.world.player.velocity.normalize_or_zero();
                self.world.player.coyote_time = Some((
                    Coyote::DrillJump { direction },
                    self.world.rules.coyote_time,
                ));
                self.spawn_particles(
                    Time::ONE,
                    self.world.player.collider.pos(),
                    direction,
                    Coord::new(0.3),
                    8,
                    Rgba::from_rgb(0.7, 0.7, 0.7),
                    Coord::new(0.2),
                );
            } else if thread_rng().gen_bool(0.2) {
                self.spawn_particles(
                    Time::ONE,
                    self.world.player.collider.pos(),
                    -self.world.player.velocity.normalize_or_zero(),
                    Coord::new(0.5),
                    2,
                    Rgba::from_rgb(0.8, 0.8, 0.8),
                    Coord::new(0.1),
                );
            }
        } else if air_drill && can_drill {
            let speed = self.world.player.velocity.len();
            let dir = self.world.player.velocity.normalize_or_zero();

            self.world.player.velocity = dir * speed.max(self.world.rules.drill_speed_min);
            self.world.player.state = PlayerState::Drilling;

            self.spawn_particles(
                Time::ONE,
                self.world.player.collider.pos(),
                -dir,
                Coord::new(0.3),
                5,
                Rgba::from_rgb(0.7, 0.7, 0.7),
                Coord::new(0.2),
            );

            let sound = self
                .world
                .drill_sound
                .get_or_insert_with(|| self.world.assets.sounds.drill.play());
            sound.set_volume(self.world.volume);
        }

        // Finish
        if !drilling
            && finished.is_none()
            && self
                .world
                .player
                .collider
                .check(&self.world.level.finish())
                .is_some()
        {
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
            self.play_sound(&self.world.assets.sounds.charm);
            return;
        }

        // Player-coins
        let mut collected = None;
        for coin in &mut self.world.level.coins {
            if !coin.collected && self.world.player.collider.check(&coin.collider).is_some() {
                self.world.coins_collected += 1;
                coin.collected = true;
                collected = Some(coin.collider.pos());
            }
        }
        self.world.level.coins.retain(|coin| !coin.collected);
        if let Some(pos) = collected {
            self.play_sound(&self.world.assets.sounds.coin);
            self.spawn_particles(
                Time::ONE,
                pos,
                vec2(Coord::ZERO, Coord::ONE),
                Coord::new(0.5),
                5,
                Rgba::try_from("#e3a912").unwrap(),
                Coord::new(0.2),
            );
        }

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
        let camera_bounds = self.world.camera_bounds();
        let target = self.world.player.collider.pos();
        let target = target.clamp_aabb(camera_bounds);
        let pos = target.map(Coord::as_f32);
        let pixel = (pos.map(|x| (x * PIXELS_PER_UNIT).round())) / PIXELS_PER_UNIT;
        self.world.camera.center = pixel;
    }
}

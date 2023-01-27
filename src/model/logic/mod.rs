use super::*;

mod particles;
mod player;

use particles::*;

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

    fn process_collisions(&mut self) {
        if let PlayerState::Respawning { .. } = self.world.player.state {
            return;
        }

        let finished = matches!(self.world.player.state, PlayerState::Finished { .. })
            .then_some(self.world.player.state);
        let air_drill = matches!(self.world.player.state, PlayerState::AirDrill { .. });
        let drilling = matches!(self.world.player.state, PlayerState::Drilling);
        let was_grounded = matches!(self.world.player.state, PlayerState::Grounded(..));

        // Level bounds
        let level = &self.world.level;
        let level_bounds = level.bounds();
        let player = &mut self.world.player;
        if player.collider.head().y > level_bounds.y_max {
            player.collider.translate(vec2(
                Coord::ZERO,
                level_bounds.y_max - player.collider.head().y,
            ));
            player.velocity.y = if drilling {
                -player.velocity.y
            } else {
                Coord::ZERO
            };
        }
        let offset = player.collider.feet().x - level_bounds.center().x;
        if offset.abs() > level_bounds.width() / Coord::new(2.0) {
            player.collider.translate(vec2(
                offset.signum() * (level_bounds.width() / Coord::new(2.0) - offset.abs()),
                Coord::ZERO,
            ));
            player.velocity.x = Coord::ZERO;
        }

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
                            self.spawn_particles(ParticleSpawn {
                                lifetime: Time::ONE,
                                position: self.world.player.collider.feet(),
                                velocity: vec2(Coord::ZERO, Coord::ONE) * Coord::new(0.5),
                                amount: 3,
                                color: Rgba::WHITE,
                                radius: Coord::new(0.1),
                                ..Default::default()
                            });
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
                    self.world.player.drill_release = Some(self.world.rules.drill_release_time);
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
                    position: self.world.player.collider.pos(),
                    velocity: direction * Coord::new(0.3),
                    amount: 8,
                    color: Rgba::from_rgb(0.7, 0.7, 0.7),
                    radius: Coord::new(0.2),
                    ..Default::default()
                });
            } else if thread_rng().gen_bool(0.2) {
                self.spawn_particles(ParticleSpawn {
                    lifetime: Time::ONE,
                    position: self.world.player.collider.pos(),
                    velocity: -self.world.player.velocity.normalize_or_zero() * Coord::new(0.5),
                    amount: 2,
                    color: Rgba::from_rgb(0.8, 0.8, 0.8),
                    radius: Coord::new(0.1),
                    ..Default::default()
                });
            }
        } else if air_drill && can_drill {
            let speed = self.world.player.velocity.len();
            let dir = self.world.player.velocity.normalize_or_zero();

            self.world.player.velocity = dir * speed.max(self.world.rules.drill_speed_min);
            self.world.player.state = PlayerState::Drilling;

            self.spawn_particles(ParticleSpawn {
                lifetime: Time::ONE,
                position: self.world.player.collider.pos(),
                velocity: -dir * Coord::new(0.3),
                amount: 5,
                color: Rgba::from_rgb(0.7, 0.7, 0.7),
                radius: Coord::new(0.2),
                ..Default::default()
            });

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
                initial_lifetime: Time::new(2.0),
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
        if let Some(position) = collected {
            self.play_sound(&self.world.assets.sounds.coin);
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

    fn process_camera(&mut self) {
        let camera_bounds = self.world.camera_bounds();
        let target = self.world.player.collider.pos();
        let target = target.clamp_aabb(camera_bounds);
        let pos = target.map(Coord::as_f32);
        let pixel = (pos.map(|x| (x * PIXELS_PER_UNIT).round())) / PIXELS_PER_UNIT;
        self.world.camera.center = pixel;
    }
}

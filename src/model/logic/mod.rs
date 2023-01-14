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
    }

    fn kill_player(&mut self) {
        self.world.player.velocity = Vec2::ZERO;
        self.world.player.state = PlayerState::Respawning { time: Time::ONE };
    }

    fn process_player(&mut self) {
        if let PlayerState::Respawning { time } = &mut self.world.player.state {
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
            let target_velocity = self.player_control.move_dir.x * self.world.rules.move_speed;
            self.world.player.velocity.x = target_velocity;
        }

        if self.player_control.jump {
            if let Some(jump_vel) = self.world.player.state.jump_velocity(&self.world.rules) {
                if let PlayerState::WallSliding { .. } = self.world.player.state {
                    self.world.player.control_timeout = Some(self.world.rules.wall_jump_timeout);
                }
                self.world.player.velocity = jump_vel;
                self.world.player.state = PlayerState::Airborn;
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

        self.world.player.state = PlayerState::Airborn;
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
                        .filter(|tile| !matches!(tile, Tile::Air))
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
                // for collision in collisions.collect::<Vec<_>>() {
                self.world
                    .player
                    .collider
                    .translate(-collision.normal * collision.penetration);
                self.world.player.velocity -=
                    collision.normal * Vec2::dot(self.world.player.velocity, collision.normal);
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

        // Screen edge
        if self.world.player.collider.feet().y < self.world.level.grid.grid_to_world(vec2(0, 0)).y {
            self.kill_player();
            return;
        }

        // Player-hazards
        for hazard in &self.world.level.hazards {
            if self.world.player.collider.check(&hazard.collider).is_some() {
                self.kill_player();
                break;
            }
        }
    }
}

use super::*;

const MOVE_SPEED: f32 = 5.0;

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

    fn process_player(&mut self) {
        self.world.player.velocity += GRAVITY.map(Coord::new) * self.delta_time;

        self.world.player.velocity.x = self.player_control.move_dir.x * Coord::new(MOVE_SPEED);

        if self.player_control.jump {
            if let Some(jump_vel) = self.world.player.state.jump_velocity() {
                self.world.player.velocity += jump_vel;
                self.world.player.state = PlayerState::Airborn;
            }
        }

        self.world
            .player
            .collider
            .translate(self.world.player.velocity * self.delta_time);
    }

    fn process_collisions(&mut self) {
        // Player-tiles
        let player_aabb = self.world.player.collider.grid_aabb(&self.world.level.grid);
        if let Some(collision) = (player_aabb.x_min..=player_aabb.x_max)
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
            .max_by_key(|collision| collision.penetration)
        {
            self.world
                .player
                .collider
                .translate(-collision.normal * collision.penetration);
            self.world.player.velocity -=
                collision.normal * Vec2::dot(self.world.player.velocity, collision.normal);
            if collision.normal.x.approx_eq(&Coord::ZERO) {
                self.world.player.state = PlayerState::Grounded;
            } else if collision.normal.y.approx_eq(&Coord::ZERO) {
                self.world.player.state = PlayerState::WallSliding {
                    wall_normal: -collision.normal,
                };
            }
        }
    }
}

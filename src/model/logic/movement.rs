use super::*;

const MOVE_STEP: f32 = 1.0 / PIXELS_PER_UNIT as f32;

#[derive(Debug, Clone, Copy)]
pub enum ColliderId {
    Tile(vec2<isize>),
    Entity(Id),
}

#[derive(Debug, Clone, Copy, Default)]
pub struct MoveCollision {
    pub x: Option<(ColliderId, Collision)>,
    pub y: Option<(ColliderId, Collision)>,
}

impl Logic<'_> {
    pub fn check_collision(&self, collider: &Collider) -> Option<(ColliderId, Collision)> {
        actor_collides(
            collider,
            vec2::ZERO,
            &self.world.room.grid,
            &self.world.room.tiles,
            &self.world.blocks,
            false,
            &self.world.rules,
        )
    }

    pub fn move_actor(
        &mut self,
        actor_id: Id,
        delta: vec2<Coord>,
        on_collision: Option<CollisionCallback>,
    ) {
        let actor = self
            .world
            .actors
            .get_mut(&actor_id)
            .unwrap_or_else(|| panic!("Unknown actor: {actor_id:?}"));

        let drill = actor.id == self.world.player.id && self.world.player.state.using_drill();
        let velocity = if actor.id == self.world.player.id {
            self.world.player.velocity
        } else {
            vec2::ZERO
        };

        let mut collision = MoveCollision { x: None, y: None };
        let (sign, mut steps) = calc_move(&mut actor.move_remainder, delta);
        if steps != vec2::ZERO {
            let step_size = Coord::new(MOVE_STEP);
            let step = sign * step_size;

            // Move X
            let step_x = vec2(step.x, Coord::ZERO);
            while steps.x > Coord::ZERO {
                steps.x -= Coord::ONE;
                actor.collider.translate(step_x);
                if let Some((block, mut col)) = actor_collides(
                    &actor.collider,
                    velocity,
                    &self.world.room.grid,
                    &self.world.room.tiles,
                    &self.world.blocks,
                    drill,
                    &self.world.rules,
                ) {
                    if col.normal.x == Coord::ZERO {
                        col = col.rotate();
                    }

                    if col.offset > Coord::ZERO
                        && col.offset < self.world.rules.edge_correction_max
                        && velocity.y.signum() == col.offset.signum()
                    {
                        // Go up the platform
                        actor.collider.translate(col.offset_dir * col.offset);
                        continue;
                    }

                    // Move back and report collision
                    actor.collider.translate(col.normal * col.penetration);
                    collision.x = Some((block, col));
                    break;
                }
            }

            // Move Y
            let step_y = vec2(Coord::ZERO, step.y);
            while steps.y > Coord::ZERO {
                steps.y -= Coord::ONE;
                actor.collider.translate(step_y);
                if let Some((block, mut col)) = actor_collides(
                    &actor.collider,
                    velocity,
                    &self.world.room.grid,
                    &self.world.room.tiles,
                    &self.world.blocks,
                    drill,
                    &self.world.rules,
                ) {
                    if col.normal.y == Coord::ZERO {
                        col = col.rotate();
                    }

                    if col.normal.y < Coord::ZERO
                        && col.offset.abs() < self.world.rules.edge_correction_max
                    {
                        // Move to the side
                        actor.collider.translate(col.offset_dir * col.offset);
                        continue;
                    }

                    // Move back and report collision
                    actor.collider.translate(col.normal * col.penetration);
                    collision.y = Some((block, col));
                    break;
                }
            }
        }

        if let Some(callback) = on_collision {
            callback(self, actor_id, collision);
        }
    }

    // pub fn move_block(&mut self, block_id: Id, delta: vec2<Coord>) {
    //     let block = self
    //         .world
    //         .blocks
    //         .get_mut(&block_id)
    //         .unwrap_or_else(|| panic!("Unknown block: {block_id:?}"));

    //     let all_actors: Vec<Id> = self.world.actors.ids().copied().collect();

    //     let (sign, mut steps) = calc_move(&mut block.move_remainder, delta);
    //     if steps != vec2::ZERO {
    //         let step_size = Coord::new(MOVE_STEP);
    //         let step = sign * step_size;

    //         let riders: Vec<Id> = get_riders(&self.world.actors, block.id).collect();

    //         // Move X
    //         let step_x = vec2(step.x, Coord::ZERO);
    //         while steps.x > Coord::ZERO {
    //             let block = self
    //                 .world
    //                 .blocks
    //                 .get_mut(&block_id)
    //                 .unwrap_or_else(|| panic!("Unknown block: {block_id:?}"));

    //             steps.x -= Coord::ONE;
    //             block.collider.translate(step_x);
    //             let collider = block.collider;
    //             for &id in &all_actors {
    //                 if let Some(actor) = self.world.actors.get(&id) {
    //                     if let Some(collision) = actor.collider.collide(&collider) {
    //                         // Push horizontally
    //                         let delta = collision.normal * collision.penetration;
    //                         self.move_actor(id, delta, Some(actor.on_squish.clone()));
    //                     } else if riders.contains(&id) {
    //                         // Carry horizontally
    //                         self.move_actor(id, step_x, None);
    //                     }
    //                 }
    //             }
    //         }

    //         // Move Y
    //         let step_y = vec2(Coord::ZERO, step.y);
    //         while steps.y > Coord::ZERO {
    //             let block = self
    //                 .world
    //                 .blocks
    //                 .get_mut(&block_id)
    //                 .unwrap_or_else(|| panic!("Unknown block: {block_id:?}"));

    //             steps.y -= Coord::ONE;
    //             block.collider.translate(step_y);
    //             let collider = block.collider;
    //             for &id in &all_actors {
    //                 if let Some(actor) = self.world.actors.get(&id) {
    //                     if let Some(collision) = actor.collider.collide(&collider) {
    //                         // Push vertically
    //                         let delta = collision.normal * collision.penetration;
    //                         self.move_actor(id, delta, Some(actor.on_squish.clone()));
    //                     } else if riders.contains(&id) {
    //                         // Carry vertically
    //                         self.move_actor(id, step_y, None);
    //                     }
    //                 }
    //             }
    //         }
    //     }
    // }
}

// fn get_riders(actors: &Collection<Actor>, block: Id) -> impl Iterator<Item = Id> + '_ {
//     actors
//         .iter()
//         .filter(move |actor| actor.riding == Some(block))
//         .map(|actor| actor.id)
// }

fn actor_collides(
    collider: &Collider,
    velocity: vec2<Coord>,
    grid: &Grid,
    tiles: &TileMap,
    blocks: &Collection<Block>,
    drill: bool,
    rules: &Rules,
) -> Option<(ColliderId, Collision)> {
    collide_tiles(collider, velocity, grid, tiles, drill, rules)
        .map(|(pos, col)| (ColliderId::Tile(pos), col))
        .or_else(|| {
            blocks.iter().find_map(|block| {
                collider
                    .collide(&block.collider)
                    .map(|collision| (ColliderId::Entity(block.id), collision))
            })
        })
}

fn calc_move(remainder: &mut vec2<Coord>, delta: vec2<Coord>) -> (vec2<Coord>, vec2<Coord>) {
    let step_size = Coord::new(MOVE_STEP);
    *remainder += delta;
    let delta = *remainder / step_size;
    let sign = delta.map(|x| x.signum());
    let dist = delta.map(|x| x.abs().floor());
    *remainder -= dist * sign * step_size;
    (sign, dist)
}

fn collide_tiles(
    collider: &Collider,
    velocity: vec2<Coord>,
    grid: &Grid,
    tiles: &TileMap,
    drill: bool,
    rules: &Rules,
) -> Option<(vec2<isize>, Collision)> {
    let aabb = collider.grid_aabb(grid);
    (aabb.min.x..=aabb.max.x)
        .flat_map(move |x| (aabb.min.y..=aabb.max.y).map(move |y| vec2(x, y)))
        .filter_map(|pos| {
            tiles
                .get_tile_isize(pos)
                .filter(|&tile| {
                    let air = *tile == "air";
                    let drill = drill && rules.tiles[tile].drillable;
                    !air && !drill
                })
                .and_then(|_| {
                    let tile = grid.cell_collider(pos);
                    collider.collide(&tile).and_then(|collision| {
                        (vec2::dot(collision.normal, velocity) <= Coord::ZERO)
                            .then_some((pos, collision))
                    })
                })
        })
        .max_by_key(|(_, col)| col.penetration)
}

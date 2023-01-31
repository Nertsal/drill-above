use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grid {
    pub cell_size: vec2<Coord>,
    pub offset: vec2<Coord>,
}

impl Grid {
    pub fn new(cell_size: vec2<Coord>) -> Self {
        Self {
            cell_size,
            offset: vec2::ZERO,
        }
    }

    pub fn matrix(&self) -> mat3<Coord> {
        mat3::translate(self.offset) * mat3::scale(self.cell_size)
    }

    pub fn grid_to_world(&self, grid_pos: vec2<isize>) -> vec2<Coord> {
        let pos = self.matrix().inverse() * grid_pos.extend(1).map(|x| Coord::new(x as f32));
        pos.xy() / pos.z
    }

    pub fn world_to_grid(&self, world_pos: vec2<Coord>) -> (vec2<isize>, vec2<Coord>) {
        let grid_pos = self.matrix() * world_pos.extend(Coord::ONE);
        let mut offset = grid_pos.xy() / grid_pos.z;
        let mut cell_pos = vec2(
            offset.x.as_f32().trunc() as _,
            offset.y.as_f32().trunc() as _,
        );
        offset = vec2(offset.x.as_f32().fract(), offset.y.as_f32().fract()).map(Coord::new);
        if offset.x < Coord::ZERO {
            offset.x += Coord::ONE;
            cell_pos.x -= 1;
        }
        if offset.y < Coord::ZERO {
            offset.y += Coord::ONE;
            cell_pos.y -= 1;
        }
        (cell_pos, offset)
    }

    pub fn tile_collisions(&self, collider: &Collider) -> impl Iterator<Item = vec2<isize>> {
        let aabb = collider.grid_aabb(self);
        (aabb.min.x..=aabb.max.x)
            .flat_map(move |x| (aabb.min.y..=aabb.max.y).map(move |y| vec2(x, y)))
    }
}

impl Default for Grid {
    fn default() -> Self {
        Self::new(vec2(Coord::ONE, Coord::ONE))
    }
}

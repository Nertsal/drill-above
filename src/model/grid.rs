use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grid {
    pub cell_size: Vec2<Coord>,
    pub offset: Vec2<Coord>,
}

impl Grid {
    pub fn new(cell_size: Vec2<Coord>) -> Self {
        Self {
            cell_size,
            offset: Vec2::ZERO,
        }
    }

    pub fn matrix(&self) -> Mat3<Coord> {
        Mat3::translate(self.offset) * Mat3::scale(self.cell_size)
    }

    pub fn grid_to_world(&self, grid_pos: Vec2<isize>) -> Vec2<Coord> {
        let pos = self.matrix().inverse() * grid_pos.extend(1).map(|x| Coord::new(x as f32));
        pos.xy() / pos.z
    }

    pub fn world_to_grid(&self, world_pos: Vec2<Coord>) -> (Vec2<isize>, Vec2<Coord>) {
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
}

impl Default for Grid {
    fn default() -> Self {
        Self::new(vec2(Coord::ONE, Coord::ONE))
    }
}

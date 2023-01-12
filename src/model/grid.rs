use super::*;

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
        Mat3::translate(self.offset) * Mat3::scale(self.cell_size) * Mat3::rotate(r32(1.0))
    }

    pub fn grid_to_world(&self, grid_pos: Vec2<i32>) -> Vec2<Coord> {
        let pos = self.matrix().inverse() * grid_pos.extend(1).map(|x| Coord::new(x as f32));
        pos.xy() / pos.z
    }

    pub fn world_to_grid(&self, world_pos: Vec2<Coord>) -> (Vec2<i32>, Vec2<Coord>) {
        let grid_pos = self.matrix() * world_pos.extend(Coord::ONE);
        let mut offset = grid_pos.xy() / grid_pos.z;
        let mut cell_pos = Vec2::ZERO;
        while offset.x < Coord::ZERO {
            offset.x += Coord::ONE;
            cell_pos.x -= 1;
        }
        while offset.x >= Coord::ONE {
            offset.x -= Coord::ONE;
            cell_pos.x += 1;
        }
        while offset.y < Coord::ZERO {
            offset.y += Coord::ONE;
            cell_pos.y -= 1;
        }
        while offset.y >= Coord::ONE {
            offset.y -= Coord::ONE;
            cell_pos.y += 1;
        }
        (cell_pos, offset)
    }
}

impl Default for Grid {
    fn default() -> Self {
        Self::new(vec2(Coord::ONE, Coord::ONE))
    }
}

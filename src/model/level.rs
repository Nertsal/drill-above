use super::*;

pub struct Level {
    pub grid: Grid,
    pub size: Vec2<usize>,
    pub tiles: TileMap,
}

impl Level {
    pub fn new(size: Vec2<usize>) -> Self {
        let mut grid = Grid::default();
        grid.offset = size.map(|x| Coord::new(x as f32 / 2.0)) * grid.cell_size;
        Self {
            grid,
            tiles: TileMap::new(size),
            size,
        }
    }
}

impl Default for Level {
    fn default() -> Self {
        Self::new(vec2(40, 23))
    }
}

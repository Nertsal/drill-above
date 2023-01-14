use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileMap {
    size: Vec2<usize>,
    tiles: Vec<Tile>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Tile {
    Air,
    Grass,
    Stone,
}

impl Tile {
    pub fn all() -> [Self; 3] {
        use Tile::*;
        [Air, Grass, Stone]
    }

    pub fn is_drillable(&self) -> bool {
        match self {
            Self::Air => true,
            Self::Grass => true,
            Self::Stone => false,
        }
    }
}

impl TileMap {
    pub fn new(size: Vec2<usize>) -> Self {
        Self {
            tiles: (0..size.y * size.x).map(|_| Tile::Air).collect(),
            size,
        }
    }

    pub fn tiles(&self) -> &Vec<Tile> {
        &self.tiles
    }

    pub fn set_tile(&mut self, pos: Vec2<usize>, tile: Tile) {
        assert!(pos.x < self.size.x);
        assert!(pos.y < self.size.y);

        let index = pos_to_index(pos, self.size.x);
        if let Some(t) = self.tiles.get_mut(index) {
            *t = tile;
        }
    }

    pub fn set_tile_isize(&mut self, pos: Vec2<isize>, tile: Tile) {
        if pos.x < 0 || pos.y < 0 {
            return;
        }
        self.set_tile(pos.map(|x| x as usize), tile);
    }

    pub fn get_tile(&self, pos: Vec2<usize>) -> Tile {
        assert!(pos.x < self.size.x);
        assert!(pos.y < self.size.y);

        let index = pos_to_index(pos, self.size.x);
        *self.tiles.get(index).unwrap()
    }

    pub fn get_tile_isize(&self, pos: Vec2<isize>) -> Option<Tile> {
        if pos.x < 0 || pos.y < 0 {
            None
        } else {
            let pos = pos.map(|x| x as usize);
            let index = pos_to_index(pos, self.size.x);
            self.tiles.get(index).copied()
        }
    }

    pub fn get_tile_connections(&self, tile: usize) -> [bool; 4] {
        let pos = index_to_pos(tile, self.size.x).map(|x| x as isize);
        let Some(center) = self.get_tile_isize(pos) else {
            return [false; 4];
        };
        let deltas = [(0, -1), (1, 0), (0, 1), (-1, 0)];
        let neighbours = deltas.map(|(x, y)| pos + vec2(x, y));
        neighbours.map(|pos| {
            self.get_tile_isize(pos)
                .filter(|tile| *tile == center)
                .is_some()
        })
    }
}

pub fn pos_to_index(pos: Vec2<usize>, width: usize) -> usize {
    pos.x + pos.y * width
}

pub fn index_to_pos(index: usize, width: usize) -> Vec2<usize> {
    vec2(index % width, index / width)
}

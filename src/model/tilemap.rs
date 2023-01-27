use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileMap {
    pub size: vec2<usize>,
    pub tiles: Vec<Tile>,
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
    pub fn new(size: vec2<usize>) -> Self {
        Self {
            tiles: (0..size.y * size.x).map(|_| Tile::Air).collect(),
            size,
        }
    }

    pub fn tiles(&self) -> &Vec<Tile> {
        &self.tiles
    }

    pub fn set_tile(&mut self, pos: vec2<usize>, tile: Tile) {
        if let Some(t) = pos_to_index(pos, self.size.x).and_then(|index| self.tiles.get_mut(index))
        {
            *t = tile;
        }
    }

    pub fn set_tile_isize(&mut self, pos: vec2<isize>, tile: Tile) {
        if pos.x < 0 || pos.y < 0 {
            return;
        }
        self.set_tile(pos.map(|x| x as usize), tile);
    }

    pub fn get_tile_isize(&self, pos: vec2<isize>) -> Option<Tile> {
        if pos.x < 0 || pos.y < 0 {
            None
        } else {
            let pos = pos.map(|x| x as usize);
            pos_to_index(pos, self.size.x).and_then(|index| self.tiles.get(index).copied())
        }
    }

    pub fn get_tile_neighbours(&self, tile: usize) -> [Option<Tile>; 8] {
        let pos = index_to_pos(tile, self.size.x).map(|x| x as isize);
        let deltas = [
            (-1, -1),
            (0, -1),
            (1, -1),
            (1, 0),
            (1, 1),
            (0, 1),
            (-1, 1),
            (-1, 0),
        ];
        deltas.map(|(x, y)| {
            let pos = pos + vec2(x, y);
            self.get_tile_isize(pos)
        })
    }

    pub fn get_tile_connections(&self, tile: usize) -> [Connection; 8] {
        let pos = index_to_pos(tile, self.size.x).map(|x| x as isize);
        let Some(center) = self.get_tile_isize(pos) else {
            return [Connection::None; 8];
        };
        self.get_tile_neighbours(tile).map(|tile| {
            tile.map(|tile| {
                if matches!(tile, Tile::Air) {
                    Connection::None
                } else if tile == center {
                    Connection::Same
                } else {
                    Connection::Other
                }
            })
            .unwrap_or(Connection::Same)
        })
    }

    pub fn change_size(&mut self, size: vec2<usize>) {
        let mut tiles = vec![Tile::Air; size.x * size.y];
        for y in 0..size.y {
            for x in 0..size.x {
                let i = x + y * size.x;
                if x < self.size.x && y < self.size.y {
                    tiles[i] = self.tiles[x + y * self.size.x];
                }
            }
        }
        self.size = size;
        self.tiles = tiles;
    }
}

pub fn pos_to_index(pos: vec2<usize>, width: usize) -> Option<usize> {
    if pos.x >= width {
        None
    } else {
        Some(pos.x + pos.y * width)
    }
}

pub fn index_to_pos(index: usize, width: usize) -> vec2<usize> {
    vec2(index % width, index / width)
}

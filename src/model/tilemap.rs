use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileMap {
    pub size: vec2<usize>,
    pub tiles: Vec<Tile>,
    geometry: Option<Vec<usize>>,
}

pub type Tile = String;

impl TileMap {
    pub fn new(size: vec2<usize>) -> Self {
        Self {
            tiles: (0..size.y * size.x).map(|_| "air".to_string()).collect(),
            geometry: None,
            size,
        }
    }

    pub fn get_tile_index(&self, tile: usize) -> usize {
        *self
            .geometry
            .as_ref()
            .unwrap()
            .get(tile)
            .expect("Failed to find tile geometry")
    }

    pub fn set_tile(&mut self, pos: vec2<usize>, tile: Tile, assets: &Assets) {
        if let Some(t) = pos_to_index(pos, self.size.x).and_then(|index| self.tiles.get_mut(index))
        {
            *t = tile;
            self.update_geometry(assets);
        }
    }

    pub fn set_tile_isize(&mut self, pos: vec2<isize>, tile: Tile, assets: &Assets) {
        if pos.x < 0 || pos.y < 0 {
            return;
        }
        self.set_tile(pos.map(|x| x as usize), tile, assets);
    }

    pub fn get_tile_isize(&self, pos: vec2<isize>) -> Option<&Tile> {
        if pos.x < 0 || pos.y < 0 {
            None
        } else {
            let pos = pos.map(|x| x as usize);
            pos_to_index(pos, self.size.x).and_then(|index| self.tiles.get(index))
        }
    }

    pub fn get_tile_neighbours(&self, tile: usize) -> [Option<&Tile>; 8] {
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

    pub fn get_tile_connections(
        &self,
        index: usize,
        tile: Option<&Tile>,
        rules: &Rules,
    ) -> [Connection; 8] {
        let Some(center) = tile.or_else(|| {
            let pos = index_to_pos(index, self.size.x).map(|x| x as isize);
            self.get_tile_isize(pos)
        }) else {
            return [Connection::None; 8];
        };
        self.get_tile_neighbours(index).map(|tile| {
            tile.map(|tile| {
                if tile == "air" {
                    Connection::None
                } else if tile == center || rules.tiles[tile].layer > rules.tiles[center].layer {
                    Connection::Same
                } else {
                    Connection::Other
                }
            })
            .unwrap_or(Connection::Same)
        })
    }

    pub fn get_tile_normals(&self, tile: usize, rules: &Rules) -> [vec2<f32>; 4] {
        let connections = self.get_tile_connections(tile, None, rules);

        [
            (vec2(-1, -1), (7, 0, 1)),
            (vec2(1, -1), (3, 2, 1)),
            (vec2(1, 1), (3, 4, 5)),
            (vec2(-1, 1), (7, 6, 5)),
        ]
        .map(|(n, (x, d, y))| {
            let n = n.map(|x| x as f32);
            let [x, d, y] = [x, d, y].map(|con| matches!(connections[con], Connection::None));
            let [x, y] = [x, y].map(|x| if x { 1.0 } else { 0.0 });
            let offset = if !d && x != y {
                -vec2(y, x)
            } else if d && x == y && x == 0.0 {
                vec2(1.0, 1.0)
            } else {
                vec2::ZERO
            };
            ((vec2(x, y) + offset) * n).normalize_or_zero()
        })
    }

    pub fn change_size(&mut self, size: vec2<usize>, assets: &Assets) {
        let mut tiles = vec!["air".to_string(); size.x * size.y];
        for y in 0..size.y {
            for x in 0..size.x {
                if x < self.size.x && y < self.size.y {
                    let i = x + y * size.x;
                    tiles[i] = self.tiles[x + y * self.size.x].to_owned();
                }
            }
        }
        self.size = size;
        self.tiles = tiles;
        self.update_geometry(assets);
    }

    pub fn translate(&mut self, delta: vec2<isize>, assets: &Assets) {
        let mut tiles = vec!["air".to_string(); self.size.x * self.size.y];
        for y in 0..self.size.y {
            for x in 0..self.size.x {
                let i = x + y * self.size.x;
                let x = x as isize - delta.x;
                let y = y as isize - delta.y;
                if x < 0 || y < 0 {
                    continue;
                }
                let x = x as usize;
                let y = y as usize;
                if x < self.size.x && y < self.size.y {
                    tiles[i] = self.tiles[x + y * self.size.x].to_owned();
                }
            }
        }
        self.tiles = tiles;
        self.update_geometry(assets);
    }

    pub fn update_geometry(&mut self, assets: &Assets) {
        let mut geometry = vec![0; self.tiles.len()];
        let mut rng = thread_rng();
        for (i, tile) in self.tiles.iter().enumerate() {
            let connections = self.get_tile_connections(i, None, &assets.rules);
            let set = assets.sprites.tiles.get_tile_set(tile);
            let options = set.get_tile_connected(connections);
            geometry[i] = *options
                .choose(&mut rng)
                .expect("Failed to find a suitable tile geometry");
        }
        self.geometry = Some(geometry);
    }

    pub fn calculate_geometry(
        &self,
        grid: &Grid,
        geng: &Geng,
        assets: &Assets,
    ) -> (
        HashMap<Tile, ugli::VertexBuffer<Vertex>>,
        HashMap<Tile, ugli::VertexBuffer<MaskedVertex>>,
    ) {
        let mut tiles_geometry = HashMap::<Tile, Vec<Vertex>>::new();
        let mut masked_geometry = HashMap::<Tile, Vec<MaskedVertex>>::new();

        let calc_geometry = |i: usize, tile: &Tile, connections: Option<[Connection; 8]>| {
            let pos = index_to_pos(i, self.size.x);
            let pos = grid.grid_to_world(pos.map(|x| x as isize));
            let pos = Aabb2::point(pos)
                .extend_positive(grid.cell_size)
                .map(Coord::as_f32);
            let set = assets.sprites.tiles.get_tile_set(tile);
            let index = if let Some(connections) = connections {
                *set.get_tile_connected(connections).first().unwrap()
            } else {
                self.get_tile_index(i)
            };
            let geometry = set.get_tile_geometry(index);
            let vertices = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
            let vertices = [0, 1, 2, 3].map(|i| Vertex {
                a_pos: vec2(vertices[i].0, vertices[i].1),
                a_uv: geometry[i],
            });
            let geometry = [
                vertices[0],
                vertices[1],
                vertices[2],
                vertices[0],
                vertices[2],
                vertices[3],
            ];
            let matrix = mat3::translate(pos.bottom_left()) * mat3::scale(pos.size());
            geometry.map(|vertex| {
                let pos = matrix * vertex.a_pos.extend(1.0);
                Vertex {
                    a_pos: pos.xy() / pos.z,
                    ..vertex
                }
            })
        };
        for (i, tile) in self.tiles.iter().enumerate() {
            if tile == "air" {
                continue;
            }

            // Put a masked tile under the current tile
            let neighbours = self.get_tile_neighbours(i);
            let masks: HashSet<&Tile> = neighbours
                .iter()
                .filter_map(|other| {
                    other.filter(|&other| {
                        other != "air"
                            && assets.rules.tiles[other].layer < assets.rules.tiles[tile].layer
                    })
                })
                .collect();
            for masked in masks {
                let connections = self.get_tile_connections(i, Some(masked), &assets.rules);
                let geometry = calc_geometry(i, masked, Some(connections));
                let mask = &assets.sprites.tiles.mask;
                let mask = mask.get_tile_geometry(
                    *mask
                        .get_tile_connected(connections)
                        .first()
                        .expect("Failed to find a suitable tile geometry"),
                );
                let idx = [0, 1, 2, 0, 2, 3];
                let geometry = geometry.into_iter().zip(idx).map(|(v, i)| v.mask(mask[i]));
                masked_geometry
                    .entry(masked.to_owned())
                    .or_default()
                    .extend(geometry);
            }

            tiles_geometry
                .entry(tile.to_owned())
                .or_default()
                .extend(calc_geometry(i, tile, None));
        }
        let tiles = tiles_geometry
            .into_iter()
            .map(|(tile, geom)| (tile, ugli::VertexBuffer::new_dynamic(geng.ugli(), geom)))
            .collect();
        let masked = masked_geometry
            .into_iter()
            .map(|(tile, geom)| (tile, ugli::VertexBuffer::new_dynamic(geng.ugli(), geom)))
            .collect();
        (tiles, masked)
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

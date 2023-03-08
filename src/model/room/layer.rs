use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomLayer {
    pub tiles: TileMap,
    #[serde(default)]
    pub props: Vec<Prop>,
}

impl RoomLayer {
    pub fn new(size: vec2<usize>) -> Self {
        Self {
            tiles: TileMap::new(size),
            props: Vec::new(),
        }
    }

    pub fn calculate_light_geometry(
        &self,
        grid: &Grid,
        geng: &Geng,
        assets: &Assets,
    ) -> ugli::VertexBuffer<NormalVertex> {
        let vertices = self
            .tiles
            .tiles
            .iter()
            .enumerate()
            .filter_map(|(i, tile)| {
                (tile != "air").then(|| {
                    let grid_pos = index_to_pos(i, self.tiles.size.x).map(|x| x as isize);
                    let pos = grid.grid_to_world(grid_pos);
                    let aabb = Aabb2::point(pos)
                        .extend_positive(grid.cell_size)
                        .map(Coord::as_f32);
                    let matrix = mat3::translate(aabb.bottom_left()) * mat3::scale(aabb.size());

                    let vs =
                        [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)].map(|(x, y)| vec2(x, y));
                    let sides = [
                        (vs[0], vs[1], vec2(0, -1)),
                        (vs[1], vs[2], vec2(1, 0)),
                        (vs[2], vs[3], vec2(0, 1)),
                        (vs[3], vs[0], vec2(-1, 0)),
                    ];
                    sides
                        .into_iter()
                        .filter_map(move |(a, b, n)| {
                            let pos = grid_pos + n;
                            self.tiles
                                .get_tile_isize(pos)
                                .filter(|&neighbour| {
                                    neighbour == "air"
                                        || assets.rules.tiles[neighbour].layer
                                            < assets.rules.tiles[tile].layer
                                })
                                .map(|_| {
                                    let a_normal = n.map(|x| x as f32);
                                    let [a, b] = [a, b].map(|v| NormalVertex {
                                        a_pos: (matrix * v.extend(1.0)).into_2d(),
                                        a_normal,
                                    });
                                    let [a1, b1] = [a, b].map(|mut v| {
                                        v.a_normal = vec2::ZERO;
                                        v
                                    });
                                    [b1, a1, a, b1, a, b]
                                })
                        })
                        .flatten()
                })
            })
            .flatten()
            .collect();
        ugli::VertexBuffer::new_dynamic(geng.ugli(), vertices)
    }

    pub fn calculate_normal_geometry(
        &self,
        grid: &Grid,
        geng: &Geng,
        assets: &Assets,
    ) -> (
        ugli::VertexBuffer<NormalVertex>,
        HashMap<Tile, ugli::VertexBuffer<Vertex>>,
    ) {
        let mut static_geom = Vec::new();
        let mut shaded_geom = HashMap::<Tile, Vec<Vertex>>::new();
        for (i, tile) in self.tiles.tiles.iter().enumerate() {
            if tile == "air" {
                continue;
            }

            let pos = index_to_pos(i, self.tiles.size.x);
            let pos = grid.grid_to_world(pos.map(|x| x as isize));
            let pos = Aabb2::point(pos)
                .extend_positive(grid.cell_size)
                .map(Coord::as_f32);
            let matrix = mat3::translate(pos.bottom_left()) * mat3::scale(pos.size());
            let vertices = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];

            let tileset = assets.sprites.tiles.get_tile_set(tile);
            match tileset.texture.normal() {
                Some(_) => {
                    let uv = tileset.get_tile_geometry(self.tiles.get_tile_index(i));
                    shaded_geom.entry(tile.to_owned()).or_default().extend(
                        std::iter::zip(vertices, uv).map(|((x, y), a_uv)| Vertex {
                            a_pos: (matrix * vec2(x, y).extend(1.0)).into_2d(),
                            a_uv,
                        }),
                    );
                }
                None => {
                    let normals = self.tiles.get_tile_normals(i, &assets.rules);
                    let indices = [0, 1, 2, 0, 2, 3];
                    static_geom.extend(indices.into_iter().map(|i| {
                        let (x, y) = vertices[i];
                        let n = normals[i];
                        NormalVertex {
                            a_pos: (matrix * vec2(x, y).extend(1.0)).into_2d(),
                            a_normal: n,
                        }
                    }));
                }
            }
        }

        let shaded_geom = shaded_geom
            .into_iter()
            .map(|(tile, geom)| (tile, ugli::VertexBuffer::new_dynamic(geng.ugli(), geom)))
            .collect();

        (
            ugli::VertexBuffer::new_dynamic(geng.ugli(), static_geom),
            shaded_geom,
        )
    }
}

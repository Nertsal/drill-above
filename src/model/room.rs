use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, geng::Assets)]
#[asset(json)]
pub struct Room {
    pub drill_allowed: bool,
    #[serde(default)]
    pub grid: Grid,
    pub size: vec2<usize>,
    pub spawn_point: vec2<Coord>,
    pub tiles: TileMap,
    #[serde(default)]
    pub hazards: Vec<Hazard>,
    #[serde(default)]
    pub coins: Vec<Coin>,
    #[serde(default)]
    pub props: Vec<Prop>,
    #[serde(default)]
    pub global_light: GlobalLightSource,
    #[serde(default)]
    pub spotlights: Vec<SpotlightSource>,
    #[serde(default)]
    pub transitions: Vec<RoomTransition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomTransition {
    pub collider: Collider,
    pub to_room: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlaceableType {
    Tile(Tile),
    Hazard(HazardType),
    Prop(PropType),
    Spotlight(SpotlightSource),
    Coin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlaceableId {
    Tile(vec2<isize>),
    Hazard(usize),
    Prop(usize),
    Coin(usize),
    Spotlight(usize),
}

#[derive(Debug, Clone)]
pub enum Placeable {
    Tile((Tile, vec2<isize>)),
    Hazard(Hazard),
    Prop(Prop),
    Coin(Coin),
    Spotlight(SpotlightSource),
}

#[derive(Debug)]
pub enum PlaceableMut<'a> {
    Tile((Tile, vec2<isize>)),
    Hazard(&'a mut Hazard),
    Prop(&'a mut Prop),
    Coin(&'a mut Coin),
    Spotlight(&'a mut SpotlightSource),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coin {
    pub collider: Collider,
    pub collected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hazard {
    pub sprite: Aabb2<Coord>,
    pub direction: Option<vec2<Coord>>,
    pub collider: Collider,
    pub hazard_type: HazardType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prop {
    pub sprite: Aabb2<Coord>,
    pub prop_type: PropType,
}

pub type HazardType = String;
pub type PropType = String;

impl Room {
    pub fn new(size: vec2<usize>) -> Self {
        let mut grid = Grid::default();
        grid.offset = size.map(|x| Coord::new(x as f32 / 2.0)) * grid.cell_size;
        Self {
            spawn_point: grid.grid_to_world(size.map(|x| x as isize / 2)),
            tiles: TileMap::new(size),
            hazards: Vec::new(),
            coins: Vec::new(),
            props: Vec::new(),
            transitions: Vec::new(),
            drill_allowed: true,
            global_light: default(),
            spotlights: Vec::new(),
            grid,
            size,
        }
    }

    pub fn bounds(&self) -> Aabb2<Coord> {
        Aabb2::from_corners(
            self.grid.grid_to_world(vec2(0, 0)),
            self.grid.grid_to_world(self.size.map(|x| x as isize)),
        )
    }

    pub fn place_block(&mut self, block: Placeable, assets: &Assets) -> PlaceableId {
        match block {
            Placeable::Tile((tile, pos)) => {
                self.tiles.set_tile_isize(pos, tile, assets);
                PlaceableId::Tile(pos)
            }
            Placeable::Hazard(hazard) => {
                self.hazards.push(hazard);
                PlaceableId::Hazard(self.hazards.len() - 1)
            }
            Placeable::Prop(prop) => {
                self.props.push(prop);
                PlaceableId::Prop(self.props.len() - 1)
            }
            Placeable::Coin(coin) => {
                self.coins.push(coin);
                PlaceableId::Coin(self.coins.len() - 1)
            }
            Placeable::Spotlight(light) => {
                self.spotlights.push(light);
                PlaceableId::Spotlight(self.spotlights.len() - 1)
            }
        }
    }

    pub fn place_hazard(&mut self, pos: vec2<Coord>, hazard: HazardType) {
        let (pos, offset) = self.grid.world_to_grid(pos);
        let connect = |pos| {
            self.tiles
                .get_tile_isize(pos)
                .map(|tile| tile != "air")
                .unwrap_or(false)
        };
        let (direction, collider) = {
            let size = vec2(0.8, 0.4);
            let direction = -[vec2(0, -1), vec2(1, 0), vec2(-1, 0), vec2(0, 1)]
                .into_iter()
                .filter(|&d| connect(pos + d))
                .min_by_key(|dir| (dir.map(|x| Coord::new(x as f32 * 0.5 + 0.5)) - offset).len())
                .unwrap_or(vec2(0, -1))
                .map(|x| x as f32);
            let pos = vec2(0.5, 0.5) - direction * 0.5;
            let aabb = Aabb2::from_corners(
                pos + vec2(-size.x * direction.y * 0.5, -size.x * direction.x * 0.5),
                pos + vec2(
                    size.x * direction.y * 0.5 + size.y * direction.x,
                    size.y * direction.y + size.x * direction.x * 0.5,
                ),
            );
            let aabb = aabb.map(Coord::new);
            (
                Some(direction.map(Coord::new)),
                Aabb2::point(aabb.bottom_left() * self.grid.cell_size)
                    .extend_positive(aabb.size() * self.grid.cell_size),
            )
        };
        let pos = self.grid.grid_to_world(pos);
        let collider = Collider::new(collider.translate(pos));
        self.hazards.push(Hazard {
            sprite: Aabb2::point(pos).extend_positive(self.grid.cell_size),
            collider,
            direction,
            hazard_type: hazard,
        });
    }

    pub fn place_prop(&mut self, pos: vec2<isize>, size: vec2<Coord>, prop: PropType) {
        let pos = self.grid.grid_to_world(pos);
        let sprite = Aabb2::point(pos).extend_symmetric(size / Coord::new(2.0));
        self.props.push(Prop {
            sprite,
            prop_type: prop,
        });
    }

    pub fn place_coin(&mut self, pos: vec2<isize>) {
        let collider = Aabb2::ZERO.extend_positive(self.grid.cell_size);
        let pos = self.grid.grid_to_world(pos);
        let collider = Collider::new(collider.translate(pos));
        self.coins.push(Coin {
            collider,
            collected: false,
        });
    }

    pub fn get_hovered(&self, aabb: Aabb2<Coord>) -> Vec<PlaceableId> {
        let grid_aabb = Collider::new(aabb).grid_aabb(&self.grid);
        itertools::chain![
            self.spotlights
                .iter()
                .enumerate()
                .filter(|(_, spotlight)| aabb
                    .intersects(&Aabb2::point(spotlight.position).extend_uniform(Coord::new(0.5))))
                .map(|(i, _)| PlaceableId::Spotlight(i)),
            self.props
                .iter()
                .enumerate()
                .filter(|(_, prop)| prop.sprite.intersects(&aabb))
                .map(|(i, _)| PlaceableId::Prop(i)),
            self.hazards
                .iter()
                .enumerate()
                .filter(|(_, hazard)| hazard.collider.raw().intersects(&aabb))
                .map(|(i, _)| PlaceableId::Hazard(i)),
            self.coins
                .iter()
                .enumerate()
                .filter(|(_, coin)| coin.collider.raw().intersects(&aabb))
                .map(|(i, _)| PlaceableId::Coin(i)),
            (grid_aabb.min.x..=grid_aabb.max.x)
                .flat_map(move |x| (grid_aabb.min.y..=grid_aabb.max.y).map(move |y| vec2(x, y)))
                .filter_map(|pos| self
                    .tiles
                    .get_tile_isize(pos)
                    .filter(|tile| *tile != "air")
                    .map(|_| PlaceableId::Tile(pos))),
        ]
        .collect()
    }

    pub fn get_block(&self, id: PlaceableId) -> Option<Placeable> {
        match id {
            PlaceableId::Tile(pos) => self
                .tiles
                .get_tile_isize(pos)
                .map(|tile| Placeable::Tile((tile.to_owned(), pos))),
            PlaceableId::Hazard(id) => self.hazards.get(id).cloned().map(Placeable::Hazard),
            PlaceableId::Prop(id) => self.props.get(id).cloned().map(Placeable::Prop),
            PlaceableId::Coin(id) => self.coins.get(id).cloned().map(Placeable::Coin),
            PlaceableId::Spotlight(id) => {
                self.spotlights.get(id).cloned().map(Placeable::Spotlight)
            }
        }
    }

    pub fn get_block_mut(&mut self, id: PlaceableId) -> Option<PlaceableMut> {
        match id {
            PlaceableId::Tile(pos) => self
                .tiles
                .get_tile_isize(pos)
                .map(|tile| PlaceableMut::Tile((tile.to_owned(), pos))),
            PlaceableId::Hazard(id) => self.hazards.get_mut(id).map(PlaceableMut::Hazard),
            PlaceableId::Prop(id) => self.props.get_mut(id).map(PlaceableMut::Prop),
            PlaceableId::Coin(id) => self.coins.get_mut(id).map(PlaceableMut::Coin),
            PlaceableId::Spotlight(id) => self.spotlights.get_mut(id).map(PlaceableMut::Spotlight),
        }
    }

    pub fn remove_blocks<'a>(
        &mut self,
        blocks: impl IntoIterator<Item = &'a PlaceableId>,
        assets: &Assets,
    ) -> Vec<Placeable> {
        let mut spotlights = Vec::new();
        let mut props = Vec::new();
        let mut hazards = Vec::new();
        let mut coins = Vec::new();
        let mut tiles = Vec::new();
        for &block in blocks {
            match block {
                PlaceableId::Tile(pos) => tiles.push(pos),
                PlaceableId::Hazard(id) => hazards.push(id),
                PlaceableId::Prop(id) => props.push(id),
                PlaceableId::Coin(id) => coins.push(id),
                PlaceableId::Spotlight(id) => spotlights.push(id),
            }
        }

        spotlights.sort_unstable();
        props.sort_unstable();
        hazards.sort_unstable();
        coins.sort_unstable();

        let mut removed = Vec::new();
        for id in spotlights.into_iter().rev() {
            let light = self.spotlights.swap_remove(id);
            removed.push(Placeable::Spotlight(light));
        }
        for id in props.into_iter().rev() {
            let prop = self.props.swap_remove(id);
            removed.push(Placeable::Prop(prop));
        }
        for id in hazards.into_iter().rev() {
            let hazard = self.hazards.swap_remove(id);
            removed.push(Placeable::Hazard(hazard));
        }
        for id in coins.into_iter().rev() {
            let coin = self.coins.swap_remove(id);
            removed.push(Placeable::Coin(coin));
        }
        for pos in tiles {
            if let Some(tile) = self.tiles.get_tile_isize(pos) {
                removed.push(Placeable::Tile((tile.to_owned(), pos)));
            }
            self.tiles.set_tile_isize(pos, "air".to_string(), assets);
        }

        removed
    }

    pub fn change_size(&mut self, size: vec2<usize>, assets: &Assets) {
        self.tiles.change_size(size, assets);
        self.size = size;
    }

    pub fn translate(&mut self, delta: vec2<isize>, assets: &Assets) {
        self.tiles.translate(delta, assets);
        self.tiles.update_geometry(assets);

        let delta = self.grid.grid_to_world(delta) - self.grid.grid_to_world(vec2::ZERO);
        self.spawn_point += delta;
        for coin in &mut self.coins {
            coin.translate(delta);
        }
        for hazard in &mut self.hazards {
            hazard.translate(delta);
        }
        for prop in &mut self.props {
            prop.translate(delta);
        }
        for light in &mut self.spotlights {
            light.position += delta;
        }
    }

    pub fn calculate_light_geometry(
        &self,
        geng: &Geng,
        assets: &Assets,
    ) -> ugli::VertexBuffer<NormalVertex> {
        let vertices = self
            .tiles
            .tiles()
            .iter()
            .enumerate()
            .filter_map(|(i, tile)| {
                (tile != "air").then(|| {
                    let grid_pos = index_to_pos(i, self.size.x).map(|x| x as isize);
                    let pos = self.grid.grid_to_world(grid_pos);
                    let aabb = Aabb2::point(pos)
                        .extend_positive(self.grid.cell_size)
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
        geng: &Geng,
        assets: &Assets,
    ) -> (
        ugli::VertexBuffer<NormalVertex>,
        HashMap<Tile, ugli::VertexBuffer<Vertex>>,
    ) {
        let mut static_geom = Vec::new();
        let mut shaded_geom = HashMap::<Tile, Vec<Vertex>>::new();
        for (i, tile) in self.tiles.tiles().iter().enumerate() {
            if tile == "air" {
                continue;
            }

            let pos = index_to_pos(i, self.size.x);
            let pos = self.grid.grid_to_world(pos.map(|x| x as isize));
            let pos = Aabb2::point(pos)
                .extend_positive(self.grid.cell_size)
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

    pub fn load(path: impl AsRef<std::path::Path>) -> anyhow::Result<Self> {
        let path = run_dir().join("assets").join("rooms").join(path);
        futures::executor::block_on(async move {
            debug!("Loading room {path:?}");
            let room = file::load_json(&path)
                .await
                .context(format!("Failed to load room {path:?}"))?;
            Ok(room)
        })
    }

    pub fn save(&self, path: impl AsRef<std::path::Path>) -> anyhow::Result<()> {
        let path = run_dir().join("assets").join("rooms").join(path);
        #[cfg(not(target_arch = "wasm32"))]
        {
            let file = std::fs::File::create(path)?;
            let writer = std::io::BufWriter::new(file);
            serde_json::to_writer_pretty(writer, self)?;
            info!("Saved the level");
            Ok(())
        }
        #[cfg(target_arch = "wasm32")]
        {
            anyhow::bail!("unimplemented")
        }
    }
}

impl Placeable {
    pub fn position(&self, grid: &Grid) -> vec2<Coord> {
        match self {
            Placeable::Tile((_, pos)) => grid.grid_to_world(*pos),
            Placeable::Hazard(hazard) => hazard.collider.feet(),
            Placeable::Prop(prop) => prop.sprite.center(),
            Placeable::Coin(coin) => coin.collider.feet(),
            Placeable::Spotlight(light) => light.position,
        }
    }

    pub fn translate(&mut self, offset: vec2<Coord>, grid: &Grid) {
        match self {
            Placeable::Tile((_, pos)) => {
                let offset = (offset / grid.cell_size).map(|x| x.round().as_f32() as isize);
                *pos += offset;
            }
            Placeable::Hazard(hazard) => hazard.translate(offset),
            Placeable::Prop(prop) => prop.translate(offset),
            Placeable::Coin(coin) => coin.translate(offset),
            Placeable::Spotlight(light) => light.position += offset,
        }
    }

    pub fn get_type(&self) -> PlaceableType {
        match self {
            Placeable::Tile((tile, _)) => PlaceableType::Tile(tile.to_owned()),
            Placeable::Hazard(hazard) => PlaceableType::Hazard(hazard.hazard_type.to_owned()),
            Placeable::Prop(prop) => PlaceableType::Prop(prop.prop_type.to_owned()),
            Placeable::Coin(_) => PlaceableType::Coin,
            Placeable::Spotlight(spotlight) => PlaceableType::Spotlight(*spotlight),
        }
    }

    pub fn sprite(&self, grid: &Grid) -> Aabb2<Coord> {
        match self {
            Placeable::Tile((_, pos)) => {
                let collider = grid.cell_collider(*pos);
                collider.raw()
            }
            Placeable::Hazard(hazard) => hazard.collider.raw(),
            Placeable::Prop(prop) => prop.sprite,
            Placeable::Coin(coin) => coin.collider.raw(),
            Placeable::Spotlight(light) => {
                Aabb2::point(light.position).extend_uniform(Coord::new(0.5))
            }
        }
    }
}

impl Hazard {
    pub fn translate(&mut self, delta: vec2<Coord>) {
        self.sprite = self.sprite.translate(delta);
        self.collider.translate(delta);
    }
}

impl Coin {
    pub fn translate(&mut self, delta: vec2<Coord>) {
        self.collider.translate(delta);
    }
}

impl Prop {
    pub fn translate(&mut self, delta: vec2<Coord>) {
        self.sprite = self.sprite.translate(delta);
    }
}

impl PlaceableId {
    pub fn fits_type(&self, ty: &PlaceableType) -> bool {
        matches!(
            (self, ty),
            (PlaceableId::Tile(_), PlaceableType::Tile(_))
                | (PlaceableId::Hazard(_), PlaceableType::Hazard(_))
                | (PlaceableId::Prop(_), PlaceableType::Prop(_))
                | (PlaceableId::Coin(_), PlaceableType::Coin)
                | (PlaceableId::Spotlight(_), PlaceableType::Spotlight(_))
        )
    }
}

impl Default for Room {
    fn default() -> Self {
        Self::new(vec2(40, 23))
    }
}

#[derive(ugli::Vertex, Debug, Clone, Copy)]
pub struct NormalVertex {
    pub a_pos: vec2<f32>,
    pub a_normal: vec2<f32>,
}

#[derive(ugli::Vertex, Debug, Clone, Copy)]
pub struct Vertex {
    pub a_pos: vec2<f32>,
    pub a_uv: vec2<f32>,
}

#[derive(ugli::Vertex, Debug, Clone, Copy)]
pub struct MaskedVertex {
    pub a_pos: vec2<f32>,
    pub a_uv: vec2<f32>,
    pub a_mask_uv: vec2<f32>,
}

impl Vertex {
    pub fn mask(self, a_mask_uv: vec2<f32>) -> MaskedVertex {
        MaskedVertex {
            a_pos: self.a_pos,
            a_uv: self.a_uv,
            a_mask_uv,
        }
    }
}

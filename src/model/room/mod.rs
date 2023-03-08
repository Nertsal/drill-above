use super::*;

mod layer;
mod placeable;

pub use layer::*;
pub use placeable::*;

#[derive(Debug, Clone, Serialize, Deserialize, geng::Assets)]
#[asset(json)]
pub struct Room {
    pub drill_allowed: bool,
    #[serde(default)]
    pub grid: Grid,
    pub size: vec2<usize>,
    pub spawn_point: vec2<Coord>,
    pub layers: RoomLayers,
    #[serde(default)]
    pub hazards: Vec<Hazard>,
    #[serde(default)]
    pub coins: Vec<Coin>,
    #[serde(default)]
    pub global_light: GlobalLightSource,
    #[serde(default)]
    pub spotlights: Vec<SpotlightSource>,
    #[serde(default)]
    pub transitions: Vec<RoomTransition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomLayers {
    pub background: RoomLayer,
    pub main: RoomLayer,
    pub foreground: RoomLayer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveLayer {
    Background,
    Main,
    Foreground,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomTransition {
    pub collider: Collider,
    /// Name of the room to which the transition moves.
    pub to_room: String,
    /// Offset applied to entities to move them into the new room's coordinate system.
    pub offset: vec2<isize>,
}

impl Room {
    pub fn new(size: vec2<usize>) -> Self {
        let grid = Grid::default();
        // grid.offset = size.map(|x| Coord::new(x as f32 / 2.0)) * grid.cell_size;
        Self {
            spawn_point: grid.grid_to_world(size.map(|x| x as isize / 2)),
            hazards: Vec::new(),
            coins: Vec::new(),
            layers: RoomLayers::new(size),
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

    pub fn place_block(
        &mut self,
        block: Placeable,
        layer: ActiveLayer,
        assets: &Assets,
    ) -> PlaceableId {
        match block {
            Placeable::Tile((tile, pos)) => {
                self.layers
                    .get_mut(layer)
                    .tiles
                    .set_tile_isize(pos, tile, assets);
                PlaceableId::Tile(pos)
            }
            Placeable::Hazard(hazard) => {
                self.hazards.push(hazard);
                PlaceableId::Hazard(self.hazards.len() - 1)
            }
            Placeable::Prop(prop) => {
                let layer = self.layers.get_mut(layer);
                layer.props.push(prop);
                PlaceableId::Prop(layer.props.len() - 1)
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

    pub fn place_tile(
        &mut self,
        pos: vec2<isize>,
        tile: Tile,
        layer: ActiveLayer,
        assets: &Assets,
    ) {
        self.layers
            .get_mut(layer)
            .tiles
            .set_tile_isize(pos, tile, assets);
    }

    pub fn place_hazard(&mut self, pos: vec2<Coord>, hazard: HazardType) {
        let (pos, offset) = self.grid.world_to_grid(pos);
        let connect = |pos| {
            self.layers
                .main
                .tiles
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
            sprite: Sprite::new(Aabb2::point(pos).extend_positive(self.grid.cell_size)),
            collider,
            direction,
            hazard_type: hazard,
        });
    }

    pub fn place_prop(
        &mut self,
        pos: vec2<isize>,
        size: vec2<Coord>,
        prop: PropType,
        layer: ActiveLayer,
    ) {
        let pos = self.grid.grid_to_world(pos);
        let sprite = Sprite::new(Aabb2::point(pos).extend_symmetric(size / Coord::new(2.0)));
        self.layers.get_mut(layer).props.push(Prop {
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

    pub fn get_hovered(&self, aabb: Aabb2<Coord>, layer: ActiveLayer) -> Vec<PlaceableId> {
        let grid_aabb = Collider::new(aabb).grid_aabb(&self.grid);
        let main_layer = matches!(layer, ActiveLayer::Main);
        let layer = self.layers.get(layer);

        let mut res: Vec<PlaceableId> = itertools::chain![
            layer
                .props
                .iter()
                .enumerate()
                .filter(|(_, prop)| prop.sprite.pos.intersects(&aabb))
                .map(|(i, _)| PlaceableId::Prop(i)),
            (grid_aabb.min.x..=grid_aabb.max.x)
                .flat_map(move |x| (grid_aabb.min.y..=grid_aabb.max.y).map(move |y| vec2(x, y)))
                .filter_map(|pos| layer
                    .tiles
                    .get_tile_isize(pos)
                    .filter(|tile| *tile != "air")
                    .map(|_| PlaceableId::Tile(pos))),
        ]
        .collect();

        if main_layer {
            res.extend(itertools::chain![
                self.spotlights
                    .iter()
                    .enumerate()
                    .filter(|(_, spotlight)| {
                        aabb.intersects(
                            &Aabb2::point(spotlight.position).extend_uniform(Coord::new(0.5)),
                        )
                    })
                    .map(|(i, _)| PlaceableId::Spotlight(i)),
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
            ]);
        }

        res
    }

    pub fn get_block(&self, id: PlaceableId, layer: ActiveLayer) -> Option<Placeable> {
        match id {
            PlaceableId::Tile(pos) => self
                .layers
                .get(layer)
                .tiles
                .get_tile_isize(pos)
                .map(|tile| Placeable::Tile((tile.to_owned(), pos))),
            PlaceableId::Hazard(id) => self.hazards.get(id).cloned().map(Placeable::Hazard),
            PlaceableId::Prop(id) => self
                .layers
                .get(layer)
                .props
                .get(id)
                .cloned()
                .map(Placeable::Prop),
            PlaceableId::Coin(id) => self.coins.get(id).cloned().map(Placeable::Coin),
            PlaceableId::Spotlight(id) => {
                self.spotlights.get(id).cloned().map(Placeable::Spotlight)
            }
        }
    }

    pub fn get_block_mut(&mut self, id: PlaceableId, layer: ActiveLayer) -> Option<PlaceableMut> {
        match id {
            PlaceableId::Tile(pos) => self
                .layers
                .get_mut(layer)
                .tiles
                .get_tile_isize(pos)
                .map(|tile| PlaceableMut::Tile((tile.to_owned(), pos))),
            PlaceableId::Hazard(id) => self.hazards.get_mut(id).map(PlaceableMut::Hazard),
            PlaceableId::Prop(id) => self
                .layers
                .get_mut(layer)
                .props
                .get_mut(id)
                .map(PlaceableMut::Prop),
            PlaceableId::Coin(id) => self.coins.get_mut(id).map(PlaceableMut::Coin),
            PlaceableId::Spotlight(id) => self.spotlights.get_mut(id).map(PlaceableMut::Spotlight),
        }
    }

    pub fn remove_blocks<'a>(
        &mut self,
        blocks: impl IntoIterator<Item = &'a PlaceableId>,
        layer: ActiveLayer,
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

        let layer = self.layers.get_mut(layer);

        let mut removed = Vec::new();
        for id in spotlights.into_iter().rev() {
            let light = self.spotlights.swap_remove(id);
            removed.push(Placeable::Spotlight(light));
        }
        for id in props.into_iter().rev() {
            let prop = layer.props.swap_remove(id);
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
            if let Some(tile) = layer.tiles.get_tile_isize(pos) {
                removed.push(Placeable::Tile((tile.to_owned(), pos)));
            }
            layer.tiles.set_tile_isize(pos, "air".to_string(), assets);
        }

        removed
    }

    pub fn change_size(&mut self, size: vec2<usize>, assets: &Assets) {
        for layer in self.layers.iter_mut() {
            layer.tiles.change_size(size, assets);
        }
        self.size = size;
    }

    fn move_entities(&mut self, move_fn: impl Fn(vec2<Coord>) -> vec2<Coord>) {
        self.spawn_point = move_fn(self.spawn_point);
        for coin in &mut self.coins {
            coin.teleport(move_fn(coin.collider.pos()));
        }
        for hazard in &mut self.hazards {
            hazard.teleport(move_fn(hazard.collider.pos()));
        }
        for light in &mut self.spotlights {
            light.position = move_fn(light.position);
        }

        for layer in self.layers.iter_mut() {
            for prop in &mut layer.props {
                prop.teleport(move_fn(prop.sprite.pos.center()));
            }
        }
    }

    pub fn translate(&mut self, delta: vec2<isize>, assets: &Assets) {
        for layer in self.layers.iter_mut() {
            layer.tiles.translate(delta, assets);
        }

        let delta = self.grid.grid_to_world(delta) - self.grid.grid_to_world(vec2::ZERO);
        self.move_entities(|pos| pos + delta);
    }

    pub fn flip_h(&mut self, assets: &Assets) {
        for layer in self.layers.iter_mut() {
            layer.tiles.flip_h(assets);
        }
        let bounds = self.bounds();
        self.move_entities(|pos| vec2(bounds.min.x + bounds.max.x - pos.x, pos.y));
    }

    pub fn flip_v(&mut self, assets: &Assets) {
        for layer in self.layers.iter_mut() {
            layer.tiles.flip_v(assets);
        }
        let bounds = self.bounds();
        self.move_entities(|pos| vec2(pos.x, bounds.min.y + bounds.max.y - pos.y));
    }

    pub fn load(name: impl AsRef<std::path::Path>) -> anyhow::Result<Self> {
        let path = room_path(name);
        futures::executor::block_on(async move {
            debug!("Loading room {path:?}");
            let room = file::load_json(&path).await?;
            Ok(room)
        })
    }

    pub fn save(&self, name: impl AsRef<std::path::Path>) -> anyhow::Result<()> {
        let path = room_path(name);
        #[cfg(not(target_arch = "wasm32"))]
        {
            let file = std::fs::File::create(&path)?;
            let writer = std::io::BufWriter::new(file);
            serde_json::to_writer_pretty(writer, self)?;
            info!("Saved the room {path:?}");
            Ok(())
        }
        #[cfg(target_arch = "wasm32")]
        {
            anyhow::bail!("unimplemented")
        }
    }
}

impl RoomLayers {
    pub fn new(size: vec2<usize>) -> Self {
        Self {
            background: RoomLayer::new(size),
            main: RoomLayer::new(size),
            foreground: RoomLayer::new(size),
        }
    }

    // pub fn iter(&self) -> impl Iterator<Item = &RoomLayer> {
    //     [&self.background, &self.main, &self.foreground].into_iter()
    // }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut RoomLayer> {
        [&mut self.background, &mut self.main, &mut self.foreground].into_iter()
    }

    pub fn get(&self, layer: ActiveLayer) -> &RoomLayer {
        match layer {
            ActiveLayer::Background => &self.background,
            ActiveLayer::Main => &self.main,
            ActiveLayer::Foreground => &self.foreground,
        }
    }

    pub fn get_mut(&mut self, layer: ActiveLayer) -> &mut RoomLayer {
        match layer {
            ActiveLayer::Background => &mut self.background,
            ActiveLayer::Main => &mut self.main,
            ActiveLayer::Foreground => &mut self.foreground,
        }
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

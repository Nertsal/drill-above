use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, geng::Assets)]
#[asset(json)]
pub struct Level {
    pub drill_allowed: bool,
    pub grid: Grid,
    pub size: vec2<usize>,
    pub spawn_point: vec2<Coord>,
    pub finish: vec2<Coord>,
    pub tiles: TileMap,
    pub hazards: Vec<Hazard>,
    pub coins: Vec<Coin>,
    #[serde(default)]
    pub props: Vec<Prop>,
    pub next_level: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum BlockType {
    Tile(Tile),
    Hazard(HazardType),
    Prop(PropType),
    Coin,
}

#[derive(Debug, Clone)]
pub enum Block {
    Tile((Tile, vec2<isize>)),
    Hazard(Hazard),
    Prop(Prop),
    Coin(Coin),
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum HazardType {
    Spikes,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PropType {
    DrillUse,
    DrillJump,
}

impl HazardType {
    pub fn all() -> [Self; 1] {
        use HazardType::*;
        [Spikes]
    }
}

impl PropType {
    pub fn all() -> [Self; 2] {
        use PropType::*;
        [DrillUse, DrillJump]
    }
}

impl Level {
    pub fn new(size: vec2<usize>) -> Self {
        let mut grid = Grid::default();
        grid.offset = size.map(|x| Coord::new(x as f32 / 2.0)) * grid.cell_size;
        Self {
            spawn_point: grid.grid_to_world(size.map(|x| x as isize / 2)),
            finish: grid.grid_to_world(size.map(|x| x as isize / 2)),
            tiles: TileMap::new(size),
            hazards: Vec::new(),
            coins: Vec::new(),
            props: Vec::new(),
            next_level: None,
            drill_allowed: true,
            grid,
            size,
        }
    }

    pub fn finish(&self) -> Collider {
        Collider::new(Aabb2::point(self.finish).extend_positive(self.grid.cell_size))
    }

    pub fn bounds(&self) -> Aabb2<Coord> {
        Aabb2::from_corners(
            self.grid.grid_to_world(vec2(0, 0)),
            self.grid.grid_to_world(self.size.map(|x| x as isize)),
        )
    }

    pub fn place_hazard(&mut self, pos: vec2<isize>, hazard: HazardType) {
        let connect = |pos| {
            self.tiles
                .get_tile_isize(pos)
                .map(|tile| !matches!(tile, Tile::Air))
                .unwrap_or(false)
        };
        let (direction, collider) = match hazard {
            HazardType::Spikes => {
                let size = vec2(0.8, 0.4);
                let direction = -[vec2(1, 0), vec2(-1, 0), vec2(0, 1)]
                    .into_iter()
                    .find(|&d| connect(pos + d))
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
            }
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

    pub fn remove_all_at(&mut self, pos: vec2<Coord>) -> Vec<Block> {
        let mut removed = Vec::new();

        // Try hazards first
        if let Some(i) = self.props.iter().position(|prop| prop.sprite.contains(pos)) {
            let prop = self.props.swap_remove(i);
            removed.push(Block::Prop(prop));
        }
        // Try hazards first
        while let Some(i) = self
            .hazards
            .iter()
            .position(|hazard| hazard.collider.contains(pos))
        {
            let hazard = self.hazards.swap_remove(i);
            removed.push(Block::Hazard(hazard));
        }
        // Try coins
        while let Some(i) = self
            .coins
            .iter()
            .position(|hazard| hazard.collider.contains(pos))
        {
            let coin = self.coins.swap_remove(i);
            removed.push(Block::Coin(coin));
        }

        // Try tiles
        let pos = self.grid.world_to_grid(pos).0;
        if let Some(tile) = self.tiles.get_tile_isize(pos) {
            removed.push(Block::Tile((tile, pos)));
            self.tiles.set_tile_isize(pos, Tile::Air);
        }

        removed
    }

    pub fn change_size(&mut self, size: vec2<usize>) {
        self.tiles.change_size(size);
        self.size = size;
    }

    pub fn load(path: impl AsRef<std::path::Path>) -> anyhow::Result<Self> {
        let path = run_dir().join("assets").join("levels").join(path);
        #[cfg(not(target_arch = "wasm32"))]
        {
            let file = std::fs::File::open(path)?;
            let reader = std::io::BufReader::new(file);
            let value: serde_json::Value = serde_json::from_reader(reader)?;
            serde_json::from_value(value.clone()).or_else(|err| {
                info!("Failed to load level, trying old format");
                let old: old::Level = serde_json::from_value(value)
                    .context(err)
                    .context("tried old format")?;
                Ok(old.into())
            })
        }
        #[cfg(target_arch = "wasm32")]
        {
            anyhow::bail!("unimplemented")
        }
    }

    pub fn save(&self, path: impl AsRef<std::path::Path>) -> anyhow::Result<()> {
        let path = run_dir().join("assets").join("levels").join(path);
        #[cfg(not(target_arch = "wasm32"))]
        {
            let file = std::fs::File::create(path)?;
            let writer = std::io::BufWriter::new(file);
            serde_json::to_writer_pretty(writer, self)?;
            Ok(())
        }
        #[cfg(target_arch = "wasm32")]
        {
            anyhow::bail!("unimplemented")
        }
    }
}

impl Default for Level {
    fn default() -> Self {
        Self::new(vec2(40, 23))
    }
}

mod old {
    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Level {
        drill_allowed: bool,
        grid: Grid,
        size: Vec2<usize>,
        spawn_point: Vec2<Coord>,
        finish: Vec2<Coord>,
        tiles: TileMap,
        #[serde(default)]
        hazards: Vec<Hazard>,
        #[serde(default)]
        coins: Vec<Coin>,
        #[serde(default)]
        props: Vec<Prop>,
        #[serde(default)]
        next_level: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct Grid {
        cell_size: Vec2<Coord>,
        offset: Vec2<Coord>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TileMap {
        size: Vec2<usize>,
        tiles: Vec<Tile>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct Collider(Aabb<Coord>);

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct Coin {
        collider: Collider,
        collected: bool,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct Hazard {
        sprite: Aabb<Coord>,
        direction: Option<Vec2<Coord>>,
        collider: Collider,
        hazard_type: HazardType,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct Prop {
        sprite: Aabb<Coord>,
        prop_type: PropType,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct Vec2<T> {
        x: T,
        y: T,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct Aabb<T> {
        x_min: T,
        x_max: T,
        y_min: T,
        y_max: T,
    }

    impl From<Level> for super::Level {
        fn from(value: Level) -> Self {
            Self {
                drill_allowed: value.drill_allowed,
                grid: value.grid.into(),
                size: value.size.into(),
                spawn_point: value.spawn_point.into(),
                finish: value.finish.into(),
                tiles: value.tiles.into(),
                hazards: value.hazards.into_iter().map(Into::into).collect(),
                coins: value.coins.into_iter().map(Into::into).collect(),
                props: value.props.into_iter().map(Into::into).collect(),
                next_level: value.next_level,
            }
        }
    }

    impl From<Grid> for super::Grid {
        fn from(value: Grid) -> Self {
            Self {
                cell_size: value.cell_size.into(),
                offset: value.offset.into(),
            }
        }
    }

    impl From<TileMap> for super::TileMap {
        fn from(value: TileMap) -> Self {
            Self {
                size: value.size.into(),
                tiles: value.tiles,
            }
        }
    }

    impl From<Collider> for super::Collider {
        fn from(value: Collider) -> Self {
            Self::new(value.0.into())
        }
    }

    impl From<Coin> for super::Coin {
        fn from(value: Coin) -> Self {
            Self {
                collider: value.collider.into(),
                collected: value.collected,
            }
        }
    }

    impl From<Hazard> for super::Hazard {
        fn from(value: Hazard) -> Self {
            Self {
                sprite: value.sprite.into(),
                direction: value.direction.map(Into::into),
                collider: value.collider.into(),
                hazard_type: HazardType::Spikes,
            }
        }
    }

    impl From<Prop> for super::Prop {
        fn from(value: Prop) -> Self {
            Self {
                sprite: value.sprite.into(),
                prop_type: value.prop_type,
            }
        }
    }

    impl<T> From<Vec2<T>> for vec2<T> {
        fn from(value: Vec2<T>) -> Self {
            Self(value.x, value.y)
        }
    }

    impl<T> From<Aabb<T>> for Aabb2<T> {
        fn from(value: Aabb<T>) -> Self {
            Self {
                min: vec2(value.x_min, value.y_min),
                max: vec2(value.x_max, value.y_max),
            }
        }
    }
}

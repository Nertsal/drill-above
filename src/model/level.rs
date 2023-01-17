use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, geng::Assets)]
#[asset(json)]
pub struct Level {
    pub drill_allowed: bool,
    pub grid: Grid,
    pub size: Vec2<usize>,
    pub spawn_point: Vec2<Coord>,
    pub finish: Vec2<Coord>,
    pub tiles: TileMap,
    pub hazards: Vec<Hazard>,
    pub coins: Vec<Coin>,
    pub next_level: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coin {
    pub collider: Collider,
    pub collected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hazard {
    pub sprite: AABB<Coord>,
    pub direction: Option<Vec2<Coord>>,
    pub collider: Collider,
    pub hazard_type: HazardType,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum HazardType {
    Spikes,
}

impl HazardType {
    pub fn all() -> [Self; 1] {
        use HazardType::*;
        [Spikes]
    }
}

impl Level {
    pub fn new(size: Vec2<usize>) -> Self {
        let mut grid = Grid::default();
        grid.offset = size.map(|x| Coord::new(x as f32 / 2.0)) * grid.cell_size;
        Self {
            spawn_point: grid.grid_to_world(size.map(|x| x as isize / 2)),
            finish: grid.grid_to_world(size.map(|x| x as isize / 2)),
            tiles: TileMap::new(size),
            hazards: Vec::new(),
            coins: Vec::new(),
            next_level: None,
            drill_allowed: true,
            grid,
            size,
        }
    }

    pub fn bounds(&self) -> AABB<Coord> {
        AABB::from_corners(
            self.grid.grid_to_world(vec2(0, 0)),
            self.grid.grid_to_world(self.size.map(|x| x as isize)),
        )
    }

    pub fn place_hazard(&mut self, pos: Vec2<isize>, hazard: HazardType) {
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
                let aabb = AABB::from_corners(
                    pos + vec2(-size.x * direction.y * 0.5, -size.x * direction.x * 0.5),
                    pos + vec2(
                        size.x * direction.y * 0.5 + size.y * direction.x,
                        size.y * direction.y + size.x * direction.x * 0.5,
                    ),
                );
                let aabb = aabb.map(Coord::new);
                (
                    Some(direction.map(|x| Coord::new(x as f32))),
                    AABB::point(aabb.bottom_left() * self.grid.cell_size)
                        .extend_positive(aabb.size() * self.grid.cell_size),
                )
            }
        };
        let pos = self.grid.grid_to_world(pos);
        let collider = Collider::new(collider.translate(pos));
        self.hazards.push(Hazard {
            sprite: AABB::point(pos).extend_positive(self.grid.cell_size),
            collider,
            direction,
            hazard_type: hazard,
        });
    }

    pub fn place_coin(&mut self, pos: Vec2<isize>) {
        let collider = AABB::ZERO.extend_positive(self.grid.cell_size);
        let pos = self.grid.grid_to_world(pos);
        let collider = Collider::new(collider.translate(pos));
        self.coins.push(Coin {
            collider,
            collected: false,
        });
    }

    pub fn change_size(&mut self, size: Vec2<usize>) {
        self.tiles.change_size(size);
        self.size = size;
    }

    pub fn load(path: impl AsRef<std::path::Path>) -> anyhow::Result<Self> {
        let path = run_dir().join("assets").join("levels").join(path);
        #[cfg(not(target_arch = "wasm32"))]
        {
            let file = std::fs::File::open(path)?;
            let reader = std::io::BufReader::new(file);
            Ok(serde_json::from_reader(reader)?)
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

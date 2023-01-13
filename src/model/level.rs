use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, geng::Assets)]
#[asset(json)]
pub struct Level {
    pub grid: Grid,
    pub size: Vec2<usize>,
    pub spawn_point: Vec2<Coord>,
    pub tiles: TileMap,
    pub hazards: Vec<Hazard>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hazard {
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
            tiles: TileMap::new(size),
            hazards: Vec::new(),
            grid,
            size,
        }
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
            Err("unimplemented")
        }
    }

    pub fn place_hazard(&mut self, pos: Vec2<isize>, hazard: HazardType) {
        let collider = match hazard {
            HazardType::Spikes => {
                AABB::ZERO.extend_positive(vec2(1.0, 1.0).map(Coord::new) * self.grid.cell_size)
            }
        };
        let pos = self.grid.grid_to_world(pos);
        let collider = Collider::new(collider.translate(pos));
        self.hazards.push(Hazard {
            collider,
            hazard_type: hazard,
        });
    }
}

impl Default for Level {
    fn default() -> Self {
        Self::new(vec2(40, 23))
    }
}

use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, geng::Assets)]
#[asset(json)]
pub struct Level {
    pub grid: Grid,
    pub size: Vec2<usize>,
    pub tiles: TileMap,
    pub spawn_point: Vec2<Coord>,
}

impl Level {
    pub fn new(size: Vec2<usize>) -> Self {
        let mut grid = Grid::default();
        grid.offset = size.map(|x| Coord::new(x as f32 / 2.0)) * grid.cell_size;
        Self {
            tiles: TileMap::new(size),
            spawn_point: grid.grid_to_world(size.map(|x| x as isize / 2)),
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
}

impl Default for Level {
    fn default() -> Self {
        Self::new(vec2(40, 23))
    }
}

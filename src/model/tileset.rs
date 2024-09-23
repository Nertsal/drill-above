use super::*;

pub struct TileSet {
    pub texture: Texture,
    pub config: TileSetConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, geng::asset::Load)]
#[load(serde = "json")]
pub struct TileSetConfig {
    pub size: vec2<usize>,
    pub tiles: Vec<([ConnectionFilter; 8], UvRect)>,
}

type UvRect = [vec2<f32>; 4];

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Connection {
    None,
    Same,
    Other,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ConnectionFilter {
    Some,
    None,
    Any,
    Same,
    Other,
}

impl TileSet {
    pub fn get_tile_geometry(&self, index: usize) -> UvRect {
        self.config
            .tiles
            .get(index)
            .expect("Failed to find the tile by index")
            .1
    }

    pub fn get_tile_connected(&self, connections: [Connection; 8]) -> Vec<usize> {
        let con_match = |pattern: &[ConnectionFilter; 8]| {
            connections
                .iter()
                .zip(pattern)
                .all(|(&con, pat)| pat.matches(con))
        };

        self.config
            .tiles
            .iter()
            .enumerate()
            .filter_map(|(i, (pattern, _))| con_match(pattern).then_some(i))
            .collect()
    }
}

pub fn get_tile_uv(index: usize, set_size: vec2<usize>) -> UvRect {
    let pos = vec2(index % set_size.x, index / set_size.x);
    get_tile_uv_xy(pos, set_size)
}

pub fn get_tile_uv_xy(pos: vec2<usize>, set_size: vec2<usize>) -> UvRect {
    let tile_size = set_size.map(|x| 1.0 / x as f32);
    let pos = pos.map(|x| x as f32) * tile_size;
    [
        pos,
        pos + vec2(tile_size.x, 0.0),
        pos + tile_size,
        pos + vec2(0.0, tile_size.y),
    ]
}

impl TileSetConfig {
    pub fn generate_from(texture: &image::RgbaImage, size: vec2<usize>) -> Self {
        let tiles = {
            let size = size.map(|x| x as u32);
            let (w, h) = texture.dimensions();
            let texture_size = vec2(w, h);
            let tile_size = texture_size / size;
            let positions = [
                (0, 0),
                (1, 0),
                (2, 0),
                (2, 1),
                (2, 2),
                (1, 2),
                (0, 2),
                (0, 1),
            ]
            .map(|(x, y)| vec2(x, y) * (tile_size - vec2(1, 1)) / 2);
            (0..size.y)
                .flat_map(|y| {
                    (0..size.x).filter_map(move |x| {
                        let pos = vec2(x, y) * tile_size;
                        let cons = positions.map(|d| {
                            let pos = pos + d;
                            let pixel = texture.get_pixel(pos.x, texture_size.y - pos.y - 1);
                            ConnectionFilter::from_color(pixel.0)
                        });
                        let mut connections = [ConnectionFilter::None; 8];
                        for i in 0..8 {
                            let Some(con) = cons[i] else {
                                return None;
                            };
                            connections[i] = con;
                        }
                        let uv =
                            get_tile_uv_xy(vec2(x as usize, y as usize), size.map(|x| x as usize));
                        Some((connections, uv))
                    })
                })
                .collect()
        };
        Self { size, tiles }
    }
}

impl ConnectionFilter {
    fn from_color(color: [u8; 4]) -> Option<Self> {
        match color {
            [_, _, _, 0] => None,
            [255, 0, 255, _] => Some(Self::Some),
            [255, 255, 255, _] => Some(Self::None),
            [0, 255, 0, _] => Some(Self::Any),
            [0, 0, 255, _] => Some(Self::Same),
            [255, 0, 0, _] => Some(Self::Other),
            _ => panic!("unknown color: {color:?}"),
        }
    }

    fn matches(&self, connection: Connection) -> bool {
        match self {
            Self::Some => !matches!(connection, Connection::None),
            Self::None => matches!(connection, Connection::None),
            Self::Any => true,
            Self::Same => matches!(connection, Connection::Same),
            Self::Other => !matches!(connection, Connection::Same),
        }
    }
}

impl geng::asset::Load for TileSet {
    fn load(
        manager: &geng::asset::Manager,
        path: &std::path::Path,
        (): &(),
    ) -> geng::asset::Future<Self> {
        let manager = manager.clone();
        let path = path.to_owned();
        async move {
            let texture = Texture::load(&manager, &path, &()).await?;

            let name = path.file_stem().unwrap().to_str().unwrap();
            let config = path.with_file_name(format!("{name}_config.json"));
            let config = TileSetConfig::load(&manager, &config, &()).await?;
            Ok(Self { texture, config })
        }
        .boxed_local()
    }

    const DEFAULT_EXT: Option<&'static str> = ugli::Texture::DEFAULT_EXT;

    type Options = ();
}

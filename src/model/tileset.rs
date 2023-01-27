use super::*;

pub struct TileSet {
    texture: ugli::Texture,
    pub config: TileSetConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, geng::Assets)]
#[asset(json)]
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
    fn new(mut texture: ugli::Texture, config: TileSetConfig) -> Self {
        texture.set_filter(ugli::Filter::Nearest);
        Self { texture, config }
    }

    pub fn texture(&self) -> &ugli::Texture {
        &self.texture
    }

    pub fn get_tile_connected(&self, connections: [Connection; 8]) -> UvRect {
        let con_match = |pattern: &[ConnectionFilter; 8]| {
            connections
                .iter()
                .zip(pattern)
                .all(|(&con, pat)| pat.matches(con))
        };

        self.config
            .tiles
            .iter()
            .find_map(|(pattern, uv)| con_match(pattern).then_some(uv).copied())
            .unwrap_or_else(|| {
                error!("Failed to find the uv for {:?}", connections);
                self.config.tiles.first().unwrap().1
            })
        // .expect("Failed to find a suitable tile")
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

impl geng::LoadAsset for TileSet {
    fn load(geng: &Geng, path: &std::path::Path) -> geng::AssetFuture<Self> {
        let geng = geng.clone();
        let path = path.to_owned();
        async move {
            let mut texture = ugli::Texture::load(&geng, &path).await?;
            texture.set_filter(ugli::Filter::Nearest);
            let name = path.file_stem().unwrap().to_str().unwrap();
            let path = path.parent().unwrap().join(format!("{}_config.json", name));
            let config = TileSetConfig::load(&geng, &path).await?;
            Ok(Self::new(texture, config))
        }
        .boxed_local()
    }

    const DEFAULT_EXT: Option<&'static str> = ugli::Texture::DEFAULT_EXT;
}

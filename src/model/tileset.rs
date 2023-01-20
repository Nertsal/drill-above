use super::*;

pub struct TileSet {
    texture: ugli::Texture,
    pub config: TileSetConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, geng::Assets)]
#[asset(json)]
pub struct TileSetConfig {
    pub size: Vec2<usize>,
    pub tiles: Vec<([Connection; 8], UvRect)>,
}

type UvRect = [Vec2<f32>; 4];

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Connection {
    Some,
    None,
    Any,
}

impl TileSet {
    fn new(mut texture: ugli::Texture, config: TileSetConfig) -> Self {
        texture.set_filter(ugli::Filter::Nearest);
        Self { texture, config }
    }

    pub fn texture(&self) -> &ugli::Texture {
        &self.texture
    }

    pub fn get_tile_connected(&self, connections: [bool; 8]) -> UvRect {
        let con_match = |pattern: &[Connection; 8]| {
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

pub fn get_tile_uv(index: usize, set_size: Vec2<usize>) -> UvRect {
    let pos = vec2(index % set_size.x, index / set_size.x);
    get_tile_uv_xy(pos, set_size)
}

pub fn get_tile_uv_xy(pos: Vec2<usize>, set_size: Vec2<usize>) -> UvRect {
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
    pub fn generate_from(texture: &image::RgbaImage, size: Vec2<usize>) -> Self {
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
                            match pixel.0 {
                                [_, _, _, 0] => None,
                                [0, 0, 0, 255] => Some(Connection::Some),
                                [255, 255, 255, 255] => Some(Connection::None),
                                [0, 255, 0, 255] => Some(Connection::Any),
                                _ => None,
                            }
                        });
                        let mut connections = [Connection::None; 8];
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

impl Connection {
    fn matches(&self, connection: bool) -> bool {
        match self {
            Self::Some => connection,
            Self::None => !connection,
            Self::Any => true,
        }
    }
}

impl geng::LoadAsset for TileSet {
    fn load(geng: &Geng, path: &std::path::Path) -> geng::AssetFuture<Self> {
        let geng = geng.clone();
        let path = path.to_owned();
        async move {
            let texture = ugli::Texture::load(&geng, &path).await?;
            let path = path.parent().unwrap().join("config.json");
            let config = TileSetConfig::load(&geng, &path).await?;
            Ok(Self::new(texture, config))
        }
        .boxed_local()
    }

    const DEFAULT_EXT: Option<&'static str> = ugli::Texture::DEFAULT_EXT;
}

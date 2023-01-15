use super::*;

const SIZE: Vec2<usize> = vec2(7, 7);

pub struct TileSet(ugli::Texture);

impl TileSet {
    pub fn new(mut texture: ugli::Texture) -> Self {
        texture.set_filter(ugli::Filter::Nearest);
        Self(texture)
    }

    pub fn texture(&self) -> &ugli::Texture {
        &self.0
    }

    pub fn get_tile_uv(&self, index: usize) -> [Vec2<f32>; 4] {
        let tile_size = SIZE.map(|x| 1.0 / x as f32);
        let pos = vec2(index % SIZE.x, index / SIZE.x);
        let pos = pos.map(|x| x as f32) * tile_size;
        [
            pos,
            pos + vec2(tile_size.x, 0.0),
            pos + tile_size,
            pos + vec2(0.0, tile_size.y),
        ]
    }

    pub fn get_tile_connected(&self, connections: [bool; 8]) -> [Vec2<f32>; 4] {
        // [bottom, right, top, left]
        let (x, y) = match connections {
            [false, _, true, _, false, _, false, _] => (0, 0),
            [false, _, true, _, false, _, true, _] => (1, 0),
            [false, _, false, _, false, _, true, _] => (2, 0),

            [false, _, false, _, false, _, false, _] => (3, 0),

            [false, _, false, _, true, _, false, _] => (3, 1),
            [true, _, false, _, true, _, false, _] => (3, 2),
            [true, _, false, _, false, _, false, _] => (3, 3),

            [false, _, true, true, true, _, false, _] => (0, 1),
            [true, true, true, true, true, _, false, _] => (0, 2),
            [true, true, true, _, false, _, false, _] => (0, 3),
            [false, _, true, true, true, true, true, _] => (1, 1),
            [true, true, true, true, true, true, true, true] => (1, 2),
            [true, true, true, _, false, _, true, true] => (1, 3),
            [false, _, false, _, true, true, true, _] => (2, 1),
            [true, _, false, _, true, true, true, true] => (2, 2),
            [true, _, false, _, false, _, true, true] => (2, 3),

            [true, true, true, false, true, _, false, _] => (0, 5),
            [true, false, true, true, true, _, false, _] => (0, 6),
            [true, _, false, _, true, false, true, true] => (1, 5),
            [true, _, false, _, true, true, true, false] => (1, 6),

            [false, _, true, false, true, true, true, _] => (2, 5),
            [true, false, true, _, false, _, true, true] => (2, 6),
            [false, _, true, true, true, false, true, _] => (3, 5),
            [true, true, true, _, false, _, true, false] => (3, 6),

            [true, true, true, false, true, true, true, false] => (2, 4),
            [true, false, true, true, true, false, true, true] => (3, 4),

            [false, _, true, false, true, _, false, _] => (4, 4),
            [true, false, true, false, true, _, false, _] => (4, 5),
            [true, false, true, _, false, _, false, _] => (4, 6),
            [false, _, true, false, true, false, true, _] => (5, 4),
            [true, false, true, false, true, false, true, false] => (5, 5),
            [true, false, true, _, false, _, true, false] => (5, 6),
            [false, _, false, _, true, false, true, _] => (6, 4),
            [true, _, false, _, true, false, true, false] => (6, 5),
            [true, _, false, _, false, _, true, false] => (6, 6),

            [true, false, true, false, true, true, true, false] => (4, 0),
            [true, false, true, false, true, false, true, true] => (4, 1),
            [true, true, true, false, true, true, true, true] => (4, 2),
            [true, false, true, true, true, true, true, true] => (4, 3),
            [true, true, true, false, true, false, true, false] => (5, 0),
            [true, false, true, true, true, false, true, false] => (5, 1),
            [true, true, true, true, true, false, true, true] => (5, 2),
            [true, true, true, true, true, true, true, false] => (5, 3),
            [true, false, true, true, true, true, true, false] => (6, 0),
            [true, false, true, false, true, true, true, true] => (6, 1),
            [true, true, true, true, true, false, true, false] => (6, 2),
            [true, true, true, false, true, false, true, true] => (6, 3),
        };
        let index = x + y * SIZE.x;
        self.get_tile_uv(index)
    }
}

impl geng::LoadAsset for TileSet {
    fn load(geng: &Geng, path: &std::path::Path) -> geng::AssetFuture<Self> {
        Box::pin(ugli::Texture::load(geng, path).map(|texture| Ok(Self::new(texture?))))
    }

    const DEFAULT_EXT: Option<&'static str> = ugli::Texture::DEFAULT_EXT;
}

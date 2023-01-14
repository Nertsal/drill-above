use super::*;

const SIZE: Vec2<usize> = vec2(4, 4);

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

    pub fn get_tile_connected(&self, connections: [bool; 4]) -> [Vec2<f32>; 4] {
        // [bottom, right, top, left]
        let (x, y) = match connections {
            [false, false, false, false] => (3, 0),
            [false, false, false, true] => (2, 0),
            [false, false, true, false] => (3, 1),
            [false, false, true, true] => (2, 1),
            [false, true, false, false] => (0, 0),
            [false, true, false, true] => (1, 0),
            [false, true, true, false] => (0, 1),
            [false, true, true, true] => (1, 1),
            [true, false, false, false] => (3, 3),
            [true, false, false, true] => (2, 3),
            [true, false, true, false] => (3, 2),
            [true, false, true, true] => (2, 2),
            [true, true, false, false] => (0, 3),
            [true, true, false, true] => (1, 3),
            [true, true, true, false] => (0, 2),
            [true, true, true, true] => (1, 2),
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

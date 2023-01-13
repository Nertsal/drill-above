use super::*;

#[derive(geng::Assets)]
pub struct Assets {
    pub shaders: Shaders,
    pub sprites: Sprites,
    pub rules: Rules,
}

#[derive(geng::Assets)]
pub struct Shaders {
    pub grid: ugli::Program,
}

#[derive(geng::Assets)]
pub struct Sprites {
    pub tiles: TileSprites,
}

#[derive(geng::Assets)]
pub struct TileSprites {
    pub air: ugli::Texture,
    pub grass: ugli::Texture,
    pub stone: ugli::Texture,
}

impl TileSprites {
    pub fn get_texture(&self, tile: &Tile) -> &ugli::Texture {
        match tile {
            Tile::Air => &self.air,
            Tile::Grass => &self.grass,
            Tile::Stone => &self.stone,
        }
    }
}

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
    pub hazards: HazardSprites,
    #[asset(postprocess = "pixel")]
    pub room: ugli::Texture,
}

#[derive(geng::Assets)]
pub struct TileSprites {
    #[asset(postprocess = "pixel")]
    pub air: ugli::Texture,
    #[asset(postprocess = "pixel")]
    pub grass: ugli::Texture,
    #[asset(postprocess = "pixel")]
    pub stone: ugli::Texture,
}

#[derive(geng::Assets)]
pub struct HazardSprites {
    #[asset(postprocess = "pixel")]
    pub spikes: ugli::Texture,
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

impl HazardSprites {
    pub fn get_texture(&self, hazard: &HazardType) -> &ugli::Texture {
        match hazard {
            HazardType::Spikes => &self.spikes,
        }
    }
}

fn pixel(texture: &mut ugli::Texture) {
    texture.set_filter(ugli::Filter::Nearest)
}

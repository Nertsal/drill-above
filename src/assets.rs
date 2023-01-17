use super::*;

#[derive(geng::Assets)]
pub struct Assets {
    pub shaders: Shaders,
    pub sprites: Sprites,
    pub sounds: Sounds,
    pub intro: Animation,
    pub rules: Rules,
}

#[derive(geng::Assets)]
pub struct Shaders {
    pub texture: ugli::Program,
    pub grid: ugli::Program,
}

#[derive(geng::Assets)]
pub struct Sounds {
    pub jump: geng::Sound,
    pub death: geng::Sound,
    pub coin: geng::Sound,
    #[asset(postprocess = "loop_sound")]
    pub drill: geng::Sound,
}

#[derive(geng::Assets)]
pub struct Sprites {
    pub tiles: TileSprites,
    pub hazards: HazardSprites,
    pub player: PlayerSprites,
    #[asset(postprocess = "pixel")]
    pub room: ugli::Texture,
    #[asset(postprocess = "pixel")]
    pub coin: ugli::Texture,
    #[asset(postprocess = "pixel")]
    pub heart4: ugli::Texture,
    #[asset(postprocess = "pixel")]
    pub heart8: ugli::Texture,
    pub background: TileSet<1, 4>,
}

#[derive(geng::Assets)]
pub struct TileSprites {
    pub air: TileSet<7, 7>,
    pub grass: TileSet<7, 7>,
    pub stone: TileSet<7, 7>,
}

#[derive(geng::Assets)]
pub struct HazardSprites {
    #[asset(postprocess = "pixel")]
    pub spikes: ugli::Texture,
}

#[derive(geng::Assets)]
pub struct PlayerSprites {
    #[asset(postprocess = "pixel")]
    pub idle0: ugli::Texture,
    #[asset(postprocess = "pixel")]
    pub slide0: ugli::Texture,
    pub drill: DrillSprites,
}

#[derive(geng::Assets)]
pub struct DrillSprites {
    #[asset(postprocess = "pixel")]
    pub drill_v0: ugli::Texture,
    #[asset(postprocess = "pixel")]
    pub drill_d0: ugli::Texture,
}

#[derive(Deref)]
pub struct Animation {
    #[deref]
    pub frames: Vec<ugli::Texture>,
}

impl TileSprites {
    pub fn get_tile_set(&self, tile: &Tile) -> &TileSet<7, 7> {
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

fn loop_sound(sound: &mut geng::Sound) {
    sound.looped = true;
}

impl Animation {
    pub fn get_frame(&self, time: Time) -> Option<&ugli::Texture> {
        let i = (time.as_f32() * self.frames.len() as f32).floor() as usize;
        self.frames.get(i)
    }
}

impl geng::LoadAsset for Animation {
    fn load(geng: &Geng, path: &std::path::Path) -> geng::AssetFuture<Self> {
        let data = <Vec<u8> as geng::LoadAsset>::load(geng, path);
        let geng = geng.clone();
        async move {
            let data = data.await?;
            use image::AnimationDecoder;
            Ok(Self {
                frames: image::codecs::gif::GifDecoder::new(data.as_slice())
                    .unwrap()
                    .into_frames()
                    .map(|frame| {
                        let frame = frame.unwrap();
                        ugli::Texture::from_image_image(geng.ugli(), frame.into_buffer())
                    })
                    .collect(),
            })
        }
        .boxed_local()
    }
    const DEFAULT_EXT: Option<&'static str> = Some("gif");
}

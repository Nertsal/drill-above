use super::*;

#[derive(geng::Assets)]
pub struct Assets {
    #[asset(load_with = "load_font(&geng, &base_path.join(\"pixel.ttf\"))")]
    pub font: Rc<geng::Font>,
    pub shaders: Shaders,
    pub sprites: Sprites,
    pub sounds: Sounds,
    #[asset(postprocess = "loop_sound")]
    pub music: geng::Sound,
    pub rules: Rules,
}

#[derive(geng::Assets)]
pub struct Shaders {
    pub texture: ugli::Program,
    pub texture_mask: ugli::Program,
    pub grid: ugli::Program,
    pub global_light: ugli::Program,
    pub spotlight: ugli::Program,
    pub point_light_shadow_map: ugli::Program,
    pub shadow_remove: ugli::Program,
    pub normal_map: ugli::Program,
    pub normal_texture: ugli::Program,
}

#[derive(geng::Assets)]
pub struct Sounds {
    pub jump: geng::Sound,
    pub death: geng::Sound,
    pub coin: geng::Sound,
    #[asset(postprocess = "loop_sound")]
    pub drill: geng::Sound,
    pub drill_jump: geng::Sound,
    pub charm: geng::Sound,
    #[asset(path = "cutscene.mp3")]
    pub cutscene: geng::Sound,
}

#[derive(geng::Assets)]
pub struct Sprites {
    pub tiles: TileSprites,
    pub hazards: SpriteCollection,
    pub player: PlayerSprites,
    pub props: SpriteCollection,
    #[asset(postprocess = "pixel")]
    pub partner: ugli::Texture,
    #[asset(postprocess = "pixel")]
    pub room: ugli::Texture,
    #[asset(postprocess = "pixel")]
    pub coin: ugli::Texture,
    #[asset(postprocess = "pixel")]
    pub heart4: ugli::Texture,
    #[asset(postprocess = "pixel")]
    pub heart8: ugli::Texture,
    #[asset(postprocess = "pixel")]
    pub background: ugli::Texture,
    #[asset(postprocess = "pixel")]
    pub sun: ugli::Texture,
    #[asset(postprocess = "pixel")]
    pub skull: ugli::Texture,
    #[asset(postprocess = "pixel")]
    pub drill_hover: ugli::Texture,
    #[asset(postprocess = "pixel")]
    pub cursor: ugli::Texture,
    #[asset(postprocess = "pixel")]
    pub spotlight: ugli::Texture,
}

pub struct TileSprites {
    pub mask: TileSet,
    pub tiles: HashMap<Tile, TileSet>,
}

pub struct SpriteCollection(pub HashMap<String, Texture>);

#[derive(geng::Assets)]
pub struct PlayerSprites {
    #[asset(postprocess = "pixel")]
    pub player: ugli::Texture,
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
pub struct Texture {
    #[deref]
    texture: Rc<ugli::Texture>,
    normal: Option<Rc<ugli::Texture>>,
}

#[derive(Deref)]
pub struct Animation {
    #[deref]
    pub frames: Vec<(ugli::Texture, f32)>,
}

impl TileSprites {
    pub fn get_tile_set(&self, tile: &Tile) -> &TileSet {
        self.tiles
            .get(tile)
            .unwrap_or_else(|| panic!("Failed to find tileset of {tile:?}"))
    }
}

impl SpriteCollection {
    pub fn get_texture(&self, name: &str) -> &Texture {
        self.0
            .get(&name.to_lowercase())
            .expect("Failed to find texture")
    }
}

fn pixel(texture: &mut ugli::Texture) {
    texture.set_filter(ugli::Filter::Nearest)
}

fn loop_sound(sound: &mut geng::Sound) {
    sound.looped = true;
}

// impl Animation {
//     pub fn get_frame(&self, time: Time) -> Option<&ugli::Texture> {
//         let i = (time.as_f32() * self.frames.len() as f32).floor() as usize;
//         self.frames.get(i)
//     }
// }

impl Texture {
    pub fn texture(&self) -> &ugli::Texture {
        self.texture.deref()
    }

    pub fn normal(&self) -> Option<&ugli::Texture> {
        self.normal.as_deref()
    }
}

impl geng::LoadAsset for Texture {
    fn load(geng: &Geng, path: &std::path::Path) -> geng::AssetFuture<Self> {
        let path = path.to_owned();
        let geng = geng.clone();

        async move {
            let mut texture = ugli::Texture::load(&geng, &path).await?;
            texture.set_filter(ugli::Filter::Nearest);
            let texture = Rc::new(texture);

            let name = path.file_stem().unwrap().to_str().unwrap();
            let normal_path = path.with_file_name(format!("{name}_normal.png"));
            let normal = util::report_warn(
                async {
                    let mut texture = ugli::Texture::load(&geng, &normal_path).await?;
                    texture.set_filter(ugli::Filter::Nearest);
                    Result::<_, anyhow::Error>::Ok(Rc::new(texture))
                }
                .await,
                format!("Failed to load normals for {name}"),
            )
            .ok();

            Ok(Self { texture, normal })
        }
        .boxed_local()
    }

    const DEFAULT_EXT: Option<&'static str> = Some("png");
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
                        let (n, d) = frame.delay().numer_denom_ms();
                        let mut texture =
                            ugli::Texture::from_image_image(geng.ugli(), frame.into_buffer());
                        texture.set_filter(ugli::Filter::Nearest);
                        (texture, n as f32 / d as f32 / 1000.0)
                    })
                    .collect(),
            })
        }
        .boxed_local()
    }
    const DEFAULT_EXT: Option<&'static str> = Some("gif");
}

impl geng::LoadAsset for TileSprites {
    fn load(geng: &Geng, path: &std::path::Path) -> geng::AssetFuture<Self> {
        let path = path.to_owned();
        let geng = geng.clone();
        async move {
            // Load the list of tiles from the rules
            let rules: Rules =
                geng::LoadAsset::load(&geng, &run_dir().join("assets").join("rules.json")).await?;

            // Load tiles
            let mut tiles = HashMap::with_capacity(rules.tiles.len());
            for tile in rules.tiles.into_keys() {
                let set: TileSet =
                    geng::LoadAsset::load(&geng, &path.join(format!("{tile}.png"))).await?;
                tiles.insert(tile, set);
            }

            // Load mask
            let mask: TileSet = geng::LoadAsset::load(&geng, &path.join("mask.png")).await?;

            Ok(Self { mask, tiles })
        }
        .boxed_local()
    }

    const DEFAULT_EXT: Option<&'static str> = None;
}

impl geng::LoadAsset for SpriteCollection {
    fn load(geng: &Geng, path: &std::path::Path) -> geng::AssetFuture<Self> {
        let path = path.to_owned();
        let geng = geng.clone();
        async move {
            // Load the list of textures
            let list: Vec<String> = file::load_json(path.join("_list.json"))
                .await
                .context("Failed to load _list.json")?;

            // Load tiles
            let mut textures = HashMap::with_capacity(list.len());
            for name in list {
                let texture: Texture =
                    geng::LoadAsset::load(&geng, &path.join(format!("{name}.png"))).await?;
                textures.insert(name, texture);
            }

            Ok(Self(textures))
        }
        .boxed_local()
    }

    const DEFAULT_EXT: Option<&'static str> = None;
}

fn load_font(geng: &Geng, path: &std::path::Path) -> geng::AssetFuture<Rc<geng::Font>> {
    let geng = geng.clone();
    let path = path.to_owned();
    async move {
        let data = <Vec<u8> as geng::LoadAsset>::load(&geng, &path).await?;
        Ok(Rc::new(geng::Font::new(
            &geng,
            &data,
            geng::ttf::Options {
                pixel_size: 64.0,
                max_distance: 0.1,
            },
        )?))
    }
    .boxed_local()
}

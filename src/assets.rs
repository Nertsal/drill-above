use super::*;

#[derive(geng::asset::Load)]
pub struct Assets {
    pub fonts: Fonts,
    pub shaders: Shaders,
    pub sprites: Sprites,
    pub sounds: Sounds,
    #[load(postprocess = "loop_sound")]
    pub music: geng::Sound,
    pub rules: Rules,
}

#[derive(geng::asset::Load)]
pub struct Fonts {
    pub pixel: Rc<geng::Font>,
    pub dialogue: Rc<geng::Font>,
}

#[derive(geng::asset::Load)]
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

#[derive(geng::asset::Load)]
pub struct Sounds {
    pub jump: geng::Sound,
    pub death: geng::Sound,
    pub coin: geng::Sound,
    #[load(postprocess = "loop_sound")]
    pub drill: geng::Sound,
    pub drill_jump: geng::Sound,
}

#[derive(geng::asset::Load)]
pub struct Sprites {
    pub tiles: TileSprites,
    pub hazards: SpriteCollection,
    pub player: PlayerSprites,
    pub props: SpriteCollection,
    pub npc: SpriteCollection,
    #[load(postprocess = "pixel")]
    pub partner: ugli::Texture,
    #[load(postprocess = "pixel")]
    pub room: ugli::Texture,
    #[load(postprocess = "pixel")]
    pub coin: ugli::Texture,
    #[load(postprocess = "pixel")]
    pub background: ugli::Texture,
    #[load(postprocess = "pixel")]
    pub sun: ugli::Texture,
    #[load(postprocess = "pixel")]
    pub skull: ugli::Texture,
    #[load(postprocess = "pixel")]
    pub drill_hover: ugli::Texture,
    #[load(postprocess = "pixel")]
    pub cursor: ugli::Texture,
    #[load(postprocess = "pixel")]
    pub spotlight: ugli::Texture,
}

pub struct TileSprites {
    pub mask: TileSet,
    pub tiles: HashMap<Tile, TileSet>,
}

pub struct SpriteCollection(pub HashMap<String, Texture>);

#[derive(geng::asset::Load)]
pub struct PlayerSprites {
    #[load(postprocess = "pixel")]
    pub player: ugli::Texture,
    #[load(postprocess = "pixel")]
    pub idle0: ugli::Texture,
    #[load(postprocess = "pixel")]
    pub slide0: ugli::Texture,
    pub drill: DrillSprites,
}

#[derive(geng::asset::Load)]
pub struct DrillSprites {
    #[load(postprocess = "pixel")]
    pub drill_v0: ugli::Texture,
    #[load(postprocess = "pixel")]
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
    sound.set_looped(true);
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

impl geng::asset::Load for Texture {
    fn load(
        manager: &geng::asset::Manager,
        path: &std::path::Path,
        (): &(),
    ) -> geng::asset::Future<Self> {
        let path = path.to_owned();
        let manager = manager.clone();

        async move {
            let mut texture =
                ugli::Texture::load(&manager, &path, &geng::asset::TextureOptions::default())
                    .await?;
            texture.set_filter(ugli::Filter::Nearest);
            let texture = Rc::new(texture);

            let name = path.file_stem().unwrap().to_str().unwrap();
            let normal_path = path.with_file_name(format!("{name}_normal.png"));
            let normal = util::report_warn(
                async {
                    let mut texture = ugli::Texture::load(
                        &manager,
                        &normal_path,
                        &geng::asset::TextureOptions::default(),
                    )
                    .await?;
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

    type Options = ();
}

impl geng::asset::Load for Animation {
    fn load(
        manager: &geng::asset::Manager,
        path: &std::path::Path,
        (): &(),
    ) -> geng::asset::Future<Self> {
        let manager = manager.clone();
        let path = path.to_owned();
        async move {
            let frames = geng_utils::gif::load_gif(
                &manager,
                &path,
                geng_utils::gif::GifOptions {
                    frame: geng::asset::TextureOptions {
                        filter: ugli::Filter::Nearest,
                        ..Default::default()
                    },
                },
            )
            .await?;

            Ok(Self {
                frames: frames
                    .into_iter()
                    .map(|frame| (frame.texture, frame.duration))
                    .collect(),
            })
        }
        .boxed_local()
    }
    const DEFAULT_EXT: Option<&'static str> = Some("gif");

    type Options = ();
}

impl geng::asset::Load for TileSprites {
    fn load(
        manager: &geng::asset::Manager,
        path: &std::path::Path,
        (): &(),
    ) -> geng::asset::Future<Self> {
        let path = path.to_owned();
        let manager = manager.clone();
        async move {
            // Load the list of tiles from the rules
            let rules: Rules = geng::asset::Load::load(
                &manager,
                &run_dir().join("assets").join("rules.json"),
                &(),
            )
            .await?;

            // Load tiles
            let mut tiles = HashMap::with_capacity(rules.tiles.len());
            for tile in rules.tiles.into_keys() {
                let set: TileSet =
                    geng::asset::Load::load(&manager, &path.join(format!("{tile}.png")), &())
                        .await?;
                tiles.insert(tile, set);
            }

            // Load mask
            let mask: TileSet =
                geng::asset::Load::load(&manager, &path.join("mask.png"), &()).await?;

            Ok(Self { mask, tiles })
        }
        .boxed_local()
    }

    const DEFAULT_EXT: Option<&'static str> = None;

    type Options = ();
}

impl geng::asset::Load for SpriteCollection {
    fn load(
        manager: &geng::asset::Manager,
        path: &std::path::Path,
        (): &(),
    ) -> geng::asset::Future<Self> {
        let path = path.to_owned();
        let manager = manager.clone();
        async move {
            // Load the list of textures
            let list: Vec<String> = file::load_json(path.join("_list.json"))
                .await
                .context("Failed to load _list.json")?;

            // Load tiles
            let mut textures = HashMap::with_capacity(list.len());
            for name in list {
                let texture: Texture =
                    geng::asset::Load::load(&manager, &path.join(format!("{name}.png")), &())
                        .await?;
                textures.insert(name, texture);
            }

            Ok(Self(textures))
        }
        .boxed_local()
    }

    const DEFAULT_EXT: Option<&'static str> = None;

    type Options = ();
}

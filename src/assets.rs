use super::*;

#[derive(geng::Assets)]
pub struct Assets {
    pub shaders: Shaders,
}

#[derive(geng::Assets)]
pub struct Shaders {
    pub grid: ugli::Program,
}

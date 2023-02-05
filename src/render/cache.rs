use super::*;

pub struct RenderCache {
    pub geometry: (
        HashMap<Tile, ugli::VertexBuffer<Vertex>>,
        HashMap<Tile, ugli::VertexBuffer<MaskedVertex>>,
    ),
    pub light_geometry: ugli::VertexBuffer<NormalVertex>,
    pub normal_geometry: ugli::VertexBuffer<NormalVertex>,
    pub normal_uv: HashMap<Tile, ugli::VertexBuffer<Vertex>>,
}

impl RenderCache {
    pub fn calculate(level: &Level, geng: &Geng, assets: &Assets) -> Self {
        let (normal_geometry, normal_uv) = level.calculate_normal_geometry(geng, assets);
        Self {
            geometry: level.tiles.calculate_geometry(&level.grid, geng, assets),
            light_geometry: level.calculate_light_geometry(geng),
            normal_geometry,
            normal_uv,
        }
    }
}

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
    pub fn calculate(room: &Room, geng: &Geng, assets: &Assets) -> Self {
        let (normal_geometry, normal_uv) = room.calculate_normal_geometry(geng, assets);
        Self {
            geometry: room.tiles.calculate_geometry(&room.grid, geng, assets),
            light_geometry: room.calculate_light_geometry(geng, assets),
            normal_geometry,
            normal_uv,
        }
    }
}

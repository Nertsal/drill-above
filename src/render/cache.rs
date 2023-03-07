use super::*;

pub type TilesGeometry = (
    HashMap<Tile, ugli::VertexBuffer<Vertex>>,
    HashMap<Tile, ugli::VertexBuffer<MaskedVertex>>,
);

pub struct RenderCache {
    pub background_geometry: TilesGeometry,
    pub main_geometry: TilesGeometry,
    pub foreground_geometry: TilesGeometry,
    pub light_geometry: ugli::VertexBuffer<NormalVertex>,
    pub normal_geometry: ugli::VertexBuffer<NormalVertex>,
    pub normal_uv: HashMap<Tile, ugli::VertexBuffer<Vertex>>,
}

impl RenderCache {
    pub fn calculate(room: &Room, geng: &Geng, assets: &Assets) -> Self {
        let (normal_geometry, normal_uv) = room
            .layers
            .main
            .calculate_normal_geometry(&room.grid, geng, assets);
        Self {
            background_geometry: room
                .layers
                .background
                .tiles
                .calculate_geometry(&room.grid, geng, assets),
            main_geometry: room
                .layers
                .main
                .tiles
                .calculate_geometry(&room.grid, geng, assets),
            foreground_geometry: room
                .layers
                .foreground
                .tiles
                .calculate_geometry(&room.grid, geng, assets),
            light_geometry: room
                .layers
                .main
                .calculate_light_geometry(&room.grid, geng, assets),
            normal_geometry,
            normal_uv,
        }
    }
}

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
            .main_layer
            .calculate_normal_geometry(&room.grid, geng, assets);
        Self {
            background_geometry: room
                .background_layer
                .tiles
                .calculate_geometry(&room.grid, geng, assets),
            main_geometry: room
                .main_layer
                .tiles
                .calculate_geometry(&room.grid, geng, assets),
            foreground_geometry: room
                .foreground_layer
                .tiles
                .calculate_geometry(&room.grid, geng, assets),
            light_geometry: room
                .main_layer
                .calculate_light_geometry(&room.grid, geng, assets),
            normal_geometry,
            normal_uv,
        }
    }
}

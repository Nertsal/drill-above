use super::*;

#[derive(Debug, Clone, PartialEq)]
pub enum PlaceableType {
    Tile(Tile),
    Hazard(HazardType),
    Prop(PropType),
    Coin,
    Npc(NpcType),
    Spotlight(SpotlightSource),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlaceableId {
    Tile(vec2<isize>),
    Hazard(usize),
    Prop(usize),
    Coin(usize),
    Npc(usize),
    Spotlight(usize),
}

#[derive(Debug, Clone)]
pub enum Placeable {
    Tile((Tile, vec2<isize>)),
    Hazard(Hazard),
    Prop(Prop),
    Coin(Coin),
    Npc(Npc),
    Spotlight(SpotlightSource),
}

#[derive(Debug)]
pub enum PlaceableMut<'a> {
    Tile((Tile, vec2<isize>)),
    Hazard(&'a mut Hazard),
    Prop(&'a mut Prop),
    Coin(&'a mut Coin),
    Npc(&'a mut Npc),
    Spotlight(&'a mut SpotlightSource),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coin {
    pub collider: Collider,
    pub collected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hazard {
    pub sprite: Sprite,
    pub direction: Option<vec2<Coord>>,
    pub collider: Collider,
    pub hazard_type: HazardType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prop {
    pub sprite: Sprite,
    pub prop_type: PropType,
}

pub type HazardType = String;
pub type PropType = String;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Sprite {
    pub pos: Aabb2<Coord>,
    pub mirror_x: bool,
    pub mirror_y: bool,
}

impl PlaceableId {
    pub fn fits_type(&self, ty: &PlaceableType) -> bool {
        matches!(
            (self, ty),
            (PlaceableId::Tile(_), PlaceableType::Tile(_))
                | (PlaceableId::Hazard(_), PlaceableType::Hazard(_))
                | (PlaceableId::Prop(_), PlaceableType::Prop(_))
                | (PlaceableId::Coin(_), PlaceableType::Coin)
                | (PlaceableId::Npc(_), PlaceableType::Npc(_))
                | (PlaceableId::Spotlight(_), PlaceableType::Spotlight(_))
        )
    }
}

impl Placeable {
    pub fn bottom_left(&self, grid: &Grid) -> vec2<Coord> {
        match self {
            Placeable::Tile((_, pos)) => grid.grid_to_world(*pos),
            Placeable::Hazard(hazard) => hazard.collider.raw().bottom_left(),
            Placeable::Prop(prop) => prop.sprite.pos.bottom_left(),
            Placeable::Coin(coin) => coin.collider.raw().bottom_left(),
            Placeable::Npc(npc) => npc.sprite.pos.bottom_left(),
            Placeable::Spotlight(light) => light.position,
        }
    }

    pub fn translate(&mut self, offset: vec2<Coord>, grid: &Grid) {
        match self {
            Placeable::Tile((_, pos)) => {
                let offset = (offset / grid.cell_size).map(|x| x.round().as_f32() as isize);
                *pos += offset;
            }
            Placeable::Hazard(hazard) => hazard.translate(offset),
            Placeable::Prop(prop) => prop.translate(offset),
            Placeable::Coin(coin) => coin.translate(offset),
            Placeable::Npc(npc) => npc.translate(offset),
            Placeable::Spotlight(light) => light.position += offset,
        }
    }

    pub fn get_type(&self) -> PlaceableType {
        match self {
            Placeable::Tile((tile, _)) => PlaceableType::Tile(tile.to_owned()),
            Placeable::Hazard(hazard) => PlaceableType::Hazard(hazard.hazard_type.to_owned()),
            Placeable::Prop(prop) => PlaceableType::Prop(prop.prop_type.to_owned()),
            Placeable::Coin(_) => PlaceableType::Coin,
            Placeable::Npc(npc) => PlaceableType::Npc(npc.npc_type.to_owned()),
            Placeable::Spotlight(spotlight) => PlaceableType::Spotlight(*spotlight),
        }
    }

    pub fn sprite(&self, grid: &Grid) -> Aabb2<Coord> {
        match self {
            Placeable::Tile((_, pos)) => {
                let collider = grid.cell_collider(*pos);
                collider.raw()
            }
            Placeable::Hazard(hazard) => hazard.collider.raw(),
            Placeable::Prop(prop) => prop.sprite.pos,
            Placeable::Coin(coin) => coin.collider.raw(),
            Placeable::Npc(npc) => npc.sprite.pos,
            Placeable::Spotlight(light) => {
                Aabb2::point(light.position).extend_uniform(Coord::new(0.5))
            }
        }
    }
}

impl Hazard {
    pub fn translate(&mut self, delta: vec2<Coord>) {
        self.sprite.translate(delta);
        self.collider.translate(delta);
    }

    pub fn teleport(&mut self, pos: vec2<Coord>) {
        self.translate(pos - self.collider.pos())
    }
}

impl Coin {
    pub fn translate(&mut self, delta: vec2<Coord>) {
        self.collider.translate(delta);
    }

    pub fn teleport(&mut self, pos: vec2<Coord>) {
        self.translate(pos - self.collider.pos())
    }
}

impl Prop {
    pub fn translate(&mut self, delta: vec2<Coord>) {
        self.sprite.translate(delta);
    }

    pub fn teleport(&mut self, pos: vec2<Coord>) {
        self.translate(pos - self.sprite.pos.center())
    }
}

impl Sprite {
    pub fn new(pos: Aabb2<Coord>) -> Self {
        Self {
            pos,
            mirror_x: false,
            mirror_y: false,
        }
    }

    pub fn translate(&mut self, delta: vec2<Coord>) {
        self.pos = self.pos.translate(delta);
    }

    pub fn render_aabb(&self) -> Aabb2<f32> {
        let mut aabb = self.pos;
        if self.mirror_x {
            std::mem::swap(&mut aabb.min.x, &mut aabb.max.x);
        }
        if self.mirror_y {
            std::mem::swap(&mut aabb.min.y, &mut aabb.max.y);
        }
        let pos = util::pixel_perfect_pos(aabb.min);
        let aabb = aabb.map(Coord::as_f32);
        aabb.translate(pos - aabb.min)
    }
}

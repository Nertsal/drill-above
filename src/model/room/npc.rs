use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Npc {
    pub interact_collider: Collider,
    pub text: text_scroller::TextConfig,
    pub sprite: Sprite,
    pub npc_type: NpcType,
}

pub type NpcType = String;

impl Npc {
    pub fn translate(&mut self, delta: vec2<Coord>) {
        self.interact_collider.translate(delta);
        self.sprite.translate(delta);
    }
}

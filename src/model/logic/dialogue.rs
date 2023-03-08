use super::*;

impl Logic<'_> {
    pub fn dialogue(&mut self) {
        self.world.dialogue = if let Some(dialogue) = self.world.dialogue.take() {
            self.update_dialogue(dialogue)
        } else {
            self.start_dialogue()
        };
    }

    fn update_dialogue(&self, mut dialogue: Dialogue) -> Option<Dialogue> {
        let player = self
            .world
            .actors
            .get(&self.world.player.id)
            .unwrap()
            .collider;
        if !player.check(&dialogue.collider) {
            return None;
        }

        dialogue.update(self.delta_time);
        Some(dialogue)
    }

    fn start_dialogue(&self) -> Option<Dialogue> {
        let player = self
            .world
            .actors
            .get(&self.world.player.id)
            .unwrap()
            .collider;
        for npc in &self.world.room.npcs {
            if player.check(&npc.interact_collider) {
                return Some(Dialogue::new(npc.text.clone(), npc.interact_collider));
            }
        }

        None
    }
}

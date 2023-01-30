use super::*;

mod movement;
mod particles;
mod player;

pub use movement::*;
use particles::*;

pub struct Logic<'a> {
    world: &'a mut World,
    player_control: PlayerControl,
    delta_time: Time,
}

impl World {
    pub fn update(&mut self, player_control: PlayerControl, delta_time: Time) {
        let mut logic = Logic {
            world: self,
            player_control,
            delta_time,
        };
        logic.process();
    }
}

impl Logic<'_> {
    fn process(&mut self) {
        if !matches!(self.world.player.state, PlayerState::Finished { .. }) {
            self.world.time += self.delta_time;
        }

        self.process_player();
        self.movement();
        self.process_particles();
        self.process_camera();
    }

    fn next_level(&mut self) {
        if let Some(level) = self.world.level.next_level.clone() {
            self.world.level_transition = Some(level);
        } else {
            self.world.level_transition = Some("credits.json".to_string());
        }
    }

    fn movement(&mut self) {
        self.player_movement();
    }

    fn process_camera(&mut self) {
        let camera_bounds = self.world.camera_bounds();
        let actor = self.world.actors.get(&self.world.player.id).unwrap();
        let target = actor.collider.pos();
        let target = target.clamp_aabb(camera_bounds);
        let pos = target.map(Coord::as_f32);
        let pixel = (pos.map(|x| (x * PIXELS_PER_UNIT).round())) / PIXELS_PER_UNIT;
        self.world.camera.center = pixel;
    }
}

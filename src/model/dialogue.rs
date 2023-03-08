use super::*;

use text_scroller::*;

#[derive(Debug, Clone)]
pub struct Dialogue {
    pub scroller: TextScroller,
    pub text: String,
    pub collider: Collider,
}

impl Dialogue {
    pub fn new(config: TextConfig, collider: Collider) -> Self {
        Self {
            scroller: TextScroller::new(config),
            text: String::new(),
            collider,
        }
    }

    pub fn update(&mut self, delta_time: Time) {
        for event in self.scroller.update(delta_time.as_f32().into()) {
            match event {
                TextEvent::Push(str) => self.text += &str,
            }
        }
    }
}

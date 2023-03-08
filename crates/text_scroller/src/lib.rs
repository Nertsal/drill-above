use std::collections::VecDeque;

use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextScroller {
    config: TextConfig,
    queue: VecDeque<String>,
    next_char_time: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextConfig {
    pub char_delay: f64,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TextEvent {
    /// Append to the end of the text.
    Push(String),
    // Custom(String),
}

impl TextScroller {
    pub fn new(text: TextConfig) -> Self {
        // TODO: parse inline text markers
        Self {
            queue: UnicodeSegmentation::graphemes(text.text.as_str(), true)
                .map(|c| c.to_owned())
                .collect(),
            config: text,
            next_char_time: 0.0,
        }
    }

    pub fn update(&mut self, delta_time: f64) -> Vec<TextEvent> {
        let mut events = vec![];

        self.next_char_time -= delta_time;
        while self.next_char_time <= 0.0 {
            self.next_char_time += self.config.char_delay;
            let Some(next) = self.queue.pop_front() else {
                break;
            };
            events.push(TextEvent::Push(next))
        }

        events
    }
}

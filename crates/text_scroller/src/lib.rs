use std::collections::VecDeque;

use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation;

mod event;

pub use event::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextScroller {
    config: TextConfig,
    state: ScrollerState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextConfig {
    pub char_delay: f64,
    pub events: VecDeque<TextEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum ScrollerState {
    Text { text: VecDeque<String>, next: f64 },
    Event { next: f64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScrollEvent {
    /// Append to the end of the text.
    Push(String),
    // Custom(String),
}

impl TextScroller {
    pub fn new(text: TextConfig) -> Self {
        Self {
            config: text,
            state: ScrollerState::Event { next: 0.0 },
        }
    }

    pub fn update(&mut self, delta_time: f64) -> Vec<ScrollEvent> {
        let mut events = vec![];

        match &mut self.state {
            ScrollerState::Text { text, next } => {
                *next -= delta_time;
                while *next <= 0.0 {
                    *next += self.config.char_delay;
                    let Some(next) = text.pop_front() else {
                        self.state = ScrollerState::Event { next: 0.0 };
                        break;
                    };
                    events.push(ScrollEvent::Push(next))
                }
            }
            ScrollerState::Event { next } => {
                *next -= delta_time;
                while *next <= 0.0 {
                    let Some(event) = self.config.events.pop_front() else {
                        break;
                    };
                    match event {
                        TextEvent::Text(text) => {
                            self.state = ScrollerState::Text {
                                text: UnicodeSegmentation::graphemes(text.as_str(), true)
                                    .map(|s| s.to_owned())
                                    .collect(),
                                next: *next,
                            };
                            break;
                        }
                        TextEvent::Delay(delay) => {
                            *next += delay;
                        }
                    }
                }
            }
        }

        events
    }
}

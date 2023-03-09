use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(from = "TextEventSerde", into = "TextEventSerde")]
pub enum TextEvent {
    Text(String),
    Delay(f64),
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum TextEventSerde {
    Text(String),
    Other(Event),
}

#[derive(Serialize, Deserialize)]
enum Event {
    Delay(f64),
}

impl From<TextEventSerde> for TextEvent {
    fn from(value: TextEventSerde) -> Self {
        match value {
            TextEventSerde::Text(text) => Self::Text(text),
            TextEventSerde::Other(event) => match event {
                Event::Delay(delay) => Self::Delay(delay),
            },
        }
    }
}

impl From<TextEvent> for TextEventSerde {
    fn from(value: TextEvent) -> Self {
        match value {
            TextEvent::Text(text) => Self::Text(text),
            TextEvent::Delay(delay) => Self::Other(Event::Delay(delay)),
        }
    }
}

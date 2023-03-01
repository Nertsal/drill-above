use super::*;

use geng::ui::*;

pub struct TextInput<'a> {
    geng: Geng,
    sense: &'a mut Sense,
    text: String,
    inner: Box<dyn Widget + 'a>,
    is_focused: &'a mut bool,
}

enum TextAction {
    Append(char),
    Pop,
}

impl<'a> TextInput<'a> {
    #[allow(clippy::unnecessary_to_owned)] // Appears to be a bug in clippy
    pub fn new(
        cx: &'a Controller,
        geng: &Geng,
        text: String,
        font: impl AsRef<geng::Font> + 'a,
        size: f32,
        mut color: Rgba<f32>,
    ) -> Self {
        let sense: &'a mut Sense = cx.get_state();
        let is_focused: &'a mut bool = cx.get_state();

        if *is_focused {
            color = Rgba::opaque(0.7, 0.7, 0.7)
        } else if sense.is_hovered() {
            color = Rgba::GRAY;
        }

        Self {
            geng: geng.clone(),
            sense,
            inner: geng::ui::Text::new(text.to_owned(), font, size, color).boxed(),
            text,
            is_focused,
        }
    }

    pub fn is_focused(&self) -> bool {
        *self.is_focused
    }

    pub fn get_text(&self) -> &String {
        &self.text
    }

    fn perform(&mut self, action: TextAction) {
        match action {
            TextAction::Append(c) => {
                self.text.push(c);
            }
            TextAction::Pop => {
                self.text.pop();
            }
        }
    }

    pub fn handle_input(&mut self, event: &geng::Event) {
        let action = match event {
            geng::Event::KeyDown { key } => {
                if let geng::Key::Escape | geng::Key::Enter = key {
                    *self.is_focused = false;
                }
                key_to_text_action(&self.geng, key)
            }
            geng::Event::MouseDown { .. } => {
                *self.is_focused = true;
                None
            }
            _ => None,
        };
        if let Some(action) = action {
            self.perform(action);
        }
    }
}

impl<'a> Widget for TextInput<'a> {
    fn calc_constraints(
        &mut self,
        children: &geng::ui::ConstraintsContext,
    ) -> geng::ui::Constraints {
        children.get_constraints(&self.inner)
    }

    fn sense(&mut self) -> Option<&mut geng::ui::Sense> {
        Some(self.sense)
    }

    fn handle_event(&mut self, event: &geng::Event) {
        self.handle_input(event)
    }

    fn walk_children_mut(&mut self, mut f: Box<dyn FnMut(&mut dyn Widget) + '_>) {
        f(&mut self.inner);
    }
}

fn key_to_text_action(geng: &Geng, key: &geng::Key) -> Option<TextAction> {
    let shift = geng.window().is_key_pressed(geng::Key::LShift);
    match key {
        geng::Key::Num0 => Some(TextAction::Append('0')),
        geng::Key::Num1 => Some(TextAction::Append('1')),
        geng::Key::Num2 => Some(TextAction::Append('2')),
        geng::Key::Num3 => Some(TextAction::Append('3')),
        geng::Key::Num4 => Some(TextAction::Append('4')),
        geng::Key::Num5 => Some(TextAction::Append('5')),
        geng::Key::Num6 => Some(TextAction::Append('6')),
        geng::Key::Num7 => Some(TextAction::Append('7')),
        geng::Key::Num8 => Some(TextAction::Append('8')),
        geng::Key::Num9 => Some(TextAction::Append('9')),
        geng::Key::A => Some(TextAction::Append(if shift { 'A' } else { 'a' })),
        geng::Key::B => Some(TextAction::Append(if shift { 'B' } else { 'b' })),
        geng::Key::C => Some(TextAction::Append(if shift { 'C' } else { 'c' })),
        geng::Key::D => Some(TextAction::Append(if shift { 'D' } else { 'd' })),
        geng::Key::E => Some(TextAction::Append(if shift { 'E' } else { 'e' })),
        geng::Key::F => Some(TextAction::Append(if shift { 'F' } else { 'f' })),
        geng::Key::G => Some(TextAction::Append(if shift { 'G' } else { 'g' })),
        geng::Key::H => Some(TextAction::Append(if shift { 'H' } else { 'h' })),
        geng::Key::I => Some(TextAction::Append(if shift { 'I' } else { 'i' })),
        geng::Key::J => Some(TextAction::Append(if shift { 'J' } else { 'j' })),
        geng::Key::K => Some(TextAction::Append(if shift { 'K' } else { 'k' })),
        geng::Key::L => Some(TextAction::Append(if shift { 'L' } else { 'l' })),
        geng::Key::M => Some(TextAction::Append(if shift { 'M' } else { 'm' })),
        geng::Key::N => Some(TextAction::Append(if shift { 'N' } else { 'n' })),
        geng::Key::O => Some(TextAction::Append(if shift { 'O' } else { 'o' })),
        geng::Key::P => Some(TextAction::Append(if shift { 'P' } else { 'p' })),
        geng::Key::Q => Some(TextAction::Append(if shift { 'Q' } else { 'q' })),
        geng::Key::R => Some(TextAction::Append(if shift { 'R' } else { 'r' })),
        geng::Key::S => Some(TextAction::Append(if shift { 'S' } else { 's' })),
        geng::Key::T => Some(TextAction::Append(if shift { 'T' } else { 't' })),
        geng::Key::U => Some(TextAction::Append(if shift { 'U' } else { 'u' })),
        geng::Key::V => Some(TextAction::Append(if shift { 'V' } else { 'v' })),
        geng::Key::W => Some(TextAction::Append(if shift { 'W' } else { 'w' })),
        geng::Key::X => Some(TextAction::Append(if shift { 'X' } else { 'x' })),
        geng::Key::Y => Some(TextAction::Append(if shift { 'Y' } else { 'y' })),
        geng::Key::Z => Some(TextAction::Append(if shift { 'Z' } else { 'z' })),
        geng::Key::Minus => Some(TextAction::Append(if shift { '_' } else { '-' })),
        geng::Key::Equals => Some(TextAction::Append(if shift { '+' } else { '=' })),
        geng::Key::Apostrophe => Some(TextAction::Append(if shift { '\"' } else { '\'' })),
        geng::Key::Semicolon => Some(TextAction::Append(if shift { ':' } else { ';' })),
        geng::Key::Grave => Some(TextAction::Append(if shift { '~' } else { '`' })),
        geng::Key::Comma => Some(TextAction::Append(',')),
        geng::Key::Period => Some(TextAction::Append('.')),
        geng::Key::Space => Some(TextAction::Append(' ')),
        geng::Key::Backspace => Some(TextAction::Pop),
        geng::Key::Delete => Some(TextAction::Pop),
        // geng::Key::Escape => todo!(),
        // geng::Key::Enter => todo!(),
        // geng::Key::Left => todo!(),
        // geng::Key::Right => todo!(),
        _ => None,
    }
}

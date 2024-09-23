use super::*;

use geng::ui::*;

pub struct CheckBox<'a> {
    sense: &'a mut Sense,
    check: bool,
    change: RefCell<&'a mut Option<bool>>,
}

impl<'a> CheckBox<'a> {
    pub fn new(cx: &'a Controller, check: bool) -> Self {
        Self {
            sense: cx.get_state(),
            change: RefCell::new(cx.get_state()),
            check,
        }
    }

    pub fn get_change(&self) -> Option<bool> {
        self.change.borrow_mut().take()
    }
}

impl<'a> Widget for CheckBox<'a> {
    fn calc_constraints(&mut self, _children: &ConstraintsContext) -> Constraints {
        Constraints {
            min_size: vec2(1.0, 1.0),
            flex: vec2(1.0, 0.0),
        }
    }

    fn sense(&mut self) -> Option<&mut Sense> {
        Some(self.sense)
    }

    fn handle_event(&mut self, _event: &geng::Event) {
        if self.sense.take_clicked() {
            **self.change.borrow_mut() = Some(!self.check);
        }
    }

    fn draw(&mut self, cx: &mut DrawContext) {
        let pos = cx.position.map(|x| x as f32).extend_uniform(-4.0);
        let pos = Aabb2::point(pos.center()).extend_uniform(pos.width().min(pos.height()) / 2.0);

        if self.sense.is_hovered() {
            cx.draw2d.draw2d(
                cx.framebuffer,
                &geng::PixelPerfectCamera,
                &draw2d::Quad::new(pos, Rgba::opaque(0.3, 0.3, 0.3)),
            );
        }

        if self.check {
            cx.draw2d.draw2d(
                cx.framebuffer,
                &geng::PixelPerfectCamera,
                &draw2d::Segment::new(
                    Segment(pos.bottom_left(), pos.top_right()),
                    4.0,
                    Rgba::opaque(0.0, 0.3, 0.5),
                ),
            );
            cx.draw2d.draw2d(
                cx.framebuffer,
                &geng::PixelPerfectCamera,
                &draw2d::Segment::new(
                    Segment(pos.top_left(), pos.bottom_right()),
                    4.0,
                    Rgba::opaque(0.0, 0.3, 0.5),
                ),
            );
        }

        cx.draw2d.draw2d(
            cx.framebuffer,
            &geng::PixelPerfectCamera,
            &draw2d::Chain::new(util::aabb_outline(pos), 2.0, Rgba::GRAY, 1),
        );
    }
}

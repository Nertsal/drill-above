use super::*;

use geng::ui::*;

pub struct PauseMenu {
    geng: Geng,
    assets: Rc<Assets>,
    state: MenuState,
    resume: bool,
}

enum MenuState {
    Paused,
    Rules,
}

impl PauseMenu {
    pub fn new(geng: &Geng, assets: &Rc<Assets>) -> Self {
        Self {
            geng: geng.clone(),
            assets: assets.clone(),
            state: MenuState::Paused,
            resume: false,
        }
    }

    pub fn pause(&mut self) {
        self.state = MenuState::Paused;
    }

    pub fn resume(&mut self) -> bool {
        std::mem::take(&mut self.resume)
    }

    pub fn ui<'a>(
        &'a mut self,
        world: &'a mut World,
        cx: &'a geng::ui::Controller,
    ) -> Box<dyn geng::ui::Widget + 'a> {
        match self.state {
            MenuState::Paused => Box::new(self.paused_ui(world, cx)),
            MenuState::Rules => Box::new(self.rules_ui(world, cx)),
        }
    }

    fn paused_ui<'a>(
        &'a mut self,
        world: &'a mut World,
        cx: &'a geng::ui::Controller,
    ) -> impl geng::ui::Widget + 'a {
        let resume = {
            let button = geng::ui::Button::new(cx, "Resume");
            if button.was_clicked() {
                self.resume = true;
            }
            button
        };

        let retry = {
            let button = geng::ui::Button::new(cx, "Retry");
            if button.was_clicked() {
                world.kill_player();
                self.resume = true;
            }
            button
        };

        let rules = {
            let button = geng::ui::Button::new(cx, "Rules");
            if button.was_clicked() {
                self.state = MenuState::Rules;
            }
            button
        };

        geng::ui::column![resume, retry, rules,].align(vec2(0.5, 0.5))
    }

    fn rules_ui<'a>(
        &'a mut self,
        world: &'a mut World,
        cx: &'a geng::ui::Controller,
    ) -> impl geng::ui::Widget + 'a {
        let back = {
            let button = geng::ui::Button::new(cx, "Back");
            if button.was_clicked() {
                self.state = MenuState::Paused;
            }
            button
        };

        geng::ui::column![back,].align(vec2(0.5, 0.5))
    }
}

use super::*;
use ui::ColorMode;

mod action;
mod draw;
mod level;
mod room;
mod ui_impl;

use level::*;
use room::*;

/// Run the editor.
pub fn run(geng: &Geng, room: Option<String>, hot_reload: bool) -> impl geng::State {
    let future = {
        let geng = geng.clone();
        async move {
            let assets: Rc<Assets> = geng::LoadAsset::load(&geng, &run_dir().join("assets"))
                .await
                .expect("Failed to load assets");
            LevelEditor::new(&geng, &assets, room, hot_reload)
        }
    };
    geng::LoadingScreen::new(geng, geng::EmptyLoadingScreen, future)
}

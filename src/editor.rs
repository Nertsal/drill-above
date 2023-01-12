use geng::Camera2d;

use super::*;

pub struct Editor {
    geng: Geng,
    assets: Rc<Assets>,
    render: Render,
    camera: Camera2d,
    framebuffer_size: Vec2<usize>,
    grid: Grid,
    draw_grid: bool,
    cursor_pos: Vec2<f64>,
    cursor_world_pos: Vec2<Coord>,
}

impl Editor {
    pub fn new(geng: &Geng, assets: &Rc<Assets>) -> Self {
        Self {
            geng: geng.clone(),
            assets: assets.clone(),
            render: Render::new(geng, assets),
            camera: Camera2d {
                center: Vec2::ZERO,
                rotation: 0.0,
                fov: 10.0,
            },
            framebuffer_size: vec2(1, 1),
            grid: Grid::default(),
            draw_grid: true,
            cursor_pos: Vec2::ZERO,
            cursor_world_pos: Vec2::ZERO,
        }
    }

    fn update_cursor(&mut self, cursor_pos: Vec2<f64>) {
        self.cursor_pos = cursor_pos;
        self.cursor_world_pos = self
            .camera
            .screen_to_world(
                self.framebuffer_size.map(|x| x as f32),
                cursor_pos.map(|x| x as f32),
            )
            .map(Coord::new);
    }

    fn click(&mut self, position: Vec2<f64>, _button: geng::MouseButton) {
        self.update_cursor(position);
    }

    fn release(&mut self, _button: geng::MouseButton) {}
}

impl geng::State for Editor {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.framebuffer_size = framebuffer.size();
        ugli::clear(framebuffer, Some(Rgba::BLACK), None, None);

        if self.draw_grid {
            self.render.draw_grid(&self.grid, &self.camera, framebuffer);
        }
    }

    fn handle_event(&mut self, event: geng::Event) {
        match event {
            geng::Event::MouseDown { position, button } => {
                self.click(position, button);
            }
            geng::Event::MouseMove { position, .. } => {
                self.update_cursor(position);
            }
            geng::Event::MouseUp { position, button } => {
                self.update_cursor(position);
                self.release(button);
            }
            _ => {}
        }
    }

    fn ui<'a>(&'a mut self, cx: &'a geng::ui::Controller) -> Box<dyn geng::ui::Widget + 'a> {
        use geng::ui::*;

        let framebuffer_size = self.framebuffer_size.map(|x| x as f32);

        let (cell_pos, cell_offset) = self.grid.world_to_grid(self.cursor_world_pos);
        let ui = geng::ui::column![Text::new(
            format!(
                "({}, {}) + ({:.1}, {:.1})",
                cell_pos.x, cell_pos.y, cell_offset.x, cell_offset.y
            ),
            self.geng.default_font(),
            framebuffer_size.y * 0.05,
            Rgba::WHITE
        )]
        .align(vec2(1.0, 1.0));

        Box::new(ui)
    }
}

pub fn run(geng: &Geng) -> impl geng::State {
    let future = {
        let geng = geng.clone();
        async move {
            let assets: Rc<Assets> = geng::LoadAsset::load(&geng, &run_dir().join("assets"))
                .await
                .expect("Failed to load assets");
            Editor::new(&geng, &assets)
        }
    };
    geng::LoadingScreen::new(geng, geng::EmptyLoadingScreen, future, |state| state)
}

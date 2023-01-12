use geng::Camera2d;

use super::*;

pub struct Editor {
    geng: Geng,
    assets: Rc<Assets>,
    render: Render,
    camera: Camera2d,
    framebuffer_size: Vec2<usize>,
    level: Level,
    draw_grid: bool,
    cursor_pos: Vec2<f64>,
    cursor_world_pos: Vec2<Coord>,
    selected_tile: Tile,
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
                fov: 22.5,
            },
            framebuffer_size: vec2(1, 1),
            level: Level::default(),
            draw_grid: true,
            cursor_pos: Vec2::ZERO,
            cursor_world_pos: Vec2::ZERO,
            selected_tile: Tile::Grass,
        }
    }

    fn place_tile(&mut self, pos: Vec2<Coord>, tile: Tile) {
        let pos = self.level.grid.world_to_grid(pos).0;
        self.level.tiles.set_tile_isize(pos, tile);
    }

    fn scroll_selected_tile(&mut self, delta: isize) {
        let current = self.selected_tile as isize;
        let target = current + delta;
        let all_tiles = Tile::all();
        let tile = target.rem_euclid(all_tiles.len() as isize) as usize;
        self.selected_tile = all_tiles[tile];
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

    fn click(&mut self, position: Vec2<f64>, button: geng::MouseButton) {
        self.update_cursor(position);

        if let geng::MouseButton::Left = button {
            self.place_tile(self.cursor_world_pos, self.selected_tile);
        }
    }

    fn release(&mut self, _button: geng::MouseButton) {}
}

impl geng::State for Editor {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.framebuffer_size = framebuffer.size();
        ugli::clear(framebuffer, Some(Rgba::BLACK), None, None);

        self.render
            .draw_level(&self.level, &self.camera, framebuffer);

        if self.draw_grid {
            self.render
                .draw_grid(&self.level.grid, &self.camera, framebuffer);
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
            geng::Event::Wheel { delta } => {
                self.scroll_selected_tile(delta.signum() as isize);
            }
            _ => {}
        }
    }

    fn ui<'a>(&'a mut self, cx: &'a geng::ui::Controller) -> Box<dyn geng::ui::Widget + 'a> {
        use geng::ui::*;

        let framebuffer_size = self.framebuffer_size.map(|x| x as f32);

        let (cell_pos, cell_offset) = self.level.grid.world_to_grid(self.cursor_world_pos);
        let cell_pos = Text::new(
            format!(
                "({}, {}) + ({:.1}, {:.1})",
                cell_pos.x, cell_pos.y, cell_offset.x, cell_offset.y
            ),
            self.geng.default_font(),
            framebuffer_size.y * 0.05,
            Rgba::WHITE,
        );

        let texture = self.assets.sprites.tiles.get_texture(&self.selected_tile);
        let selected_tile = ui::TextureBox::new(texture).fixed_size(
            vec2(framebuffer_size.y * 0.05, framebuffer_size.y * 0.05).map(|x| x as f64),
        );

        let ui = geng::ui::stack![
            cell_pos.align(vec2(1.0, 1.0)),
            selected_tile
                .align(vec2(1.0, 0.0))
                .uniform_padding(framebuffer_size.y as f64 * 0.05)
        ];

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

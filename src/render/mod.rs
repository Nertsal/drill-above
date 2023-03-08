use super::*;

mod cache;
mod dialogue;
mod lights;
mod util;
mod world;

pub use cache::*;
pub use dialogue::*;
pub use lights::*;
pub use util::*;
pub use world::*;

pub struct GameRender {
    geng: Geng,
    assets: Rc<Assets>,
    lights: LightsRender,
    world: WorldRender,
}

impl GameRender {
    pub fn new(geng: &Geng, assets: &Rc<Assets>) -> Self {
        Self {
            geng: geng.clone(),
            assets: assets.clone(),
            lights: LightsRender::new(geng, assets),
            world: WorldRender::new(geng, assets),
        }
    }

    pub fn draw_world(
        &mut self,
        world: &World,
        draw_hitboxes: bool,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        // Draw the world and normals ignoring lighting
        let (mut world_framebuffer, mut normal_framebuffer) = self.lights.start_render(framebuffer);

        // Render world
        self.world.draw_world(
            world,
            draw_hitboxes,
            &mut world_framebuffer,
            Some(&mut normal_framebuffer),
        );

        self.lights
            .finish_render(&world.room, &world.cache, &world.camera, framebuffer);
    }

    pub fn draw_ui(
        &self,
        show_time: Option<Time>,
        world: &World,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        let framebuffer_size = framebuffer.size().map(|x| x as f32);

        // Coins collected
        let texture = &self.assets.sprites.coin;
        let size = framebuffer_size.y * 0.07;
        let size = texture
            .size()
            .map(|x| x as f32 / texture.size().x as f32 * size);
        let pos = vec2(0.05, 0.95) * framebuffer_size;
        self.geng.draw_2d(
            framebuffer,
            &geng::PixelPerfectCamera,
            &draw_2d::TexturedQuad::new(
                Aabb2::point(pos).extend_right(size.x).extend_down(size.y),
                texture,
            ),
        );
        self.geng.draw_2d(
            framebuffer,
            &geng::PixelPerfectCamera,
            &draw_2d::Text::unit(
                &*self.assets.font,
                format!("{}", world.coins_collected),
                Rgba::try_from("#e3a912").unwrap(),
            )
            .scale_uniform(size.y * 0.3)
            .align_bounding_box(vec2(0.0, 0.5))
            .translate(pos + vec2(size.x * 1.5, -size.y / 2.0)),
        );

        if let Some(dialogue) = &world.dialogue {
            self.draw_dialogue(dialogue, framebuffer);
        }

        if let Some(time) = show_time {
            // Speedrun timer
            let pos = framebuffer_size * vec2(0.77, 0.95);
            let size = framebuffer_size.x * 0.01;
            let (m, s, ms) = time_ms(time);
            self.geng.draw_2d(
                framebuffer,
                &geng::PixelPerfectCamera,
                &draw_2d::Text::unit(
                    &*self.assets.font,
                    format!("{:02}:{:02}.{:03}", m, s, ms.floor()),
                    Rgba::WHITE,
                )
                .scale_uniform(size)
                .align_bounding_box(vec2(0.0, 1.0))
                .translate(pos),
            );
            let (m, s, ms) = time_ms(world.time);
            self.geng.draw_2d(
                framebuffer,
                &geng::PixelPerfectCamera,
                &draw_2d::Text::unit(
                    &*self.assets.font,
                    format!("{:02}:{:02}.{:03}", m, s, ms.floor()),
                    Rgba::WHITE,
                )
                .scale_uniform(size * 0.7)
                .align_bounding_box(vec2(0.0, 1.0))
                .translate(pos - vec2(0.0, size * 2.5)),
            );
        }
    }
}

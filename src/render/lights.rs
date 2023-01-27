use super::*;

pub struct LightsRender {
    geng: Geng,
    assets: Rc<Assets>,
    pub buffers: Buffers,
}

/// All buffers used in the lighting pipeline.
pub struct Buffers {
    /// Current size of all the buffers.
    framebuffer_size: vec2<usize>,
    /// Constant unit quad geometry.
    quad_geometry: ugli::VertexBuffer<draw_2d::Vertex>,
    /// Texture for the render of the world ignoring lighting.
    pub world_texture: ugli::Texture,
    /// Texture of normal vectors.
    pub normal_texture: ugli::Texture,
    /// Stencil buffer to count shadow casters.
    shadow_stencil: ugli::Renderbuffer<ugli::DepthStencilValue>,
    /// Final texture available for postprocessing.
    pub postprocess_texture: ugli::Texture,
}

impl LightsRender {
    pub fn new(geng: &Geng, assets: &Rc<Assets>) -> Self {
        Self {
            geng: geng.clone(),
            assets: assets.clone(),
            buffers: Buffers::new(geng),
        }
    }

    pub fn start_render(
        &mut self,
        framebuffer: &mut ugli::Framebuffer,
    ) -> (ugli::Framebuffer, ugli::Framebuffer) {
        self.buffers.update(framebuffer.size(), &self.geng);

        let mut world_framebuffer = attach_texture(&mut self.buffers.world_texture, &self.geng);
        let mut normal_framebuffer = attach_texture(&mut self.buffers.normal_texture, &self.geng);
        ugli::clear(&mut world_framebuffer, Some(Rgba::BLACK), None, None);
        ugli::clear(&mut normal_framebuffer, Some(Rgba::BLACK), None, None);

        (world_framebuffer, normal_framebuffer)
    }

    pub fn finish_render(
        &mut self,
        level: &Level,
        camera: &Camera2d,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        // Calculate geometry
        let geometry = self.calculate_light_geometry(level);

        // Render normal map
        self.render_normal_map(camera, &geometry);

        // Render lights
        self.render_lights(level, camera, &geometry);

        // Draw the texture to the screen
        self.geng.draw_2d(
            framebuffer,
            &geng::PixelPerfectCamera,
            &draw_2d::TexturedQuad::new(
                Aabb2::ZERO.extend_positive(framebuffer.size().map(|x| x as f32)),
                &self.buffers.postprocess_texture,
            ),
        );
    }

    /// Renders the world for each light separately onto the postprocessing texture.
    pub fn render_lights(&mut self, level: &Level, camera: &Camera2d, geometry: &[StaticPolygon]) {
        self.render_global_light(level);
        self.render_spotlights(level, camera, geometry);
    }

    /// Renders the world for the global light onto the postprocessing texture.
    /// Should be called as the first light render as it clears the texture.
    pub fn render_global_light(&mut self, level: &Level) {
        let mut world_framebuffer =
            attach_texture(&mut self.buffers.postprocess_texture, &self.geng);
        let framebuffer_size = world_framebuffer.size();
        // Clear the framebuffer assuming the global light is rendered first.
        ugli::clear(&mut world_framebuffer, Some(Rgba::BLACK), None, Some(0));

        ugli::draw(
            &mut world_framebuffer,
            &self.assets.shaders.global_light,
            ugli::DrawMode::TriangleFan,
            &self.buffers.quad_geometry,
            ugli::uniforms! {
                u_framebuffer_size: framebuffer_size,
                u_source_texture: &self.buffers.world_texture,
                u_light_color: level.global_light.color,
                u_light_intensity: level.global_light.intensity,
            },
            ugli::DrawParameters {
                blend_mode: Some(ugli::BlendMode::combined(ugli::ChannelBlendMode {
                    src_factor: ugli::BlendFactor::One,
                    dst_factor: ugli::BlendFactor::One,
                })),
                ..Default::default()
            },
        );
    }

    pub fn render_spotlights(
        &mut self,
        level: &Level,
        camera: &Camera2d,
        geometry: &[StaticPolygon],
    ) {
        for spotlight in &level.spotlights {
            // Using `world_texture` here but it is not actually used by the shader
            let mut light_framebuffer = ugli::Framebuffer::new(
                self.geng.ugli(),
                ugli::ColorAttachment::Texture(&mut self.buffers.world_texture),
                ugli::DepthAttachment::RenderbufferWithStencil(&mut self.buffers.shadow_stencil),
            );
            let framebuffer_size = light_framebuffer.size().map(|x| x as f32);
            ugli::clear(&mut light_framebuffer, Some(Rgba::BLACK), None, Some(0));

            // Shadow map
            for polygon in geometry {
                // Cast shadow
                ugli::draw(
                    &mut light_framebuffer,
                    &self.assets.shaders.point_light_shadow_map,
                    ugli::DrawMode::TriangleFan,
                    &polygon.doubled,
                    (
                        ugli::uniforms! {
                            u_model_matrix: mat3::identity(),
                            u_light_pos: spotlight.position.map(Coord::as_f32),
                        },
                        geng::camera2d_uniforms(camera, framebuffer_size),
                    ),
                    ugli::DrawParameters {
                        // Just in case the shader writes something in the texture,
                        // discard it during blending.
                        blend_mode: Some(ugli::BlendMode::combined(ugli::ChannelBlendMode {
                            src_factor: ugli::BlendFactor::Zero,
                            dst_factor: ugli::BlendFactor::One,
                        })),
                        // Increment the shadow casters count
                        stencil_mode: Some(ugli::StencilMode::always(ugli::FaceStencilMode {
                            test: ugli::StencilTest {
                                condition: ugli::Condition::Always,
                                reference: 0,
                                mask: 0xFF,
                            },
                            op: ugli::StencilOp::always(ugli::StencilOpFunc::Increment),
                        })),
                        ..Default::default()
                    },
                );
                // Remove self-shadow
                ugli::draw(
                    &mut light_framebuffer,
                    &self.assets.shaders.solid,
                    ugli::DrawMode::TriangleFan,
                    &polygon.vertices,
                    (
                        ugli::uniforms! {
                            u_model_matrix: mat3::identity(),
                            u_color: Rgba::TRANSPARENT_BLACK,
                        },
                        geng::camera2d_uniforms(camera, framebuffer_size),
                    ),
                    ugli::DrawParameters {
                        // Decrement the shadow casters count
                        stencil_mode: Some(ugli::StencilMode::always(ugli::FaceStencilMode {
                            test: ugli::StencilTest {
                                condition: ugli::Condition::Always,
                                reference: 0,
                                mask: 0xFF,
                            },
                            op: ugli::StencilOp::always(ugli::StencilOpFunc::Decrement),
                        })),
                        ..Default::default()
                    },
                );
            }

            // Render the world for that light
            let mut world_framebuffer = ugli::Framebuffer::new(
                self.geng.ugli(),
                ugli::ColorAttachment::Texture(&mut self.buffers.postprocess_texture),
                ugli::DepthAttachment::RenderbufferWithStencil(&mut self.buffers.shadow_stencil),
            );
            let framebuffer_size = world_framebuffer.size().map(|x| x as f32);
            ugli::draw(
                &mut world_framebuffer,
                &self.assets.shaders.spotlight,
                ugli::DrawMode::TriangleFan,
                &self.buffers.quad_geometry,
                (
                    ugli::uniforms! {
                        u_model_matrix: mat3::identity(),
                        u_light_pos: spotlight.position.map(Coord::as_f32),
                        u_light_angle: spotlight.angle,
                        u_light_angle_range: spotlight.angle_range,
                        u_light_color: spotlight.color,
                        u_light_intensity: spotlight.intensity,
                        u_light_max_distance: spotlight.max_distance.as_f32(),
                        u_light_volume: spotlight.volume,
                        u_normal_texture: &self.buffers.normal_texture,
                        u_source_texture: &self.buffers.world_texture,
                        u_framebuffer_size: self.buffers.normal_texture.size(),
                    },
                    geng::camera2d_uniforms(camera, framebuffer_size),
                ),
                ugli::DrawParameters {
                    blend_mode: Some(ugli::BlendMode::combined(ugli::ChannelBlendMode {
                        src_factor: ugli::BlendFactor::One,
                        dst_factor: ugli::BlendFactor::One,
                    })),
                    // Ignore the parts in shadow
                    stencil_mode: Some(ugli::StencilMode::always(ugli::FaceStencilMode {
                        test: ugli::StencilTest {
                            condition: ugli::Condition::Equal,
                            reference: 0, // 0 shadow casters means the point is lit up.
                            mask: 0xFF,
                        },
                        op: ugli::StencilOp::always(ugli::StencilOpFunc::Keep),
                    })),
                    ..Default::default()
                },
            );
        }
    }

    pub fn render_normal_map(&mut self, camera: &Camera2d, geometry: &[StaticPolygon]) {
        let mut normal_framebuffer = attach_texture(&mut self.buffers.normal_texture, &self.geng);
        let framebuffer_size = normal_framebuffer.size().map(|x| x as f32);

        for polygon in geometry {
            // Render the polygon's normal map
            ugli::draw(
                &mut normal_framebuffer,
                &self.assets.shaders.normal_map,
                ugli::DrawMode::TriangleFan,
                &polygon.vertices,
                (
                    ugli::uniforms! {
                        u_model_matrix: mat3::identity(),
                        u_normal_influence: 1.0,
                    },
                    geng::camera2d_uniforms(camera, framebuffer_size),
                ),
                ugli::DrawParameters {
                    blend_mode: Some(ugli::BlendMode::default()),
                    ..Default::default()
                },
            );
        }
    }

    pub fn calculate_light_geometry(&self, level: &Level) -> Vec<StaticPolygon> {
        itertools::chain![level
            .tiles
            .tiles()
            .iter()
            .enumerate()
            .filter_map(|(i, tile)| {
                (!matches!(tile, Tile::Air)).then(|| {
                    let pos = index_to_pos(i, level.size.x);
                    let pos = level.grid.grid_to_world(pos.map(|x| x as isize));
                    let pos = Aabb2::point(pos)
                        .extend_positive(level.grid.cell_size)
                        .map(Coord::as_f32);
                    let matrix = mat3::translate(pos.bottom_left()) * mat3::scale(pos.size());
                    StaticPolygon::new(
                        &self.geng,
                        &[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]
                            .map(|(x, y)| (matrix * vec2(x, y).extend(1.0)).into_2d()),
                    )
                })
            })]
        .collect()
    }
}

impl Buffers {
    pub fn new(geng: &Geng) -> Self {
        Self {
            framebuffer_size: vec2(1, 1),
            world_texture: new_texture(geng),
            normal_texture: new_texture(geng),
            shadow_stencil: ugli::Renderbuffer::new(geng.ugli(), vec2(1, 1)),
            postprocess_texture: new_texture(geng),
            quad_geometry: ugli::VertexBuffer::new_static(
                geng.ugli(),
                vec![
                    draw_2d::Vertex {
                        a_pos: vec2(-1.0, -1.0),
                    },
                    draw_2d::Vertex {
                        a_pos: vec2(1.0, -1.0),
                    },
                    draw_2d::Vertex {
                        a_pos: vec2(1.0, 1.0),
                    },
                    draw_2d::Vertex {
                        a_pos: vec2(-1.0, 1.0),
                    },
                ],
            ),
        }
    }

    pub fn update(&mut self, framebuffer_size: vec2<usize>, geng: &Geng) {
        if self.framebuffer_size != framebuffer_size {
            // Framebuffer size has changed -> update textures
            for texture in [
                &mut self.world_texture,
                &mut self.normal_texture,
                &mut self.postprocess_texture,
            ] {
                if texture.size() != framebuffer_size {
                    *texture =
                        ugli::Texture::new_with(geng.ugli(), framebuffer_size, |_| Rgba::BLACK);
                }
            }
            self.shadow_stencil = ugli::Renderbuffer::new(geng.ugli(), framebuffer_size);

            self.framebuffer_size = framebuffer_size;
        }
    }
}
